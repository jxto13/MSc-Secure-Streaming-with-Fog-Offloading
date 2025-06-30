// lib.rs
use std::fs;

use serde::Deserialize;
use toml::de::Error as TomlError;
use std::error::Error;
use log::error; // using `log` crate

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
    pub port_multicast: u16,
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
    pub unicast_bitrate: usize,
    pub interval: usize,
    pub deadline: u64,
    pub port: usize,
}

#[derive(Deserialize, Debug)]
pub struct Prometheus {
    pub port: usize
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub timeout_clients: u64,
    pub client_reporting_interval: u64,
    pub client_sliding_window: u64
}


pub fn read_config(filename: &str) -> Result<Config, Box<dyn Error>> {
    let config_content = fs::read_to_string(filename)?;

    match toml::from_str::<Config>(&config_content) {
        Ok(config) => Ok(config),
        Err(err) => {
            log_toml_error(&err, &config_content);
            Err(Box::new(err))
        }
    }
}

fn log_toml_error(err: &TomlError, config_content: &str) {
    // General deserialization error message
    error!("Failed to deserialize config: {}", err);

    // Attempt to provide context by highlighting the line causing the error
    if let Some(span) = err.span() {
        let start = span.start;
        let lines: Vec<&str> = config_content.lines().collect();
        let mut byte_index = 0;

        for (i, line) in lines.iter().enumerate() {
            let next_index = byte_index + line.len() + 1; // +1 accounts for newline
            if start >= byte_index && start < next_index {
                error!("Error appears near line {}: {}", i + 1, line);
                break;
            }
            byte_index = next_index;
        }
    }
}
