// use std::net::{Ipv4Addr, SocketAddr};
// use std::io;
// use std::time::{Duration, Instant};

// use crossbeam_channel::Sender;
// use log::{error, info};
// use mio::{Events, Interest, Poll, Token};
// use mio::net::UdpSocket;

// use crate::utils::ReceiverType;

// const MULTICAST_TOKEN: Token = Token(0);
// const UNICAST_TOKEN: Token = Token(1);

// pub fn server(multicast_ip: String, multicast_port: String, data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>) -> Result<(), Box<dyn std::error::Error>> {
//     info!("Starting the multicast + unicast receiver");

//     let multicast_address: Ipv4Addr = multicast_ip.parse().map_err(|e| {
//         error!("Error parsing IPv4 address: {}", e);
//         e
//     })?;

//     let multicast_port: u16 = multicast_port.parse().map_err(|e| {
//         error!("Error parsing port: {}", e);
//         e
//     })?;

//     let multicast_socket_addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), multicast_port);
//     let unicast_socket_addr = "0.0.0.0:50052".parse::<SocketAddr>()?;

//     // Bind multicast socket and join group
//     let mut multicast_socket = UdpSocket::bind(multicast_socket_addr)?;
//     multicast_socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED)?;
//     info!("Multicast socket bound and joined group: {}", multicast_address);

//     // Bind unicast socket
//     let mut unicast_socket = UdpSocket::bind(unicast_socket_addr)?;
//     info!("Unicast socket bound: {}", unicast_socket_addr);

//     // Create a poll instance
//     let mut poll = Poll::new()?;
//     poll.registry().register(&mut multicast_socket, MULTICAST_TOKEN, Interest::READABLE)?;
//     poll.registry().register(&mut unicast_socket, UNICAST_TOKEN, Interest::READABLE)?;
//     info!("Both sockets registered with poll instance");

//     let mut events = Events::with_capacity(2048);
//     let mut buf = [0; 65535];

//     // Independent tracking
//     let (mut multicast_counter, mut multicast_total_bytes) = (0u64, 0u64);
//     let (mut unicast_counter, mut unicast_total_bytes) = (0u64, 0u64);
//     let (mut start_time_multicast, mut start_time_unicast) = (Instant::now(), Instant::now());

//     loop {
//         if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(1))) {
//             if err.kind() == io::ErrorKind::Interrupted {
//                 continue;
//             }
//             return Err(Box::new(err));
//         }

//         for event in events.iter() {
//             match event.token() {
//                 MULTICAST_TOKEN => {
//                     loop {
//                         if start_time_multicast.elapsed() > Duration::from_secs(1) {
//                             let bitrate = (multicast_counter * 8) / 1000;
//                             info!("Multicast: Bytes received: {} | Bitrate: {} Kbit/s", multicast_total_bytes, bitrate);
//                             start_time_multicast = Instant::now();
//                             multicast_counter = 0;
//                         }

//                         match multicast_socket.recv_from(&mut buf) {
//                             Ok((size, _source)) => {
//                                 data_receiver_sender.send((ReceiverType::Multicast, buf[..size].to_vec())).unwrap();
//                                 multicast_counter += size as u64;
//                                 multicast_total_bytes += size as u64;
//                             }
//                             Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
//                             Err(e) => {
//                                 error!("Multicast receive error: {}", e);
//                                 break;
//                             }
//                         }
//                     }
//                 }

//                 UNICAST_TOKEN => {
//                     loop {
//                         if start_time_unicast.elapsed() > Duration::from_secs(1) {
//                             let bitrate = (unicast_counter * 8) / 1000;
//                             info!("Unicast: Bytes received: {} | Bitrate: {} Kbit/s", unicast_total_bytes, bitrate);
//                             start_time_unicast = Instant::now();
//                             unicast_counter = 0;
//                         }

//                         match unicast_socket.recv_from(&mut buf) {
//                             Ok((size, _source)) => {
//                                 data_receiver_sender.send((ReceiverType::Unicast, buf[..size].to_vec())).unwrap();
//                                 unicast_counter += size as u64;
//                                 unicast_total_bytes += size as u64;
//                             }
//                             Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
//                             Err(e) => {
//                                 error!("Unicast receive error: {}", e);
//                                 break;
//                             }
//                         }
//                     }
//                 }

//                 _ => unreachable!(),
//             }
//         }
//     }
// }

// use std::collections::HashMap;
// use std::io;
// use std::net::{Ipv4Addr, SocketAddr};
// use std::sync::atomic::{AtomicUsize, Ordering};
// use std::time::{Duration, Instant};

