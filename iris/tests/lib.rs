fn insert_value<T>(matrix: &mut Vec<Vec<T>>, position: usize, matrix_size: usize, value: T) {
    let row = position / matrix_size;
    let col = position % matrix_size;

    if row < matrix_size && col < matrix_size { 
        matrix[row][col] = value; 
     } else { 
        println!("Invalid position!"); 
     }
}


#[cfg(test)]
mod tests {
    
    
    
    #[test]
    fn test_reed_solomon() {
        use reed_solomon_16::{ReedSolomonDecoder, ReedSolomonEncoder};
        
        // use prost instead of tonic custom trait implementation
        use prost::Message;
        use grid::Grid as Matrix;

        use std::sync::mpsc;
        use std::thread;

        // Import protobuff for testing
        mod proto {
            tonic::include_proto!("streaming_server");
        }
        
        const ROW : usize = 3;
        const COL : usize = 4;

        const ROW_FEC : usize = 2;
        const COL_FEC : usize = 1;

        const SHARD_SIZE : usize = 64;

        // This is the matrix we will save globally
        let mut matrix: Matrix<Vec<u8>> = Matrix::new(0, 0);

        let data = b"hello data".to_vec();

        for _ in 0..ROW {
            let mut row : Vec<Vec<u8>> = vec![vec![0; SHARD_SIZE]; COL];

            for j in 0..COL {
                // Replace row index with encoded data 
                row[j][0..data.len()].copy_from_slice(&data);
            }
            matrix.push_row(row);
        }

        println!("{:?}", matrix.size());

        // Create transmittion matrix

        let mut row_encoder = ReedSolomonEncoder::new(
            COL, // total number of col, witch gives the length of row
            ROW_FEC, // total number of row recovery shards
            SHARD_SIZE, // shard size in bytes
        ).unwrap();

        let mut col_encoder = ReedSolomonEncoder::new(
            ROW, // total number of col, witch gives the length of row
            COL_FEC, // total number of row recovery shards
            SHARD_SIZE, // shard size in bytes
        ).unwrap();


        let mut trasmition_matrix: Matrix<proto::TransmitionPacket> = Matrix::new(0, 0);

        let mut matrix_counter = 0;
        let mut matrix_index = 0;

        for rows in matrix.iter_rows() {

            let mut row: Vec<proto::TransmitionPacket> = Vec::new();

            for (item_index, item) in rows.enumerate() {

                // All this packets are DataPackets 
                row.push(proto::TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index,
                    rows: (ROW + COL_FEC) as i32,
                    cols: (COL + ROW_FEC) as i32,
                    item_type: vec![
                        proto::ItemType{ 
                            code: 2, // its data packet
                            index: item_index as i32 // index of the shard in the row
                        }],
                    payload: item.to_vec(), // copy data to TransmitionPacket
                });

                row_encoder.add_original_shard(item).unwrap();
                matrix_index += 1;
            }

            // Create FEC Shards for the ROW
            let result = row_encoder.encode().unwrap();

            for (item_index, item) in result.recovery_iter().enumerate() {

                // create TransmitionPacket with FEC
                row.push(proto::TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index,
                    rows: (ROW + COL_FEC) as i32,
                    cols: (COL + ROW_FEC) as i32,
                    item_type: vec![
                        proto::ItemType{ 
                            code: 0, // its row FEC packet
                            index: item_index as i32 // index of the shard in the row
                        }],
                    payload: item.to_vec(), // copy data to TransmitionPacket
                });
                matrix_index += 1;
            }
            trasmition_matrix.push_row(row);
        }

        // Add COL_FEC as empty rows
        for _ in 0..COL_FEC {
            trasmition_matrix.push_row(vec![proto::TransmitionPacket { matrix: 0, index: 0, rows: 0 , cols: 0, item_type: vec![proto::ItemType { code: 0, index: 0 };0],  payload: vec![0;0]} ; COL + ROW_FEC]);
        }

        let column_data: Vec<Vec<proto::TransmitionPacket>> = trasmition_matrix.iter_cols().map(|col| col.cloned().collect()).collect();

        // Iterate over collumns of trasmition matrix to update the extra FEC info 
        for (col_index, col) in column_data.iter().enumerate() {
            
            for item in col.iter().take(ROW) {
                // update col paridy index
                col_encoder.add_original_shard(&item.payload.to_vec()).unwrap();
            }

            // Create FEC Shards for the ROW
            let result = col_encoder.encode().unwrap();

            for (item_index, item) in result.recovery_iter().enumerate() {
                // create TransmitionPacket with FEC
                trasmition_matrix[(ROW + item_index, col_index)] = proto::TransmitionPacket {
                    matrix: matrix_counter,
                    index: ((ROW + item_index) * (COL + ROW_FEC) + col_index) as i32 ,                    
                    rows: (ROW + COL_FEC) as i32,
                    cols: (COL + ROW_FEC) as i32,
                    item_type: vec![
                        proto::ItemType{ 
                            code: 1, // its col FEC packet
                            index: item_index as i32 // index of the shard in the row
                        }],
                    payload: item.to_vec(), // copy data to TransmitionPacket
                };             
                matrix_counter += 1;
   
            }
        }
        
        for (row_index, row) in trasmition_matrix.iter_rows().enumerate(){
            for (col_index, col) in trasmition_matrix.iter_cols().enumerate() {
                println!("({:?}, {:?}) = {:?}", row_index, col_index, trasmition_matrix[(row_index, col_index)].index);
            }
        }
        println!("{:?}", trasmition_matrix.size());

        // Channel for communication between threads
        let (tx, rx) = mpsc::channel();

        // Sender thread
        let sender = thread::spawn(move || {
            for row in trasmition_matrix.iter_rows() {
                for packet in row {
                    let mut buf = Vec::new();
                    packet.encode(&mut buf).unwrap();
                    tx.send(buf).unwrap();
                }
            }
        });

        // Receiver thread
        let receiver = thread::spawn(move || {
            while let Ok(received) = rx.recv() {
                let packet = proto::TransmitionPacket::decode(&*received).unwrap();
                println!("Received packet: {:?}", packet);
            }
        });

        sender.join().unwrap();
        receiver.join().unwrap();

    }


    #[test]
    fn reed_solomon_16_default() {
        let original = [
            b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do ",
            b"eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut e",
            b"nim ad minim veniam, quis nostrud exercitation ullamco laboris n",
        ];

        let recovery = reed_solomon_16::encode(
            3, // total number of original shards
            5, // total number of recovery shards
            original, // all original shards
        ).unwrap();

        let restored = reed_solomon_16::decode(
            3, // total number of original shards
            5, // total number of recovery shards
            [  // provided original shards with indexes
                (1, &original[1]),
            ],
            [  // provided recovery shards with indexes
                (1, &recovery[1]),
                (4, &recovery[4]),
            ],
        ).unwrap();

        assert_eq!(restored[&0], original[0]);
        assert_eq!(restored[&2], original[2]);
    }
    
    #[test]
    fn insertion() {
        let mut numbers = vec![1, 2, 3, 4, 5];
        let index_to_add_to = 2; // Index at which you want to add the number
        let number_to_add = 10;
    
        // Ensure the index is within the bounds of the vector
        if index_to_add_to <= numbers.len() {
            numbers.insert(index_to_add_to, number_to_add);
            println!("Updated vector: {:?}", numbers);
        } else {
            println!("Index out of bounds");
        }
    }

    #[test]
    fn insert_value_valid_position() {
        use super::insert_value;

        let matrix_size = 4;
        let mut matrix = vec![vec![0; matrix_size]; matrix_size];
        let postion = 5;
        // Calculate the row and column indices without subtracting 1
        let row = postion / matrix_size;
        let col = postion  % matrix_size;
        insert_value(&mut matrix, postion, matrix_size, 10);

        assert_eq!(matrix[row][col], 10);
    }

    #[test]
    fn test_encrypt_decrypt() {
        use openssl::symm::{Cipher, Crypter, Mode};
        use openssl::rand::rand_bytes;
        
        fn generate_random_bytes(size: usize) -> Vec<u8> {
            let mut buf = vec![0; size];
            rand_bytes(&mut buf).expect("Failed to generate random bytes");
            buf
        }

        fn encrypt(cipher: Cipher, data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
            let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
            let mut ciphertext = vec![0; data.len() + cipher.block_size()];
            let mut count = crypter.update(data, &mut ciphertext)?;
            count += crypter.finalize(&mut ciphertext[count..])?;
            ciphertext.truncate(count);
            Ok(ciphertext)
        }
        
        fn decrypt(cipher: Cipher, data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
            let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))?;
            let mut plaintext = vec![0; data.len() + cipher.block_size()];
            let mut count = crypter.update(data, &mut plaintext)?;
            count += crypter.finalize(&mut plaintext[count..])?;
            plaintext.truncate(count);
            Ok(plaintext)
        }
       
        let data = b"Hello, OpenSSL with Rust!";
        
        let key = generate_random_bytes(32); // 256 bits for AES-256
        let iv = generate_random_bytes(16);  // 128 bits for AES block size
        
        let encrypted_data = encrypt(Cipher::aes_256_cbc(), data, &key, &iv)
            .expect("Failed to encrypt data");
        
        
        let decrypted_data = decrypt(Cipher::aes_256_cbc(), &encrypted_data, &key, &iv)
            .expect("Failed to decrypt data");
                
        assert_eq!(data.to_vec(), decrypted_data);
    }
    
    #[test]
    fn add_row_fec(){
       
        // Define the input vectors
        let input = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
        ];

        // Transpose the input using map and zip
        let transposed: Vec<Vec<_>> = (0..input[0].len())
            .map(|col_index| input.iter().map(|row| row[col_index]).collect())
            .collect();

        // Print the transposed vector of arrays
        for row in &transposed {
            println!("{:?}", row);
        }
        println!("------");
        
        // Define the input vectors

        let input = vec![
            vec![1, 4],
            vec![2, 5],
            vec![3, 6],
        ];

        // Transpose the rows, appending columns to the matrix
        let transposed_fec: Vec<Vec<_>> = (0..input[0].len())
        .map(|row_index| {
            input.iter().map(|row| row[row_index].clone()).collect()
        })
        .collect();


        // Print the transposed vector of arrays
        for row in &transposed_fec {
            println!("{:?}", row);
        }
            
        
        //assert_eq!(dm, dm1);

    }
}
