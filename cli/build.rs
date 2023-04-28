fn main() {
	if let Ok(profile) = std::env::var("PROFILE") {
		println!("cargo:rustc-cfg=build_type=\"{}\"", profile);
	}
	substrate_build_script_utils::generate_cargo_keys();
}
