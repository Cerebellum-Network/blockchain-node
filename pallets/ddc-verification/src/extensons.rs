use sp_runtime_interface::runtime_interface;

#[runtime_interface]
pub trait Images {
	fn decode_jpeg(data: &[u8]) -> Option<u32> {
		#[cfg(feature = "std")]
		{
			use std::collections::HashMap;

			let mut map: HashMap<String, u32> = HashMap::new();
			let key1 = String::from("111");
			map.insert(key1.clone(), 4u32);

			Some(map.get(&key1).unwrap_or(&10u32).clone())
		}

		#[cfg(not(feature = "std"))]
		{
			unimplemented!()
		}
	}
}
