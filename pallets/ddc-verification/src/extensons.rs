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

#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

#[cfg(feature = "std")]
sp_externalities::decl_extension! {
	pub struct CustomExt(u32);
}

// #[cfg(feature = "std")]
// pub type HostFunctions = (custom::HostFunctions,);

#[runtime_interface]
pub trait Custom {
	fn get_val(&mut self) -> Option<u32> {
		self.extension::<CustomExt>().map(|ext| ext.0)
	}
}
