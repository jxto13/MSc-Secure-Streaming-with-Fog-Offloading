use crate::CONFIG;

use crossbeam_channel::Sender;
use log::{info, error};
use std::process::{Command, Stdio};
use std::io::{BufReader, Read};
use std::thread;
use grid::Grid as Matrix;

#[allow(non_snake_case)]
pub fn read_ffmpeg_and_send(tx: Sender<Matrix<Vec<u8>>>) {
    let config = &CONFIG.get().unwrap().multicast;

    let ROW: usize = config.row;
    let COL: usize = config.col;
    let DATA_SIZE: usize = config.shard_size;


    let args: Vec<_> = "-loglevel quiet -f v4l2 -video_size 1280x720 -i /dev/video0 -c:v libx264 -b:v 5000k -maxrate 5000k -bufsize 10000k -preset ultrafast -tune zerolatency -streaming 1 -f mpegts pipe:1"
        .split_whitespace()
        .collect();
    
    thread::spawn(move || {
        // Command to execute ffmpeg and capture its output
        let mut ffmpeg = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::null())  // Avoid blocking the program's stdin
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start ffmpeg");

        let stdout = ffmpeg.stdout.take().expect("Failed to open stdout");

        let mut reader = BufReader::new(stdout);

        let mut buffer = vec![0; DATA_SIZE];

        'main_loop: loop {
            let mut matrix: Matrix<Vec<u8>> = Matrix::new(0, 0);

            for _ in 0..ROW {
                let mut row: Vec<Vec<u8>> = Vec::with_capacity(COL);

                for _ in 0..COL {
                    let mut total_read = 0;
                    while total_read < DATA_SIZE {
                        match reader.read(&mut buffer[total_read..]) {
                            Ok(0) => {
                                info!("END OF FILE");
                                break 'main_loop;
                            }
                            Ok(n) => {
                                total_read += n;
                            }
                            Err(e) => {
                                error!("Error reading data from ffmpeg output: {}", e);
                                return;
                            }
                        }
                    }
                    row.push(buffer.clone());
                }
                matrix.push_row(row);
            }

            if let Err(e) = tx.send(matrix) {
                error!("Error sending data to encoder: {:?}", e);
                return;
            }
        }
    });
}
