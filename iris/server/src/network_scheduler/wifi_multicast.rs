use iris_shared::bandwidth_utils::BandwidthManager;
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use crossbeam_channel::Receiver;
use pnet::ipnetwork::IpNetwork;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};
use pnet::datalink::interfaces;
use log::{info, error};

use crate::metrics::get_metrics;
use crate::CONFIG;

// A token to allow us to identify which event is for the `UdpSocket`.
const SERVER: Token = Token(0);


/// A struct representing a WiFi Multicast sender.
pub struct WiFiMulticast{
    bandwidth_manager: BandwidthManager,
}

impl WiFiMulticast {
    /// Creates a new instance of `WiFiMulticast`.
    pub fn new() -> Self {
        let max_bandwidth = CONFIG.get().unwrap().multicast.bitrate;
        Self {
            bandwidth_manager: BandwidthManager::new(max_bandwidth),
        }
    }

    /// Starts the multicast sender.
    ///
    /// # Arguments
    ///
    /// * `data_receiver` - A receiver for receiving data to be sent over multicast.
    /// * `retransmissions_rx` - A receiver for retransmitted data to be sent over multicast.
    pub fn server(&mut self, data_receiver: Receiver<Vec<u8>>, retransmissions_rx: Receiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
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

        let ip_multicast = &CONFIG.get().unwrap().multicast.ip_multicast;
        let addr = ip_multicast.parse::<SocketAddr>().unwrap();

        // Define the multicast address and port.
        let multicast_address = match addr.ip() {
            IpAddr::V4(ipv4) => ipv4,
            _ => panic!("Expected an IPv4 address"),
        };

        let multicast_port = addr.port();
        let multicast_dest = SocketAddr::new(IpAddr::V4(multicast_address), multicast_port);

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

        // Metrics for bandwidth logging.
        let metrics = get_metrics();
        let multicast_sender_rate = metrics.multicast_sender_rate.lock().unwrap();
        let retransmissions_sender_rate = metrics.retransmissions_sender_rate.lock().unwrap();

        // Start an event loop.
        loop {
            // Poll Mio for events, blocking until we get an event.
            if let Err(err) = poll.poll(&mut events, None) {
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(Box::new(err));
            }

            // Process each event.
            for event in events.iter() {
                match event.token() {
                    SERVER => {                                
                        // Handle data from both channels. If data is available on both channels, the retransmissions channel will be prioritized.
                        crossbeam_channel::select_biased! {
                            recv(retransmissions_rx) -> retransmission => {
                                match retransmission {
                                    Ok(data) => {
                                        // Send the retransmitted data over multicast.
                                        if let Err(e) = socket.send(&data) {
                                            error!("Failed to send retransmission over multicast: {}", e);
                                        }
                                        
                                        // Record the retransmission data sent.
                                        self.bandwidth_manager.record_retransmission(data.len());
                                        // Update the retransmissions sender rate.
                                        retransmissions_sender_rate.add(data.len() as f64); // Update the gauge value
                                        
                                    }
                                    Err(err) => {
                                        // Retransmission channel closed
                                        panic!("Retransmission channel closed\n{}",err);
                                    }
                                }
                            }
                            recv(data_receiver) -> data => {
                                match data {
                                    Ok(data) => {
                                        // Send the data over multicast.
                                        if let Err(e) = socket.send(&data) {
                                            error!("Failed to send data over multicast: {}", e);
                                        }
                                        
                                        // Record the multicast data sent.
                                        self.bandwidth_manager.record_multicast(data.len());
                                        // Update the multicast sender rate.
                                        multicast_sender_rate.add(data.len() as f64 ); // Update the gauge value
                                        
                                    }
                                    Err(err) => {
                                        // Data channel closed.
                                        panic!("Data channel closed.\n{}",err);
                                    }
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
