use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/protos/activity.proto"], &["src/"])?;
    Ok(())
}
