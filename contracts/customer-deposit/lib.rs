#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::env::Environment;
use ddc_primitives::{Balance as ChainBalance, BlockNumber as ChainBlockNumber, Timestamp as ChainTimestamp};
use sp_runtime::AccountId32;

#[derive(Debug, Clone, PartialEq, Eq)]
#[ink::scale_derive(TypeInfo)]
pub enum CereEnvironment {}

impl Environment for CereEnvironment {
	const MAX_EVENT_TOPICS: usize = <ink::env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;
	type AccountId = <ink::env::DefaultEnvironment as Environment>::AccountId;
	type Hash = <ink::env::DefaultEnvironment as Environment>::Hash;
	type Balance = ChainBalance;
	type BlockNumber = ChainBlockNumber;
	type Timestamp = ChainTimestamp;
	type ChainExtension = ();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum Error {
	InsufficientDeposit,
	ArithmeticOverflow,
	ArithmeticUnderflow,
	TransferFailed,
	NotOwner,
	NoLedger,
	NothingToWithdraw,
}

#[ink::contract(env = crate::CereEnvironment)]
mod customer_deposit {
	use ink::{prelude::vec::Vec, storage::Mapping};

	use super::Error;

	pub type ClusterId = [u8; 20];
	pub const UNLOCK_DELAY_BLOCKS: u32 = 10;
	pub const MIN_EXISTENTIAL_DEPOSIT: Balance = 10000000000;

	#[derive(Debug, Clone, PartialEq, Eq)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	#[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
	pub struct Ledger {
		/// The owner account whose balance is actually locked and can be used to pay for DDC
		/// network usage.
		pub owner: AccountId,
		/// The total amount of the owner's balance that we are currently accounting for.
		/// It's just `active` plus all the `unlocking` balances.
		pub total: Balance,
		/// The total amount of the owner's balance that will be accessible for DDC network payouts
		/// in any forthcoming rounds.
		pub active: Balance,
		/// Any balance that is becoming free, which may eventually be transferred out of the owner
		/// (assuming that the content owner has to pay for network usage). It is assumed that this
		/// will be treated as a first in, first out queue where the new (higher value) eras get
		/// pushed on the back.
		pub unlocking: Vec<UnlockChunk>,
	}

	impl Ledger {
		/// Remove unlocked chunks and update total balance (like pallet).
		fn consolidate_unlocked(mut self, current_block: BlockNumber) -> Self {
			let mut total = self.total;
			self.unlocking.retain(|chunk| {
				if chunk.block <= current_block {
					total = total.saturating_sub(chunk.value);
					false
				} else {
					true
				}
			});
			Self { total, ..self }
		}
	}

	#[derive(Debug, Clone, PartialEq, Eq)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	#[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
	pub struct UnlockChunk {
		/// Amount of funds to be unlocked.
		value: Balance,
		/// Block number at which point it'll be unlocked.
		block: BlockNumber,
	}

	#[ink(event)]
	pub struct Deposited {
		#[ink(topic)]
		cluster_id: ClusterId,
        #[ink(topic)]
		owner_id: AccountId,
		amount: Balance,
	}

	#[ink(event)]
	pub struct InitialDepositUnlock {
		#[ink(topic)]
		cluster_id: ClusterId,
        #[ink(topic)]
		owner_id: AccountId,
		amount: Balance,
	}

	#[ink(event)]
	pub struct Withdrawn {
		#[ink(topic)]
		cluster_id: ClusterId,
        #[ink(topic)]
		owner_id: AccountId,
		amount: Balance,
	}

    #[ink(event)]
	pub struct Charged {
		#[ink(topic)]
		cluster_id: ClusterId,
		#[ink(topic)]
		owner_id: AccountId,
		charged: Balance,
		expected_to_charge: Balance,
	}

	/// Defines the storage of our contract.
	///
	/// Here we store the random seed fetched from the chain.
	#[ink(storage)]
	pub struct CustomerDepositContract {
		cluster_id: ClusterId,
		balances: Mapping<AccountId, Ledger>,
	}

