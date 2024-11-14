#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, v2::*};
use frame_system::RawOrigin;
use sp_io::hashing::blake2_256;
use sp_runtime::{SaturatedConversion, Saturating};
use sp_std::vec;

use super::*;
use crate::EraActivity;
#[allow(unused)]
use crate::Pallet as DdcVerification;

#[benchmarks]
mod benchmarks {
	use super::*;

	fn create_validator_account<T: Config>() -> T::AccountId {
		let validator = create_account::<T>("validator_account", 0, 0);
		<DdcVerification<T> as ValidatorVisitor<T>>::setup_validators(vec![(
			validator.clone(),
			validator.clone(),
		)]);
		validator
	}

	fn create_account<T: Config>(name: &'static str, idx: u32, seed: u32) -> T::AccountId {
		account::<T::AccountId>(name, idx, seed)
	}

	fn assert_has_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
		frame_system::Pallet::<T>::assert_has_event(generic_event.into());
	}

	fn endow_account<T: Config>(account: &T::AccountId, amount: u128) {
		let balance = amount.saturated_into::<BalanceOf<T>>();
		let _ = T::Currency::make_free_balance_be(account, balance);
	}

	#[benchmark]
	fn set_prepare_era_for_payout(b: Linear<1, 5>) {
		let validator = create_validator_account::<T>();
		let cluster_id = ClusterId::from([1; 20]);
		let era = EraActivity { id: 1, start: 1000, end: 2000 };

		let payers_merkle_root_hash = blake2_256(&1.encode());
		let payees_merkle_root_hash = blake2_256(&2.encode());

		let mut payers_batch_merkle_root_hashes = vec![];
		let mut payees_batch_merkle_root_hashes = vec![];

		for i in 0..b {
			payers_batch_merkle_root_hashes.push(blake2_256(&(i + 10).encode()));
			payees_batch_merkle_root_hashes.push(blake2_256(&(i + 100).encode()))
		}

		#[extrinsic_call]
		set_prepare_era_for_payout(
			RawOrigin::Signed(validator),
			cluster_id,
			era.clone(),
			payers_merkle_root_hash,
			payees_merkle_root_hash,
			payers_batch_merkle_root_hashes,
			payees_batch_merkle_root_hashes,
		);

		assert!(<EraValidations<T>>::contains_key(cluster_id, era.id));
		assert_has_event::<T>(Event::EraValidationReady { cluster_id, era_id: era.id }.into());
	}

	#[benchmark]
	fn set_validator_key() {
		let validator = create_account::<T>("new_validator_account", 0, 0);
		endow_account::<T>(&validator, 10000000000000000);

		let min_bond = T::ValidatorStaking::minimum_validator_bond();
		let bond = min_bond.saturating_add(T::Currency::minimum_balance());

		T::ValidatorStaking::bond(&validator, bond, &validator).expect("Bond to be created");

		ValidatorSet::<T>::put(vec![validator.clone()]);

		#[extrinsic_call]
		set_validator_key(RawOrigin::Signed(validator.clone()), validator.clone());

		assert!(<ValidatorToStashKey<T>>::contains_key(validator.clone()));
		assert_has_event::<T>(Event::ValidatorKeySet { validator }.into());
	}

	#[benchmark]
	fn emit_consensus_errors(b: Linear<1, 5>) {
		let validator = create_validator_account::<T>();
		let mut errros = vec![];

		for _i in 0..b {
			errros.push(OCWError::SendChargingCustomersBatchTransactionError {
				cluster_id: ClusterId::from([1; 20]),
				era_id: 1,
				batch_index: 0,
			});
		}

		#[extrinsic_call]
		emit_consensus_errors(RawOrigin::Signed(validator.clone()), errros);
	}

	impl_benchmark_test_suite!(DdcVerification, crate::mock::new_test_ext(), crate::mock::Test);
}
