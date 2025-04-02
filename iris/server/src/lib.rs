// lib.rs
use std::fs;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub ca: CA,
    pub server_cert: ServerCert,
    pub grpc: GRPC,
    pub unicast: Unicast,
    pub multicast: Multicast,
    pub retransmission: Retransmission,
    pub prometheus: Prometheus,
    pub settings: Settings
}

#[derive(Deserialize, Debug)]
pub struct CA {
    pub c: String,
    pub st: String,
    pub o: String,
    pub cn: String,
}

#[derive(Deserialize, Debug)]
pub struct ServerCert {
    pub c: String,
    pub st: String,
    pub o: String,
    pub cn: String,
}

#[derive(Deserialize, Debug)]
pub struct GRPC {
    pub port: u16,
}

#[derive(Deserialize, Debug)]
pub struct Unicast {
    pub port: u16
}

#[derive(Deserialize, Debug)]
pub struct Multicast {
    // Stream Config
    pub data_source: String,
    pub ip_source: String,
    // Matrix Config
    pub interface: String,
    pub ip_multicast: String,
    pub bitrate: usize,
    pub row: usize,
    pub col: usize,
    pub row_fec: usize,
    pub col_fec: usize,

    pub epoch: usize,
       
    pub data_size: usize,
    pub shard_size: usize,
    pub mtu_size: usize
}

#[derive(Deserialize, Debug)]
pub struct Retransmission {
    pub wifi_multicast: bool,
    pub wifi_unicast: bool,
    pub interval: usize
}

#[derive(Deserialize, Debug)]
pub struct Prometheus {
    pub port: usize
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub timeout_clients: u64,
    pub transmissions_interval: u64
}

pub fn read_config(filename: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(filename)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}