	impl CustomerDepositContract {
		#[ink(constructor)]
		pub fn new(cluster_id: ClusterId) -> Self {
			let balances = Mapping::default();
			Self { balances, cluster_id }
		}

		/// Get the deposit balance for specific owner
		#[ink(message)]
		pub fn balance(&self, owner: AccountId) -> Option<Ledger> {
			self.balances.get(&owner)
		}

		/// Top up deposit balance on behalf its owner
		#[ink(message, payable)]
		pub fn deposit(&mut self) -> Result<(), Error> {
			let owner = self.env().caller();
			let value = self.env().transferred_value();

			// Reject dust deposits
			if value < MIN_EXISTENTIAL_DEPOSIT {
				return Err(Error::InsufficientDeposit);
			}

			if let Some(mut ledger) = self.balances.get(&owner) {
				// Existing ledger - update balances
				ledger.total = ledger.total.checked_add(value).ok_or(Error::ArithmeticOverflow)?;
				ledger.active =
					ledger.active.checked_add(value).ok_or(Error::ArithmeticOverflow)?;

				// Defensive check against dust
				if ledger.active < MIN_EXISTENTIAL_DEPOSIT {
					return Err(Error::InsufficientDeposit);
				}

				self.balances.insert(owner, &ledger);
			} else {
				// New ledger
				let ledger =
					Ledger { owner, total: value, active: value, unlocking: Default::default() };
				self.balances.insert(owner, &ledger);
			}

			self.env().emit_event(Deposited {
				cluster_id: self.cluster_id,
				owner_id: owner,
				amount: value,
			});

			Ok(())
		}

		/// Top up deposit balance for specific owner on behalf faucet
		#[ink(message, payable)]
		pub fn deposit_for(&mut self, owner: AccountId) -> Result<(), Error> {
			let _funder = self.env().caller();
			let value = self.env().transferred_value();

			// Reject dust deposits
			if value < MIN_EXISTENTIAL_DEPOSIT {
				return Err(Error::InsufficientDeposit);
			}

			if !self.balances.contains(&owner) {
				// New ledger - no need for existential deposit check since contract holds all
				// tokens
				let ledger =
					Ledger { owner, total: value, active: value, unlocking: Default::default() };

				self.balances.insert(owner, &ledger);
			} else {
				// Existing ledger - update balances
				let mut ledger = self.balances.get(&owner).ok_or(Error::NotOwner)?;
				ledger.total = ledger.total.checked_add(value).ok_or(Error::ArithmeticOverflow)?;
				ledger.active =
					ledger.active.checked_add(value).ok_or(Error::ArithmeticOverflow)?;

				// Ensure active balance doesn't become dust (defensive programming)
				if ledger.active < MIN_EXISTENTIAL_DEPOSIT {
					return Err(Error::InsufficientDeposit);
				}

				self.balances.insert(owner, &ledger);
			}

			self.env().emit_event(Deposited {
				cluster_id: self.cluster_id,
				owner_id: owner,
				amount: value,
			});

			Ok(())
		}

