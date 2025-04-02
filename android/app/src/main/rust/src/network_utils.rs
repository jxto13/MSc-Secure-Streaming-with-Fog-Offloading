use crossbeam_channel::Receiver;
use log::info;
use mio::event::Event;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::str::from_utf8;

use std::time::Duration;

// Setup some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);

pub fn loopback_tcp_server( transport_layer_channel_receiver: Receiver<Vec<u8>>) -> io::Result<()> {

    // Create a poll instance.
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    // Setup the TCP server socket.
    let addr = "192.168.1.128:9000".parse().unwrap();
    let mut server = TcpListener::bind(addr)?;

    // Register the server with poll.
    poll.registry().register(&mut server, SERVER, Interest::READABLE)?;

    let mut connections = HashMap::new();
    let mut unique_token = Token(SERVER.0 + 1);

    println!("Server running on 127.0.0.1:9000");



    loop {
        if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(100))) {
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }

        // Check if there's a message to broadcast
        if let Ok(message) = transport_layer_channel_receiver.try_recv() {
            broadcast_message(&mut connections, message);
        }

        for event in events.iter() {
            match event.token() {
                // Checks for new clients to add to server
                SERVER => loop {
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    };

                    info!("Accepted connection from: {}", address);
                    let token = next(&mut unique_token);
                    poll.registry().register(
                        &mut connection,
                        token,
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;
                    connections.insert(token, connection);
                },

                // Checks clients for disconnections
                token => {
                    if let Some(connection) = connections.get_mut(&token) {
                        let done = handle_connection_event(connection, event)?;
                        if done {
                            if let Some(mut connection) = connections.remove(&token) {
                                poll.registry().deregister(&mut connection)?;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}

/// Broadcasts a message to all connected clients.
fn broadcast_message(connections: &mut HashMap<Token, TcpStream>, message: Vec<u8>) {
    let data = &message;

    for (token, connection) in connections.iter_mut() {
        match connection.write_all(data) {
            Ok(_) => println!("Sent to {:?}", token),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Write to {:?} would block", token)
            }
            Err(e) => {
                println!("Error sending to {:?}: {}", token, e);
            }
        }
    }
}

/// Handles readable and writable events.
fn handle_connection_event(
    connection: &mut TcpStream,
    event: &Event,
) -> io::Result<bool> {
    if event.is_readable() {
        let mut buffer = vec![0; 4096];
        match connection.read(&mut buffer) {
            Ok(0) => {
                info!("Client disconnected");
                return Ok(true);
            }
            Ok(n) => {
                let received_data = &buffer[..n];
                if let Ok(str_buf) = from_utf8(received_data) {
                    println!("Received: {}", str_buf.trim_end());
                } else {
                    println!("Received non UTF-8 data");
                }
            }
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(err),
        }
    }

    Ok(false)
}
