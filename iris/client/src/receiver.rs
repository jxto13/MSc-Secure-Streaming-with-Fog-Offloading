use std::collections::HashMap;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use log::{error, info};
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};

macro_rules! create_enum_and_counters {
    (
        $enum_name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $enum_name {
            $($variant),*
        }

        #[derive(Debug, Default)]
        pub struct RetransmissionCounters {
            $(
                #[allow(non_snake_case)]
                pub $variant: usize,
            )*
        }

        impl RetransmissionCounters {
            pub fn increment(&mut self, receiver_type: $enum_name) {
                match receiver_type {
                    $( $enum_name::$variant => self.$variant += 1 ),*
                }
            }

            pub fn get(&self, receiver_type: $enum_name) -> usize {
                match receiver_type {
                    $( $enum_name::$variant => self.$variant ),*
                }
            }
        }
    };
}

create_enum_and_counters! {
    ReceiverType {
        Multicast,
        Unicast,
        WifiAware,
    }
}

#[derive(Debug)]
pub enum Command {
    AddSocket(UdpSocket, ReceiverType),
    RemoveSocket(Token),
}

#[derive(Debug)]
pub struct DynamicReceiver {
    poll: Poll,
    sockets: HashMap<Token, UdpSocket>,
    receiver_types: HashMap<Token, ReceiverType>,
    next_token_id: usize,
    counters: HashMap<Token, (u64, u64, Instant)>, // (packet_bytes_in_last_second, total_bytes, last_reset)
    last_received: HashMap<Token, Instant>,        // Track last received time for cleanup
    data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>,
    command_receiver: Receiver<Command>,
    shutdown_flag: Arc<AtomicBool>,
}

impl DynamicReceiver {
    pub fn new(
        data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>,
        command_receiver: crossbeam_channel::Receiver<Command>,
        shutdown_flag: Arc<AtomicBool>,
    ) -> io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            sockets: HashMap::new(),
            receiver_types: HashMap::new(),
            next_token_id: 0,
            counters: HashMap::new(),
            last_received: HashMap::new(),
            data_receiver_sender,
            command_receiver,
            shutdown_flag,
        })
    }

    /// Run the event loop indefinitely.
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut events = Events::with_capacity(1024);
        let mut buf = [0u8; 65535];

        loop {
            // Check for shutdown signal
            if self.shutdown_flag.load(Ordering::Relaxed) {
                info!("DynamicReceiver shutting down gracefully");
                return Ok(());
            }

            // Process commands first
            while let Ok(cmd) = self.command_receiver.try_recv() {
                match cmd {
                    Command::AddSocket(mut socket, receiver_type) => {
                        let token = Token(self.next_token_id);
                        self.next_token_id += 1;

                        self.poll
                            .registry()
                            .register(&mut socket, token, Interest::READABLE)?;
                        self.sockets.insert(token, socket);
                        self.receiver_types.insert(token, receiver_type);
                        self.counters.insert(token, (0, 0, Instant::now()));
                        self.last_received.insert(token, Instant::now());

                        info!("Added {:?} socket with token {:?}", receiver_type, token);
                    }
                    Command::RemoveSocket(token) => {
                        if let Some(mut socket) = self.sockets.remove(&token) {
                            self.poll.registry().deregister(&mut socket)?;
                            self.receiver_types.remove(&token);
                            self.counters.remove(&token);
                            self.last_received.remove(&token);
                            info!("Removed socket with token {:?}", token);
                        } else {
                            error!("Attempted to remove unknown socket with token {:?}", token);
                        }
                    }
                }
            }

            // Check for WifiAware socket cleanup (remove if no data received for 5 seconds)
            let mut tokens_to_remove = Vec::new();
            let now = Instant::now();

            for (&token, &receiver_type) in &self.receiver_types {
                if receiver_type == ReceiverType::WifiAware {
                    if let Some(&last_received_time) = self.last_received.get(&token) {
                        if now.duration_since(last_received_time) >= Duration::from_secs(5) {
                            tokens_to_remove.push(token);
                        }
                    }
                }
            }

            // Remove inactive WifiAware sockets
            for token in tokens_to_remove {
                if let Some(mut socket) = self.sockets.remove(&token) {
                    self.poll.registry().deregister(&mut socket)?;
                    self.receiver_types.remove(&token);
                    self.counters.remove(&token);
                    self.last_received.remove(&token);
                    info!("Cleaned up inactive WifiAware socket with token {:?} (no data for 5 seconds)", token);
                }
            }

            // Poll Mio for events, blocking until we get an event.
            self.poll
                .poll(&mut events, Some(Duration::from_millis(100)))?;

            for event in events.iter() {
                let token = event.token();

                if let Some(socket) = self.sockets.get(&token) {
                    let receiver_type = *self.receiver_types.get(&token).unwrap();
                    let (ref mut last_sec_bytes, ref mut total_bytes, ref mut last_reset) =
                        self.counters.get_mut(&token).unwrap();

                    loop {
                        match socket.recv_from(&mut buf) {
                            Ok((size, _src)) => {
                                // Update last received time
                                if let Some(last_time) = self.last_received.get_mut(&token) {
                                    *last_time = Instant::now();
                                }

                                // Send data through the channel
                                self.data_receiver_sender
                                    .send((receiver_type, buf[..size].to_vec()))
                                    .unwrap();

                                // Update counters
                                *last_sec_bytes += size as u64;
                                *total_bytes += size as u64;

                                // Check if 1 second passed to log bitrate
                                if last_reset.elapsed() >= Duration::from_secs(1) {
                                    let bitrate_kbps = (*last_sec_bytes * 8) / 1000;
                                    info!(
                                        "{:?} token {:?}: Bytes received: {} | Bitrate: {} Kbit/s | Total bytes: {}",
                                        receiver_type, token, *last_sec_bytes, bitrate_kbps, *total_bytes
                                    );
                                    *last_sec_bytes = 0;
                                    *last_reset = Instant::now();
                                }
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => {
                                error!(
                                    "{:?} token {:?} receive error: {}",
                                    receiver_type, token, e
                                );
                                break;
                            }
                        }
                    }
                } else {
                    error!("Received event for unknown token: {:?}", token);
                }
            }
        }
    }
}
