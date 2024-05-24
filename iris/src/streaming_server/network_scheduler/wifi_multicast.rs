use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use crossbeam_channel::Receiver;
use std::net::{Ipv4Addr, SocketAddr};
use std::thread::{sleep, Thread};
use std::time::Duration;

use log::{info,error};

// A token to allow us to identify which event is for the `UdpSocket`.
const SERVER: Token = Token(0);

/// A struct representing a WiFi Multicast sender.
pub struct WiFiMulticast;

impl WiFiMulticast {
    /// Creates a new instance of `WiFiMulticast`.
    pub fn new() -> Self {
        Self
    }

    /// Starts the multicast sender.
    ///
    /// # Arguments
    ///
    /// * `data_receiver` - A receiver for receiving data to be sent over multicast.
    pub fn start_multicast_sender(&self, data_receiver: Receiver<Vec<u8>>) {
        // Setup the UDP socket.
        let addr = "0.0.0.0:9000".parse().unwrap();
        let mut socket = UdpSocket::bind(addr).unwrap();

        // Define the multicast address and port.
        let multicast_address = Ipv4Addr::new(224, 0, 0, 123);
        let multicast_port = 12345;
        let multicast_dest: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);

        // Join the multicast group.
        if let Err(e) = socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED) {
            eprintln!("Failed to join multicast group: {}", e);
            return;
        };

        socket.connect(multicast_dest);

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
            poll.poll(&mut events, Some(Duration::from_nanos(100_000))).unwrap();

            // Process each event.
            for event in events.iter() {
                // We can use the token we previously provided to `register` to
                // determine for which socket the event is.
                match event.token() {
                    SERVER => {
                        // Check if there is data to be sent over multicast.
                        // If there's data available, send it over multicast.
                        if let Ok(data) = data_receiver.recv() {
                            // Send the data over multicast.
                            if let Err(e) = socket.send(&data) {
                                error!("Failed to send data over multicast: {}", e);
                            }
                            sleep(Duration::from_micros(1_000_00));
                        }
                    }
                    // We don't expect any events with tokens other than those we provided.
                    _ => unreachable!(),
                }
            }
        }
    }
}
