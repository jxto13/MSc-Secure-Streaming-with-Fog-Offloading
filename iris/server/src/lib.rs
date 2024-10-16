// lib.rs
use std::fs;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub ca: CA,
    pub grpc: GRPC,
    pub multicast: Multicast,
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
pub struct GRPC {
    pub port: u16,
}

#[derive(Deserialize, Debug)]
pub struct Multicast {
    // Strean Config
    pub data_source: String,
    pub ip_source: String,
    pub interface: String,
    pub ip: String,
    pub port: u16,
    // Matrix Config
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
pub struct Prometheus {
    pub ip: String
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub timeout_clients: u64
}

pub fn read_config(filename: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(filename)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}


