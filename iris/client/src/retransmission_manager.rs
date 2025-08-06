use log::{error, info};
use prost::Message;
use std::{
    sync::Arc,
    thread,
    time::Duration,
};
use tokio::runtime::Runtime;

use crate::{
    proto::{
        self, mesh_packet::PacketType, MeshPacket, PacketWrapper, RetransmissionResponse,
        TransmissionPacket,
    },
    receiver::ReceiverType,
};

#[derive(Debug)]
pub struct RetransmissionManager {
    runtime: Arc<Runtime>, // Tokio runtime for async operations
    packet_requester: crossbeam_channel::Sender<(
        u32,
        u32,
        crossbeam_channel::Sender<Option<crate::proto::TransmissionPacket>>,
    )>, // Channel to request packets
    requests_channel: loole::Receiver<MeshPacket>, // Channel for retransmission requests
    data_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>, // Channel to send data packets
}

impl RetransmissionManager {
    pub fn new(
        runtime: Arc<Runtime>,
        packet_requester: crossbeam_channel::Sender<(
            u32,
            u32,
            crossbeam_channel::Sender<Option<crate::proto::TransmissionPacket>>,
        )>,
        requests_channel: loole::Receiver<MeshPacket>,
        data_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>,
    ) -> Self {
        Self {
            runtime,
            packet_requester,
            requests_channel,
            data_sender,
        }
    }

    pub fn middleware(
        &self,
        node_rx: loole::Sender<MeshPacket>,
        node_tx: &loole::Receiver<MeshPacket>,
    ) {
        info!("RetransmissionManager middleware started");
        loop {
            // Requests from Decoder, sends to mesh network
            if let Ok(msg) = self.requests_channel.try_recv() {
                // info!("Processing request from | DECODER | to MESH: {:?}", msg);

                node_rx.send(msg).unwrap_or_else(|e| {
                    error!("Failed to send message to MESH channel: {}", e);
                });
            }

            // Receiving request from MESH channel, this is receiving a packet from mesh network
            if let Ok(mesh_packet) = node_tx.try_recv() {
                // info!("Received request from | MESH |  {:?}", mesh_packet);

                // Get the packet from the decoder if we have it
                match mesh_packet.packet_type {
                    Some(PacketType::Request(request)) => {
                        let matrix_index = request.matrix as u32;
                        let packet_index = request.packet as u32;

                        // Request the packet from the decoder
                        let (response_sender, response_receiver) = crossbeam_channel::unbounded();
                        if self
                            .packet_requester
                            .send((matrix_index, packet_index, response_sender))
                            .is_ok()
                        {
                            // Wait for response with a short timeout
                            match response_receiver
                                .recv_timeout(std::time::Duration::from_millis(50))
                            {
                                Ok(Some(packet)) => {
                                    let payload_len = packet.payload.len();
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
                                    } else {
                                        info!(
                                            "Sent response for matrix {} and index {}: data len = {}",
                                            matrix_index, packet_index, payload_len
                                        );
                                    }
                                }
                                Ok(None) => {
                                    error!(
                                        "No packet found for matrix {} and index {}",
                                        matrix_index, packet_index
                                    );
                                }
                                Err(_) => {
                                    error!(
                                        "Timeout requesting packet for matrix {} and index {}",
                                        matrix_index, packet_index
                                    );
                                }
                            }
                        } else {
                            error!("Failed to send packet request");
                        }
                    }

                    Some(PacketType::Response(data)) => {
                        info!("Received response packet: {:?}", data);
                    
                        
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
