use crate::metrics::get_metrics;
use crate::CONFIG;
use crossbeam_channel::{unbounded, Sender, TryRecvError};
use log::{error, info, warn};
use grid::Grid as Matrix;
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
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
        let metrics = get_metrics();

        let data_source_rate = metrics.data_source_rate.lock().unwrap();

        
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
                                    data_source_rate.add(packet_size as f64 ); // Update the gauge value
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

    const TS_PACKET_SIZE: usize = 188;
    const TS_SYNC_BYTE: u8 = 0x47;

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
        let shard_size: usize = config.shard_size;

        // Ensure constraints for data_size and shard_size
        assert!(data_size % 188 == 0, "data_size must be a multiple of 188 bytes.");
        assert!(data_size + 113 < shard_size, "data_size + 113 must be less than shard_size.");

        let mut ts_buffer = Vec::new(); // Buffer to store incoming MPEG-TS data.

        let mut matrix: Matrix<Vec<u8>> = Matrix::new(num_rows, num_cols);
        let mut current_index = 0;

        // Temporary buffer to accumulate data_size bytes
        let mut chunk_buffer = Vec::with_capacity(data_size);

        // Receive data from the UDP thread.
        while let Ok(packet) = ipc_channel_rx.recv() {
            ts_buffer.extend_from_slice(&packet);

            let mut start_index = 0;
            while start_index + TS_PACKET_SIZE <= ts_buffer.len() {
                // Look for the sync byte.
                if ts_buffer[start_index] == TS_SYNC_BYTE {
                    // Extract a complete 188-byte TS frame.
                    let ts_frame = ts_buffer[start_index..start_index + TS_PACKET_SIZE].to_vec();
                    start_index += TS_PACKET_SIZE;

                    // Add the TS frame to the chunk buffer
                    chunk_buffer.extend(ts_frame);

                    // If the chunk_buffer has reached data_size, add it to the matrix
                    if chunk_buffer.len() == data_size {
                        let coords = index_to_coords(current_index, num_cols);
                        matrix[coords] = chunk_buffer.clone(); // Store the chunk in the matrix
                        chunk_buffer.clear(); // Clear the buffer for the next chunk
                        current_index += 1;

                        // If the matrix is filled, send it and reset.
                        if current_index == (num_cols * num_rows) {
                            if let Err(e) = tx.send(matrix.clone()) {
                                panic!("Error sending data from udp_source: {:?}", e);
                            }

                            // Reinitialize the matrix and reset index
                            matrix = Matrix::new(num_rows, num_cols);
                            current_index = 0;
                        }
                    }
                } else {
                    // Increment by 1 to continue searching for the sync byte.
                    start_index += 1;
                }
            }

            // Keep only unprocessed data in ts_buffer.
            ts_buffer.drain(..start_index);
        }

        

    });

    Ok(())
}

// Function to get row and column from a 1D index
fn index_to_coords(index: usize, num_cols: usize) -> (usize, usize) {
    (index / num_cols, index % num_cols)
}
