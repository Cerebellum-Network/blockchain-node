#![cfg(feature = "runtime-benchmarks")]
use super::*;
use sp_std::vec;

#[allow(unused)]
use crate::Pallet as DdcVerification;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_billing_reports() {
		let cluster_id = ClusterId::from([1; 20]);
		let era : DdcEra = 1;
		let merkel_root_hash: MmrRootHash = array_bytes::hex_n_into_unchecked(
			"95803defe6ea9f41e7ec6afa497064f21bfded027d8812efacbdf984e630cbdc"
		);
		let caller: T::AccountId = whitelisted_caller();
		#[extrinsic_call]
		create_billing_reports(RawOrigin::Signed(caller), cluster_id, era, merkel_root_hash, vec![]);

		assert!(ActiveBillingReports::<T>::contains_key(cluster_id, era));
		let billing_report = ActiveBillingReports::<T>::get(cluster_id, era).unwrap();
		assert_eq!(billing_report.merkle_root_hash, merkel_root_hash);
	}

	impl_benchmark_test_suite!(DdcVerification, crate::mock::new_test_ext(), crate::mock::Test);
}