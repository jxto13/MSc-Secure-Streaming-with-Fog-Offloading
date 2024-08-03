use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use std::net::SocketAddr;
use grid::Grid as Matrix;
use std::io;
use crossbeam_channel::Sender;
use log::{info, error};

use crate::CONFIG;

const UDP_SOCKET: Token = Token(0);

pub fn udp_receiver_and_send(tx: Sender<Matrix<Vec<u8>>>) {
    let config = &CONFIG.get().unwrap().multicast;

    let row: usize = config.row;
    let col: usize = config.col;
    let data_size: usize = config.data_size;

    let addr: SocketAddr = "127.0.0.1:8080".parse().expect("Unable to parse socket address");
    let mut socket = UdpSocket::bind(addr).unwrap();
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    poll.registry().register(&mut socket, UDP_SOCKET, Interest::READABLE).unwrap();

    let mut buffer = vec![0; data_size];

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            if event.token() == UDP_SOCKET && event.is_readable() {
                match socket.recv_from(&mut buffer) {
                    Ok((n, _)) => {
                        if n == 0 {
                            info!("No more data received, exiting.");
                            break;
                        }

                        let mut matrix: Matrix<Vec<u8>> = Matrix::new(0, 0);

                        let mut buf_index = 0;
                        'outer: for _ in 0..row {
                            let mut row_data: Vec<Vec<u8>> = Vec::with_capacity(col);

                            for _ in 0..col {
                                if buf_index + data_size > n {
                                    break 'outer;
                                }
                                row_data.push(buffer[buf_index..buf_index + data_size].to_vec());
                                buf_index += data_size;
                            }

                            matrix.push_row(row_data);
                        }

                        if let Err(e) = tx.send(matrix) {
                            error!("Error sending data to encoder: {:?}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error receiving data: {}", e);
                        break;
                    }
                }
            }
        }
    }
}
