use std::{
    env,
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("../../proto");
    let proto_file = proto_root.join("item/v1/item.proto");
    let descriptor = PathBuf::from(env::var("OUT_DIR")?).join("item_v1_descriptor.bin");

    tonic_prost_build::configure()
        .file_descriptor_set_path(descriptor)
        .compile_protos(std::slice::from_ref(&proto_file), &[proto_root])?;

    println!("cargo:rerun-if-changed={}", proto_file.display());
    Ok(())
}
