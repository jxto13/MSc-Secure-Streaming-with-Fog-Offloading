use mio::net::UdpSocket;
use mio::{Events, Poll, Token};
use std::net::{SocketAddr, ToSocketAddrs};
use std::str;

type BoxResult<T> = Result<T, Box<dyn std::error::Error>>;

const CLIENT_TOKEN: Token = Token(0);

pub fn start_client(server_addr: &str) -> BoxResult<()> {
    // Parse the server address
    let server_addr: SocketAddr = server_addr
        .to_socket_addrs()
        .expect("Failed to parse server address")
        .next()
        .expect("Failed to get the first socket address");
    
    let mut udp_socket = UdpSocket::bind(server_addr).expect("Failed to bind UDP socket");

    // Create a Poll instance
    let mut poll = Poll::new().unwrap();

    // Register the UDP socket with the Poll instance
    poll.registry().register(&mut udp_socket, CLIENT_TOKEN, mio::Interest::READABLE)
        .expect("Failed to register UDP socket with poll");

    // Set up the event loop
    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None).expect("Poll error");

        for event in &events {
            match event.token() {
                CLIENT_TOKEN => {
                    // Handle events for the UDP socket
                    if event.is_readable() {
                        let mut buf = [0; 1024];
                        match udp_socket.recv_from(&mut buf) {
                            Ok((size, _)) => {
                                let message = str::from_utf8(&buf[..size]).unwrap();
                                log::info!("Received message: {}", message);
                            }
                            Err(e) => {
                                log::info!("Error receiving message: {:?}", e);
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("Unexpected token: {:?}", event.token());
                }
            }
        }
    }
}
