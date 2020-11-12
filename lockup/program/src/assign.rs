use crate::access_control;
use serum_common::pack::Pack;
use serum_lockup::accounts::Vesting;
use serum_lockup::error::{LockupError, LockupErrorCode};
use solana_program::info;
use solana_sdk::account_info::{next_account_info, AccountInfo};
use solana_sdk::pubkey::Pubkey;
use std::convert::Into;

pub fn handler(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    beneficiary: Pubkey,
) -> Result<(), LockupError> {
    info!("handler: claim");

    let acc_infos = &mut accounts.iter();

    let vesting_acc_info = next_account_info(acc_infos)?;
    let safe_acc_info = next_account_info(acc_infos)?;
    let needs_assignment_authority_acc_info = next_account_info(acc_infos)?;

    access_control(AccessControlRequest {
        program_id,
        safe_acc_info,
        vesting_acc_info,
        needs_assignment_authority_acc_info,
        beneficiary,
    })?;

    Vesting::unpack_mut(
        &mut vesting_acc_info.try_borrow_mut_data()?,
        &mut |vesting: &mut Vesting| {
            state_transition(StateTransitionRequest {
                vesting,
                beneficiary,
            })
            .map_err(Into::into)
        },
    )?;

    Ok(())
}

fn access_control(req: AccessControlRequest) -> Result<(), LockupError> {
    info!("access-control: claim");

    let AccessControlRequest {
        program_id,
        safe_acc_info,
        vesting_acc_info,
        needs_assignment_authority_acc_info,
        beneficiary,
    } = req;

    // Needs assignment authority authorization.
    if !needs_assignment_authority_acc_info.is_signer {
        return Err(LockupErrorCode::Unauthorized)?;
    }

    // Account validation.
    let _safe = access_control::safe(safe_acc_info, program_id)?;
    let vesting = access_control::vesting_raw(program_id, safe_acc_info.key, vesting_acc_info)?;

    // Assign specific.
    match vesting.needs_assignment {
        None => return Err(LockupErrorCode::AlreadyAssigned)?,
        Some(needs_assignment) => {
            // Check program derived address.
            if needs_assignment.authority != *needs_assignment_authority_acc_info.key {
                return Err(LockupErrorCode::AssignmentAuthMismatch)?;
            }
            // Auhtenticate the given beneficiary.
            let seeds: &[&[u8]] = &[
                needs_assignment.identifier.as_ref(),
                beneficiary.as_ref(),
                &[needs_assignment.nonce],
            ];
            let program_derived_address =
                Pubkey::create_program_address(seeds, &needs_assignment.program_id)
                    .map_err(|_| LockupErrorCode::InvalidAccount)?;
            if needs_assignment.authority != program_derived_address {
                return Err(LockupErrorCode::AssignmentAuthMismatch)?;
            }
        }
    };

    Ok(())
}

fn state_transition(req: StateTransitionRequest) -> Result<(), LockupError> {
    info!("state-transition: claim");

    let StateTransitionRequest {
        vesting,
        beneficiary,
    } = req;

    vesting.needs_assignment = None;
    vesting.beneficiary = beneficiary;

    Ok(())
}

struct AccessControlRequest<'a, 'b> {
    needs_assignment_authority_acc_info: &'a AccountInfo<'b>,
    vesting_acc_info: &'a AccountInfo<'b>,
    safe_acc_info: &'a AccountInfo<'b>,
    program_id: &'a Pubkey,
    beneficiary: Pubkey,
}

struct StateTransitionRequest<'a> {
    vesting: &'a mut Vesting,
    beneficiary: Pubkey,
}
