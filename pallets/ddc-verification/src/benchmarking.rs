use ddc_primitives::{
	ClusterId, ClusterParams, ClusterProtocolParams, NodePubKey, DOLLARS as CERE,
};
use frame_benchmarking::{account, v2::*};
use frame_system::RawOrigin;
use sp_runtime::{AccountId32, Perquintill, SaturatedConversion, Saturating};
use sp_std::vec;

use super::*;
#[allow(unused)]
use crate::Pallet as DdcVerification;

#[benchmarks]
mod benchmarks {
	use super::*;

	fn create_validator_account<T: Config>() -> T::AccountId {
		let validator = create_account::<T>("validator_account", 0, 0);
		ValidatorToStashKey::<T>::insert(validator.clone(), validator.clone());
		ValidatorSet::<T>::put(vec![validator.clone()]);
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

	fn create_cluster<T: Config>(
		cluster_id: ClusterId,
		cluster_manager_id: T::AccountId,
		cluster_reserve_id: T::AccountId,
		cluster_params: ClusterParams<T::AccountId>,
		cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>>,
	) {
		T::ClusterCreator::create_cluster(
			cluster_id,
			cluster_manager_id,
			cluster_reserve_id,
			cluster_params,
			cluster_protocol_params,
		)
		.expect("Cluster is not created");
	}

	fn create_default_cluster<T: Config>(cluster_id: ClusterId) {
		let cluster_manager = create_account::<T>("cm", 0, 0);
		let cluster_reserve = create_account::<T>("cr", 0, 0);
		let cluster_params = ClusterParams {
			node_provider_auth_contract: Default::default(),
			erasure_coding_required: 4,
			erasure_coding_total: 6,
			replication_total: 3,
		};
		let cluster_protocol_params: ClusterProtocolParams<BalanceOf<T>, BlockNumberFor<T>> =
			ClusterProtocolParams {
				treasury_share: Perquintill::from_percent(5),
				validators_share: Perquintill::from_percent(10),
				cluster_reserve_share: Perquintill::from_percent(15),
				unit_per_mb_stored: CERE,
				unit_per_mb_streamed: CERE,
				unit_per_put_request: CERE,
				unit_per_get_request: CERE,
				..Default::default()
			};

		create_cluster::<T>(
			cluster_id,
			cluster_manager,
			cluster_reserve,
			cluster_params,
			cluster_protocol_params,
		);
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

	#[benchmark]
	fn force_skip_inspection() {
		let cluster_id = ClusterId::from([1; 20]);
		let payment_era = 1;
		let collector_key = NodePubKey::StoragePubKey(AccountId32::from([0u8; 32]));
		let ehd_id = EHDId(cluster_id, collector_key, payment_era);

		create_default_cluster::<T>(cluster_id);

		#[extrinsic_call]
		force_skip_inspection(RawOrigin::Root, cluster_id, ehd_id.into());
	}
}