		/// Initiate unlocking of deposit balance on behalf its owner
		#[ink(message)]
		pub fn unlock_deposit(&mut self, value: Balance) -> Result<(), Error> {
			let owner = self.env().caller();
			let mut ledger = self.balances.get(&owner).ok_or(Error::NotOwner)?;

			// Ensure sufficient active balance
			if value > ledger.active {
				return Err(Error::InsufficientDeposit);
			}

			// Prevent unlocking dust amounts
			if value < MIN_EXISTENTIAL_DEPOSIT {
				return Err(Error::InsufficientDeposit);
			}

			// Update balances (pallet-like dust handling)
			let mut value = value;
			ledger.active = ledger.active.checked_sub(value).ok_or(Error::ArithmeticUnderflow)?;

			// Avoid dust in active balance (like pallet)
			if ledger.active < MIN_EXISTENTIAL_DEPOSIT {
				value = value.checked_add(ledger.active).ok_or(Error::ArithmeticOverflow)?;
				ledger.active = 0;
			}

			// Schedule unlock
			let current_block = self.env().block_number();
			let unlock_block = current_block
				.checked_add(UNLOCK_DELAY_BLOCKS)
				.ok_or(Error::ArithmeticOverflow)?;

			// Merge chunks unlocking at the same block (like pallet)
			if let Some(chunk) =
				ledger.unlocking.last_mut().filter(|chunk| chunk.block == unlock_block)
			{
				// To keep the chunk count down, we only keep one chunk per era. Since
				// `unlocking` is a FiFo queue, if a chunk exists for `era` we know that it will
				// be the last one.
				chunk.value = chunk.value.saturating_add(value);
			} else {
				ledger.unlocking.push(UnlockChunk { value, block: unlock_block });
			}

			self.balances.insert(&owner, &ledger);

			// Emit event (like pallet)
			self.env().emit_event(InitialDepositUnlock {
				cluster_id: self.cluster_id,
				owner_id: owner,
				amount: value,
			});

			Ok(())
		}

		/// Withdraw unlocked deposit balance on behalf its owner
		#[ink(message)]
		pub fn withdraw_unlocked(&mut self) -> Result<(), Error> {
			let owner = self.env().caller();
			let current_block = self.env().block_number();

			// Get ledger or fail
			let mut ledger = self.balances.get(&owner).ok_or(Error::NoLedger)?;

			// Consolidate unlocked chunks and update total
			let old_total = ledger.total;
			ledger = ledger.consolidate_unlocked(current_block);

			// Calculate withdrawn amount
			let withdrawn =
				old_total.checked_sub(ledger.total).ok_or(Error::ArithmeticUnderflow)?;
			if withdrawn == 0 {
				return Err(Error::NothingToWithdraw);
			}

			self.env().transfer(owner, withdrawn).map_err(|_| Error::TransferFailed)?;

			// Clean up ledger if balance is dust and no unlocking chunks
			if ledger.total < MIN_EXISTENTIAL_DEPOSIT && ledger.unlocking.is_empty() {
				self.balances.remove(&owner);
			} else {
				self.balances.insert(&owner, &ledger);
			}

			// Emit event
			self.env().emit_event(Withdrawn {
				cluster_id: self.cluster_id,
				owner_id: owner,
				amount: withdrawn,
			});

			Ok(())
		}
	}

	impl ddc_primitives::traits::DdcPayoutsPayer for CustomerDepositContract {
        #[ink(message)]
        fn charge(
            &mut self,
            payout_vault: crate::AccountId32,
            batch: Vec<(crate::AccountId32, u128)>,
        ) -> Vec<(crate::AccountId32, u128)> {

            let mut charged_amounts = Vec::new();
			let payout_vault = from_chain_account_id(&payout_vault);

            for (chain_customer_id, amount_to_deduct) in batch {
				let customer_id = from_chain_account_id(&chain_customer_id);
				
                // Check if the customer has a ledger
                if let Some(mut ledger) = self.balances.get(&customer_id) {
                    // Calculate the actual amount that can be charged (partial or full)
                    let actually_charged = ledger.active.min(amount_to_deduct);
        
                    // Deduct the charged amount from the active and total balances
                    ledger.active = ledger.active.saturating_sub(actually_charged);
                    ledger.total = ledger.total.saturating_sub(actually_charged);
                    self.balances.insert(&customer_id, &ledger);
        
                    // Transfer the tokens from the contract's vault to the payout_vault
                    if actually_charged > 0 {
                        if let Err(_) = self.env().transfer(payout_vault, actually_charged) {
                            // Revert balance changes if transfer fails
                            ledger.active = ledger.active.saturating_add(actually_charged);
                            ledger.total = ledger.total.saturating_add(actually_charged);
                            self.balances.insert(&customer_id, &ledger);
                            // Skip adding to `charged_amounts` if transfer failed
                            continue;
                        }
        
                        // Emit `Charged` event (partial or full)
                        self.env().emit_event(Charged {
                            cluster_id: self.cluster_id,
                            owner_id: customer_id,
                            charged: actually_charged,
                            expected_to_charge: amount_to_deduct,
                        });
        
                        // Record the successfully charged amount
                        charged_amounts.push((chain_customer_id, actually_charged));
                    }
                }
            }

			charged_amounts
        }
	}

