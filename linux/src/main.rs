use log::{error, info, warn};
use std::error::Error;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    // Attempt to start the receiver
    let receiver = match iris_client::start_client(&"stream1.iris.dcc.fc.up.pt".to_string()) {
        Ok(receiver) => receiver,
        Err(e) => {
            error!("Failed to start receiver: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to start receiver").into());
        }
    };

    // Start the ffplay (mpv) process
    let mut ffplay = Command::new("mpv")
        .args(&["-"])
        .stdin(Stdio::piped())
        // .stdout(Stdio::piped()) // Capture stdout for packet handling
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            error!("Failed to start ffplay process: {}", e);
            io::Error::new(io::ErrorKind::Other, "Failed to start ffplay process")
        })?;

    // Get handle to ffplay's stdin
    let mut ffplay_stdin = ffplay.stdin.take().ok_or_else(|| {
        error!("Failed to open ffplay stdin");
        io::Error::new(io::ErrorKind::Other, "Failed to open ffplay stdin")
    })?;

    // Spawn a thread to handle data reception
    let receiver_thread = thread::spawn(move || {
        for data in receiver {
            if let Err(e) = ffplay_stdin.write_all(&data[..]) {
                error!("Failed to write to ffplay stdin: {}", e);
                break;
            }
        }
    });

    // If ffmpeg's stdout is present, read and log it in 188-byte chunks
    if let Some(mut stdout) = ffplay.stdout.take() {
        let mut buffer = [0u8; 188];
        loop {
            match stdout.read_exact(&mut buffer) {
                Ok(_) => {
                    warn!("Received 188-byte packet: {:?}", &buffer);
                }
                Err(e) => {
                    error!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
    }

    // Wait for the receiver thread to finish
    receiver_thread.join().map_err(|e| {
        error!("Receiver thread panicked: {:?}", e);
        io::Error::new(io::ErrorKind::Other, "Receiver thread panicked")
    })?;

    // Wait for the ffplay process to finish
    match ffplay.wait() {
        Ok(status) => info!("ffplay exited with: {}", status),
        Err(e) => {
            error!("Failed to wait on ffplay process: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}
