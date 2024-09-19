use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Retrieve the OUT_DIR environment variable, or return an error if it's not set
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // Generate the descriptor for streaming_server.proto
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("streaming_server_descriptor.bin"))
        .compile(&["proto/streaming_server.proto"], &["proto"])?;
    
    // Generate the descriptor for edge_client.proto
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("edge_client_descriptor.bin"))
        .compile(&["proto/edge_client.proto"], &["proto"])?;
    
    Ok(())
}
