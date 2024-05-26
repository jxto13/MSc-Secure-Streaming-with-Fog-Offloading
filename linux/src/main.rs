use std::error::Error;
use std::process::{Command, Stdio};
use std::io::{self, Write};

use log::{debug, info};

fn main() -> Result<(), Box<dyn Error>> {
    // Init the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    // Attempt to start the receiver
    let receiver = match iris::edge_client::core::start() {
        Ok(receiver) => receiver,
        Err(e) => {
            debug!("Failed to start receiver: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to start receiver").into());
        }
    };

    let mut mpv = Command::new("mpv")
        .args(&["-"])
        .stdin(Stdio::piped())
        .spawn()?;
    
    // Get handle to mpv's stdin
    let mut mpv_stdin = match mpv.stdin.take() {
        Some(stdin) => stdin,
        None => {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to open mpv stdin").into());
        }
    };

    // for data in receiver {
    //     // Write data to mpv's stdin
    //     if let Err(e) = mpv_stdin.write_all(&data) {
    //         debug!("Failed to write to mpv stdin: {}", e);
    //         return Err(Box::new(e));
    //     }
    // }

    // // Ensure all data is flushed before closing the pipe
    // drop(mpv_stdin);

    // Wait for the command to finish
    match mpv.wait() {
        Ok(status) => debug!("mpv exited with: {}", status),
        Err(e) => {
            debug!("Failed to wait on mpv process: {}", e);
            return Err(Box::new(e));

        }
    }

    Ok(())
}
