#[deprecated(
    since = "1.8.0",
    note = "Please use `solana_sdk::stake::instruction` or `solana_program::stake::instruction` instead"
)]
pub use solana_sdk::stake::instruction::*;
use {
    crate::{config, stake_state::StakeAccount},
    log::*,
    solana_program_runtime::{
        invoke_context::InvokeContext, sysvar_cache::get_sysvar_with_account_check2,
    },
    solana_sdk::{
        feature_set,
        instruction::InstructionError,
        keyed_account::keyed_account_at_index,
        program_utils::limited_deserialize,
        stake::{
            instruction::StakeInstruction,
            program::id,
            state::{Authorized, Lockup},
        },
        sysvar::clock::Clock,
    },
};

pub mod instruction_account_indices {
    pub enum Initialize {
        StakeAccount = 0,
        Rent = 1,
    }

    pub enum Authorize {
        StakeAccount = 0,
        Clock = 1,
        // CurrentAuthority = 2,
        Custodian = 3,
    }

    pub enum AuthorizeWithSeed {
        StakeAccount = 0,
        AuthorityBase = 1,
        Clock = 2,
        Custodian = 3,
    }

    pub enum DelegateStake {
        StakeAccount = 0,
        VoteAccount = 1,
        Clock = 2,
        StakeHistory = 3,
        ConfigAccount = 4,
    }

    pub enum Split {
        StakeAccount = 0,
        SplitTo = 1,
    }

    pub enum Merge {
        StakeAccount = 0,
        MergeFrom = 1,
        Clock = 2,
        StakeHistory = 3,
    }

    pub enum Withdraw {
        StakeAccount = 0,
        Recipient = 1,
        Clock = 2,
        StakeHistory = 3,
        WithdrawAuthority = 4,
        Custodian = 5,
    }

    pub enum Deactivate {
        StakeAccount = 0,
        Clock = 1,
    }

    pub enum SetLockup {
        StakeAccount = 0,
        // Clock = 1,
    }

    pub enum InitializeChecked {
        StakeAccount = 0,
        Rent = 1,
        AuthorizedStaker = 2,
        AuthorizedWithdrawer = 3,
    }

    pub enum AuthorizeChecked {
        StakeAccount = 0,
        Clock = 1,
        // CurrentAuthority = 2,
        Authorized = 3,
        Custodian = 4,
    }

    pub enum AuthorizeCheckedWithSeed {
        StakeAccount = 0,
        AuthorityBase = 1,
        Clock = 2,
        Authorized = 3,
        Custodian = 4,
    }

    pub enum SetLockupChecked {
        StakeAccount = 0,
        // Clock = 1,
        Custodian = 2,
    }
}

