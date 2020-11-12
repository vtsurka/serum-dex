use crate::entity::{with_entity, EntityContext};
use crate::pool::{pool_check_create, Pool, PoolConfig};
use serum_common::pack::Pack;
use serum_registry::access_control;
use serum_registry::accounts::{vault, Entity, Member, Registrar};
use serum_registry::error::{RegistryError, RegistryErrorCode};
use solana_program::info;
use solana_sdk::account_info::{next_account_info, AccountInfo};
use solana_sdk::program_option::COption;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::sysvar::clock::Clock;
use spl_token::instruction as token_instruction;
use spl_token::state::Account as TokenAccount;

#[inline(never)]
pub fn handler(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    spt_amount: u64,
) -> Result<(), RegistryError> {
    info!("handler: stake");

    let acc_infos = &mut accounts.iter();

    let member_acc_info = next_account_info(acc_infos)?;
    let beneficiary_acc_info = next_account_info(acc_infos)?;
    let entity_acc_info = next_account_info(acc_infos)?;
    let registrar_acc_info = next_account_info(acc_infos)?;
    let clock_acc_info = next_account_info(acc_infos)?;
    let token_program_acc_info = next_account_info(acc_infos)?;

    let ref pool = Pool::parse_accounts(
        acc_infos,
        PoolConfig::Execute {
            registrar_acc_info,
            token_program_acc_info,
            is_create: true,
        },
    )?;

    let ctx = EntityContext {
        entity_acc_info,
        registrar_acc_info,
        clock_acc_info,
        program_id,
        prices: pool.prices(),
    };
    with_entity(ctx, &mut |entity: &mut Entity,
                           registrar: &Registrar,
                           _: &Clock| {
        let AccessControlResponse { pool_token } = access_control(AccessControlRequest {
            member_acc_info,
            registrar_acc_info,
            beneficiary_acc_info,
            entity_acc_info,
            spt_amount,
            entity,
            program_id,
            registrar,
            pool,
        })?;
        Member::unpack_mut(
            &mut member_acc_info.try_borrow_mut_data()?,
            &mut |member: &mut Member| {
                state_transition(StateTransitionRequest {
                    beneficiary_acc_info,
                    token_program_acc_info,
                    registrar_acc_info,
                    registrar,
                    pool_token,
                    entity,
                    member,
                    spt_amount,
                    pool,
                })
                .map_err(Into::into)
            },
        )
        .map_err(Into::into)
    })
}

fn access_control(req: AccessControlRequest) -> Result<AccessControlResponse, RegistryError> {
    info!("access-control: stake");

    let AccessControlRequest {
        member_acc_info,
        beneficiary_acc_info,
        entity_acc_info,
        registrar_acc_info,
        registrar,
        spt_amount,
        entity,
        program_id,
        pool,
    } = req;

    // Beneficiary authorization.
    if !beneficiary_acc_info.is_signer {
        return Err(RegistryErrorCode::Unauthorized)?;
    }

    // Account validation.
    access_control::entity_check(entity, entity_acc_info, registrar_acc_info, program_id)?;
    let member = access_control::member_join(
        member_acc_info,
        entity_acc_info,
        beneficiary_acc_info,
        program_id,
    )?;
    let pool_token = pool_check_create(program_id, pool, registrar_acc_info, registrar, &member)?;

    // Stake specific.
    {
        // Can the member afford the staking tokens?
        if !member.can_afford(pool.prices(), spt_amount, pool.is_mega())? {
            return Err(RegistryErrorCode::InsufficientStakeIntentBalance)?;
        }
        // All stake from a previous generation must be withdrawn before adding
        // stake for a new generation.
        if member.generation != entity.generation {
            if !member.stake_is_empty() {
                return Err(RegistryErrorCode::StaleStakeNeedsWithdrawal)?;
            }
        }
        // Only activated nodes can stake.
        if !entity.meets_activation_requirements(pool.prices(), &registrar) {
            return Err(RegistryErrorCode::EntityNotActivated)?;
        }

        // Will this new stake put the entity over the maximum allowable limit?
        let spt_worth = pool.prices().srm_equivalent(spt_amount, pool.is_mega());
        if spt_worth + entity.amount_equivalent(pool.prices()) > registrar.max_stake_per_entity {
            return Err(RegistryErrorCode::EntityMaxStake)?;
        }
    }

    Ok(AccessControlResponse { pool_token })
}

#[inline(always)]
fn state_transition(req: StateTransitionRequest) -> Result<(), RegistryError> {
    info!("state-transition: stake");

    let StateTransitionRequest {
        beneficiary_acc_info,
        token_program_acc_info,
        registrar_acc_info,
        registrar,
        pool_token,
        entity,
        member,
        spt_amount,
        pool,
    } = req;

    // Approve the beneficiary as delegate on the staking token, if not already.
    if pool_token.delegate == COption::None {
        approve_delegate(
            beneficiary_acc_info,
            token_program_acc_info,
            registrar_acc_info,
            registrar,
            pool,
        )?;
    }

    // Transfer funds into the staking pool, minting to the staking token.
    pool.create(spt_amount)?;

    // Update accounts for bookeeping.
    member.generation = entity.generation;
    member.spt_did_create(pool.prices(), spt_amount, pool.is_mega())?;
    entity.spt_did_create(pool.prices(), spt_amount, pool.is_mega())?;

    Ok(())
}

#[inline(always)]
fn approve_delegate<'a, 'b, 'c>(
    beneficiary_acc_info: &'a AccountInfo<'b>,
    token_program_acc_info: &'a AccountInfo<'b>,
    registrar_acc_info: &'a AccountInfo<'b>,
    registrar: &'c Registrar,
    pool: &'c Pool<'a, 'b>,
) -> Result<(), RegistryError> {
    let approve_instr = token_instruction::approve(
        &spl_token::ID,
        pool.pool_token_acc_info.unwrap().key,
        &beneficiary_acc_info.key,
        pool.registry_signer_acc_info.unwrap().key,
        &[],
        0,
    )?;
    solana_sdk::program::invoke_signed(
        &approve_instr,
        &[
            pool.pool_token_acc_info.unwrap().clone(),
            beneficiary_acc_info.clone(),
            pool.registry_signer_acc_info.unwrap().clone(),
            token_program_acc_info.clone(),
        ],
        &[&vault::signer_seeds(
            registrar_acc_info.key,
            &registrar.nonce,
        )],
    )?;

    Ok(())
}

struct AccessControlRequest<'a, 'b, 'c> {
    member_acc_info: &'a AccountInfo<'b>,
    beneficiary_acc_info: &'a AccountInfo<'b>,
    entity_acc_info: &'a AccountInfo<'b>,
    registrar_acc_info: &'a AccountInfo<'b>,
    program_id: &'a Pubkey,
    registrar: &'c Registrar,
    pool: &'c Pool<'a, 'b>,
    entity: &'c Entity,
    spt_amount: u64,
}

struct AccessControlResponse {
    pool_token: TokenAccount,
}

struct StateTransitionRequest<'a, 'b, 'c> {
    registrar_acc_info: &'a AccountInfo<'b>,
    beneficiary_acc_info: &'a AccountInfo<'b>,
    token_program_acc_info: &'a AccountInfo<'b>,
    registrar: &'c Registrar,
    pool: &'c Pool<'a, 'b>,
    pool_token: TokenAccount,
    entity: &'c mut Entity,
    member: &'c mut Member,
    spt_amount: u64,
}
