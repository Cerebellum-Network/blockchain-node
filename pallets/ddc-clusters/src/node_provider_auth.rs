use codec::Encode;
use ddc_primitives::{NodePubKey, NodeType};
use frame_support::weights::Weight;
use hex_literal::hex;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::Hash;
use sp_std::{prelude::Vec, vec};

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
	pub contract_id: T::AccountId,
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
			pallet_contracts::Determinism::Deterministic,
		)
		.result
		.map_err(|_| NodeProviderAuthContractError::ContractCallFailed)?
		.data
		.first()
		.is_some_and(|x| *x == 1);

		Ok(is_authorized)
	}

	pub fn deploy_contract(
		&self,
		caller_id: T::AccountId,
	) -> Result<Self, NodeProviderAuthContractError> {
		pub const CTOR_SELECTOR: [u8; 4] = hex!("9bae9d5e");

		fn encode_constructor() -> Vec<u8> {
			let mut call_data = CTOR_SELECTOR.to_vec();
			let x = 0_u128;
			for _ in 0..9 {
				x.encode_to(&mut call_data);
			}
			call_data
		}

		// Load the contract code.
		let wasm = &include_bytes!("./test_data/node_provider_auth_white_list.wasm")[..];
		let _wasm_hash = <T as frame_system::Config>::Hashing::hash(wasm);
		let contract_args = encode_constructor();

		// Deploy the contract.
		let contract_id = pallet_contracts::Pallet::<T>::bare_instantiate(
			caller_id.clone(),
			Default::default(),
			EXTENSION_CALL_GAS_LIMIT,
			None,
			wasm.into(),
			contract_args,
			vec![],
			false,
		)
		.result
		.map_err(|_| NodeProviderAuthContractError::ContractDeployFailed)?
		.account_id;

		Ok(Self::new(contract_id, caller_id))
	}

	pub fn authorize_node(
		&self,
		node_pub_key: NodePubKey,
	) -> Result<bool, NodeProviderAuthContractError> {
		pub const ADD_DDC_NODE_SELECTOR: [u8; 4] = hex!("7a04093d");

		let call_data = {
			// is_authorized(node_provider: AccountId, node: Vec<u8>, node_variant: u8) -> bool
			let args: ([u8; 4], Vec<u8>) =
				(ADD_DDC_NODE_SELECTOR, node_pub_key.encode()[1..].to_vec());
			args.encode()
		};

		let _ = pallet_contracts::Pallet::<T>::bare_call(
			self.caller_id.clone(),
			self.contract_id.clone(),
			Default::default(),
			EXTENSION_CALL_GAS_LIMIT,
			None,
			call_data,
			false,
			pallet_contracts::Determinism::Deterministic,
		)
		.result
		.map_err(|_| NodeProviderAuthContractError::NodeAuthorizationNotSuccessful)?;

		Ok(true)
	}

	pub fn new(contract_id: T::AccountId, caller_id: T::AccountId) -> Self {
		Self { contract_id, caller_id }
	}
}

pub enum NodeProviderAuthContractError {
	ContractCallFailed,
	ContractDeployFailed,
	NodeAuthorizationNotSuccessful,
}
