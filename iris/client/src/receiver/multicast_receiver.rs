// use std::net::{Ipv4Addr, SocketAddr};
// use std::io;
// use std::time::Duration;
// use crossbeam_channel::Sender;
// use log::{error, info};
// use mio::{Events, Interest, Poll, Token};
// use mio::net::UdpSocket;

// // A token to allow us to identify which event is for the `UdpSocket`.
// const CLIENT: Token = Token(0);

// pub fn server(ip: String, port: String, sender: Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
//     info!("Starting the multicast receiver");

//     // Parse the IP address
//     let multicast_address = ip.parse::<Ipv4Addr>().map_err(|e| {
//         info!("Error parsing IPv4 address: {}", e);
//         e
//     }).unwrap();

//     info!("Parsed IPv4 address: {}", multicast_address);

//     // Parse the port
//     let multicast_port = port.parse::<u16>().map_err(|e| {
//         info!("Error parsing port: {}", e);
//         e
//     }).unwrap();
    
//     let multicast_dest: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);

//     info!("Binding to multicast destination: {}", multicast_dest);

//     // Setup the UDP socket.
//     let mut socket = UdpSocket::bind(format!("{}:{}", "0.0.0.0", multicast_port).parse().unwrap()).unwrap();

//     info!("Socket bound successfully");


//     // Join the multicast group.
//     socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED).expect("Failed to join multicast group");
//     // socket.join_multicast_v4(&multicast_address, &Ipv4Addr::new(192, 168, 2, 119)).expect("Failed to join multicast group");

    
//     // Create a poll instance.
//     let mut poll = Poll::new().unwrap();
//     info!("Poll instance created");

//     // Register the socket with the poll instance.
//     poll.registry().register(&mut socket, CLIENT, Interest::READABLE).unwrap();

//     info!("Socket registered with poll instance");

//     // Create storage for events.
//     let mut events = Events::with_capacity(1024 * 10);

//     // Initialize a buffer for the UDP packet.
//     let mut buf = [0; 1 << 16]; // Maximum UDP packet size is 65535 bytes.

//     let mut counter = 0;
//     let mut total_bytes = 0;

//     let mut start_time = std::time::Instant::now();
//     // Start an event loop.
//     loop {
//         // Poll Mio for events, blocking until we get an event.
//         if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(1))) {
//             if err.kind() == io::ErrorKind::Interrupted {
//                 continue;
//             }
//             return Err(Box::new(err));
//         }

//         // Process each event.
//         for event in events.iter() {
//             match event.token() {
//                 CLIENT => {
//                     loop {

//                         if start_time.elapsed() > std::time::Duration::from_secs(1) {
//                             let bitrate = (counter * 8) / 1000;
//                             info!("Bytes received: {} | Bitrate: {} Kbit/s", total_bytes, bitrate);

//                             start_time = std::time::Instant::now();
//                             counter = 0;
//                         }

//                         // Receive all packets queued for the socket.
//                         match socket.recv_from(&mut buf) {
//                             Ok((size, _source_address)) => {
//                                 // Send data to matrix decoder.
//                                 sender.send(buf[..size].to_vec()).unwrap();
//                                 counter += size as u64;
//                                 total_bytes += size;
                                
//                             }
//                             Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
//                                 // Would block if no more packets are available to be read.
//                                 break;
//                             }
//                             Err(e) => {
//                                 error!("Error receiving packet: {}", e);
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


use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;
use tokio::time::{interval, Duration, Instant};
use crossbeam_channel::Sender;
use log::{info, error};

pub async fn server(ip: String, port: String, sender: Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting the multicast receiver");

    // Parse IP and port
    let multicast_address: Ipv4Addr = ip.parse().map_err(|e| {
        error!("Error parsing IPv4 address: {}", e);
        e
    })?;

    let multicast_port: u16 = port.parse().map_err(|e| {
        error!("Error parsing port: {}", e);
        e
    })?;

    let multicast_dest = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);
    info!("Multicast destination: {}", multicast_dest);

    // Bind to 0.0.0.0:<port> to receive multicast packets
    let socket = UdpSocket::bind(("0.0.0.0", multicast_port)).await?;
    info!("Socket bound successfully");

    // Join the multicast group on the unspecified interface (0.0.0.0)
    socket.join_multicast_v4(multicast_address, Ipv4Addr::UNSPECIFIED)?;

    info!("Joined multicast group: {}", multicast_address);

    let mut buf = vec![0u8; 65536]; // max UDP packet size

    // For bitrate logging every second
    let mut ticker = interval(Duration::from_secs(1));
    let mut counter: u64 = 0;
    let mut total_bytes: u64 = 0;

    loop {
        tokio::select! {
            recv = socket.recv_from(&mut buf) => {
                match recv {
                    Ok((size, _src)) => {
                        // Send the received data to the channel
                        sender.send(buf[..size].to_vec())?;

                        counter += size as u64;
                        total_bytes += size as u64;
                    }
                    Err(e) => {
                        error!("Error receiving packet: {}", e);
                    }
                }
            }

            _ = ticker.tick() => {
                // Calculate bitrate (kilobits per second)
                let bitrate = (counter * 8) / 1000;
                info!("Bytes received: {} | Bitrate: {} Kbit/s", total_bytes, bitrate);
                counter = 0;
            }
        }
    }
}
