use crate::metrics::get_metrics;
use crate::CONFIG;
use crossbeam_channel::{unbounded, Sender, TryRecvError};
use log::{error, info, warn};
use grid::Grid as Matrix;
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use std::time::{Duration, Instant};
use std::{ io, thread};

// A token to allow us to identify which event is for the `UdpSocket`.
const UDP_SOCKET: Token = Token(0);

pub fn read(ip_source: String, tx: Sender<Matrix<Vec<u8>>>) -> Result<(), io::Error> {

    let (ipc_channel_tx, ipc_channel_rx) = unbounded();
    
    thread::spawn(move || {
      
        // Buffer to store the UDP packet data.
        let mut buf = [0; 1 << 16];

        // Create a poll instance.
        let mut poll = Poll::new().unwrap();

        // Create storage for events.
        let mut events = Events::with_capacity(1);

        // Setup the UDP socket.
        let addr = match ip_source.parse() {
            Ok(addr) => addr,
            Err(e) => {
                error!("Invalid IP address: {}", e);
                return;
            }
        };

        let mut socket = match UdpSocket::bind(addr) {
            Ok(socket) => socket,
            Err(e) => {
                error!("Failed to bind UDP socket: {}", e);
                return;
            }
        };

        // Register the socket with the token and interest.
        if let Err(e) = poll.registry().register(&mut socket, UDP_SOCKET, Interest::READABLE) {
            error!("Failed to register socket with poll: {}", e);
            return;
        }

        // Variables for bandwidth logging
        let mut total_bytes_received = 0;
        let mut last_log_time = Instant::now();
        let metrics = get_metrics();

        let bandwidth_used = metrics.multicast_bandwidth_used.lock().unwrap();

        
        // Event loop.
        loop {
            // Poll to check for events.
            if let Err(e) = poll.poll(&mut events, None) {
                if e.kind() != io::ErrorKind::Interrupted {
                    error!("Poll error: {}", e);
                    return;
                }
                continue;
            }

            for event in events.iter() {
                if event.token() == UDP_SOCKET {
                    loop {
                        match socket.recv_from(&mut buf) {
                            Ok((packet_size, _source_address)) => {

                                if let Err(e) = ipc_channel_tx.send(buf[..packet_size].to_vec()) {
                                    panic!("Error sending data from udp_source: {:?}", e);
                                }

                                {
                                    bandwidth_used.add(packet_size as f64 ); // Update the gauge value
                                }
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                warn!("Error receiving data: {}", e);
                                return;
                            }
                        }
                    }
                } else {
                    warn!("Unexpected token: {:?}", event.token());
                }
            }
        }
    });

    thread::spawn(move || {
        // Retrieve configuration.
        let config = match CONFIG.get() {
            Some(config) => &config.multicast,
            None => {
                warn!("Configuration not found");
                return;
            }
        };

        let num_rows: usize = config.row;
        let num_cols: usize = config.col;
        let data_size: usize = config.data_size;

        // Initialize matrix and index
        let mut matrix: Matrix<Vec<u8>> = Matrix::new(num_rows, num_cols);
        let mut current_index = 0;

        let mut buffer = vec![];


        loop {
            match ipc_channel_rx.try_recv() {
                Ok(item) => {
                   

                    // Write the new item to the buffer
                    buffer.extend_from_slice(&item);
                    
                    // Split off a chunk of data_size
                    let drain_size = std::cmp::min(buffer.len(), data_size);
                    let chunk = buffer.drain(..drain_size).collect::<Vec<_>>();

                    // Update matrix with received data.
                    let coords = index_to_coords(current_index, num_cols);

                    matrix[coords] = chunk;
                    current_index += 1;

                    // If the matrix is filled, send it and reset.
                    if current_index == (num_cols * num_rows) - 1 {
                        if let Err(e) = tx.send(matrix.clone()) {
                            panic!("Error sending data from udp_source: {:?}", e);
                        }
                        
                        // Reinitialize the matrix and reset index
                        matrix = Matrix::new(num_rows, num_cols);
                        current_index = 0;
                    }
                }

                Err(TryRecvError::Empty) => {
                    // No message available, continue looping
                }
                Err(TryRecvError::Disconnected) => {
                    // Channel is disconnected, break the loop
                    break;
                }
            }
        }

    });

    Ok(())
}

// Function to get row and column from a 1D index
fn index_to_coords(index: usize, num_cols: usize) -> (usize, usize) {
    (index / num_cols, index % num_cols)
}