	pub fn from_chain_account_id(account_id: &crate::AccountId32) -> AccountId {
		AccountId::from(<[u8; 32]>::from(account_id.clone()))
	}

}


#[cfg(test)]
mod tests {
	use ink::env::test;

	use super::*;
	use crate::customer_deposit::{
		ClusterId, CustomerDepositContract, MIN_EXISTENTIAL_DEPOSIT, UNLOCK_DELAY_BLOCKS
	};
	use sp_runtime::AccountId32;
    use ddc_primitives::traits::DdcPayoutsPayer;

	const TEST_CLUSTER_ID: ClusterId = [0; 20];
	const ENDOWMENT: Balance = MIN_EXISTENTIAL_DEPOSIT * 1_000_000;

	type Balance = <ink::env::DefaultEnvironment as Environment>::Balance;

	fn setup() -> (CustomerDepositContract, test::DefaultAccounts<ink::env::DefaultEnvironment>) {
		let contract = CustomerDepositContract::new(TEST_CLUSTER_ID);
		let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
		ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
			accounts.alice,
			ENDOWMENT,
		);

		(contract, accounts)
	}

	fn to_account_32(account_id: &<ink::env::DefaultEnvironment as Environment>::AccountId) -> Result<AccountId32, ()> {
		if let Ok(bytes) = <[u8; 32]>::try_from(account_id.as_ref()) {
			Ok(AccountId32::from(bytes))
		} else {
			Err(())
		}
	}

