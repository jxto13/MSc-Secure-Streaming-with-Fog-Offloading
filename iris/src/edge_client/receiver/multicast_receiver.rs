use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use std::io;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;

use log::info;
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;


// A token to allow us to identify which event is for the `UdpSocket`.
const CLIENT: Token = Token(0);

pub fn start(ip: String, port: String, sender: Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    
    info!("Starting the multicast receiver");

    // Parse the string into an Ipv4Addr struct in one line
    let multicast_address = ip.parse::<Ipv4Addr>().map(|ip| {
        info!("Parsed IPv4 address: {}", ip);
        ip
    }).unwrap_or_else(|e| {
        info!("Error parsing IPv4 address: {}", e);
        // You can provide a default value or handle the error as needed here
        Ipv4Addr::new(224, 0, 0, 123)
    });
    
    let multicast_port = port.parse().unwrap(); //need to handle error 
    let multicast_dest: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);

    info!("Binding to multicast destination: {}", multicast_dest);
    // Setup the UDP socket.
    let mut socket = UdpSocket::bind(format!("{}:{}", multicast_address, multicast_port).parse().unwrap()).unwrap();
    info!("Socket bound successfully");

    // Join the multicast group.
    socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED).map_err(|err| {
        info!("Failed to join multicast group: {}", err);
        err
    })?;
    info!("Joined multicast group successfully");
    
    // Create a poll instance.
    let mut poll = Poll::new().unwrap();
    info!("Poll instance created");

    // Register the socket with the poll instance.
    poll.registry()
        .register(&mut socket, CLIENT, Interest::READABLE).unwrap();
    info!("Socket registered with poll instance");

    // Create storage for events.
    let mut events = Events::with_capacity(1024 * 10);

    // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
    // packet, which is the maximum value of 16 a bit integer.
    let mut buf = [0; 1 << 16];   
    let mut index = 1;

    let mut total_bytes = 0;
    let mut start_time = Instant::now();


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
                CLIENT => {
                    loop {
                        
                        // In this loop we receive all packets queued for the socket.
                        match socket.recv_from(&mut buf) {
                            Ok((size, source_address)) => {
                                
                                // info!("Packet received number: {} from {}. Size: {}", index, source_address, size);
                                
                                index +=1;
                                
                                total_bytes += size;
                                
                                let elapsed = start_time.elapsed();
                                if elapsed >= Duration::from_secs(1) {
                                    let bitrate_mbps = (total_bytes as f64 * 8.0) / 1_000_000.0 / elapsed.as_secs_f64();
                                    info!("Bitrate: {:.2} Mbps", bitrate_mbps);
                                    total_bytes = 0;
                                    start_time = Instant::now();
                                    info!("packets_received {}",index);
                                }
                                
                                // Send data to matrix decoder
                                // sender.send(buf[..size].to_vec())?;
                                
                                // Notify that the multicast_receiver has started
                                // Send callback notification after the first packet
                                // if !callback_sent {
                                    //     callback.send(()).unwrap();
                                    //     callback_sent = true;
                                    // }
                                    
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    // info!("WouldBlock error, no more packets queued");
                                    // If we get a `WouldBlock` error we know our socket
                                    // has no more packets queued, so we can return to
                                    // polling and wait for some more.
                                    break;
                                }
                                Err(e) => {
                                    info!("Error receiving packet: {}", e);
                                    // If it was any other kind of error, something went
                                    // wrong and we terminate with an error.
                                    break;
                                }
                            }
                        }
                }
                // We don't expect any events with tokens other than those we provided.
                _ => unreachable!(),
            }
        }
    }
}
