use std::{net::SocketAddr, sync::{Arc, Mutex, RwLock}, time::Duration};

use chrono::Utc;
use circular_buffer::CircularBuffer;
use crossbeam_channel::Sender;
use grid::Grid;
use log::info;
use prost::Message;
use crate::{proto::RetransmissionPacket, TransmissionPacket, CONFIG, FEC_PARAMS};
use crate::{membership_manager::client_sessions::{self, ClientSessions}, MATRIX_BUFFER};
use crate::metrics::get_metrics;



// Retransmission manager struct
pub struct RetransmissionManager {
    
}

impl RetransmissionManager {
    // Initialize the manager with receivers for multicast and unicast retransmissions
    pub fn new() -> Self {
        Self {
        }
    }

    // Run the retransmission handler that listens to both channels
    pub fn handle_retransmissions(client_sessions: Arc<RwLock<ClientSessions>>, multicast_tx: Sender<Vec<u8>>, unicast_tx: Sender<(Vec<u8>,SocketAddr)>) {

        
        let retransmission_interval = Duration::from_millis(200); // Define the interval for retransmission rounds

        fn find_zero_indices(vec: Vec<u8>) -> Vec<usize> {
            vec.iter()
                .enumerate()
                .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
                .collect()
        }
        
        fn apply_to_tuples(vec_of_tuples: Vec<(i32, Vec<u8>)>) -> Vec<(i32, usize)> {
            vec_of_tuples
                .into_iter()
                .map(|(num, vec)| (num, find_zero_indices(vec).len()))
                .collect()
        }

        fn adjust_fec_values(entropy: f64, current_fec_rows: usize, current_fec_cols: usize) -> (usize, usize) {

            if entropy < 0.2 {
                (current_fec_rows, current_fec_cols)
            } else if entropy < 0.4 {
                // Moderate entropy, slight increase
                (current_fec_rows + 1, current_fec_cols + 1)
            } else if entropy < 1.0 {
                // High entropy, double the FEC
                (current_fec_rows + 2, current_fec_cols + 2)
            } else {
                (1, 1)
            }
        }

        // Check the time for sending reports
        let mut fec_timer = Utc::now();        

        let metrics = get_metrics();

        let mut best_entropy = 0.0;

        fn find_most_missing_packets(grid: &Grid<u8>) -> Vec<(usize, u32)> {
            let cols = grid.cols();
        
            // Initialize a vector to hold the count of missing packets for each column
            let mut missing_counts = vec![0; cols];
        
            for (col_index,col_iter) in grid.iter_cols().enumerate() {
                for item in col_iter {
                    if *item == 0 {
                        missing_counts[col_index] += 1;
                    }
                }
            }

            let result: Vec<(usize, u32)> = missing_counts
                .iter()
                .enumerate() // Add indices
                .filter(|&(_, &count)| count != 0) // Filter out zeros
                .map(|(index, &count)| (index, count)) // Create tuples
                .collect(); // Collect into a Vec

            result
        }
        let matrix_buffer = MATRIX_BUFFER.get().unwrap();
        let mut counter = 0;
        loop {

            {
                let matrix_buffer_guard = matrix_buffer.read().unwrap();
                let client_sessions = client_sessions.read().unwrap();

                for (client_address, client_data) in client_sessions.clients.iter() {
                    let client_data = client_data.read().unwrap();
                    let result = apply_to_tuples(client_data.read().unwrap().reports.clone());
                    
                    info!("Client: {:?} \n {:?}", client_address, result);
                }

                let packet_loss = client_sessions.calculate_packet_loss() as f64;

                {
                    let binding = metrics.packet_loss.lock().unwrap();
                    binding.set(packet_loss);
                }

                if client_sessions.clients.len() > 0 {
                    info!("Packet loss: {:?}", packet_loss);

                }

                let nc_matrix = client_sessions.generate_nc_matrix();

                
                {
                    // matrix_index, tuple is a tuple of (row, col) and vector of client addresses 
                    for (matrix_index, (matrix_bitmap, clients)) in nc_matrix.iter() {



                        // because the nc_matrix is a btree map, the matrix_index is always sorted from lower to higher
                        // matrix_bitmap is a grid, every row is a client and every column is a packet bitmap
                        // the clients vector is the list of clients that are in the matrix, sorted by the order of the rows in the matrix

                        

                        // let multicast_cost = 1.0;
                        // let unicast_cost = 1.5;

                        // if let Some(matrix_option) = matrix_buffer_guard.get((*matrix_index).try_into().unwrap()) {
                        //     if let Some(grid) = matrix_option {
                                
                                // let vec_retransmissions = iris_shared::nc_type::retransmit(matrix_bitmap, multicast_cost, unicast_cost);

                                // info!("Retransmissions: {:?}", vec_retransmissions);
                                // for item in vec_retransmissions {
                                //     match item {
                                //         iris_shared::nc_type::Transmission::Multicast(packet_index) => {
                                //             let (row, col) = (packet_index / grid.cols(), packet_index % grid.cols());
    
                                //             if let Some(packet) = grid.get(row, col) {
                                //                 multicast_tx.send(packet.encode_to_vec()).unwrap();
                                //             }
                                //         },
                                //         iris_shared::nc_type::Transmission::Unicast(client_index, packet_index) => {
                                //             let (row, col) = (packet_index / grid.cols(), packet_index % grid.cols());
    
                                //             if let Some(packet) = grid.get(row, col) {
                                //                 let mut addr = clients[client_index].clone();
                                //                 addr.set_port(CONFIG.get().unwrap().unicast.port);
                                //                 unicast_tx.send((packet.encode_to_vec(), addr)).unwrap();
                                //             }
                                //         },
                                //         iris_shared::nc_type::Transmission::XorMulticast(packet_index_1,packet_index_2) => {
                                //         }
                                //     }
                                // }
                                
                        //     } else {
                        //         log::error!("No grid found.");
                        //     }
                            
                        // } else {
                        //     log::error!("Matrix {} not found in the buffer", matrix_index);
                            
                        // }
                            


                        
                        // // Find packets missing the most
                        // let most_missing = find_most_missing_packets(&matrix_bitmap);
                        // // info!("Most missing packets: {:?}", most_missing);
                        // for (packet_index , _) in &most_missing {

                        //     if let Some(matrix_option) = matrix_buffer_guard.get((*matrix_index).try_into().unwrap()) {
                        //         if let Some(grid) = matrix_option {
                                    
                        //             let (row, col) = (packet_index / grid.cols(), packet_index % grid.cols());
    
                        //             if let Some(packet) = grid.get(row, col) {
                        //                 let mut addr = clients[0].clone();
                        //                 addr.set_port(CONFIG.get().unwrap().unicast.port);
                        //                 unicast_tx.send((packet.encode_to_vec(), addr)).unwrap();
                        //                 counter += 1;
                        //                 // multicast_tx.send(packet.encode_to_vec()).unwrap();
                        //             }
                                    
                        //         } else {
                        //             log::error!("No grid found.");
                        //         }
                                
                        //     } else {
                        //         log::error!("Matrix {} not found in the buffer", matrix_index);
                                
                        //     }
                            
                            
                        // }
                    }
                }

                // {
                //     let binding = metrics.fec_row.lock().unwrap();
                //     binding.set(FEC_PARAMS.get().unwrap().read().unwrap().0.try_into().unwrap());

                //     let binding = metrics.fec_col.lock().unwrap();
                //     binding.set(FEC_PARAMS.get().unwrap().read().unwrap().1.try_into().unwrap());

                // }
                // info!("FEC_PARAMS: {:?}", FEC_PARAMS.get().unwrap().read().unwrap());

                // {   
                //     let entropy = client_sessions_guard.calculate_entropy();
                //     info!("Client Entropy: {:?}",entropy );

                //     // Check the time for sending reports
                //     let now = Utc::now();

                //     if now - fec_timer >= chrono::Duration::seconds(5) {
                //         if entropy < best_entropy {
                //             let new_fec = adjust_fec_values(entropy, FEC_PARAMS.get().unwrap().read().unwrap().0, FEC_PARAMS.get().unwrap().read().unwrap().1);
                            
                //             info!("New FEC_PARAMS: {:?}", new_fec);
                            
                //             FEC_PARAMS.get().unwrap().write().unwrap().0 = new_fec.0;
                //             FEC_PARAMS.get().unwrap().write().unwrap().1 = new_fec.1;
    
                //             fec_timer = now;
                //             best_entropy = entropy;
                //         }
                //     }
                // }
            }
            // info!("Sent {} retransmission packets", counter);
            counter = 0;
            
            std::thread::sleep(retransmission_interval);
        
        }

    }

    fn encoder() {

    }

}