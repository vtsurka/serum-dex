#![cfg_attr(feature = "strict", deny(warnings))]

use serum_common::pack::Pack;
use serum_registry::error::{RegistryError, RegistryErrorCode};
use serum_registry::instruction::RegistryInstruction;
use solana_sdk::account_info::AccountInfo;
use solana_sdk::entrypoint::ProgramResult;
use solana_sdk::pubkey::Pubkey;

mod assign_vesting;
mod common;
mod create_entity;
mod create_member;
mod deposit;
mod end_stake_withdrawal;
mod entity;
mod initialize;
mod mark_generation;
mod pool;
mod slash;
mod stake;
mod start_stake_withdrawal;
mod switch_entity;
mod update_entity;
mod update_member;
mod update_registrar;
mod withdraw;

solana_program::entrypoint!(entry);
fn entry(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let instruction: RegistryInstruction = RegistryInstruction::unpack(instruction_data)
        .map_err(|_| RegistryError::ErrorCode(RegistryErrorCode::WrongSerialization))?;

    let result = match instruction {
        RegistryInstruction::Initialize {
            authority,
            nonce,
            withdrawal_timelock,
            deactivation_timelock,
            reward_activation_threshold,
            max_stake_per_entity,
        } => initialize::handler(
            program_id,
            accounts,
            authority,
            nonce,
            withdrawal_timelock,
            deactivation_timelock,
            reward_activation_threshold,
            max_stake_per_entity,
        ),
        RegistryInstruction::UpdateRegistrar {
            new_authority,
            withdrawal_timelock,
            deactivation_timelock,
            reward_activation_threshold,
            max_stake_per_entity,
        } => update_registrar::handler(
            program_id,
            accounts,
            new_authority,
            withdrawal_timelock,
            deactivation_timelock,
            reward_activation_threshold,
            max_stake_per_entity,
        ),
        RegistryInstruction::CreateEntity => create_entity::handler(program_id, accounts),
        RegistryInstruction::UpdateEntity { leader } => {
            update_entity::handler(program_id, accounts, leader)
        }
        RegistryInstruction::CreateMember { delegate, nonce } => {
            create_member::handler(program_id, accounts, delegate, nonce)
        }
        RegistryInstruction::UpdateMember { delegate } => {
            update_member::handler(program_id, accounts, delegate)
        }
        RegistryInstruction::SwitchEntity => switch_entity::handler(program_id, accounts),
        RegistryInstruction::Deposit { amount } => deposit::handler(program_id, accounts, amount),
        RegistryInstruction::Withdraw { amount } => withdraw::handler(program_id, accounts, amount),
        RegistryInstruction::Stake { amount } => stake::handler(program_id, accounts, amount),
        RegistryInstruction::MarkGeneration => mark_generation::handler(program_id, accounts),
        RegistryInstruction::StartStakeWithdrawal { amount } => {
            start_stake_withdrawal::handler(program_id, accounts, amount)
        }
        RegistryInstruction::EndStakeWithdrawal => {
            end_stake_withdrawal::handler(program_id, accounts)
        }
        RegistryInstruction::AssignVesting => assign_vesting::handler(program_id, accounts),
        RegistryInstruction::Slash { amount } => slash::handler(program_id, accounts, amount),
    };

    result?;

    Ok(())
}
