use codec::Encode;
use ddc_primitives::{NodePubKey, NodeType};
use frame_support::weights::Weight;
use pallet_contracts::chain_extension::UncheckedFrom;
use sp_std::prelude::Vec;

use crate::Config;

/// ink! 4.x selector for the "is_authorized" message, equals to the first four bytes of the
/// blake2("is_authorized"). See also: https://use.ink/basics/selectors#selector-calculation/,
/// https://use.ink/macros-attributes/selector/.
const INK_SELECTOR_IS_AUTHORIZED: [u8; 4] = [0x96, 0xb0, 0x45, 0x3e];

/// The maximum amount of weight that the cluster extension contract call is allowed to consume.
/// See also https://github.com/paritytech/substrate/blob/a3ed0119c45cdd0d571ad34e5b3ee7518c8cef8d/frame/contracts/rpc/src/lib.rs#L63.
const EXTENSION_CALL_GAS_LIMIT: Weight =
	Weight::from_ref_time(5_000_000_000_000).set_proof_size(u64::MAX);

pub struct NodeProviderAuthContract<T: Config> {
	contract_id: T::AccountId,
	caller_id: T::AccountId,
}

impl<T: Config> NodeProviderAuthContract<T>
where
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
	pub fn is_authorized(
		&self,
		node_provider_id: T::AccountId,
		node_pub_key: NodePubKey,
		node_type: NodeType,
	) -> Result<bool, NodeProviderAuthContractError> {
		let call_data = {
			// is_authorized(node_provider: AccountId, node: Vec<u8>, node_variant: u8) -> bool
			let args: ([u8; 4], T::AccountId, Vec<u8>, u8) = (
				INK_SELECTOR_IS_AUTHORIZED,
				node_provider_id,
				/* remove the first byte* added by SCALE */
				node_pub_key.encode()[1..].to_vec(),
				node_type.into(),
			);
			args.encode()
		};

		let is_authorized = pallet_contracts::Pallet::<T>::bare_call(
			self.caller_id.clone(),
			self.contract_id.clone(),
			Default::default(),
			EXTENSION_CALL_GAS_LIMIT,
			None,
			call_data,
			false,
		)
		.result
		.map_err(|_| NodeProviderAuthContractError::ContractCallFailed)?
		.data
		.first()
		.is_some_and(|x| *x == 1);

		Ok(is_authorized)
	}

	pub fn new(contract_id: T::AccountId, caller_id: T::AccountId) -> Self {
		Self { contract_id, caller_id }
	}
}

pub enum NodeProviderAuthContractError {
	ContractCallFailed,
}
