use crate::CONFIG;
use crossbeam_channel::Sender;
use log::error;
use std::process::{Command, Stdio};
use std::io::{BufReader, Read};
use std::thread;
use grid::Grid as Matrix;

pub fn read_ffmpeg_and_send(tx: Sender<Matrix<Vec<u8>>>) {
    let config = &CONFIG.get().unwrap().multicast;

    let num_rows: usize = config.row;
    let num_cols: usize = config.col;
    let data_size: usize = config.data_size;
    let ffmpeg_input: String = config.ffmpeg_input.clone();
    let ffmpeg_options: String = config.ffmpeg_options.clone();

    thread::spawn( move || {
        // Convert the format string to a String before splitting it into arguments
        let args_string = format!("-loglevel quiet -i {} {} -f mpegts pipe:1", ffmpeg_input, ffmpeg_options);
        let args: Vec<&str> = args_string.split_whitespace().collect();
        
        let mut ffmpeg = match Command::new("./binaries/ffmpeg-amd64/ffmpeg")
            .args(&args)
            .stdin(Stdio::null())  // Avoid blocking the program's stdin
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn() {
                Ok(process) => process,
                Err(e) => {
                    error!("Failed to start ffmpeg: {:?}", e);
                    return;
                }
            };
    
        let stdout = match ffmpeg.stdout.take() {
            Some(stdout) => stdout,
            None => {
                error!("Failed to capture stdout of ffmpeg");
                return;
            }
        };
    
        let mut reader = BufReader::new(stdout);
    
        let mut buffer = vec![0; data_size];
    
        loop {
            if let Some(status) = ffmpeg.try_wait().unwrap() {
                error!("ffmpeg process ended with status: {:?}", status);
                return;
            }
    
            let mut matrix: Matrix<Vec<u8>> = Matrix::new(0, 0);
    
            for _ in 0..num_rows {
                let mut row: Vec<Vec<u8>> = Vec::with_capacity(num_cols);
    
                for _ in 0..num_cols {
                    let mut total_read = 0;
    
                    while total_read < data_size {
                        match reader.read(&mut buffer[total_read..]) {
                            Ok(0) => {
                                // EOF reached
                                error!("ffmpeg output ended unexpectedly");
                                return;
                            }
                            Ok(n) => {
                                total_read += n;
                            }
                            Err(e) => {
                                error!("Error reading data from ffmpeg: {:?}", e);
                                return;
                            }
                        }
                    }
    
                    row.push(buffer.clone());
                }
                matrix.push_row(row);
            }
    
            if let Err(e) = tx.send(matrix) {
                error!("Error sending data to channel: {:?}", e);
                return;
            }
        }
    });
}
