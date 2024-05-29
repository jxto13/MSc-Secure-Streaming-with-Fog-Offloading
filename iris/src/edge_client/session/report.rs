use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use log::{info, warn, error, debug};
use mio::net::UdpSocket;

use std::time::{Duration, Instant};
use std::{io, thread};

use mio::{Events, Interest, Poll, Token};

// A token to allow us to identify which event is for the `UdpSocket`.
const UDP_SOCKET: Token = Token(0);

pub struct ClientReport {
    last_send_time: Instant,
}

impl ClientReport {
    pub fn new() -> Self {
        Self {
            last_send_time: Instant::now(),
        }
    }

    pub fn start_report_client(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create a poll instance.
        let mut poll = Poll::new()?;
        // Create storage for events. Since we will only register a single socket, a
        // capacity of 1 will do.
        let mut events = Events::with_capacity(1);

        // Setup the UDP socket.
        let local_addr: SocketAddr = "0.0.0.0:0".parse()?;
        let remote_addr: SocketAddr = "192.168.0.66:9000".parse()?;
        let mut socket = UdpSocket::bind(local_addr)?;

        // Register our socket with the token defined above and an interest in being
        // `READABLE` and `WRITABLE`.
        poll.registry()
            .register(&mut socket, UDP_SOCKET, Interest::READABLE | Interest::WRITABLE)?;

        // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
        // packet, which is the maximum value of a 16-bit integer.
        let buf = [0; 1 << 16];

        // Our event loop.
        loop {
            // Poll to check if we have events waiting for us.
            if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(100))) {
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(Box::new(err));
            }

            // Process each event.
            for event in events.iter() {
                // Validate the token we registered our socket with,
                // in this example it will only ever be one but we
                // make sure it's valid none the less.
                match event.token() {
                    UDP_SOCKET => loop {
                        // Check if it's time to send the message.
                        if self.last_send_time.elapsed() >= Duration::from_secs(1) {
                            let empty_buf: Vec<u8> = vec![];

                            // Send data here
                            // For demonstration, let's just send an empty UDP packet
                            match socket.send_to(&empty_buf, remote_addr) {
                                Ok(_) => {
                                    info!("Data sent.");
                                    self.last_send_time = Instant::now();
                                }
                                Err(e) => {
                                    error!("Failed to send data: {:?}", e);
                                }
                            }
                        }
                    }
                    _ => {
                        // This should never happen as we only registered our
                        // `UdpSocket` using the `UDP_SOCKET` token, but if it ever
                        // does we'll log it.
                        warn!("Got event for unexpected token: {:?}", event);
                    }
                }
            }
        }
    }
}
