#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::env::Environment;
use ddc_primitives::{
	contracts::types::{ClusterId, AccountId as AccountId32, Balance as BalanceU128},
	contracts::customer_deposit::{
		types::{Ledger, UnlockChunk},
		traits::{DdcBalancesFetcher, DdcBalancesDepositor, DdcPayoutsPayer},
		events::{DdcBalanceDeposited, DdcBalanceUnlocked, DdcBalanceWithdrawn, DdcBalanceCharged},
		errors::Error as CustomerDepositError,
	},
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[ink::scale_derive(TypeInfo)]
pub enum CereEnvironment {}

impl Environment for CereEnvironment {
	const MAX_EVENT_TOPICS: usize = <ink::env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;
	type AccountId = <ink::env::DefaultEnvironment as Environment>::AccountId;
	type Hash = <ink::env::DefaultEnvironment as Environment>::Hash;
	type Balance = <ink::env::DefaultEnvironment as Environment>::Balance;
	type BlockNumber = <ink::env::DefaultEnvironment as Environment>::BlockNumber;
	type Timestamp = <ink::env::DefaultEnvironment as Environment>::Timestamp;
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

	use super::{
		Error, AccountId32, ClusterId, BalanceU128,
		DdcBalanceDeposited, DdcBalanceUnlocked, DdcBalanceWithdrawn, DdcBalanceCharged, 
		DdcBalancesFetcher, DdcBalancesDepositor, DdcPayoutsPayer, CustomerDepositError,
		Ledger, UnlockChunk
	};

	pub const UNLOCK_DELAY_BLOCKS: u32 = 10;
	pub const MIN_EXISTENTIAL_DEPOSIT: Balance = 10000000000;

	// todo(yahortsaryk): request from the chain via extension
	pub const PAYOUTS_PALLET: AccountId32 = AccountId32::new([
		0x6d, 0x6f, 0x64, 0x6c, 0x70, 0x61, 0x79, 0x6f, 0x75, 0x74, 0x73, 0x5f, 0x00, 0x00, 0x00,
		0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
		0x00, 0x00,
	]);

	#[derive(Debug, Clone, PartialEq, Eq)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	#[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
	pub struct CustomerLedger {
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
		pub unlocking: Vec<LinearUnlockChunk>,
	}

	impl CustomerLedger {
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
	pub struct LinearUnlockChunk {
		/// Amount of funds to be unlocked.
		value: Balance,
		/// Block number at which point it'll be unlocked.
		block: BlockNumber,
	}

	/// Defines the storage of the contract.
	#[ink(storage)]
	pub struct CustomerDepositContract {
		cluster_id: ClusterId,
		balances: Mapping<AccountId, CustomerLedger>,
	}

	impl CustomerDepositContract {
		#[ink(constructor)]
		pub fn new(cluster_id: ClusterId) -> Self {
			let balances = Mapping::default();
			Self { balances, cluster_id }
		}
	}

	impl DdcBalancesFetcher for CustomerDepositContract {
		/// Fetches customer balance in DDC cluster.
		#[ink(message)]
		fn get_balance(&self, owner: AccountId32) -> Option<Ledger> {
			let ledger = self.balances.get(&from_account_32(&owner))?;
			Some(ledger.into())
		}
	}

	impl DdcBalancesDepositor for CustomerDepositContract {
		/// Top up deposit balance on behalf its owner.
		#[ink(message, payable)]
		fn deposit(&mut self) -> Result<(), CustomerDepositError> {
			let owner = self.env().caller();
			let value = self.env().transferred_value();

			// Reject dust deposits
			if value < MIN_EXISTENTIAL_DEPOSIT {
				return Err(Error::InsufficientDeposit.into());
			}

			if let Some(mut ledger) = self.balances.get(&owner) {
				// Existing ledger - update balances
				ledger.total = ledger.total.checked_add(value).ok_or(Error::ArithmeticOverflow)?;
				ledger.active =
					ledger.active.checked_add(value).ok_or(Error::ArithmeticOverflow)?;

				// Defensive check against dust
				if ledger.active < MIN_EXISTENTIAL_DEPOSIT {
					return Err(Error::InsufficientDeposit.into());
				}

				self.balances.insert(owner, &ledger);
			} else {
				// New ledger
				let ledger =
					CustomerLedger { owner, total: value, active: value, unlocking: Default::default() };
				self.balances.insert(owner, &ledger);
			}

			self.env().emit_event(DdcBalanceDeposited {
				cluster_id: self.cluster_id,
				owner_id: to_account_32(&owner).unwrap(),
				amount: value,
			});

			Ok(())
		}
	
		/// Top up deposit balance for specific owner on behalf faucet.
		#[ink(message, payable)]
		fn deposit_for(&mut self, owner: AccountId32) -> Result<(), CustomerDepositError> {
			let owner = from_account_32(&owner);
			let _funder = self.env().caller();
			let value = self.env().transferred_value();

			// Reject dust deposits
			if value < MIN_EXISTENTIAL_DEPOSIT {
				return Err(Error::InsufficientDeposit.into());
			}

			if !self.balances.contains(&owner) {
				// New ledger - no need for existential deposit check since contract holds all
				// tokens
				let ledger =
					CustomerLedger { owner, total: value, active: value, unlocking: Default::default() };

				self.balances.insert(owner, &ledger);
			} else {
				// Existing ledger - update balances
				let mut ledger = self.balances.get(&owner).ok_or(Error::NotOwner)?;
				ledger.total = ledger.total.checked_add(value).ok_or(Error::ArithmeticOverflow)?;
				ledger.active =
					ledger.active.checked_add(value).ok_or(Error::ArithmeticOverflow)?;

				// Ensure active balance doesn't become dust (defensive programming)
				if ledger.active < MIN_EXISTENTIAL_DEPOSIT {
					return Err(Error::InsufficientDeposit.into());
				}

				self.balances.insert(owner, &ledger);
			}

			self.env().emit_event(DdcBalanceDeposited {
				cluster_id: self.cluster_id,
				owner_id: to_account_32(&owner).unwrap(),
				amount: value,
			});

			Ok(())
		}
	
		/// Initiate unlocking of deposit balance on behalf its owner.
		#[ink(message)]
		fn unlock_deposit(&mut self, value: BalanceU128) -> Result<(), CustomerDepositError> {
			let owner = self.env().caller();
			let mut ledger = self.balances.get(&owner).ok_or(Error::NotOwner)?;

			// Ensure sufficient active balance
			if value > ledger.active {
				return Err(Error::InsufficientDeposit.into());
			}

			// Prevent unlocking dust amounts
			if value < MIN_EXISTENTIAL_DEPOSIT {
				return Err(Error::InsufficientDeposit.into());
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
				ledger.unlocking.push(LinearUnlockChunk { value, block: unlock_block });
			}

			self.balances.insert(&owner, &ledger);

			// Emit event (like pallet)
			self.env().emit_event(DdcBalanceUnlocked {
				cluster_id: self.cluster_id,
				owner_id: to_account_32(&owner).unwrap(),
				amount: value,
			});

			Ok(())
		}
	
		/// Withdraw unlocked deposit balance on behalf its owner.
		#[ink(message)]
		fn withdraw_unlocked(&mut self) -> Result<(), CustomerDepositError> {
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
				return Err(Error::NothingToWithdraw.into());
			}

			self.env().transfer(owner, withdrawn).map_err(|_| Error::TransferFailed)?;

			// Clean up ledger if balance is dust and no unlocking chunks
			if ledger.total < MIN_EXISTENTIAL_DEPOSIT && ledger.unlocking.is_empty() {
				self.balances.remove(&owner);
			} else {
				self.balances.insert(&owner, &ledger);
			}

			// Emit event
			self.env().emit_event(DdcBalanceWithdrawn {
				cluster_id: self.cluster_id,
				owner_id: to_account_32(&owner).unwrap(),
				amount: withdrawn,
			});

			Ok(())
		}
	}

	impl DdcPayoutsPayer for CustomerDepositContract {
		/// Charges customers for DDC service usage while DAC-based payouts are in progress.
		#[ink(message)]
		fn charge(
			&mut self,
			payout_vault: AccountId32,
			batch: Vec<(AccountId32, BalanceU128)>,
		) -> Vec<(AccountId32, BalanceU128)> {
			let caller = self.env().caller();

			assert!(caller == from_account_32(&PAYOUTS_PALLET));

			let mut charged_amounts = Vec::new();
			let payout_vault = from_account_32(&payout_vault);

			for (chain_customer_id, amount_to_deduct) in batch {
				let customer_id = from_account_32(&chain_customer_id);

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
						self.env().emit_event(DdcBalanceCharged {
							cluster_id: self.cluster_id,
							owner_id: to_account_32(&customer_id).unwrap(),
							charged: actually_charged,
							expected: amount_to_deduct,
						});

						// Record the successfully charged amount
						charged_amounts.push((chain_customer_id, actually_charged));
					}
				}
			}

			charged_amounts
		}
	}

	pub fn from_account_32(account_id: &AccountId32) -> AccountId {
		AccountId::from(<[u8; 32]>::from(account_id.clone()))
	}

	pub fn to_account_32(
		account_id: &<ink::env::DefaultEnvironment as ink::env::Environment>::AccountId,
	) -> Result<AccountId32, ()> {
		if let Ok(bytes) = <[u8; 32]>::try_from(account_id.as_ref()) {
			Ok(AccountId32::from(bytes))
		} else {
			Err(())
		}
	}

	impl From<Ledger> for CustomerLedger {
		fn from(other: Ledger) -> Self {
			CustomerLedger {
				owner: from_account_32(&other.owner),
				total: other.total,
				active: other.active,
				unlocking: other.unlocking.into_iter().map(|chunk| chunk.into()).collect(),
			}
		}
	}

	impl Into<Ledger> for CustomerLedger {
		fn into(self) -> Ledger {
			Ledger {
				owner: to_account_32(&self.owner).unwrap(),
				total: self.total,
				active: self.active,
				unlocking: self.unlocking.into_iter().map(|chunk| chunk.into()).collect(),
			}
		}
	}

	impl From<UnlockChunk> for LinearUnlockChunk {
		fn from(other: UnlockChunk) -> Self {
			LinearUnlockChunk {
				value: other.value,
				block: other.block,
			}
		}
	}

	impl Into<UnlockChunk> for LinearUnlockChunk {
		fn into(self) -> UnlockChunk {
			UnlockChunk {
				value: self.value,
				block: self.block,
			}
		}
	}

	impl From<Error> for CustomerDepositError {
		fn from(err: Error) -> Self {
			match err {
				Error::InsufficientDeposit => CustomerDepositError::Code(1),
				Error::ArithmeticOverflow => CustomerDepositError::Code(2),
				Error::ArithmeticUnderflow => CustomerDepositError::Code(3),
				Error::TransferFailed => CustomerDepositError::Code(4),
				Error::NotOwner => CustomerDepositError::Code(5),
				Error::NoLedger => CustomerDepositError::Code(6),
				Error::NothingToWithdraw => CustomerDepositError::Code(7),
			}
		}
	}
	
}

