use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use std::io;
use std::net::{IpAddr, SocketAddr};
// use pnet_datalink::interfaces;
use log::{info, error};

use crate::metrics::get_metrics;
use crate::CONFIG;

 // A token to allow us to identify which event is for the `UdpSocket`.
 const UDP_SOCKET: Token = Token(0);

/// A struct representing a WiFi Unicast sender.
pub struct WiFiUnicast {
}

impl WiFiUnicast {
    /// Creates a new instance of `WiFiUnicast` with a specified bitrate.
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn server(&mut self, rx: crossbeam_channel::Receiver<(Vec<u8>, IpAddr)>) -> io::Result<()> {

        // Create a poll instance.
        let mut poll = Poll::new()?;

        // Create storage for events. Since we will only register a single socket, a capacity of 1 will do.
        let mut events = Events::with_capacity(1);

        // Setup the UDP socket.
        let addr = "0.0.0.0:0".parse().unwrap();

        let mut socket = UdpSocket::bind(addr)?;

        // Register our socket with the token defined above and an interest in being `WRITABLE`.
        poll.registry().register(&mut socket, UDP_SOCKET, Interest::WRITABLE)?;

        let port = CONFIG.get().unwrap().unicast.port;

        // Get metrics 
        let metrics = get_metrics();

        // Our event loop.
        loop {
            // Poll to check if we have events waiting for us.
            if let Err(err) = poll.poll(&mut events, None) {
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }

            // Process each event.
            for event in events.iter() {
                // Validate the token we registered our socket with,
                // in this example it will only ever be one but we
                // make sure it's valid none the less.
                match event.token() {
                    UDP_SOCKET => {
                        
                        if let Ok(message) = rx.recv() {
                            let client = SocketAddr::new(message.1, port);
                            if let Err(err) = socket.send_to(&message.0, client) {
                                error!("Failed to send UDP message: {}", err);
                            } 

                            metrics.unicast_sender_rate.add(message.0.len() as i64);
                        }
                        
                    },
                    _ => {
                        // This should never happen as we only registered our
                        // `UdpSocket` using the `UDP_SOCKET` token, but if it ever
                        // does we'll log it.
                        log::warn!("Got event for unexpected token: {:?}", event);
                    }
                }
            }
        }
    }
}

// create a mio pooling group, create a queue for the unicast packet, wawit for the socket to be ready to write, send the packet


