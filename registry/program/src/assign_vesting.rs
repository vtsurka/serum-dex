use serum_common::pack::Pack;
use serum_lockup::instruction::LockupInstruction;
use serum_registry::access_control;
use serum_registry::accounts::Member;
use serum_registry::error::{RegistryError, RegistryErrorCode};
use solana_program::info;
use solana_sdk::account_info::{next_account_info, AccountInfo};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

pub fn handler(program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), RegistryError> {
    info!("handler: end_stake_withdrawl");

    let acc_infos = &mut accounts.iter();

    let member_acc_info = next_account_info(acc_infos)?;
    let entity_acc_info = next_account_info(acc_infos)?;
    let registrar_acc_info = next_account_info(acc_infos)?;
    let lockup_program_acc_info = next_account_info(acc_infos)?;
    let vesting_acc_info = next_account_info(acc_infos)?;
    let safe_acc_info = next_account_info(acc_infos)?;
    let member_signer_acc_info = next_account_info(acc_infos)?;

    let AccessControlResponse { member } = access_control(AccessControlRequest {
        registrar_acc_info,
        member_acc_info,
        entity_acc_info,
        program_id,
    })?;

    state_transition(StateTransitionRequest {
        member,
        vesting_acc_info,
        lockup_program_acc_info,
        safe_acc_info,
        member_signer_acc_info,
    })?;

    Ok(())
}

fn access_control(req: AccessControlRequest) -> Result<AccessControlResponse, RegistryError> {
    info!("access-control: end_stake_withdrawal");

    let AccessControlRequest {
        registrar_acc_info,
        member_acc_info,
        entity_acc_info,
        program_id,
    } = req;

    // Authorization: None.

    // Account validation.
    access_control::registrar(registrar_acc_info, program_id)?;
    access_control::entity(entity_acc_info, registrar_acc_info, program_id)?;
    let member = access_control::member_raw(member_acc_info, entity_acc_info, program_id)?;

    // EndStakeWithdrawal specific.
    if !member.balances.stake_is_empty() {
        return Err(RegistryErrorCode::StakeNotEmpty)?;
    }

    Ok(AccessControlResponse { member })
}

#[inline(always)]
fn state_transition(req: StateTransitionRequest) -> Result<(), RegistryError> {
    info!("state-transition: end_stake_withdrawal");

    let StateTransitionRequest {
        member,
        lockup_program_acc_info,
        vesting_acc_info,
        safe_acc_info,
        member_signer_acc_info,
    } = req;

    let accounts = vec![
        AccountMeta::new(*vesting_acc_info.key, false),
        AccountMeta::new_readonly(*safe_acc_info.key, false),
        AccountMeta::new_readonly(*member_signer_acc_info.key, true),
    ];
    let assign_instr = {
        let i = LockupInstruction::Assign {
            beneficiary: member.beneficiary,
        };
        let mut data = vec![0u8; i.size()? as usize];
        LockupInstruction::pack(i, &mut data)?;

        Instruction {
            program_id: *lockup_program_acc_info.key,
            accounts,
            data,
        }
    };
    let signer_seeds = &[member.beneficiary.as_ref(), &[member.nonce]];
    solana_sdk::program::invoke_signed(
        &assign_instr,
        &[
            vesting_acc_info.clone(),
            safe_acc_info.clone(),
            member_signer_acc_info.clone(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}

struct AccessControlRequest<'a, 'b> {
    registrar_acc_info: &'a AccountInfo<'b>,
    member_acc_info: &'a AccountInfo<'b>,
    entity_acc_info: &'a AccountInfo<'b>,
    program_id: &'a Pubkey,
}

struct AccessControlResponse {
    member: Member,
}

struct StateTransitionRequest<'a, 'b> {
    member: Member,
    lockup_program_acc_info: &'a AccountInfo<'b>,
    vesting_acc_info: &'a AccountInfo<'b>,
    safe_acc_info: &'a AccountInfo<'b>,
    member_signer_acc_info: &'a AccountInfo<'b>,
}
