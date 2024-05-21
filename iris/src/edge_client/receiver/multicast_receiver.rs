use crate::proto::{self, IrisPacket};

use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::mpsc::Sender;

use std::io::{self, Cursor, Write};
use std::time::Duration;

use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;

use mpeg2ts::ts::{ReadTsPacket, TsPacketReader};
use prost::Message;
use proto::{DataPacket};
// A token to allow us to identify which event is for the `UdpSocket`.
const CLIENT: Token = Token(0);

pub fn start(ip: String, port: String, sender: Sender<Vec<u8>>) -> Result<(), Box<dyn Error>> {

    // Parse the string into an Ipv4Addr struct in one line
    let multicast_address = ip.parse::<Ipv4Addr>().map(|ip| {
        println!("Parsed IPv4 address: {}", ip);
        ip
    }).unwrap_or_else(|e| {
        println!("Error parsing IPv4 address: {}", e);
        // You can provide a default value or handle the error as needed here
        Ipv4Addr::new(224, 0, 0, 123)
    });
    
    let multicast_port = port.parse().unwrap(); //need to handle error 
    let multicast_dest: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);

    // Setup the UDP socket.
    let mut socket = UdpSocket::bind(multicast_dest).unwrap();

    // Join the multicast group.
    socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED).map_err(|err| {
        eprintln!("Failed to join multicast group: {}", err);
        err
    })?;

    
    // Create a poll instance.
    let mut poll = Poll::new().unwrap();

    // Register the socket with the poll instance.
    poll.registry()
        .register(&mut socket, CLIENT, Interest::READABLE).unwrap();

    // Create storage for events.
    let mut events = Events::with_capacity(1024 * 10);

    // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
    // packet, which is the maximum value of 16 a bit integer.
    let mut buf = [0; 1 << 16];   
    let mut index = 1;
    
    // Start an event loop.
    loop {
        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, Some(Duration::from_nanos(100_000))).unwrap();

        // Process each event.
        for event in events.iter() {
            // We can use the token we previously provided to `register` to
            // determine for which socket the event is.
            match event.token() {
                CLIENT => {
                    // In this loop we receive all packets queued for the socket.
                    match socket.recv_from(&mut buf) {
                        Ok((size, source_address)) => {
                            // Send data to matrix decoder
                            sender.send(buf[..size].to_vec())?;
                            println!("Packet received number: {} from {}", index, source_address);
                            index +=1;
                            
                            // let deserialized_matrix_cell = IrisPacket::decode(&buf[..size]).unwrap_or_else(|err| {
                            //     // Handle the error if creating the runtime fails
                            //     eprintln!("Error decoding packet: {}", err);
                            //     // create and empty cell, will discart latter
                            //     IrisPacket {
                            //         payload: vec![DataPacket {
                            //             matrix: 0,
                            //             index: 0,
                            //             width: 0,
                            //             height: 0,
                            //             fec: false,
                            //             data: vec![0, 0],
                            //         }],
                            //     }
                            // });
                            // println!("Matrix: {:?} ", deserialized_matrix_cell.payload.get(0).unwrap().matrix);
                    
                                           
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
                            println!("{}",e);
                            break;
                        }
                    }
                }
                // We don't expect any events with tokens other than those we provided.
                _ => unreachable!(),
            }
        }
    }
}