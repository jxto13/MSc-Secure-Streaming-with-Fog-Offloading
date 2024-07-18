use crate::CONFIG;

use crossbeam_channel::Sender;
use log::{info, error};
use std::fs::File;
use std::io::{BufReader, Read};
use grid::Grid as Matrix;

#[allow(non_snake_case)]
pub fn read_file_and_send(file_path: &str, tx: Sender<Matrix<Vec<u8>>>) {
    let config = &CONFIG.get().unwrap().multicast;

    let ROW: usize = config.row;
    let COL: usize = config.col;
    let DATA_SIZE: usize = config.shard_size;

    // Open the file
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            error!("Error opening file: {}", e);
            return;
        }
    };
    let mut reader = BufReader::new(file);

    // Buffer to store the read bytes
    let mut buffer = vec![0; DATA_SIZE];

    'main_loop: loop {
        // This is the matrix we will save globally
        let mut matrix: Matrix<Vec<u8>> = Matrix::new(0, 0);

        for _ in 0..ROW {
            let mut row: Vec<Vec<u8>> = Vec::with_capacity(COL);

            for _ in 0..COL {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("END OF FILE");
                        // End of file reached, break the reading loop
                        break 'main_loop;
                    }
                    Ok(n) => {
                        row.push(buffer[..n].to_vec());
                    }
                    Err(e) => {
                        error!("Error reading data from file: {}", e);
                        return;
                    }
                }
            }
            matrix.push_row(row);
        }

        // Send the read data to the encoder
        if let Err(e) = tx.send(matrix) {
            error!("Error sending data to encoder: {:?}", e);
            return;
        }
    }
}
