// use std::io;
// use std::time::Duration;
// use crossbeam_channel::Sender;
// use log::{error, info};
// use mio::{Events, Interest, Poll, Token};
// use mio::net::UdpSocket;

// // A token to allow us to identify which event is for the `UdpSocket`.
// const CLIENT: Token = Token(0);

// pub fn server(sender: Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
//     info!("Starting the unicast receiver");

//     // Setup the UDP socket.
//     let addr = "0.0.0.0:50052".parse().unwrap();

//     // Setup the UDP socket.
//     let mut socket = UdpSocket::bind(addr)?;
//     info!("Unicast Socket bound successfully");
    
//     // Create a poll instance.
//     let mut poll = Poll::new().unwrap();
//     info!("Unicast Poll instance created");

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
//                             info!("Unicast Retransmition: Bytes received: {} | Bitrate: {} Kbit/s", total_bytes, bitrate);

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

use crossbeam_channel::Sender;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration, Instant};
use log::{error, info};

pub async fn server(sender: Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting the unicast receiver");

    // Bind UDP socket
    let socket = UdpSocket::bind("0.0.0.0:50052").await?;
    info!("Unicast Socket bound successfully");

    let mut buf = [0u8; 1 << 16]; // 65536 bytes

    let mut counter: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut start_time = Instant::now();

    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, _peer)) => {
                        sender.send(buf[..size].to_vec())?;
                        counter += size as u64;
                        total_bytes += size as u64;
                    }
                    Err(e) => {
                        error!("Error receiving packet: {}", e);
                        continue;
                    }
                }
            }

            _ = time::sleep_until(start_time + Duration::from_secs(1)) => {
                let bitrate = (counter * 8) / 1000;
                info!("Unicast Retransmission: Bytes received: {} | Bitrate: {} Kbit/s", total_bytes, bitrate);
                start_time = Instant::now();
                counter = 0;
            }
        }
    }
}
