use log::{error, info};
use prost::Message;
use std::{
    collections::VecDeque,
    sync::Mutex,
    thread,
    time::Duration,
};
use crossbeam_channel::Receiver;

use crate::{
    proto::{
        self, mesh_packet::PacketType, MeshPacket, PacketWrapper, RetransmissionResponse,
    },
    receiver::ReceiverType,
    decoder::MatrixTracker,
};

/// Manages retransmission requests between the decoder and mesh network.
/// 
/// This manager maintains a local cache of matrix trackers received from the decoder
/// during periodic synchronization. The cache grows up to a configurable limit,
/// allowing the manager to serve requests for older matrices that may no longer
/// be in the decoder's active buffer.
#[derive(Debug)]
pub struct RetransmissionManager {
    buffer_sync_receiver: Option<Receiver<Vec<MatrixTracker>>>, // Channel to receive buffer snapshots from decoder
    requests_channel: loole::Receiver<MeshPacket>, // Channel for retransmission requests  
    data_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>, // Channel to send data packets
    cached_buffer: Mutex<VecDeque<MatrixTracker>>, // Local buffer cache (FIFO, oldest matrices removed first) - thread-safe
    max_cache_size: Mutex<usize>, // Maximum number of matrices to keep in cache - thread-safe
}

impl RetransmissionManager {
    pub fn new(
        buffer_sync_receiver: Option<Receiver<Vec<MatrixTracker>>>,
        requests_channel: loole::Receiver<MeshPacket>,
        data_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>,
    ) -> Self {
        Self {
            buffer_sync_receiver,
            requests_channel,
            data_sender,
            cached_buffer: Mutex::new(VecDeque::new()),
            max_cache_size: Mutex::new(50), // Keep up to 50 matrices in cache (configurable)
        }
    }


    pub fn middleware(
        &self,
        node_rx: loole::Sender<MeshPacket>,
        node_tx: &loole::Receiver<MeshPacket>,
    ) {
        info!("RetransmissionManager middleware started");
        
        loop {
            // Update cached buffer from decoder if new data is available
            if let Some(ref sync_receiver) = self.buffer_sync_receiver {
                if let Ok(new_buffer) = sync_receiver.try_recv() {
                    
                    if let Ok(mut cached_buffer) = self.cached_buffer.lock() {
                        // Add new matrices to cache, replacing existing ones with latest state
                        for new_tracker in new_buffer {
                            let matrix_index = new_tracker.matrix_index;
                            
                            // Check if we already have this matrix in cache
                            let mut found = false;
                            for existing_tracker in cached_buffer.iter_mut() {
                                if existing_tracker.matrix_index == matrix_index {
                                    // Always replace with the newer tracker for latest state
                                    *existing_tracker = new_tracker.clone();
                                    found = true;
                                    break;
                                }
                            }
                            
                            // If not found, add as new matrix
                            if !found {
                                cached_buffer.push_back(new_tracker);
                            }
                        }
                        
                        // Maintain cache size limit by removing oldest matrices
                        let max_size = self.max_cache_size.lock().map(|size| *size).unwrap_or(50);
                        while cached_buffer.len() > max_size {
                            if let Some(_removed) = cached_buffer.pop_front() {
                                // Silently remove old matrices
                            }
                        }
                        
                    }
                }
            }

            // Requests from Decoder, sends to mesh network
            if let Ok(msg) = self.requests_channel.try_recv() {
                node_rx.send(msg).unwrap_or_else(|e| {
                    error!("Failed to send message to MESH channel: {}", e);
                });
            }

            // Receiving request from MESH channel, this is receiving a packet from mesh network
            if let Ok(mesh_packet) = node_tx.try_recv() {
                // Get the packet from the cached buffer if we have it
                match mesh_packet.packet_type {
                    Some(PacketType::Request(request)) => {
                        let matrix_index = request.matrix as u32;
                        let packet_index = request.packet as u32;

                        // Search for the packet in our cached buffer
                        let mut found_packet = None;
                        if let Ok(cached_buffer) = self.cached_buffer.lock() {
                            for tracker in cached_buffer.iter() {
                                if tracker.matrix_index == matrix_index {
                                    let row = packet_index as usize / (tracker.cols + tracker.rows_fec) as usize;
                                    let col = packet_index as usize % (tracker.cols + tracker.rows_fec) as usize;

                                    if let Some(packet) = tracker.get_packet(row, col) {
                                        found_packet = Some(packet.clone());
                                        break;
                                    }
                                }
                            }
                        }

                        match found_packet {
                            Some(packet) => {
                                let mesh_data_packet = MeshPacket {
                                    ttl: 3, // Allow responses to propagate back
                                    packet_type: Some(PacketType::Response(
                                        RetransmissionResponse {
                                            payload: PacketWrapper {
                                                packet_type:
                                                    Some(proto::packet_wrapper::PacketType::TransmissionPacket(
                                                        packet.clone(),
                                                    )),
                                            }.encode_to_vec() 
                                        },
                                    )),
                                };

                                // Send the response back through the mesh
                                if let Err(e) = node_rx.send(mesh_data_packet) {
                                    error!("Failed to send response to mesh: {}", e);
                                }
                            }
                            None => {
                                // Packet not found in cache
                            }
                        }
                    }

                    Some(PacketType::Response(data)) => {
                        self.data_sender
                            .send((ReceiverType::WifiAware, data.payload))
                            .unwrap_or_else(|e| {
                                error!("Failed to send data packet: {}", e);
                            });
                            
                    }
                    _ => {
                        error!(
                            "Received unexpected packet type: {:?}",
                            mesh_packet.packet_type
                        );
                    }
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}
