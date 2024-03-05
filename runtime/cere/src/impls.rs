// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Some configurable implementations as associated type for the substrate runtime.

use core::marker::PhantomData;

use frame_support::traits::{Currency, OnUnbalanced};
use hex_literal::hex;
use pallet_session::SessionManager;
use sp_std::prelude::*;

use crate::{Authorship, Balances, NegativeImbalance};

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		if let Some(author) = Authorship::author() {
			Balances::resolve_creating(&author, amount);
		}
	}
}

pub struct CereSessionManager<T>(PhantomData<T>);
impl<T: frame_system::Config> SessionManager<T::AccountId> for CereSessionManager<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
{
	fn new_session(_new_index: sp_staking::SessionIndex) -> Option<Vec<T::AccountId>> {
		const VALIDATOR1: [u8; 32] =
			hex!("6ca3a3f6a78889ed70a6b46c2d621afcd3da2ea68e20a2eddd6f095e7ded586d");
		const VALIDATOR2: [u8; 32] =
			hex!("fa63378688e615e71b10ddb392482076b4e639c9d31181f370fe0858b8db7006");
		const VALIDATOR3: [u8; 32] =
			hex!("9e0e0270982a25080e436f7de803f06ed881b15209343c0dd16984dcae267406");
		const VALIDATOR4: [u8; 32] =
			hex!("0634cd2127a7bd444f7d004f78fa6ba771faa62991fa3f138c59926fd8cd971c");

		Some(vec![VALIDATOR1.into(), VALIDATOR2.into(), VALIDATOR3.into(), VALIDATOR4.into()])
	}

	fn end_session(_end_index: sp_staking::SessionIndex) {
		// Do nothing
	}

	fn start_session(_start_index: sp_staking::SessionIndex) {
		// Do nothing
	}
}
