use std::io::Result;

fn main() -> Result<()> {
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
	let mut prost_build = prost_build::Config::new();
	prost_build.protoc_arg("--experimental_allow_proto3_optional");
	prost_build.compile_protos(&["src/protos/activity.proto"], &["src/"])?;
	Ok(())
=======
    prost_build::compile_protos(&["src/protos/activity.proto"], &["src/"])?;
    Ok(())
>>>>>>> a2bb827e (Compile protobuf)
=======
	prost_build::compile_protos(&["src/protos/activity.proto"], &["src/"])?;
=======
	let mut prost_build = prost_build::Config::new();
	prost_build.protoc_arg("--experimental_allow_proto3_optional");
	prost_build.compile_protos(&["src/protos/activity.proto"], &["src/"])?;
>>>>>>> 6c146e14 (Allow protobuf optional fields)
	Ok(())
>>>>>>> e157006b (cargo fmt)
}
