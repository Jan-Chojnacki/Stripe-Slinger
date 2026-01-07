use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    let repo_root = manifest_dir.join("../../../..");
    let proto_root = repo_root.join("proto");
    let proto_file = proto_root.join("metrics/v1/ingest.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&[proto_file], &[proto_root])?;

    Ok(())
}
