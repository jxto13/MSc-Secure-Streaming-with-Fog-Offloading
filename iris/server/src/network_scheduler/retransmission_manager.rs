use std::{
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

use crate::{metrics::get_metrics, proto::RetransmissionPacket};
use crate::{membership_manager::client_sessions::ClientSessions, MATRIX_BUFFER};

use crossbeam_channel::Sender;
use log::info;
use prost::Message;

// Retransmission manager struct
pub struct RetransmissionManager {}

impl RetransmissionManager {
    // Initialize the manager with receivers for multicast and unicast retransmissions
    pub fn new() -> Self {
        Self {}
    }

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
                                        unicast_tx.send((retransmission.encode_to_vec(), *ip)).unwrap();
                                        // multicast_tx.send(retransmission.encode_to_vec()).unwrap();
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
            

            if packet_loss != -1.0 {
                metrics.packet_loss.set(packet_loss.into());
            }
                

            std::thread::sleep(retransmission_interval);

        }
        
    }
}
