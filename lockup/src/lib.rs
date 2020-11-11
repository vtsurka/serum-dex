#![cfg_attr(feature = "strict", deny(warnings))]
#![allow(dead_code)]

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use serum_common::pack::*;
use solana_client_gen::prelude::*;

pub mod accounts;
pub mod error;

#[cfg_attr(feature = "client", solana_client_gen)]
pub mod instruction {
    use super::*;
    #[derive(Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
    pub enum LockupInstruction {
        /// Accounts:
        ///
        /// 0. `[writable]` Safe.
        /// 1. `[writable]` Whitelist.
        /// 2. `[]`         Vault.
        /// 3. `[]`         Mint of the SPL token locked.
        /// 4. `[]`         Rent sysvar.
        Initialize {
            /// The priviledged account.
            authority: Pubkey,
            /// The nonce to use to create the Safe's derived-program address,
            /// which is used as the authority for the safe's token vault.
            nonce: u8,
        },
        /// Accounts:
        ///
        /// 0. `[writable]  Vesting.
        /// 1. `[writable]` Depositor token account.
        /// 2. `[signer]`   The authority||owner||delegate of Accounts[1].
        /// 3. `[writable]` Vault.
        /// 4. `[]`         Safe.
        /// 5. `[writable]` Token mint representing the lSRM receipt.
        /// 6. `[]`         Vault authority.
        /// 7. `[]`         SPL token program.
        /// 8. `[]`         Rent sysvar.
        /// 9. `[]`         Clock sysvar.
        CreateVesting {
            /// The beneficiary of the vesting account, i.e.,
            /// the user who will own the SRM upon vesting. Zero initialized
            /// if activated is Some.
            beneficiary: Pubkey,
            /// The unix timestamp at which point the entire deposit will
            /// be vested.
            end_ts: i64,
            /// The number of vesting periods for the account. For example,
            /// a vesting yearly over seven years would make this 7.
            period_count: u64,
            /// The amount to deposit into the vesting account.
            deposit_amount: u64,
            /// The NeedsAssignment option determining the program-derived
            /// address that can assign this vesting account to a beneficiary.
            /// If Some, the given `beneficiary` can be overriden, and so
            /// should be zero initialized.
            needs_assignment: Option<crate::accounts::NeedsAssignment>,
        },
        /// Accounts:
        ///
        /// 0. `[writable]` Vesting.
        /// 1. `[]`         Safe.
        /// 2. `[signer]`   Needs assignment authority matching the given
        ///                 Vesting account.
        Assign { beneficiary: Pubkey },
        /// Accounts:
        ///
        /// 0. `[signer]`   Vesting account beneficiary.
        /// 1. `[writable]` Vesting account.
        /// 2. `[]`         Safe instance.
        /// 3. `[]`         Safe's vault authority, a program derived address.
        /// 4. `[]`         SPL token program.
        /// 5. `[writable]` Token mint representing the lSRM receipt.
        /// 6  `[writable]` Token account associated with the mint.
        Claim,
        /// Accounts:
        ///
        /// 0. `[signer]`   Beneficiary.
        /// 1. `[writable]` Vesting.
        /// 2. `[writable]` SPL token account to withdraw to.
        /// 3. `[writable]` Vault.
        /// 4. `[]`         Vault authority.
        /// 5  `[]`         Safe.
        /// 6. `[writable]` NFT token being redeemed.
        /// 7. `[writable]` NFT mint to burn the token being redeemed.
        /// 8. `[]`         SPL token program.
        /// 9. `[]`         Clock sysvar.
        Redeem { amount: u64 },
        /// Accounts:
        ///
        /// 0. `[signer]`   Beneficiary.
        /// 1. `[writable]` Vesting.
        /// 2. `[]`         Safe.
        /// 3. `[]`         Vault authority.
        /// 4. `[]`         Whitelisted program to invoke.
        /// 5. `[]`         Whitelist.
        ///
        /// All accounts below will be relayed to the whitelisted program.
        ///
        /// 6.  `[]`         Vault authority.
        /// 7.  `[writable]` Vault.
        /// 8.  `[writable]` Whitelisted target vault which will receive funds.
        /// 9.  `[]`         Whitelisted vault authority.
        /// 10. `[]`         Token program id.
        /// ..  `[writable]` Variable number of program specific accounts to
        ///                  relay to the program.
        WhitelistWithdraw {
            /// Amount of funds the whitelisted program is approved to
            /// transfer to itself. Must be less than or equal to the vesting
            /// account's whitelistable balance.
            amount: u64,
            /// Opaque instruction data to relay to the whitelisted program.
            instruction_data: Vec<u8>,
        },
        /// Accounts:
        ///
        /// Same as WhitelistWithdraw.
        WhitelistDeposit { instruction_data: Vec<u8> },
        /// Accounts:
        ///
        /// 0. `[signed]`   Safe authority.
        /// 1. `[]`         Safe account.
        /// 2. `[writable]` Whitelist.
        WhitelistAdd {
            entry: crate::accounts::WhitelistEntry,
        },
        /// Accounts:
        ///
        /// 0. `[signed]`   Safe authority.
        /// 1. `[]`         Safe account.
        /// 2. `[writable]` Whitelist.
        WhitelistDelete {
            entry: crate::accounts::WhitelistEntry,
        },
        /// Accounts:
        ///
        /// 0. `[signer]`   Current safe authority.
        /// 1. `[writable]` Safe instance.
        SetAuthority { new_authority: Pubkey },
        /// Accounts:
        ///
        /// 0. `[signer]`   Safe's authority.
        /// 1  `[writable]` Safe account.
        /// 2. `[writable]` Safe's token vault from which we are transferring
        ///                 all tokens out of.
        /// 3. `[readonly]` Safe's vault authority, i.e., the program derived
        ///                 address.
        /// 4. `[writable]` Token account to receive the new tokens.
        /// 5. `[]`         SPL token program.
        Migrate,
    }
}

serum_common::packable!(instruction::LockupInstruction);