	#[ink::test]
	fn test_deposit_new_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT);

		// Deposit succeeds
		assert!(contract.deposit().is_ok());

		// Verify ledger
		let ledger = contract.balance(accounts.alice).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT);
	}

	#[ink::test]
	fn test_deposit_existing_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		// First deposit
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT);
		assert!(contract.deposit().is_ok());

		// Second deposit
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 2);
		assert!(contract.deposit().is_ok());

		// Verify ledger
		let ledger = contract.balance(accounts.alice).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 3);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 3);
	}

	#[ink::test]
	fn test_deposit_dust_fails() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT - 1);

		// Deposit fails
		assert_eq!(contract.deposit().unwrap_err(), Error::InsufficientDeposit);
	}

	#[ink::test]
	fn test_deposit_for_new_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 2);

		// Deposit for Bob (new account)
		assert!(contract.deposit_for(accounts.bob).is_ok());

		// Verify Bob's ledger
		let ledger = contract.balance(accounts.bob).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 2);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 2);
	}

	#[ink::test]
	fn test_deposit_for_existing_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		// First deposit for Bob
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT);
		assert!(contract.deposit_for(accounts.bob).is_ok());

		// Second deposit for Bob
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 2);
		assert!(contract.deposit_for(accounts.bob).is_ok());

		// Verify Bob's ledger
		let ledger = contract.balance(accounts.bob).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 3);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 3);
	}

	#[ink::test]
	fn test_deposit_for_dust_fails() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT - 1);

		// Deposit fails (dust)
		assert_eq!(contract.deposit_for(accounts.bob).unwrap_err(), Error::InsufficientDeposit);
	}

	#[ink::test]
	fn test_unlock_deposit_valid() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 10);
		assert!(contract.deposit().is_ok());

		// Unlock half
		assert!(contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT * 5).is_ok());

		// Verify ledger
		let ledger = contract.balance(accounts.alice).unwrap();
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 5);
		assert_eq!(ledger.unlocking.len(), 1);
	}

	#[ink::test]
	fn test_unlock_deposit_insufficient_balance() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT);
		assert!(contract.deposit().is_ok());

		// Attempt to unlock more than active balance
		assert_eq!(
			contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT * 2).unwrap_err(),
			Error::InsufficientDeposit
		);
	}

	#[ink::test]
	fn test_unlock_deposit_dust_fails() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 10);
		assert!(contract.deposit().is_ok());

		// Attempt to unlock dust
		assert_eq!(
			contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT - 1).unwrap_err(),
			Error::InsufficientDeposit
		);
	}

	#[ink::test]
	fn test_withdraw_unlocked() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 10);
		assert!(contract.deposit().is_ok());

		// Unlock half (reduces active balance)
		assert!(contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT * 5).is_ok());
		for _ in 0..UNLOCK_DELAY_BLOCKS {
			test::advance_block::<ink::env::DefaultEnvironment>();
		}

		// Withdraw unlocked (should not touch active balance)
		assert!(contract.withdraw_unlocked().is_ok());

		// Verify ledger
		let ledger = contract.balance(accounts.alice).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 5); // Withdrawn 5, remaining 5
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 5); // Active balance unchanged
		assert!(ledger.unlocking.is_empty());
	}

	#[ink::test]
	fn test_withdraw_unlocked_early_fails() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 10);
		assert!(contract.deposit().is_ok());

		// Unlock but don't advance blocks
		assert!(contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT * 5).is_ok());

		// Attempt early withdraw
		assert_eq!(contract.withdraw_unlocked().unwrap_err(), Error::NothingToWithdraw);
	}

	#[ink::test]
	fn test_withdraw_unlocked_cleans_up_ledger() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 2); // Deposit more than MIN
		assert!(contract.deposit().is_ok());

		// Unlock MIN_EXISTENTIAL_DEPOSIT (partial unlock)
		assert!(contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT).is_ok());
		for _ in 0..UNLOCK_DELAY_BLOCKS {
			test::advance_block::<ink::env::DefaultEnvironment>();
		}

		// Withdraw unlocked (partial withdraw)
		assert!(contract.withdraw_unlocked().is_ok());

		// Verify ledger is NOT cleaned up (balance is still above dust)
		assert!(contract.balance(accounts.alice).is_some());

		// Unlock and withdraw remaining balance
		assert!(contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT).is_ok());
		for _ in 0..UNLOCK_DELAY_BLOCKS {
			test::advance_block::<ink::env::DefaultEnvironment>();
		}
		assert!(contract.withdraw_unlocked().is_ok());

		// Now ledger should be cleaned up
		assert!(contract.balance(accounts.alice).is_none());
	}

    #[ink::test]
    fn test_charge_return_value() {
        let (mut contract, accounts) = setup();
        let alice = accounts.alice;
        let bob = accounts.bob;
        let payout_vault = to_account_32(&accounts.charlie).expect("Invalid contract account id");

        // Deposit funds for Alice (10 units) and Bob (5 units)
        test::set_caller::<ink::env::DefaultEnvironment>(alice);
        test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 10);
        assert!(contract.deposit().is_ok());

        test::set_caller::<ink::env::DefaultEnvironment>(bob);
        test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 5);
        assert!(contract.deposit().is_ok());

        // Charge Alice (10 units) and Bob (7 units, partial charge expected)
        let batch = vec![
            (to_account_32(&alice).expect("Invalid contract account id"), MIN_EXISTENTIAL_DEPOSIT * 10),
            (to_account_32(&bob).expect("Invalid contract account id"), MIN_EXISTENTIAL_DEPOSIT * 7),
        ];
        let charged_amounts = contract.charge(payout_vault, batch);

        // Verify return value:
        // - Alice: 10 units charged (full)
        // - Bob: 5 units charged (partial)
        assert_eq!(charged_amounts.len(), 2);
        assert!(charged_amounts.contains(&(to_account_32(&alice).expect("Invalid contract account id"), MIN_EXISTENTIAL_DEPOSIT * 10)));
        assert!(charged_amounts.contains(&(to_account_32(&bob).expect("Invalid contract account id"), MIN_EXISTENTIAL_DEPOSIT * 5)));
    }
}
