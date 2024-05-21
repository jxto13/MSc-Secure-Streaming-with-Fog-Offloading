use log::{debug, error, info, warn};
use mio::{Events, Interest, Poll, Token};
use std::{
    io::{self, Write}, 
    net::{Ipv4Addr, SocketAddr}, 
    sync::mpsc::Sender, 
    time::Duration
};
use mio::net::UdpSocket;
use prost::Message;
use proto::{DataPacket, IrisPacket};

pub mod proto {
    tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
}

// A token to allow us to identify which event is for the `UdpSocket`.
const UDP_SOCKET: Token = Token(0);

pub fn start(sender: Sender<Vec<u8>>) -> io::Result<()> {
    // Create a poll instance.
    let mut poll = Poll::new()?;
    // Create storage for events. Since we will only register a single socket, a capacity of 1 will do.
    let mut events = Events::with_capacity(1);
    
    // Define the multicast address and port.
    let multicast_address = Ipv4Addr::new(224, 0, 0, 123);
    let multicast_port = 12345;
    let multicast_dest = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);

    // Setup the UDP socket.
    let mut socket = UdpSocket::bind(multicast_dest)?;
    info!("Socket bound to {}", multicast_dest);

    // Join the multicast group.
    socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED).map_err(|err| {
        error!("Failed to join multicast group: {}", err);
        err
    })?;
    info!("Joined multicast group {}", multicast_address);

    // Register our socket with the token defined above and an interest in being `READABLE`.
    poll.registry().register(&mut socket, UDP_SOCKET, Interest::READABLE)?;
    info!("Socket registered with poll");

    // Initialize a buffer for the UDP packet. We use the maximum size of a UDP packet, which is the maximum value of a 16-bit integer.
    let mut buf = [0; 1 << 16];
    let mut index =  1;

    // Start an event loop.
    loop {
        // Poll Mio for events, blocking until we get an event.
        debug!("Polling for events");
        poll.poll(&mut events, Some(Duration::from_nanos(100_000)))?;

        // Process each event.
        for event in events.iter() {
            debug!("Processing event: {:?}", event);
            // We can use the token we previously provided to `register` to determine for which socket the event is.
            if event.token() == UDP_SOCKET {
                debug!("UDP_SOCKET event received");
                // In this loop we receive all packets queued for the socket.
                loop {
                    match socket.recv_from(&mut buf) {
                        Ok((packet_size, source_address)) => {
                            info!("Packet received number: {} from {}", index, source_address);
                            index += 1;

                            // Process the packet
                            match IrisPacket::decode(&buf[..packet_size]) {
                                Ok(deserialized_matrix_cell) => {
                                    for packet in deserialized_matrix_cell.payload {
                                        if !packet.fec {
                                            // Uncomment the sender line to send data if needed
                                            // sender.send(packet.data).unwrap();
                                            // info!("matrix {} | index {}", packet.matrix, packet.index);
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!("Error decoding packet: {}", err);
                                }
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // If we get a `WouldBlock` error we know our socket has no more packets queued, so we can return to polling and wait for some more.
                            debug!("Socket would block, no more packets queued");
                            break;
                        }
                        Err(e) => {
                            // If it was any other kind of error, something went wrong and we terminate with an error.
                            error!("Socket error: {}", e);
                            return Err(e);
                        }
                    }
                }
            } else {
                // This should never happen as we only registered our `UdpSocket` using the `UDP_SOCKET` token, but if it ever does we'll log it.
                warn!("Got event for unexpected token: {:?}", event);
            }
        }
    }
}
