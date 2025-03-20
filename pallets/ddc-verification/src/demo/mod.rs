pub mod v1 {
	use ddc_primitives::traits::InspReceiptsInterceptor;
	use scale_info::prelude::{format, string::String, vec::Vec};
	use sp_core::offchain::StorageKind;
	use sp_io::offchain::local_storage_get;

	use crate::HashedInspPathReceipt;

	pub struct DemoReceiptsInterceptor;
	impl InspReceiptsInterceptor for DemoReceiptsInterceptor {
		type Receipt = HashedInspPathReceipt;

		fn intercept(receipts: Vec<HashedInspPathReceipt>) -> Vec<HashedInspPathReceipt> {
			let receipts = if Self::is_lagging_actor() {
				receipts
					.into_iter()
					.enumerate()
					.filter_map(|(i, receipt)| {
						if i % 2 == 0 {
							Some(receipt)
						} else {
							log::info!(
								"🦥 Path {:?} is skipped",
								format!("0x{}", hex::encode(&receipt.path_id.0[..])),
							);
							None
						}
					})
					.collect::<Vec<_>>()
			} else {
				receipts
			};

			let receipts = if Self::is_bad_actor() {
				receipts
					.into_iter()
					.map(|mut receipt| {
						let seed = sp_io::offchain::random_seed();
						let bad_hash = format!("0x{}", hex::encode(&seed[..]));
						receipt.hash = bad_hash;
						log::info!(
							"😈 Bad hash of Path {:?} is {:?}",
							format!("0x{}", hex::encode(&receipt.path_id.0[..])),
							receipt.hash
						);
						receipt
					})
					.collect::<Vec<_>>()
			} else {
				receipts
			};

			receipts
		}
	}

	impl DemoReceiptsInterceptor {
		pub(crate) fn is_bad_actor() -> bool {
			let bad_actor_key = String::from("offchain::insp::demo::v1::is_bad").into_bytes();
			let encoded_data = match local_storage_get(StorageKind::PERSISTENT, &bad_actor_key) {
				Some(data) => data,
				None => return false,
			};
			if let Some(is_bad_actor) = encoded_data.get(0) {
				if *is_bad_actor == 0x01 {
					log::info!("😈😈😈 Intercepted by Bad Actor !");
					true
				} else {
					false
				}
			} else {
				false
			}
		}

		pub(crate) fn is_lagging_actor() -> bool {
			let lagging_actor_key =
				String::from("offchain::insp::demo::v1::is_lagging").into_bytes();
			let encoded_data = match local_storage_get(StorageKind::PERSISTENT, &lagging_actor_key)
			{
				Some(data) => data,
				None => return false,
			};
			if let Some(is_lagging_actor) = encoded_data.get(0) {
				if *is_lagging_actor == 0x01 {
					log::info!("🦥🦥🦥 Intercepted by Lagging Actor !");
					true
				} else {
					false
				}
			} else {
				false
			}
		}
	}
}
