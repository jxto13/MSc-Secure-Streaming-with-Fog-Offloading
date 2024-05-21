use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("streaming_server_descriptor.bin"))
        .compile(&["proto/streaming_server.proto"], &["proto"])?;

    // Add code to generate descriptor for edge_client.proto
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("edge_client_descriptor.bin"))
        .compile(&["proto/edge_client.proto"], &["proto"])?;

    Ok(())
}
