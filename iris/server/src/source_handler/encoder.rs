use crate::metrics::get_metrics;
use crate::proto::packet_wrapper::PacketType;
use crate::proto::{PacketWrapper};
use crate::{proto, CONFIG, MANAGER, MATRIX_BUFFER};

use crossbeam_channel::{Receiver, Sender};
use iris_shared::crypto_utils::encrypt;
use iris_shared::padding;
use prost::Message;
use proto::{TransmissionPacket, VideoData};

use openssl::sha::sha256;
use openssl::ecdsa::EcdsaSig;

use reed_solomon_16::ReedSolomonEncoder;
use grid::Grid as Matrix;


// This is running inside a separate task
pub fn start(rx: Receiver<Matrix<Vec<u8>>>, tx: Sender<Vec<u8>>) {
    let config = &CONFIG.get().unwrap().multicast;
    let matrix_buffer = MATRIX_BUFFER.get().unwrap();
    
    let num_rows: usize = config.row;
    let num_cols: usize = config.col;

    let shard_size: usize = config.shard_size;
    
    assert!(num_rows > 0);
    assert!(num_cols > 0);
    assert!(config.mtu_size > 0);
    assert!(shard_size < config.mtu_size);

    // Create counters
    let mut matrix_counter = 0;
    let mut epoch = 0;
    
    let metrics = get_metrics();
    
    // Create transmission matrix
    while let Ok(matrix) = rx.recv() {
        
        let mut transmission_matrix: Matrix<TransmissionPacket> = Matrix::new(0, 0);
        
        let mut matrix_index = 0;

        let manager = MANAGER.get().unwrap();

        // If epoch is equal to config, rotate keys and reset counter
        if epoch == config.epoch {

            // Get write lock 
            let mut binding_write = manager.write().unwrap();

            // Rotate keys
            binding_write.rotate_key();
           
            // Reset epoch
            epoch = 0;
        }
        
        // Get keys outside the loop
        let binding = manager.read().unwrap();

        let ec_key = binding.get_ec_key();
        let current_key = binding.get_current_key();
        let next_key = binding.get_next_key();

        let fec_params = (metrics.fec_row.get() as usize, metrics.fec_col.get() as usize);


        // Create encoders objects (This can be put on the loop for dynamic fec_rows and fec_cols changes)
        let mut row_encoder = match ReedSolomonEncoder::new(
            num_cols,       // total number of col, which gives the length of row
            fec_params.0,   // total number of row recovery shards
            shard_size // shard size in bytes
        ) {
            Ok(encoder) => Some(encoder),
            Err(_) => {
                None
            }
        };
        
        let mut col_encoder = match ReedSolomonEncoder::new(
            num_rows,       // total number of col, which gives the length of row
            fec_params.1,   // total number of row recovery shards
            shard_size // shard size in bytes
        ) {
            Ok(encoder) => Some(encoder),
            Err(_) => {
                None
            }
        };

        // Iterate over the matrix and create the packets
        for rows in matrix.iter_rows() {
            let mut row: Vec<TransmissionPacket> = Vec::new();

            for item in rows {
                let mut video_data = VideoData {
                    data: item.clone(),
                    signature: vec![0,255],
                    next_key: Some(next_key.clone()),
                };

                // Create hash of message and sign
                let message_hash = sha256(&video_data.encode_to_vec());
                let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();

                video_data.signature = signature;

                let mut encryped_payload = encrypt(&video_data.encode_to_vec(), &current_key.key, &current_key.iv).unwrap();
                
                // padding the payload to match the shard size
                padding::pad(&mut encryped_payload, shard_size);

                let mut packet = TransmissionPacket {
                    matrix: matrix_counter,
                    index: matrix_index as u32,
                    rows: num_rows as u32,
                    cols: num_cols as u32,
                    rows_fec: fec_params.0 as u32,
                    cols_fec: fec_params.1 as u32,
                    payload: encryped_payload,
                    packet_signature: vec![0,255],
                };

                let message_hash = sha256(&packet.encode_to_vec());
                let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();
                
                packet.packet_signature = signature;
                
                row.push(packet.clone());
                
                // Check if we have a row_encoder
                if let Some(ref mut row_encoder) = row_encoder {
                    row_encoder.add_original_shard(&packet.payload).unwrap();                    
                }

                matrix_index += 1;
            }


            if let Some(ref mut row_encoder) = row_encoder {

                // Create FEC Shards for the ROW
                let result = row_encoder.encode().unwrap();

                for item in result.recovery_iter() {

                    let mut packet = TransmissionPacket {
                        matrix: matrix_counter,
                        index: matrix_index,
                        rows: num_rows as u32,
                        cols: num_cols as u32,
                        rows_fec: fec_params.0 as u32,
                        cols_fec: fec_params.1 as u32,
                        payload: item.to_vec(),
                        packet_signature: vec![0,255],
                    };

                    let message_hash = sha256(&packet.encode_to_vec());
                    let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();

                    packet.packet_signature = signature;

                    row.push(packet);

                    matrix_index += 1;
                }
            }
            transmission_matrix.push_row(row);
        }

        // Check if we have a col_encoder
        if let Some(ref mut col_encoder) = col_encoder {
           
            // Add COL_FEC as empty rows
            for _ in 0..fec_params.1 {
                transmission_matrix.push_row(vec![TransmissionPacket::default(); num_cols + fec_params.0]);
            }

            let column_data: Vec<Vec<TransmissionPacket>> = transmission_matrix.iter_cols().map(|col| col.cloned().collect()).collect();

            // Iterate over columns of transmission matrix to update the extra FEC info
            for (col_index, col) in column_data.iter().enumerate() {
                for item in col.iter().take(num_rows) {
                    // update col parity index
                    col_encoder.add_original_shard(&item.payload).unwrap();
                }

                // Create FEC Shards for the ROW
                let result = col_encoder.encode().unwrap();

                for (item_index, item) in result.recovery_iter().enumerate() {

                    let mut packet = TransmissionPacket {
                        matrix: matrix_counter,
                        index: ((num_rows + item_index) * (num_cols + fec_params.0) + col_index) as u32,
                        rows: num_rows as u32,
                        cols: num_cols as u32,
                        rows_fec: fec_params.0 as u32,
                        cols_fec: fec_params.1 as u32,
                        payload: item.to_vec(),
                        packet_signature: vec![0,255],
                    };

                    let message_hash = sha256(&packet.encode_to_vec());
                    let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();

                    packet.packet_signature = signature;

                    transmission_matrix[(num_rows + item_index, col_index)] = packet;
                }
            }
        }
        
        // Loop the matrix col by col and send the data to the client via UDP multicast
        for col in transmission_matrix.iter_cols() {

            for item in col {
                // Wrap the item in PacketWrapper
                let packet = PacketWrapper {
                    packet_type: Some(PacketType::TransmissionPacket(item.clone())),
                };

                // Serialize IrisPacket
                let iris_serialized_data = packet.encode_to_vec();
                
                // Send the buffer
                tx.send(iris_serialized_data).unwrap_or_else(|e| {
                    panic!("error sending data {:?}", e);
                });
            }
        }

        // Add the matrix to the history
        matrix_buffer.add_matrix(matrix_counter as usize, transmission_matrix);

        // Increment epoch
        epoch += 1;

        // Increment matrix counter
        matrix_counter += 1;

        
    }
}
