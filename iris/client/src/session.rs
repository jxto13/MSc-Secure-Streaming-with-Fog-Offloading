use chrono::{DateTime, FixedOffset};

use openssl::ec::EcKey;
use openssl::pkey::Private;

use crate::proto::SessionKey;

#[derive(Debug, Clone, Default)]
pub struct ClientSession {
    pub current_key: SessionKey,
    pub next_key: SessionKey,
    pub ec_server_certificate: Vec<u8>,
    pub client_ec_certificate: Vec<u8>,
    pub client_ec_key: Option<EcKey<Private>>,
    pub server_time: DateTime<FixedOffset>,
    pub epoch: u32,
    pub client_reporting_interval: u32,
    pub client_sliding_window: u32,
    pub server_ip: String,
    pub retransmission_port: u32,
    pub unicast_port: u32,
    pub multicast_address: String, // This is the address for multicast packets
    pub multicast_port: u32,
}
