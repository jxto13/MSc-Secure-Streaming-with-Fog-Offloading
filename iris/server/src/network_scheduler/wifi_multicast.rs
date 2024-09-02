use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use crossbeam_channel::Receiver;
use pnet::ipnetwork::IpNetwork;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};
use pnet_datalink::interfaces;
use log::{info, error};

use crate::CONFIG;

// A token to allow us to identify which event is for the `UdpSocket`.
const SERVER: Token = Token(0);

/// A struct representing a simple bandwidth manager.
pub struct BandwidthManager {
    pub bitrate: usize,              // Maximum bits per second
    bytes_per_second: usize,         // Maximum bytes per second
    interval: Duration,              // Time interval for rate limiting
    last_check: Instant,             // Last time we checked the bandwidth usage
    bytes_sent: usize,               // Bytes sent in the current interval
    packets_sent: usize,             // Number of packets sent
}

impl BandwidthManager {
    /// Creates a new bandwidth manager with a specified bitrate.
    pub fn new(bitrate: usize) -> Self {
        let bytes_per_second = bitrate / 8;
        let interval = Duration::from_secs(1);
        Self {
            bitrate,
            bytes_per_second,
            interval,
            last_check: Instant::now(),
            bytes_sent: 0,
            packets_sent: 0,
        }
    }

    /// Checks if we can send the given number of bytes without exceeding the bitrate.
    /// If the rate limit is exceeded, it waits until it can send the data.
    pub fn wait_until_can_send(&mut self, bytes: usize) {
        loop {
            let now = Instant::now();
            if now.duration_since(self.last_check) >= self.interval {
                self.last_check = now;
                info!("Bandwidth Usage: {} Mbit/s", (self.bytes_sent * 8) as f64 / 1_000_000.0);
                // Reset bytes_sent outside of the loop
                self.bytes_sent = 0;
            }
            
            if self.bytes_sent + bytes <= self.bytes_per_second {
                self.bytes_sent += bytes;
                self.packets_sent += 1;
                break;
            } else {
                // Calculate how long to sleep to avoid busy-waiting.
                let remaining_time = self.interval - now.duration_since(self.last_check);
                sleep(remaining_time);
            }
        }
    }    
}

/// A struct representing a WiFi Multicast sender.
pub struct WiFiMulticast {
    pub bandwidth_manager: BandwidthManager,
}

impl WiFiMulticast {
    /// Creates a new instance of `WiFiMulticast` with a specified bitrate.
    pub fn new() -> Self {
        let max_bandwidth = CONFIG.get().unwrap().multicast.max_bandwidth * 1_000_000;
        Self {
            bandwidth_manager: BandwidthManager::new(max_bandwidth),
        }
    }

    /// Starts the multicast sender.
    ///
    /// # Arguments
    ///
    /// * `data_receiver` - A receiver for receiving data to be sent over multicast.
    pub fn server(&mut self, data_receiver: Receiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        // Get the list of network interfaces.
        let interfaces = interfaces();

        // Find the specific network interface by name.
        let interface = interfaces.into_iter()
            .find(|iface| iface.name == CONFIG.get().unwrap().multicast.interface)
            .expect("Failed to get the network interface");

        // Initialize the IPv4 address variable.
        let mut ipv4_address = None;

        // Iterate through the IPs of the interface to find the IPv4 address.
        for ip in interface.ips {
            match ip {
                IpNetwork::V4(ipv4) => {
                    ipv4_address = Some(ipv4.ip());
                    break; // Exit the loop once an IPv4 address is found.
                }
                IpNetwork::V6(_) => (),
            }
        }

        // Ensure an IPv4 address was found.
        let ipv4_address = ipv4_address.expect("No IPv4 address found for the interface");

        // Setup the UDP socket and bind it to the specific IPv4 address.
        let addr = SocketAddr::new(IpAddr::V4(ipv4_address), 0);
        let mut socket = UdpSocket::bind(addr)?;

        // Define the multicast address and port.
        let multicast_address = Ipv4Addr::from_str(&CONFIG.get().unwrap().multicast.ip).unwrap();
        let multicast_port = CONFIG.get().unwrap().multicast.port;
        let multicast_dest = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);
    
        // Join the multicast group.
        if let Err(e) = socket.join_multicast_v4(&multicast_address, &ipv4_address) {
            error!("Failed to join multicast group: {}", e);
            return Err(Box::new(e));
        };

        socket.connect(multicast_dest)?;

        // Create a poll instance.
        let mut poll = Poll::new().unwrap();

        // Register the socket with the poll instance.
        poll.registry()
            .register(&mut socket, SERVER, Interest::WRITABLE).unwrap();

        // Create storage for events.
        let mut events = Events::with_capacity(1024);

        // Start an event loop.
        loop {
            // Poll Mio for events, blocking until we get an event.
            // Poll to check if we have events waiting for us.
            if let Err(err) = poll.poll(&mut events, None) {
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(Box::new(err));
            }

            // Process each event.
            for event in events.iter() {
                // We can use the token we previously provided to `register` to
                // determine for which socket the event is.
                match event.token() {
                    SERVER => {
                        // Check if there is data to be sent over multicast.
                        // If there's data available, send it over multicast.
                        match data_receiver.recv() {
                            Ok(data) => {
                                // Wait until we can send this data without exceeding the rate limit.
                                self.bandwidth_manager.wait_until_can_send(data.len());
                                
                                // Send the data over multicast.
                                if let Err(e) = socket.send(&data) {
                                    error!("Failed to send data over multicast: {}", e);
                                }
                            }
                            Err(_) => {
                                // Channel is closed, handle the error or break out of the loop.
                                info!("Packets Sent: {}", self.bandwidth_manager.packets_sent);
                                error!("Data channel closed, exiting loop");
                                break; // Exit the loop if the channel is closed
                            }
                        }
                    }
                    // We don't expect any events with tokens other than those we provided.
                    _ => unreachable!(),
                }
            }
        }
    }
}
