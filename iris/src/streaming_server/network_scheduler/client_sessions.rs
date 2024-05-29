use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use log::{info, warn, error};
use mio::net::UdpSocket;

use std::time::{Duration, Instant};
use std::io;

use mio::{Events, Interest, Poll, Token};

// Time interval for checking and removing timed-out clients
const CLEANUP_INTERVAL: Duration = Duration::from_secs(5); // Adjust as needed


// A token to allow us to identify which event is for the `UdpSocket`.
const UDP_SOCKET: Token = Token(0);

pub struct ClientSessions {
    clients: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn update_client(&mut self, client_addr: SocketAddr) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(client_addr, Instant::now());
        // info!("Updated clients: {:?}", *clients);
    }

    fn remove_timed_out_clients(&mut self) {
        let timeout = Duration::from_secs(5);
        let now = Instant::now();
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|_, &mut last_seen| now.duration_since(last_seen) <= timeout);
        // info!("Clients after timeout removal: {:?}", *clients);
    }

    pub fn start_receiver_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create a poll instance.
        let mut poll = Poll::new()?;
        // Create storage for events. Since we will only register a single socket, a
        // capacity of 1 will do.
        let mut events = Events::with_capacity(1);

        // Setup the UDP socket.
        let addr = "0.0.0.0:9000".parse().unwrap();

        let mut socket = UdpSocket::bind(addr)?;

        // Register our socket with the token defined above and an interest in being
        // `READABLE`.
        poll.registry()
            .register(&mut socket, UDP_SOCKET, Interest::READABLE)?;

        // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
        // packet, which is the maximum value of 16 a bit integer.
        let mut buf = [0; 1 << 16];

        // Our event loop.
        loop {

            // Poll to check if we have events waiting for us.
            if let Err(err) = poll.poll(&mut events, None) {
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
                        // In this loop we receive all packets queued for the socket.
                        match socket.recv_from(&mut buf) {
                            Ok((_, source_address)) => {
                                info!("packet received {}",source_address);
                                // Insert or update client in the hashmap
                                self.update_client(source_address);
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // If we get a `WouldBlock` error we know our socket
                                // has no more packets queued, so we can return to
                                // polling and wait for some more.
                                break;
                            }
                            Err(e) => {
                                // If it was any other kind of error, something went
                                // wrong and we terminate with an error.
                                return Err(Box::new(e));
                            }
                        }
                    },
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