// use crossbeam_channel::Sender;
// use dashmap::DashMap;
// use log::{error, info};
// use mio::{Events, Interest, Poll, Token};
// use mio::net::UdpSocket;

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum ReceiverType {
//     Multicast,
//     Unicast,
//     WifiAware
// }

// #[derive(Debug)]
// pub struct DynamicReceiver {
//     poll: Poll,
//     sockets: DashMap<Token, UdpSocket>,
//     receiver_types: DashMap<Token, ReceiverType>,
//     next_token_id: AtomicUsize,
//     data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>,
//     counters: DashMap<Token, (u64, u64, Instant)>, // (packet_bytes_in_last_second, total_bytes, last_reset)
// }

// impl DynamicReceiver {
//     pub fn new(data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>) -> io::Result<Self> {
//         Ok(Self {
//             poll: Poll::new()?,
//             sockets: DashMap::new(),
//             receiver_types: DashMap::new(),
//             next_token_id: AtomicUsize::new(0),
//             data_receiver_sender,
//             counters: DashMap::new(),
//         })
//     }

//     /// Adds a UDP socket to the poll and internal maps.
//     pub fn add_socket(&self, mut socket: UdpSocket, receiver_type: ReceiverType) -> io::Result<Token> {
//         let token = Token(self.next_token_id.fetch_add(1, Ordering::Relaxed));

//         self.poll.registry().register(&mut socket, token, Interest::READABLE)?;
//         self.sockets.insert(token, socket);
//         self.receiver_types.insert(token, receiver_type);
//         self.counters.insert(token, (0, 0, Instant::now()));

//         info!("Added {:?} socket with token {:?}", receiver_type, token);

//         Ok(token)
//     }

//     /// Removes a UDP socket from the poll and internal maps.
//     pub fn remove_socket(&self, token: Token) -> io::Result<()> {
//         if let Some((_, mut socket)) = self.sockets.remove(&token) {
//             self.poll.registry().deregister(&mut socket)?;
//             self.receiver_types.remove(&token);
//             self.counters.remove(&token);
//             info!("Removed socket with token {:?}", token);
//         } else {
//             error!("Attempted to remove unknown socket with token {:?}", token);
//         }
//         Ok(())
//     }


//     /// Run the event loop indefinitely.
//     pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
//         let mut events = Events::with_capacity(1024);
//         let mut buf = [0u8; 65535];

//         loop {
//             self.poll.poll(&mut events, Some(Duration::from_millis(100)))?;

//             for event in events.iter() {
//                 let token = event.token();

//                 if let Some(socket_entry) = self.sockets.get(&token) {
//                     let socket = socket_entry.value();
//                     let receiver_type = *self.receiver_types.get(&token).unwrap();
//                     let mut counters_entry = self.counters.get_mut(&token).unwrap();
//                     let (ref mut last_sec_bytes, ref mut total_bytes, ref mut last_reset) = &mut *counters_entry;

//                     loop {
//                         match socket.recv_from(&mut buf) {
//                             Ok((size, _src)) => {
//                                 // Send data through the channel
//                                 self.data_receiver_sender.send((receiver_type, buf[..size].to_vec())).unwrap();

//                                 // Update counters
//                                 *last_sec_bytes += size as u64;
//                                 *total_bytes += size as u64;

//                                 // Check if 1 second passed to log bitrate
//                                 if last_reset.elapsed() >= Duration::from_secs(1) {
//                                     let bitrate_kbps = (*last_sec_bytes * 8) / 1000;
//                                     info!(
//                                         "{:?} token {:?}: Bytes received: {} | Bitrate: {} Kbit/s | Total bytes: {}",
//                                         receiver_type, token, *last_sec_bytes, bitrate_kbps, *total_bytes
//                                     );
//                                     *last_sec_bytes = 0;
//                                     *last_reset = Instant::now();
//                                 }
//                             }
//                             Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
//                             Err(e) => {
//                                 error!("{:?} token {:?} receive error: {}", receiver_type, token, e);
//                                 break;
//                             }
//                         }
//                     }
//                 } else {
//                     error!("Received event for unknown token: {:?}", token);
//                 }
//             }
//         }
//     }
// }

// /// Example function to create and run the server
// pub fn server(
//     multicast_ip: &str,
//     multicast_port: u16,
//     data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut server = DynamicReceiver::new(data_receiver_sender)?;

//     // Setup multicast socket
//     let multicast_addr = SocketAddr::new(multicast_ip.parse()?, multicast_port);
//     let multicast_socket = UdpSocket::bind(multicast_addr)?;
//     multicast_socket.join_multicast_v4(&multicast_ip.parse()?, &Ipv4Addr::UNSPECIFIED)?;
//     server.add_socket(multicast_socket, ReceiverType::Multicast)?;

//     // Setup unicast socket (example fixed port)
//     let unicast_addr = "0.0.0.0:50052".parse()?;
//     let unicast_socket = UdpSocket::bind(unicast_addr)?;
//     server.add_socket(unicast_socket, ReceiverType::Unicast)?;

//     // Run server event loop
//     server.run()
// }


use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use log::{error, info};
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReceiverType {
    Multicast,
    Unicast,
    WifiAware
}

pub enum Command {
    AddSocket(UdpSocket, ReceiverType),
    RemoveSocket(Token),
}


pub struct DynamicReceiver {
    poll: Poll,
    sockets: HashMap<Token, UdpSocket>,
    receiver_types: HashMap<Token, ReceiverType>,
    next_token_id: usize,
    counters: HashMap<Token, (u64, u64, Instant)>, // (packet_bytes_in_last_second, total_bytes, last_reset)
    data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>,
    command_receiver: Receiver<Command>,
}

impl DynamicReceiver {
    pub fn new(data_receiver_sender: Sender<(ReceiverType, Vec<u8>)>, command_receiver: crossbeam_channel::Receiver<Command>) -> io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            sockets: HashMap::new(),
            receiver_types: HashMap::new(),
            next_token_id: 0,
            counters: HashMap::new(),
            data_receiver_sender,
            command_receiver
        })
    }

    /// Adds a UDP socket to the poll and internal maps.
    // pub fn add_socket(&mut self, mut socket: UdpSocket, receiver_type: ReceiverType) -> io::Result<Token> {
    //     let token = Token(self.next_token_id);
    //     self.next_token_id += 1;

    //     self.poll.registry().register(&mut socket, token, Interest::READABLE)?;
    //     self.sockets.insert(token, socket);
    //     self.receiver_types.insert(token, receiver_type);
    //     self.counters.insert(token, (0, 0, Instant::now()));

    //     info!("Added {:?} socket with token {:?}", receiver_type, token);

    //     Ok(token)
    // }

    /// Removes a UDP socket from the poll and internal maps.
    // pub fn remove_socket(&mut self, token: Token) -> io::Result<()> {
    //     if let Some(mut socket) = self.sockets.remove(&token) {
    //         self.poll.registry().deregister(&mut socket)?;
    //         self.receiver_types.remove(&token);
    //         self.counters.remove(&token);
    //         info!("Removed socket with token {:?}", token);
    //     } else {
    //         error!("Attempted to remove unknown socket with token {:?}", token);
    //     }
    //     Ok(())
    // }

    /// Run the event loop indefinitely.
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut events = Events::with_capacity(1024);
        let mut buf = [0u8; 65535];

        loop {

            // Process commands first
            while let Ok(cmd) = self.command_receiver.try_recv() {
                match cmd {
                    Command::AddSocket(mut socket, receiver_type) => {
                        let token = Token(self.next_token_id);
                        self.next_token_id += 1;

                        self.poll.registry().register(&mut socket, token, Interest::READABLE)?;
                        self.sockets.insert(token, socket);
                        self.receiver_types.insert(token, receiver_type);
                        self.counters.insert(token, (0, 0, Instant::now()));

                        info!("Added {:?} socket with token {:?}", receiver_type, token);

        
                    }
                    Command::RemoveSocket(token) => {
                        if let Some(mut socket) = self.sockets.remove(&token) {
                            self.poll.registry().deregister(&mut socket)?;
                            self.receiver_types.remove(&token);
                            self.counters.remove(&token);
                            info!("Removed socket with token {:?}", token);
                        } else {
                            error!("Attempted to remove unknown socket with token {:?}", token);
                        }
                    }
                }
            }

            // Poll Mio for events, blocking until we get an event.
            self.poll.poll(&mut events, Some(Duration::from_millis(100)))?;

            for event in events.iter() {
                let token = event.token();

                if let Some(socket) = self.sockets.get(&token) {
                    let receiver_type = *self.receiver_types.get(&token).unwrap();
                    let (ref mut last_sec_bytes, ref mut total_bytes, ref mut last_reset) = self.counters.get_mut(&token).unwrap();

                    loop {
                        match socket.recv_from(&mut buf) {
                            Ok((size, _src)) => {
                                // Send data through the channel
                                self.data_receiver_sender.send((receiver_type, buf[..size].to_vec())).unwrap();

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
                                error!("{:?} token {:?} receive error: {}", receiver_type, token, e);
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

