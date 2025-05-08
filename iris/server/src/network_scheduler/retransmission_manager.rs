use std::{
    net::IpAddr,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::{metrics::get_metrics, proto::RetransmissionPacket};
use crate::{membership_manager::client_sessions::ClientSessions, MATRIX_BUFFER};

use crossbeam_channel::Sender;
use grid::Grid;
use log::info;
use prost::Message;

// Retransmission manager struct
pub struct RetransmissionManager {}

impl RetransmissionManager {
    // Initialize the manager with receivers for multicast and unicast retransmissions
    pub fn new() -> Self {
        Self {}
    }

    // pub fn handle_retransmissions(
    //     client_sessions: Arc<RwLock<ClientSessions>>,
    //     multicast_tx: Sender<Vec<u8>>,
    //     unicast_tx: Sender<(Vec<u8>, IpAddr)>,
    // ) {
    // let retransmission_interval = Duration::from_millis(200); // Define the interval for retransmission rounds

    // fn find_zero_indices(vec: Vec<u8>) -> Vec<usize> {
    //     vec.iter()
    //         .enumerate()
    //         .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
    //         .collect()
    // }

    // fn apply_to_tuples(vec_of_tuples: Vec<(i32, Vec<u8>)>) -> Vec<(i32, usize)> {
    //     vec_of_tuples
    //         .into_iter()
    //         .map(|(num, vec)| (num, find_zero_indices(vec).len()))
    //         .collect()
    // }

    // let metrics = get_metrics();

    // let matrix_buffer = MATRIX_BUFFER.get().unwrap();
    // let mut packet_loss_metric=  Vec::new();
    // let mut last_log = Instant::now(); // Track the last time we logged
    // loop {
    //     {
    //         let client_sessions_guard = client_sessions.read().unwrap();
    //         let matrix_buffer_guard = matrix_buffer.read().unwrap();

    //         for client in client_sessions_guard.clients.iter() {
    //             let result = apply_to_tuples(client.1.reports.clone());

    //             info!("Client: {:?} \n {:?}", client.0, result);
    //         }

    //         if last_log.elapsed() >= Duration::from_secs(1) {

    //             let binding = metrics.packet_loss.lock().unwrap();

    //             if !packet_loss_metric.is_empty() {
    //                 let sum: f64 = packet_loss_metric.iter().sum();
    //                 let avg = sum / packet_loss_metric.len() as f64;

    //                 binding.set(avg);

    //                 info!("Average Packet Loss: {:?}", avg);
    //             }

    //             packet_loss_metric.clear(); // Clear the metric vector after logging

    //             last_log = Instant::now(); // Reset the log timer
    //         }

    //         let packet_loss = client_sessions_guard.calculate_packet_loss() as f64;

    //         {
    //             // Update the packet loss metric, -1.0 indicates no data
    //             if packet_loss != -1.0 {
    //                 packet_loss_metric.push(packet_loss);
    //             }
    //         }

    //         std::thread::sleep(retransmission_interval);
    //     }
    // }
    // }

    // Run the retransmission handler that listens to both channels
    pub fn handle_retransmissions(
        client_sessions: Arc<ClientSessions>,
        multicast_tx: Sender<Vec<u8>>,
        unicast_tx: Sender<(Vec<u8>, IpAddr)>,
    ) {
        let retransmission_interval = Duration::from_millis(200); // Define the interval for retransmission rounds

        let matrix_buffer = MATRIX_BUFFER.get().unwrap();
        let metrics = get_metrics();

        
        loop {

            
            // let matrix_buffer_guard = matrix_buffer.read().unwrap();

            // let buffer_copy = matrix_buffer_guard.clone();
            
            // drop(matrix_buffer_guard); 
            for entry in client_sessions.clients.iter() {
                let ip = entry.key();
                let client_data = entry.value();
                
                // info!("Client data: {:?}", client_data.last_seen);
                
                //  Check missing packets

                if let Some(missing_packets) = client_data.get_missing_packets() {

                    // info!("Missing packets: {:?}", missing_packets);
                    // Send the missing packets to the client
                    for (matrix_missing_index, bitmap) in missing_packets {

                        
                        if matrix_buffer.is_valid_index(matrix_missing_index as usize) {
                            if let Some(matrix) = matrix_buffer.get_matrix(matrix_missing_index as usize) {
                                
                                for packet_index in bitmap {

                                    let (row, col) = (
                                        packet_index / matrix.cols(),
                                        packet_index % matrix.cols(),
                                    );

                                    if let Some(packet) = matrix.get(row, col) {
                                        log::debug!("matrix {} - packet index - {}", packet.matrix, packet.index);

                                        let retransmission = RetransmissionPacket {
                                            packet: Some(packet.clone())
                                        };
                                        multicast_tx.send(retransmission.encode_to_vec()).unwrap();
                                    }
                                }
                               
                            }
                        } else {
                            println!("Requested index {} is too old or not yet available", matrix_missing_index);
                        }
                        
                    }

                } else {
                    info!("No missing packets for client: {:?}", ip);
                }

                


                // You can now inspect or act on client_data
                // println!("Client IP: {}, Data: {:?}", ip, client_data);
            }

            let packet_loss = client_sessions.calculate_packet_loss();
            // Update the packet loss metric, -1.0 indicates no data
            
            {   
                if packet_loss != -1.0 {
                    let binding = metrics.packet_loss.lock().unwrap();
                    binding.set(packet_loss as f64);
                }
                // metrics.packet_loss.lock().unwrap().set(packet_loss as f64);
            }

            std::thread::sleep(retransmission_interval);

        }
        // fn find_zero_indices(vec: Vec<u8>) -> Vec<usize> {
        //     vec.iter()
        //         .enumerate()
        //         .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
        //         .collect()
        // }

        // fn apply_to_tuples(vec_of_tuples: Vec<(i32, Vec<u8>)>) -> Vec<(i32, usize)> {
        //     vec_of_tuples
        //         .into_iter()
        //         .map(|(num, vec)| (num, find_zero_indices(vec).len()))
        //         .collect()
        // }

        // let metrics = get_metrics();

        // let matrix_buffer = MATRIX_BUFFER.get().unwrap();

        // loop {
        //     {
        //         let client_sessions_guard = client_sessions.read().unwrap();
        //         let matrix_buffer_guard = matrix_buffer.read().unwrap();

        //         for client in client_sessions_guard.clients.iter() {
        //             let result = apply_to_tuples(client.1.reports.clone());

        //             info!("Client: {:?} \n {:?}", client.0, result);
        //         }

        //         let packet_loss = client_sessions_guard.calculate_packet_loss() as f64;

        //         {
        //             // Update the packet loss metric, -1.0 indicates no data
        //             if packet_loss != -1.0 {
        //                 let binding = metrics.packet_loss.lock().unwrap();
        //                 binding.set(packet_loss);
        //             }
        //         }

        //         if client_sessions_guard.clients.len() > 0 {
        //             info!("Packet loss: {:?}", packet_loss);
        //         }

        //         // Network matrix
        //         let nc_matrix: std::collections::BTreeMap<i32, (Grid<u8>, Vec<IpAddr>)> =
        //             client_sessions_guard.generate_nc_matrix();

        //         // Drop the locks to avoid deadlocks
        //         drop(client_sessions_guard);

        //         // info!("NC Matrix: {:?}", nc_matrix);
        //         {
        //             // matrix_index, tuple is a tuple of (row, col) and vector of client addresses
        //             for (matrix_index, (matrix_bitmap, clients)) in nc_matrix.iter() {
        //                 // log::debug!("matrix_index: {:?}", matrix_index);

        //                 // Get the matrix options from circular buffer
        //                 if let Some(matrix_option) =
        //                     matrix_buffer_guard.get((*matrix_index).try_into().unwrap())
        //                 {
        //                     // If exist get the grid
        //                     if let Some(grid) = matrix_option {
        //                         // Loop rows to get clients
        //                         // for (client_index, client_iter) in
        //                         //     matrix_bitmap.iter_rows().enumerate()
        //                         // {
        //                         //     let addr = &clients[client_index];
        //                         //     let mut tmp = Vec::new();

        //                         //     // Loop all collumns to send to missing clients
        //                         //     for (packet_pos, packet_bool) in client_iter.enumerate() {
        //                         //         if *packet_bool == 0 {
        //                         //             let packet_index = packet_pos;

        //                         //             tmp.push(packet_index);

        //                         //             let (row, col) = (
        //                         //                 packet_index / grid.cols(),
        //                         //                 packet_index % grid.cols(),
        //                         //             );

        //                         //             if let Some(packet) = grid.get(row, col) {
        //                         //                 // log::debug!("matrix {} - packet index - {}", packet.matrix, packet.index);
        //                         //                 unicast_tx
        //                         //                     .send((packet.encode_to_vec(), *addr))
        //                         //                     .unwrap();
        //                         //             }
        //                         //         }
        //                         //     }

        //                         //     // log::debug!("Client {} missing packets: {:?}", client_index, tmp);

        //                         //     tmp.clear();
        //                         // }
        //                     }
        //                 }
        //             }
        //         }

        //         std::thread::sleep(retransmission_interval);
        //     }
        // }
    }
}
