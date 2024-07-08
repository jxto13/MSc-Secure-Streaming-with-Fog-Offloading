use std::fs;

use serde::Deserialize;


// Import protobuff
pub mod proto {
    tonic::include_proto!("streaming_server");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("streaming_server_descriptor");
}

#[derive(Deserialize)]
pub struct Config {
    pub grpc_service_port: u16,
    pub multicast_ip: String,
    pub url_stream: String,
    pub port: u16,
}

pub fn read_config(filename: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(filename)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}