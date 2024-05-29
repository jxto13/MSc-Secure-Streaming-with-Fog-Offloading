use std::error::Error;
use std::process::{Command, Stdio};
use std::io::{self, Write, BufRead, BufReader};
use std::thread;

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

    let mut ffplay = Command::new("mpv")
        .args(&["-","--force-seekable=yes"])
        .stdin(Stdio::piped())
        .spawn()?;
    
    // Get handles to ffplay's stdin and stdout
    let mut ffplay_stdin = match ffplay.stdin.take() {
        Some(stdin) => stdin,
        None => {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to open ffplay stdin").into());
        }
    };

    for data in receiver {
        // Write data to ffplay's stdin
        if let Err(e) = ffplay_stdin.write_all(&data) {
            debug!("Failed to write to ffplay stdin: {}", e);
            return Err(Box::new(e));
        }
    }

    // Wait for the command to finish
    match ffplay.wait() {
        Ok(status) => debug!("ffplay exited with: {}", status),
        Err(e) => {
            debug!("Failed to wait on ffplay process: {}", e);
            return Err(Box::new(e));
        }
    }

    // Ensure all data is flushed before closing the pipe
    drop(ffplay_stdin);

    Ok(())
}
