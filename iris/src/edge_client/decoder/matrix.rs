use std::{
    io::{self, Cursor, Write},
    process::{Command, Stdio},
    sync::mpsc::Receiver,
    vec,
    error::Error,
};

use mpeg2ts::ts::{ReadTsPacket, TsPacketReader};
use prost::Message;

use crate::proto::{DataPacket, IrisPacket};

pub fn start(receiver: Receiver<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    let mut mpv = Command::new("mpv")
        .args(&["-"])
        .stdin(Stdio::piped())
        .spawn()?;

    // Get handle to mpv's stdin
    let mut mpv_stdin = mpv.stdin.take().ok_or("Failed to open mpv stdin")?;

    loop {
        // Check if the mpv process has exited
        match mpv.try_wait() {
            Ok(Some(status)) => {
                eprintln!("mpv exited with status: {}", status);
                return Ok(());
            }
            Ok(None) => {
                // Process is still running, continue with the next iteration
            }
            Err(err) => {
                eprintln!("Failed to check mpv status: {}", err);
                return Err(Box::new(err));
            }
        }

        // Blocking until cells in the channel
        let matrix_cell = match receiver.recv() {
            Ok(matrix_cell) => matrix_cell,
            Err(err) => {
                eprintln!("Receiver error: {}", err);
                return Err(Box::new(err));
            }
        };

        let deserialized_matrix_cell = IrisPacket::decode(&*matrix_cell).unwrap_or_else(|err| {
            // Handle the error if creating the runtime fails
            eprintln!("Error decoding packet: {}", err);
            // Create an empty cell, will discard later
            IrisPacket {
                payload: vec![DataPacket {
                    matrix: 0,
                    index: 0,
                    width: 0,
                    height: 0,
                    fec: false,
                    data: vec![0, 0],
                }],
            }
        });

        for packet in deserialized_matrix_cell.payload {
            if !packet.fec {
                mpv_stdin.write_all(&packet.data)?;
            }
        }
    }

    // Ensure mpv process is cleaned up
    let _ = mpv.wait();
    Ok(())
}
