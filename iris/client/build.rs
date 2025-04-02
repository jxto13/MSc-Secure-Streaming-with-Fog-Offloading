
fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    // Generate the descriptor for streaming_server.proto
    tonic_build::configure().compile_protos(&["proto/streaming_server.proto"], &["proto"])?;
    tonic_build::configure().compile_protos(&["proto/edge_client.proto"], &["proto"])?;
    
    Ok(())
}
