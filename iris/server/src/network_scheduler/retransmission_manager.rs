use std::collections::HashSet;
use std::thread::sleep;
use std::{
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

use crate::{metrics::get_metrics, proto::RetransmissionPacket};
use crate::{membership_manager::client_sessions::ClientSessions, MATRIX_BUFFER};
use grid::Grid;
use iris_shared::source_coding::{find_optimal_retransmissions, Transmission};

use crossbeam_channel::Sender;
use log::info;
use prost::Message;
use rand::{rng, thread_rng, Rng};

// Retransmission manager struct
pub struct RetransmissionManager {}

impl RetransmissionManager {
    // Initialize the manager with receivers for multicast and unicast retransmissions
    pub fn new() -> Self {
        Self {}
    }

    //  Convert from Grid to Vec<Vec<>>
    fn grid_to_vec<T: Clone>(grid: &Grid<T>) -> Vec<Vec<T>> {
        let rows = grid.rows();
        let cols = grid.cols();
        
        let mut result = Vec::with_capacity(rows);
        
        for row in 0..rows {
            let mut row_vec = Vec::with_capacity(cols);
            for col in 0..cols {
                row_vec.push(grid.get(row, col).cloned().unwrap());
            }
            result.push(row_vec);
        }
        
        result
    }

    // Run the retransmission handler that listens to both channels
    pub fn handle_retransmissions(
        client_sessions: Arc<ClientSessions>,
        multicast_tx: Sender<Vec<u8>>,
        unicast_tx: Sender<(Vec<u8>, IpAddr)>,
    ) {
        let retransmission_interval = Duration::from_millis(200); // Define the interval for retransmission rounds

        let matrix_buffer = MATRIX_BUFFER.get().unwrap();

  
        loop {

            let nc_matrix = client_sessions.generate_nc_matrix();

            // info!("NC Matrix: {:?}", nc_matrix);

            for (matrix_index, (grid, clients_vec)) in nc_matrix.iter() {
                let matrix =  matrix_buffer.get_matrix(*matrix_index as usize).unwrap();
                let vec = RetransmissionManager::grid_to_vec(grid);

                // info!("Matrix {}: {:?}", matrix_index, vec);

                let transmissions = find_optimal_retransmissions(vec);
                for t in transmissions {
                    match t {
                        Transmission::Single(p) => {
                            // log::debug!("Send packet {}", p);
                            let (row, col) = (
                                p / matrix.cols(),
                                p % matrix.cols(),
                            );

                            if let Some(packet) = matrix.get(row, col) {
                                assert!(packet.index == p as u32);
                                let retransmission = RetransmissionPacket {
                                    packet: Some(packet.clone())
                                };
                                // multicast_tx.send(retransmission.encode_to_vec()).unwrap();
                            }
                        } 
                        Transmission::Xor(ref combo) => {
                            // log::debug!("Send XOR({})", combo.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", "))
                        }
                    }
                }
            }


            
            //                 metrics.clients.with_label_values(&[&client_id]).set(1.0);
            //                 let _ = metrics.clients.remove_label_values(&[&client_id]);
            

            // for entry in client_sessions.clients.iter() {
            //     let (ip, client_data) = entry.pair();
                
            //     info!("Client data: {:?}", client_data.last_seen);
                
            //     //  Check missing packets

            //     if let Some(missing_packets) = client_data.get_missing_packets() {

            //         // info!("Missing packets: {:?}", missing_packets);
            //         // Send the missing packets to the client
            //         for (matrix_missing_index, bitmap) in missing_packets {

                        
            //             if matrix_buffer.is_valid_index(matrix_missing_index as usize) {
            //                 if let Some(matrix) = matrix_buffer.get_matrix(matrix_missing_index as usize) {
                                
            //                     for packet_index in bitmap {

            //                         let (row, col) = (
            //                             packet_index / matrix.cols(),
            //                             packet_index % matrix.cols(),
            //                         );

            //                         if let Some(packet) = matrix.get(row, col) {
            //                             // log::debug!("matrix {} - packet index - {}", packet.matrix, packet.index);

            //                             let retransmission = RetransmissionPacket {
            //                                 packet: Some(packet.clone())
            //                             };
            //                             unicast_tx.send((retransmission.encode_to_vec(), *ip)).unwrap();
            //                             // multicast_tx.send(retransmission.encode_to_vec()).unwrap();
            //                         }
            //                     }
                               
            //                 }
            //             } else {
            //                 println!("Requested index {} is too old or not yet available", matrix_missing_index);
            //             }
                        
            //         }

            //     } else {
            //         // info!("No missing packets for client: {:?}", ip);
            //     }

                


            //     // You can now inspect or act on client_data
            //     // println!("Client IP: {}, Data: {:?}", ip, client_data);
            // }

            
                

            std::thread::sleep(retransmission_interval);

        }
        
    }
}
