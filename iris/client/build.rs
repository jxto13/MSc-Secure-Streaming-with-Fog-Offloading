fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure the output directory exists
    std::fs::create_dir_all("proto/generated")?;

    // Generate the descriptor for streaming_server.proto
    tonic_build::configure()
        .out_dir("proto/generated") // specify the output directory here
        .compile_protos(&["proto/streaming_server.proto"], &["proto"])?;

    tonic_build::configure()
        .out_dir("proto/generated") // specify the output directory here
        .compile_protos(&["proto/edge_client.proto"], &["proto"])?;

    Ok(())
}