pub fn process_instruction(
    first_instruction_account: usize,
    data: &[u8],
    invoke_context: &mut InvokeContext,
) -> Result<(), InstructionError> {
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let keyed_accounts = invoke_context.get_keyed_accounts()?;

    trace!("process_instruction: {:?}", data);

    let me = &keyed_account_at_index(keyed_accounts, first_instruction_account)?;
    if me.owner()? != id() {
        return Err(InstructionError::InvalidAccountOwner);
    }

    let signers = instruction_context.get_signers(transaction_context);
    match limited_deserialize(data)? {
        StakeInstruction::Initialize(authorized, lockup) => {
            let rent = get_sysvar_with_account_check2::rent(
                invoke_context,
                instruction_context,
                instruction_account_indices::Initialize::Rent as usize,
            )?;
            me.initialize(&authorized, &lockup, &rent)
        }
        StakeInstruction::Authorize(authorized_pubkey, stake_authorize) => {
            let require_custodian_for_locked_stake_authorize = invoke_context
                .feature_set
                .is_active(&feature_set::require_custodian_for_locked_stake_authorize::id());

            if require_custodian_for_locked_stake_authorize {
                let clock = get_sysvar_with_account_check2::clock(
                    invoke_context,
                    instruction_context,
                    instruction_account_indices::Authorize::Clock as usize,
                )?;
                let custodian = keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::Authorize::Custodian as usize,
                )
                .ok()
                .map(|ka| ka.unsigned_key());

                me.authorize(
                    &signers,
                    &authorized_pubkey,
                    stake_authorize,
                    require_custodian_for_locked_stake_authorize,
                    &clock,
                    custodian,
                )
            } else {
                me.authorize(
                    &signers,
                    &authorized_pubkey,
                    stake_authorize,
                    require_custodian_for_locked_stake_authorize,
                    &Clock::default(),
                    None,
                )
            }
        }
        StakeInstruction::AuthorizeWithSeed(args) => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let authority_base = keyed_account_at_index(
                keyed_accounts,
                first_instruction_account
                    + instruction_account_indices::AuthorizeWithSeed::AuthorityBase as usize,
            )?;
            let require_custodian_for_locked_stake_authorize = invoke_context
                .feature_set
                .is_active(&feature_set::require_custodian_for_locked_stake_authorize::id());

            if require_custodian_for_locked_stake_authorize {
                let clock = get_sysvar_with_account_check2::clock(
                    invoke_context,
                    instruction_context,
                    instruction_account_indices::AuthorizeWithSeed::Clock as usize,
                )?;
                let custodian = keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::AuthorizeWithSeed::Custodian as usize,
                )
                .ok()
                .map(|ka| ka.unsigned_key());

                me.authorize_with_seed(
                    authority_base,
                    &args.authority_seed,
                    &args.authority_owner,
                    &args.new_authorized_pubkey,
                    args.stake_authorize,
                    require_custodian_for_locked_stake_authorize,
                    &clock,
                    custodian,
                )
            } else {
                me.authorize_with_seed(
                    authority_base,
                    &args.authority_seed,
                    &args.authority_owner,
                    &args.new_authorized_pubkey,
                    args.stake_authorize,
                    require_custodian_for_locked_stake_authorize,
                    &Clock::default(),
                    None,
                )
            }
        }
        StakeInstruction::DelegateStake => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let vote = keyed_account_at_index(
                keyed_accounts,
                first_instruction_account
                    + instruction_account_indices::DelegateStake::VoteAccount as usize,
            )?;
            let clock = get_sysvar_with_account_check2::clock(
                invoke_context,
                instruction_context,
                instruction_account_indices::DelegateStake::Clock as usize,
            )?;
            let stake_history = get_sysvar_with_account_check2::stake_history(
                invoke_context,
                instruction_context,
                instruction_account_indices::DelegateStake::StakeHistory as usize,
            )?;
            instruction_context.check_number_of_instruction_accounts(5)?;
            let config_account = keyed_account_at_index(
                keyed_accounts,
                first_instruction_account
                    + instruction_account_indices::DelegateStake::ConfigAccount as usize,
            )?;
            if !config::check_id(config_account.unsigned_key()) {
                return Err(InstructionError::InvalidArgument);
            }
            let config = config::from(&*config_account.try_account_ref()?)
                .ok_or(InstructionError::InvalidArgument)?;
            me.delegate(vote, &clock, &stake_history, &config, &signers)
        }
        StakeInstruction::Split(lamports) => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let split_stake = &keyed_account_at_index(
                keyed_accounts,
                first_instruction_account + instruction_account_indices::Split::SplitTo as usize,
            )?;
            me.split(lamports, split_stake, &signers)
        }
        StakeInstruction::Merge => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let source_stake = &keyed_account_at_index(
                keyed_accounts,
                first_instruction_account + instruction_account_indices::Merge::MergeFrom as usize,
            )?;
            let clock = get_sysvar_with_account_check2::clock(
                invoke_context,
                instruction_context,
                instruction_account_indices::Merge::Clock as usize,
            )?;
            let stake_history = get_sysvar_with_account_check2::stake_history(
                invoke_context,
                instruction_context,
                instruction_account_indices::Merge::StakeHistory as usize,
            )?;
            me.merge(
                invoke_context,
                source_stake,
                &clock,
                &stake_history,
                &signers,
            )
        }
        StakeInstruction::Withdraw(lamports) => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let to = &keyed_account_at_index(
                keyed_accounts,
                first_instruction_account
                    + instruction_account_indices::Withdraw::Recipient as usize,
            )?;
            let clock = get_sysvar_with_account_check2::clock(
                invoke_context,
                instruction_context,
                instruction_account_indices::Withdraw::Clock as usize,
            )?;
            let stake_history = get_sysvar_with_account_check2::stake_history(
                invoke_context,
                instruction_context,
                instruction_account_indices::Withdraw::StakeHistory as usize,
            )?;
            instruction_context.check_number_of_instruction_accounts(5)?;
            me.withdraw(
                lamports,
                to,
                &clock,
                &stake_history,
                keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::Withdraw::WithdrawAuthority as usize,
                )?,
                keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::Withdraw::Custodian as usize,
                )
                .ok(),
            )
        }
        StakeInstruction::Deactivate => {
            let clock = get_sysvar_with_account_check2::clock(
                invoke_context,
                instruction_context,
                instruction_account_indices::Deactivate::Clock as usize,
            )?;
            me.deactivate(&clock, &signers)
        }
        StakeInstruction::SetLockup(lockup) => {
            let clock = invoke_context.get_sysvar_cache().get_clock()?;
            me.set_lockup(&lockup, &signers, &clock)
        }
        StakeInstruction::InitializeChecked => {
            if invoke_context
                .feature_set
                .is_active(&feature_set::vote_stake_checked_instructions::id())
            {
                instruction_context.check_number_of_instruction_accounts(4)?;
                let authorized = Authorized {
                    staker: *keyed_account_at_index(
                        keyed_accounts,
                        first_instruction_account
                            + instruction_account_indices::InitializeChecked::AuthorizedStaker
                                as usize,
                    )?
                    .unsigned_key(),
                    withdrawer: *keyed_account_at_index(
                        keyed_accounts,
                        first_instruction_account
                            + instruction_account_indices::InitializeChecked::AuthorizedWithdrawer
                                as usize,
                    )?
                    .signer_key()
                    .ok_or(InstructionError::MissingRequiredSignature)?,
                };

                let rent = get_sysvar_with_account_check2::rent(
                    invoke_context,
                    instruction_context,
                    instruction_account_indices::InitializeChecked::Rent as usize,
                )?;
                me.initialize(&authorized, &Lockup::default(), &rent)
            } else {
                Err(InstructionError::InvalidInstructionData)
            }
        }
        StakeInstruction::AuthorizeChecked(stake_authorize) => {
            if invoke_context
                .feature_set
                .is_active(&feature_set::vote_stake_checked_instructions::id())
            {
                let clock = get_sysvar_with_account_check2::clock(
                    invoke_context,
                    instruction_context,
                    instruction_account_indices::AuthorizeChecked::Clock as usize,
                )?;
                instruction_context.check_number_of_instruction_accounts(4)?;
                let authorized_pubkey = &keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::AuthorizeChecked::Authorized as usize,
                )?
                .signer_key()
                .ok_or(InstructionError::MissingRequiredSignature)?;
                let custodian = keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::AuthorizeChecked::Custodian as usize,
                )
                .ok()
                .map(|ka| ka.unsigned_key());

                me.authorize(
                    &signers,
                    authorized_pubkey,
                    stake_authorize,
                    true,
                    &clock,
                    custodian,
                )
            } else {
                Err(InstructionError::InvalidInstructionData)
            }
        }
        StakeInstruction::AuthorizeCheckedWithSeed(args) => {
            if invoke_context
                .feature_set
                .is_active(&feature_set::vote_stake_checked_instructions::id())
            {
                instruction_context.check_number_of_instruction_accounts(2)?;
                let authority_base = keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::AuthorizeCheckedWithSeed::AuthorityBase
                            as usize,
                )?;
                let clock = get_sysvar_with_account_check2::clock(
                    invoke_context,
                    instruction_context,
                    instruction_account_indices::AuthorizeCheckedWithSeed::Clock as usize,
                )?;
                instruction_context.check_number_of_instruction_accounts(4)?;
                let authorized_pubkey = &keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::AuthorizeCheckedWithSeed::Authorized
                            as usize,
                )?
                .signer_key()
                .ok_or(InstructionError::MissingRequiredSignature)?;
                let custodian = keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::AuthorizeCheckedWithSeed::Custodian as usize,
                )
                .ok()
                .map(|ka| ka.unsigned_key());

                me.authorize_with_seed(
                    authority_base,
                    &args.authority_seed,
                    &args.authority_owner,
                    authorized_pubkey,
                    args.stake_authorize,
                    true,
                    &clock,
                    custodian,
                )
            } else {
                Err(InstructionError::InvalidInstructionData)
            }
        }
        StakeInstruction::SetLockupChecked(lockup_checked) => {
            if invoke_context
                .feature_set
                .is_active(&feature_set::vote_stake_checked_instructions::id())
            {
                let custodian = if let Ok(custodian) = keyed_account_at_index(
                    keyed_accounts,
                    first_instruction_account
                        + instruction_account_indices::SetLockupChecked::Custodian as usize,
                ) {
                    Some(
                        *custodian
                            .signer_key()
                            .ok_or(InstructionError::MissingRequiredSignature)?,
                    )
                } else {
                    None
                };

                let lockup = LockupArgs {
                    unix_timestamp: lockup_checked.unix_timestamp,
                    epoch: lockup_checked.epoch,
                    custodian,
                };
                let clock = invoke_context.get_sysvar_cache().get_clock()?;
                me.set_lockup(&lockup, &signers, &clock)
            } else {
                Err(InstructionError::InvalidInstructionData)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::stake_state::{Meta, StakeState},
        bincode::serialize,
        solana_program_runtime::invoke_context::mock_process_instruction,
        solana_sdk::{
            account::{self, AccountSharedData},
            instruction::{AccountMeta, Instruction},
            pubkey::Pubkey,
            rent::Rent,
            stake::{
                config as stake_config,
                instruction::{self, LockupArgs},
                state::{Authorized, Lockup, StakeAuthorize},
            },
            sysvar::{self, stake_history::StakeHistory},
        },
        std::{collections::HashSet, str::FromStr},
    };

    fn create_default_account() -> AccountSharedData {
        AccountSharedData::new(0, 0, &Pubkey::new_unique())
    }

    fn create_default_stake_account() -> AccountSharedData {
        AccountSharedData::new(0, 0, &id())
    }

    fn invalid_stake_state_pubkey() -> Pubkey {
        Pubkey::from_str("BadStake11111111111111111111111111111111111").unwrap()
    }

    fn invalid_vote_state_pubkey() -> Pubkey {
        Pubkey::from_str("BadVote111111111111111111111111111111111111").unwrap()
    }

    fn spoofed_stake_state_pubkey() -> Pubkey {
        Pubkey::from_str("SpoofedStake1111111111111111111111111111111").unwrap()
    }

    fn spoofed_stake_program_id() -> Pubkey {
        Pubkey::from_str("Spoofed111111111111111111111111111111111111").unwrap()
    }

    fn process_instruction(
        instruction_data: &[u8],
        transaction_accounts: Vec<(Pubkey, AccountSharedData)>,
        instruction_accounts: Vec<AccountMeta>,
        expected_result: Result<(), InstructionError>,
    ) -> Vec<AccountSharedData> {
        mock_process_instruction(
            &id(),
            Vec::new(),
            instruction_data,
            transaction_accounts,
            instruction_accounts,
            expected_result,
            super::process_instruction,
        )
    }

    fn process_instruction_as_one_arg(
        instruction: &Instruction,
        expected_result: Result<(), InstructionError>,
    ) -> Vec<AccountSharedData> {
        let mut pubkeys: HashSet<Pubkey> = instruction
            .accounts
            .iter()
            .map(|meta| meta.pubkey)
            .collect();
        pubkeys.insert(sysvar::clock::id());
        let transaction_accounts = pubkeys
            .iter()
            .map(|pubkey| {
                (
                    *pubkey,
                    if sysvar::clock::check_id(pubkey) {
                        account::create_account_shared_data_for_test(
                            &sysvar::clock::Clock::default(),
                        )
                    } else if sysvar::rewards::check_id(pubkey) {
                        account::create_account_shared_data_for_test(
                            &sysvar::rewards::Rewards::new(0.0),
                        )
                    } else if sysvar::stake_history::check_id(pubkey) {
                        account::create_account_shared_data_for_test(&StakeHistory::default())
                    } else if stake_config::check_id(pubkey) {
                        config::create_account(0, &stake_config::Config::default())
                    } else if sysvar::rent::check_id(pubkey) {
                        account::create_account_shared_data_for_test(&Rent::default())
                    } else if *pubkey == invalid_stake_state_pubkey() {
                        AccountSharedData::new(0, 0, &id())
                    } else if *pubkey == invalid_vote_state_pubkey() {
                        AccountSharedData::new(0, 0, &solana_vote_program::id())
                    } else if *pubkey == spoofed_stake_state_pubkey() {
                        AccountSharedData::new(0, 0, &spoofed_stake_program_id())
                    } else {
                        AccountSharedData::new(0, 0, &id())
                    },
                )
            })
            .collect();
        process_instruction(
            &instruction.data,
            transaction_accounts,
            instruction.accounts.clone(),
            expected_result,
        )
    }

    #[test]
    fn test_stake_process_instruction() {
        process_instruction_as_one_arg(
            &instruction::initialize(
                &Pubkey::new_unique(),
                &Authorized::default(),
                &Lockup::default(),
            ),
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::authorize(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                StakeAuthorize::Staker,
                None,
            ),
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::split(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                100,
                &invalid_stake_state_pubkey(),
            )[2],
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::merge(
                &Pubkey::new_unique(),
                &invalid_stake_state_pubkey(),
                &Pubkey::new_unique(),
            )[0],
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::split_with_seed(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                100,
                &invalid_stake_state_pubkey(),
                &Pubkey::new_unique(),
                "seed",
            )[1],
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::delegate_stake(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &invalid_vote_state_pubkey(),
            ),
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::withdraw(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                100,
                None,
            ),
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::deactivate_stake(&Pubkey::new_unique(), &Pubkey::new_unique()),
            Err(InstructionError::InvalidAccountData),
        );
        process_instruction_as_one_arg(
            &instruction::set_lockup(
                &Pubkey::new_unique(),
                &LockupArgs::default(),
                &Pubkey::new_unique(),
            ),
            Err(InstructionError::InvalidAccountData),
        );
    }

    #[test]
    fn test_spoofed_stake_accounts() {
        process_instruction_as_one_arg(
            &instruction::initialize(
                &spoofed_stake_state_pubkey(),
                &Authorized::default(),
                &Lockup::default(),
            ),
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::authorize(
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                StakeAuthorize::Staker,
                None,
            ),
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::split(
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
                100,
                &Pubkey::new_unique(),
            )[2],
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::split(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                100,
                &spoofed_stake_state_pubkey(),
            )[2],
            Err(InstructionError::IncorrectProgramId),
        );
        process_instruction_as_one_arg(
            &instruction::merge(
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
            )[0],
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::merge(
                &Pubkey::new_unique(),
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
            )[0],
            Err(InstructionError::IncorrectProgramId),
        );
        process_instruction_as_one_arg(
            &instruction::split_with_seed(
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
                100,
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                "seed",
            )[1],
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::delegate_stake(
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
            ),
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::withdraw(
                &spoofed_stake_state_pubkey(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                100,
                None,
            ),
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::deactivate_stake(&spoofed_stake_state_pubkey(), &Pubkey::new_unique()),
            Err(InstructionError::InvalidAccountOwner),
        );
        process_instruction_as_one_arg(
            &instruction::set_lockup(
                &spoofed_stake_state_pubkey(),
                &LockupArgs::default(),
                &Pubkey::new_unique(),
            ),
            Err(InstructionError::InvalidAccountOwner),
        );
    }

    #[test]
    fn test_stake_process_instruction_decode_bail() {
        // these will not call stake_state, have bogus contents
        let stake_address = Pubkey::new_unique();
        let stake_account = create_default_stake_account();
        let rent_address = sysvar::rent::id();
        let rent_account = account::create_account_shared_data_for_test(&Rent::default());
        let rewards_address = sysvar::rewards::id();
        let rewards_account =
            account::create_account_shared_data_for_test(&sysvar::rewards::Rewards::new(0.0));
        let stake_history_address = sysvar::stake_history::id();
        let stake_history_account =
            account::create_account_shared_data_for_test(&StakeHistory::default());
        let vote_address = Pubkey::new_unique();
        let vote_account = AccountSharedData::new(0, 0, &solana_vote_program::id());
        let clock_address = sysvar::clock::id();
        let clock_account =
            account::create_account_shared_data_for_test(&sysvar::clock::Clock::default());
        let config_address = stake_config::id();
        let config_account = config::create_account(0, &stake_config::Config::default());

        // gets the "is_empty()" check
        process_instruction(
            &serialize(&StakeInstruction::Initialize(
                Authorized::default(),
                Lockup::default(),
            ))
            .unwrap(),
            Vec::new(),
            Vec::new(),
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // no account for rent
        process_instruction(
            &serialize(&StakeInstruction::Initialize(
                Authorized::default(),
                Lockup::default(),
            ))
            .unwrap(),
            vec![(stake_address, stake_account.clone())],
            vec![AccountMeta {
                pubkey: stake_address,
                is_signer: false,
                is_writable: false,
            }],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // fails to deserialize stake state
        process_instruction(
            &serialize(&StakeInstruction::Initialize(
                Authorized::default(),
                Lockup::default(),
            ))
            .unwrap(),
            vec![
                (stake_address, stake_account.clone()),
                (rent_address, rent_account),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: rent_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::InvalidAccountData),
        );

        // gets the first check in delegate, wrong number of accounts
        process_instruction(
            &serialize(&StakeInstruction::DelegateStake).unwrap(),
            vec![(stake_address, stake_account.clone())],
            vec![AccountMeta {
                pubkey: stake_address,
                is_signer: false,
                is_writable: false,
            }],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // gets the sub-check for number of args
        process_instruction(
            &serialize(&StakeInstruction::DelegateStake).unwrap(),
            vec![(stake_address, stake_account.clone())],
            vec![AccountMeta {
                pubkey: stake_address,
                is_signer: false,
                is_writable: false,
            }],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // gets the check non-deserialize-able account in delegate_stake
        process_instruction(
            &serialize(&StakeInstruction::DelegateStake).unwrap(),
            vec![
                (stake_address, stake_account.clone()),
                (vote_address, vote_account.clone()),
                (clock_address, clock_account),
                (stake_history_address, stake_history_account.clone()),
                (config_address, config_account),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: vote_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: clock_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: stake_history_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: config_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::InvalidAccountData),
        );

        // Tests 3rd keyed account is of correct type (Clock instead of rewards) in withdraw
        process_instruction(
            &serialize(&StakeInstruction::Withdraw(42)).unwrap(),
            vec![
                (stake_address, stake_account.clone()),
                (vote_address, vote_account),
                (rewards_address, rewards_account.clone()),
                (stake_history_address, stake_history_account),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: vote_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: rewards_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: stake_history_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::InvalidArgument),
        );

        // Tests correct number of accounts are provided in withdraw
        process_instruction(
            &serialize(&StakeInstruction::Withdraw(42)).unwrap(),
            vec![(stake_address, stake_account.clone())],
            vec![AccountMeta {
                pubkey: stake_address,
                is_signer: false,
                is_writable: false,
            }],
            Err(InstructionError::NotEnoughAccountKeys),
        );

        // Tests 2nd keyed account is of correct type (Clock instead of rewards) in deactivate
        process_instruction(
            &serialize(&StakeInstruction::Deactivate).unwrap(),
            vec![
                (stake_address, stake_account),
                (rewards_address, rewards_account),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: rewards_address,
                    is_signer: false,
                    is_writable: false,
                },
            ],
            Err(InstructionError::InvalidArgument),
        );

        // Tests correct number of accounts are provided in deactivate
        process_instruction(
            &serialize(&StakeInstruction::Deactivate).unwrap(),
            Vec::new(),
            Vec::new(),
            Err(InstructionError::NotEnoughAccountKeys),
        );
    }

    #[test]
    fn test_stake_checked_instructions() {
        let stake_address = Pubkey::new_unique();
        let staker = Pubkey::new_unique();
        let staker_account = create_default_account();
        let withdrawer = Pubkey::new_unique();
        let withdrawer_account = create_default_account();
        let authorized_address = Pubkey::new_unique();
        let authorized_account = create_default_account();
        let new_authorized_account = create_default_account();
        let clock_address = sysvar::clock::id();
        let clock_account = account::create_account_shared_data_for_test(&Clock::default());
        let custodian = Pubkey::new_unique();
        let custodian_account = create_default_account();

        // Test InitializeChecked with non-signing withdrawer
        let mut instruction =
            initialize_checked(&stake_address, &Authorized { staker, withdrawer });
        instruction.accounts[3] = AccountMeta::new_readonly(withdrawer, false);
        process_instruction_as_one_arg(
            &instruction,
            Err(InstructionError::MissingRequiredSignature),
        );

        // Test InitializeChecked with withdrawer signer
        let stake_account = AccountSharedData::new(
            1_000_000_000,
            std::mem::size_of::<crate::stake_state::StakeState>(),
            &id(),
        );
        let rent_address = sysvar::rent::id();
        let rent_account = account::create_account_shared_data_for_test(&Rent::default());
        process_instruction(
            &serialize(&StakeInstruction::InitializeChecked).unwrap(),
            vec![
                (stake_address, stake_account),
                (rent_address, rent_account),
                (staker, staker_account),
                (withdrawer, withdrawer_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: rent_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: staker,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: withdrawer,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Ok(()),
        );

        // Test AuthorizeChecked with non-signing authority
        let mut instruction = authorize_checked(
            &stake_address,
            &authorized_address,
            &staker,
            StakeAuthorize::Staker,
            None,
        );
        instruction.accounts[3] = AccountMeta::new_readonly(staker, false);
        process_instruction_as_one_arg(
            &instruction,
            Err(InstructionError::MissingRequiredSignature),
        );

        let mut instruction = authorize_checked(
            &stake_address,
            &authorized_address,
            &withdrawer,
            StakeAuthorize::Withdrawer,
            None,
        );
        instruction.accounts[3] = AccountMeta::new_readonly(withdrawer, false);
        process_instruction_as_one_arg(
            &instruction,
            Err(InstructionError::MissingRequiredSignature),
        );

        // Test AuthorizeChecked with authority signer
        let stake_account = AccountSharedData::new_data_with_space(
            42,
            &StakeState::Initialized(Meta::auto(&authorized_address)),
            std::mem::size_of::<StakeState>(),
            &id(),
        )
        .unwrap();
        process_instruction(
            &serialize(&StakeInstruction::AuthorizeChecked(StakeAuthorize::Staker)).unwrap(),
            vec![
                (stake_address, stake_account.clone()),
                (clock_address, clock_account.clone()),
                (authorized_address, authorized_account.clone()),
                (staker, new_authorized_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: clock_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: authorized_address,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: staker,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Ok(()),
        );

        process_instruction(
            &serialize(&StakeInstruction::AuthorizeChecked(
                StakeAuthorize::Withdrawer,
            ))
            .unwrap(),
            vec![
                (stake_address, stake_account),
                (clock_address, clock_account.clone()),
                (authorized_address, authorized_account.clone()),
                (withdrawer, new_authorized_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: clock_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: authorized_address,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: withdrawer,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Ok(()),
        );

        // Test AuthorizeCheckedWithSeed with non-signing authority
        let authorized_owner = Pubkey::new_unique();
        let seed = "test seed";
        let address_with_seed =
            Pubkey::create_with_seed(&authorized_owner, seed, &authorized_owner).unwrap();
        let mut instruction = authorize_checked_with_seed(
            &stake_address,
            &authorized_owner,
            seed.to_string(),
            &authorized_owner,
            &staker,
            StakeAuthorize::Staker,
            None,
        );
        instruction.accounts[3] = AccountMeta::new_readonly(staker, false);
        process_instruction_as_one_arg(
            &instruction,
            Err(InstructionError::MissingRequiredSignature),
        );

        let mut instruction = authorize_checked_with_seed(
            &stake_address,
            &authorized_owner,
            seed.to_string(),
            &authorized_owner,
            &staker,
            StakeAuthorize::Withdrawer,
            None,
        );
        instruction.accounts[3] = AccountMeta::new_readonly(staker, false);
        process_instruction_as_one_arg(
            &instruction,
            Err(InstructionError::MissingRequiredSignature),
        );

        // Test AuthorizeCheckedWithSeed with authority signer
        let stake_account = AccountSharedData::new_data_with_space(
            42,
            &StakeState::Initialized(Meta::auto(&address_with_seed)),
            std::mem::size_of::<StakeState>(),
            &id(),
        )
        .unwrap();
        process_instruction(
            &serialize(&StakeInstruction::AuthorizeCheckedWithSeed(
                AuthorizeCheckedWithSeedArgs {
                    stake_authorize: StakeAuthorize::Staker,
                    authority_seed: seed.to_string(),
                    authority_owner: authorized_owner,
                },
            ))
            .unwrap(),
            vec![
                (address_with_seed, stake_account.clone()),
                (authorized_owner, authorized_account.clone()),
                (clock_address, clock_account.clone()),
                (staker, new_authorized_account.clone()),
            ],
            vec![
                AccountMeta {
                    pubkey: address_with_seed,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: authorized_owner,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: clock_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: staker,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Ok(()),
        );

        process_instruction(
            &serialize(&StakeInstruction::AuthorizeCheckedWithSeed(
                AuthorizeCheckedWithSeedArgs {
                    stake_authorize: StakeAuthorize::Withdrawer,
                    authority_seed: seed.to_string(),
                    authority_owner: authorized_owner,
                },
            ))
            .unwrap(),
            vec![
                (address_with_seed, stake_account),
                (authorized_owner, authorized_account),
                (clock_address, clock_account.clone()),
                (withdrawer, new_authorized_account),
            ],
            vec![
                AccountMeta {
                    pubkey: address_with_seed,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: authorized_owner,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: clock_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: withdrawer,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Ok(()),
        );

        // Test SetLockupChecked with non-signing lockup custodian
        let mut instruction = set_lockup_checked(
            &stake_address,
            &LockupArgs {
                unix_timestamp: None,
                epoch: Some(1),
                custodian: Some(custodian),
            },
            &withdrawer,
        );
        instruction.accounts[2] = AccountMeta::new_readonly(custodian, false);
        process_instruction_as_one_arg(
            &instruction,
            Err(InstructionError::MissingRequiredSignature),
        );

        // Test SetLockupChecked with lockup custodian signer
        let stake_account = AccountSharedData::new_data_with_space(
            42,
            &StakeState::Initialized(Meta::auto(&withdrawer)),
            std::mem::size_of::<StakeState>(),
            &id(),
        )
        .unwrap();

        process_instruction(
            &instruction.data,
            vec![
                (clock_address, clock_account),
                (stake_address, stake_account),
                (withdrawer, withdrawer_account),
                (custodian, custodian_account),
            ],
            vec![
                AccountMeta {
                    pubkey: stake_address,
                    is_signer: false,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: withdrawer,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: custodian,
                    is_signer: true,
                    is_writable: false,
                },
            ],
            Ok(()),
        );
    }
}
