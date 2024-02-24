use frame_system::Config;
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD

use crate::{
	BatchIndex, BucketId, BucketUsage, ClusterId, DdcEra, MMRProof, NodePubKey, NodeUsage,
};
=======
=======
use scale_info::prelude::string::String;
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
#[cfg(feature = "runtime-benchmarks")]
use sp_std::prelude::*;
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
=======
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
#[cfg(feature = "runtime-benchmarks")]
use scale_info::prelude::vec::Vec;
>>>>>>> 8a36f637 (fix: clippy and formatiing)
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)

use crate::{
	BatchIndex, BucketId, BucketUsage, ClusterId, DdcEra, MMRProof, NodePubKey, NodeUsage,
};
=======
use sp_runtime::Percent;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)

pub trait ValidatorVisitor<T: Config> {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
=======
	#[cfg(feature = "runtime-benchmarks")]
<<<<<<< HEAD
>>>>>>> 80f9b2bb (chore: retrieving verification key from the keystore)
	fn setup_validators(validators: Vec<T::AccountId>);
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
	fn setup_validators(validators_with_keys: Vec<(T::AccountId, T::AccountId)>);
	#[cfg(feature = "runtime-benchmarks")]
	fn setup_validation_era(
		cluster_id: ClusterId,
		era_id: DdcEra,
		era_validation: EraValidation<T>,
	);
>>>>>>> e2d1813f (fix: benchmarking is fixed for payouts pallet)
=======
>>>>>>> 54e582cd (refactor: ddc-verification benchmarks)
	fn is_ocw_validator(caller: T::AccountId) -> bool;
<<<<<<< HEAD
	fn is_customers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
		max_batch_index: BatchIndex,
		payers: &[(NodePubKey, BucketId, BucketUsage)],
<<<<<<< HEAD
=======
		payers: &[(T::AccountId, BucketId, CustomerUsage)],
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		payers: &[(T::AccountId, String, BucketId, CustomerUsage)],
>>>>>>> b2f51555 (wip: verifying customers batch hash in merkle path)
=======
=======
		max_batch_index: BatchIndex,
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
		payers: &[(NodePubKey, BucketId, CustomerUsage)],
>>>>>>> 04bac35e (chore: fetching bucket owner from the bucket data during payout)
=======
>>>>>>> 411b1b73 (refactor: 'CustomerUsage' is renamed to 'BucketUsage')
		batch_proof: &MMRProof,
	) -> bool;
	fn is_providers_batch_valid(
		cluster_id: ClusterId,
		era: DdcEra,
		batch_index: BatchIndex,
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 0c3d2872 (chore: calculating mmr size based on max batch index)
		max_batch_index: BatchIndex,
		payees: &[(NodePubKey, NodeUsage)],
=======
		payees: &[(T::AccountId, NodeUsage)],
>>>>>>> 33240126 (OCW-DAC-Validation changes (#397))
=======
		payees: &[(T::AccountId, String, NodeUsage)],
>>>>>>> acbb6a47 (fix: checking MMR proof for a batch)
=======
		payees: &[(NodePubKey, NodeUsage)],
>>>>>>> c054878f (chore: fetching node provider from the node data during payout)
		batch_proof: &MMRProof,
	) -> bool;
=======
	fn is_quorum_reached(quorum: Percent, members_count: usize) -> bool;
>>>>>>> 67b2560f (feat: input for payout batches is decoupled from the verified delta usage and merged with the current usage)
}