#[cfg(test)]
mod tests {
	use ddc_primitives::contracts::{
		customer_deposit::traits::{DdcPayoutsPayer, DdcBalancesFetcher, DdcBalancesDepositor}, 
		types::ClusterId
	};
	use ink::env::test;

	use super::*;
	use crate::customer_deposit::{
		from_account_32, to_account_32, CustomerDepositContract, 
		MIN_EXISTENTIAL_DEPOSIT, PAYOUTS_PALLET, UNLOCK_DELAY_BLOCKS,
	};

	const CLUSTER_ID: ClusterId = [0; 20];
	const ENDOWMENT: Balance = MIN_EXISTENTIAL_DEPOSIT * 1_000_000;

	type Balance = <ink::env::DefaultEnvironment as Environment>::Balance;

	fn setup() -> (CustomerDepositContract, test::DefaultAccounts<ink::env::DefaultEnvironment>) {
		let contract = CustomerDepositContract::new(CLUSTER_ID);
		let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
		ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
			accounts.alice,
			ENDOWMENT,
		);

		(contract, accounts)
	}

	#[ink::test]
	fn test_deposit_new_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT);

		// Deposit succeeds
		assert!(contract.deposit().is_ok());

		// Verify ledger
		let ledger = contract.get_balance(to_account_32(&accounts.alice).unwrap()).unwrap();
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
		let ledger = contract.get_balance(to_account_32(&accounts.alice).unwrap()).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 3);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 3);
	}

	#[ink::test]
	fn test_deposit_dust_fails() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT - 1);

		// Deposit fails
		assert_eq!(contract.deposit().unwrap_err(), Error::InsufficientDeposit.into());
	}

	#[ink::test]
	fn test_deposit_for_new_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 2);

		// Deposit for Bob (new account)
		assert!(contract.deposit_for(to_account_32(&accounts.bob).unwrap()).is_ok());

		// Verify Bob's ledger
		let ledger = contract.get_balance(to_account_32(&accounts.bob).unwrap()).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 2);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 2);
	}

	#[ink::test]
	fn test_deposit_for_existing_account() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);

		// First deposit for Bob
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT);
		assert!(contract.deposit_for(to_account_32(&accounts.bob).unwrap()).is_ok());

		// Second deposit for Bob
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT * 2);
		assert!(contract.deposit_for(to_account_32(&accounts.bob).unwrap()).is_ok());

		// Verify Bob's ledger
		let ledger = contract.get_balance(to_account_32(&accounts.bob).unwrap()).unwrap();
		assert_eq!(ledger.total, MIN_EXISTENTIAL_DEPOSIT * 3);
		assert_eq!(ledger.active, MIN_EXISTENTIAL_DEPOSIT * 3);
	}

	#[ink::test]
	fn test_deposit_for_dust_fails() {
		let (mut contract, accounts) = setup();
		test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
		test::set_value_transferred::<ink::env::DefaultEnvironment>(MIN_EXISTENTIAL_DEPOSIT - 1);

		// Deposit fails (dust)
		assert_eq!(contract.deposit_for(to_account_32(&accounts.bob).unwrap()).unwrap_err(), Error::InsufficientDeposit.into());
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
		let ledger = contract.get_balance(to_account_32(&accounts.alice).unwrap()).unwrap();
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
			Error::InsufficientDeposit.into()
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
			Error::InsufficientDeposit.into()
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
		let ledger = contract.get_balance(to_account_32(&accounts.alice).unwrap()).unwrap();
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
		assert_eq!(contract.withdraw_unlocked().unwrap_err(), Error::NothingToWithdraw.into());
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
		assert!(contract.get_balance(to_account_32(&accounts.alice).unwrap()).is_some());

		// Unlock and withdraw remaining balance
		assert!(contract.unlock_deposit(MIN_EXISTENTIAL_DEPOSIT).is_ok());
		for _ in 0..UNLOCK_DELAY_BLOCKS {
			test::advance_block::<ink::env::DefaultEnvironment>();
		}
		assert!(contract.withdraw_unlocked().is_ok());

		// Now ledger should be cleaned up
		assert!(contract.get_balance(to_account_32(&accounts.alice).unwrap()).is_none());
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

		let payout_pallet_id = from_account_32(&PAYOUTS_PALLET);
		test::set_caller::<ink::env::DefaultEnvironment>(payout_pallet_id);
		ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
			payout_pallet_id,
			ENDOWMENT,
		);

		// Charge Alice (10 units) and Bob (7 units, partial charge expected)
		let batch = vec![
			(
				to_account_32(&alice).expect("Invalid contract account id"),
				MIN_EXISTENTIAL_DEPOSIT * 10,
			),
			(
				to_account_32(&bob).expect("Invalid contract account id"),
				MIN_EXISTENTIAL_DEPOSIT * 7,
			),
		];
		let charged_amounts = contract.charge(payout_vault, batch);

		// Verify return value:
		// - Alice: 10 units charged (full)
		// - Bob: 5 units charged (partial)
		assert_eq!(charged_amounts.len(), 2);
		assert!(charged_amounts.contains(&(
			to_account_32(&alice).expect("Invalid contract account id"),
			MIN_EXISTENTIAL_DEPOSIT * 10
		)));
		assert!(charged_amounts.contains(&(
			to_account_32(&bob).expect("Invalid contract account id"),
			MIN_EXISTENTIAL_DEPOSIT * 5
		)));
	}
}
