use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use crossbeam_channel::Receiver;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};
// use pnet_datalink::interfaces;
use log::{info, error};

use crate::metrics::get_metrics;
use crate::CONFIG;

// A token to allow us to identify which event is for the `UdpSocket`.
const SERVER: Token = Token(0);

/// A struct representing a WiFi Unicast sender.
pub struct WiFiUnicast {
}

impl WiFiUnicast {
    /// Creates a new instance of `WiFiUnicast` with a specified bitrate.
    pub fn new() -> Self {
        Self {
        }
    }
}

// create a mio pooling group, create a queue for the unicast packet, wawit for the socket to be ready to write, send the packet


