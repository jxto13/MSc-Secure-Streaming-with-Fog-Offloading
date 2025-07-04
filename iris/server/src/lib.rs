// lib.rs
use std::fs;

use log::error;
use serde::Deserialize;
use std::error::Error;
use toml::de::Error as TomlError; // using `log` crate

#[derive(Deserialize, Debug)]
pub struct Config {
    pub ca: CA,
    pub server_cert: ServerCert,
    pub grpc: GRPC,
    pub unicast: Unicast,
    pub multicast: Multicast,
    pub retransmission: Retransmission,
    pub reinforcement_learning: ReinforcementLearning,
    pub prometheus: Prometheus,
    pub settings: Settings,
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
    pub port: u16,
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
    pub mtu_size: usize,
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
pub struct ReinforcementLearning {
    pub agent_data_path: String,
    pub learning_rate: f64,    // alpha
    pub discount_factor: f64,  // gamma
    pub exploration_rate: f64, // epsilon
}

#[derive(Deserialize, Debug)]
pub struct Prometheus {
    pub port: usize,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub timeout_clients: u64,
    pub client_reporting_interval: u64,
    pub client_sliding_window: u64,
}

#[derive(Deserialize, Debug)]
pub struct DynamicValues {
    pub fec_row_redundancy: usize,
    pub fec_col_redundancy: usize,
    pub data_row_size: usize,
    pub data_col_size: usize,
    pub block_size: usize,
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
    let err_msg = err.to_string();
    error!("Failed to deserialize config: {}", err_msg);

    // Look for missing field info in the error message (serde error messages often contain `missing field 'fieldname'`)
    let missing_fields: Vec<&str> = err_msg
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.starts_with("missing field") {
                // Extract the field name between single quotes
                let start_quote = part.find('\'')?;
                let end_quote = part[start_quote + 1..].find('\'')?;
                Some(&part[start_quote + 1..start_quote + 1 + end_quote])
            } else {
                None
            }
        })
        .collect();

    if !missing_fields.is_empty() {
        error!("Missing required field(s): {:?}", missing_fields);
    }

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
