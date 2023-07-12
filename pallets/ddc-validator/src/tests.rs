use crate::mock::*;
use frame_support::{assert_ok, traits::{OffchainWorker, OnInitialize}};
use sp_runtime::DispatchResult;
use crate::mock::Timestamp;
use sp_core::{
    offchain::{testing, OffchainWorkerExt, OffchainDbExt, TransactionPoolExt},
	crypto::{KeyTypeId}
};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use std::sync::Arc;

#[test]
fn save_validated_data_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(DispatchResult::Ok(()));
	});
}

const PHRASE: &str = 
"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dacv");

#[test]
fn it_triggers_offchain_worker() {

	let mut t = new_test_ext();

	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainDbExt::new(offchain.clone()));
	t.register_extension(OffchainWorkerExt::new(offchain));
	
    let keystore = KeyStore::new();
    keystore
        .sr25519_generate_new(KEY_TYPE, Some(PHRASE))
        .unwrap();
    t.register_extension(KeystoreExt(Arc::new(keystore)));

	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
    t.register_extension(TransactionPoolExt::new(pool));

	{
		let mut state = offchain_state.write();

		let mut expect_request = |url: &str, response: &[u8]| {
			state.expect_request(testing::PendingRequest {
				method: "GET".into(),
				uri: url.to_string(),
				response: Some(response.to_vec()),
				sent: true,
				..Default::default()
			});
		};

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:aggregation:nodes:132855/$.0101010101010101010101010101010101010101010101010101010101010101",
			include_bytes!("./mock_data/aggregation:nodes:era:edge.json")
		);

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:data:file:84640a53-fc1f-4ac5-921c-6695056840bc",
			include_bytes!("./mock_data/data:file:84640a53-fc1f-4ac5-921c-6695056840bc.json")
		);

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:data:file:d0a55c8b-fcb9-41b5-aa9a-8b40e9c4edf7",
			include_bytes!("./mock_data/data:file:d0a55c8b-fcb9-41b5-aa9a-8b40e9c4edf7.json")
		);

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:data:file:80a62530-fd76-40b5-bc53-dd82365e89ce",
			include_bytes!("./mock_data/data:file:80a62530-fd76-40b5-bc53-dd82365e89ce.json")
		);

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:data:file:6e86ac08-4af9-4353-9fec-0f3e563661d6",
			include_bytes!("./mock_data/data:file:6e86ac08-4af9-4353-9fec-0f3e563661d6.json")
		);

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:data:file:7af575b7-9a83-40b6-88a7-19549b9bbc38",
			include_bytes!("./mock_data/data:file:7af575b7-9a83-40b6-88a7-19549b9bbc38.json")
		);

		expect_request(
			"http://redis:6379/FCALL/save_validation_result_by_node/1/d2bf4b844dfefd6772a8843e669f943408966a977e3ae2af1dd78e0f55f4df67:0101010101010101010101010101010101010101010101010101010101010101:3/%7B%22data%22%3A%22eyJlZGdlIjoiMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMTAxMDEwMSIsInJlc3VsdCI6dHJ1ZSwicGF5bG9hZCI6WzE5OCwxMTIsMzAsNywxNzksMTE2LDExNiwxODIsMjE2LDE4OCw3NSwxNDgsMTcsMTYwLDI1MSwxNTcsMTQzLDE3NiwxOTEsMTQyLDE4OCwxNTcsOTYsMjIsMTU0LDE2OCwxMTYsMTE1LDM3LDIyMiw0OSw2NV0sInRvdGFscyI6eyJyZWNlaXZlZCI6ODAwLCJzZW50Ijo4MDAsImZhaWxlZF9ieV9jbGllbnQiOjAsImZhaWx1cmVfcmF0ZSI6MH19%22%2C%22result%22%3Atrue%7D",
			include_bytes!("./mock_data/fcall:save:validation.json")
		);

		expect_request(
			"http://redis:6379/JSON.GET/ddc:dac:shared:nodes:3",
			include_bytes!("./mock_data/shared:nodes:era.json")
		);

	}	

	t.execute_with(|| {

		System::set_block_number(1); // required for randomness

		Timestamp::set_timestamp(1_672_531_200_000 + 120_000 * 2);
		DdcValidator::on_initialize(2);

		Timestamp::set_timestamp(1_672_531_200_000 + 120_000 * 4);
		DdcValidator::offchain_worker(3);

	})
}


