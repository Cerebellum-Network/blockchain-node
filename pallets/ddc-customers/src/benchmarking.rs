//! DdcStaking pallet benchmarking.

use super::*;
use crate::Pallet as DdcCustomers;
use ddc_primitives::{BucketId, ClusterId};
use pallet_ddc_clusters::{Pallet as DdcClusters, 	cluster::{Cluster, ClusterGovParams, ClusterParams}};
use sp_runtime::Perbill;
use pallet_contracts::chain_extension::UncheckedFrom;

pub type BalanceOf<T> =
	<<T as pallet_ddc_clusters::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// use testing_utils::*;

use sp_std::prelude::*;

pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;

const USER_SEED: u32 = 999666;

benchmarks! {
  where_clause { where T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]> }
	create_bucket {
		let cluster_id = ClusterId::from([1; 20]);
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);

    let cluster_gov_params: ClusterGovParams<BalanceOf<T>, T::BlockNumber> = ClusterGovParams {
			treasury_share: Perbill::default(),
			validators_share: Perbill::default(),
			cluster_reserve_share: Perbill::default(),
			cdn_bond_size: 100u32.into(),
			cdn_chill_delay: 50u32.into(),
			cdn_unbonding_delay: 50u32.into(),
			storage_bond_size: 100u32.into(),
			storage_chill_delay: 50u32.into(),
			storage_unbonding_delay: 50u32.into(),
			unit_per_mb_stored: 10,
			unit_per_mb_streamed: 10,
			unit_per_put_request: 10,
			unit_per_get_request: 10,
		};

    pallet_ddc_clusters::Pallet::<T>::create_cluster(
      RawOrigin::Signed(user.clone()).into(),
      ClusterId::from([1; 20]),
      user.clone(),
      user.clone(),
      ClusterParams { node_provider_auth_contract: user.clone() },
      cluster_gov_params
    ).unwrap();

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user), cluster_id)
	verify {
		assert_eq!(Pallet::<T>::buckets_count(), 1);
	}

	deposit {
		let user = account::<T::AccountId>("user", USER_SEED, 0u32);
		let balance = <T as pallet::Config>::Currency::minimum_balance() * 100u32.into();
		let _ = <T as pallet::Config>::Currency::make_free_balance_be(&user, balance);
		let amount = <T as pallet::Config>::Currency::minimum_balance() * 50u32.into();

		whitelist_account!(user);
	}: _(RawOrigin::Signed(user.clone()), amount)
	verify {
		assert!(Ledger::<T>::contains_key(user));
	}


	// set_node_params {
	//   let (user, node, cdn_node_params, cdn_node_params_new) = create_user_and_config::<T>("user", USER_SEED);

	// DdcNodes::<T>::create_node(RawOrigin::Signed(user.clone()).into(),node.clone(), cdn_node_params)?;

	// whitelist_account!(user);
	// }: _(RawOrigin::Signed(user.clone()), node, cdn_node_params_new)
	// verify {
	//   assert_eq!(CDNNodes::<T>::try_get(
	//   CDNNodePubKey::new([0; 32])).unwrap().props,
	// 	CDNNodeProps {
	// 		host: vec![2u8, 255].try_into().unwrap(),
	// 		http_port: 45000u16,
	// 		grpc_port: 55000u16,
	// 		p2p_port: 65000u16,
	// 	});
	// }

  impl_benchmark_test_suite!(
    DdcCustomers,
    crate::mock::ExtBuilder::default().build(),
    crate::mock::Test,
  );
}
