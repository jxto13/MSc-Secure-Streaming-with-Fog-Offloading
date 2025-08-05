use crossbeam_channel::select;
use log::{error, info};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::runtime::Runtime;

use crate::{decoder::Decoder, proto::MeshPacket, receiver::ReceiverType};

#[derive(Debug)]
pub struct RetransmissionManager {
    runtime: Arc<Runtime>, // Tokio runtime for async operations
    decoder: Arc<Decoder>, // Decoder for processing packets
    requests_channel: loole::Receiver<MeshPacket>, // Channel for retransmission requests
    data_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>, // Channel to send data packets
    ipc_channels: (loole::Sender<MeshPacket>, loole::Receiver<MeshPacket>),
    ipc_active: Arc<AtomicBool>, // Tracks if middleware is running
}

impl RetransmissionManager {
    pub fn new(
        runtime: Arc<Runtime>,
        decoder: Arc<Decoder>,
        requests_channel: loole::Receiver<MeshPacket>,
        data_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>,
    ) -> Self {
        let ipc_channels = loole::unbounded();
        Self {
            runtime,
            decoder,
            requests_channel,
            data_sender,
            ipc_channels,
            ipc_active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn middleware(
        &self,
        request_to_mesh: loole::Sender<MeshPacket>,
        request_from_mesh: &loole::Receiver<MeshPacket>,
    ) {
        self.ipc_active.store(true, Ordering::SeqCst); // Mark IPC as active

        let mut idle_count = 0;
        let max_idle = 5; // After 5 idle checks, start sleeping

        loop {
            let mut got_message = false;

            if let Ok(msg) = self.ipc_channels.1.try_recv() {
                info!("Sending request to | MESH |  {:?}", msg);

                request_to_mesh.send(msg).unwrap_or_else(|e| {
                    error!("Failed to send message to MESH channel: {}", e);
                });

                got_message = true;
            }

            if let Ok(msg) = request_from_mesh.try_recv() {
                info!("Received request from | MESH |  {:?}", msg);

                got_message = true;
            }

            if got_message {
                idle_count = 0; // reset idle counter if we got something
            } else {
                idle_count += 1;

                if idle_count > max_idle {
                    // Exponential backoff sleep
                    let sleep_ms = (idle_count - max_idle).min(50); // cap at 50ms
                    thread::sleep(Duration::from_millis(sleep_ms as u64));
                } else {
                    // Short yield if still within fast-polling threshold
                    std::thread::yield_now();
                }
            }
        }
    }

    /// Processes all active retransmission requests only if IPC is active.
    pub fn process_requests(&self, shutdown_flag: Arc<AtomicBool>) {
        let requests_channel = self.requests_channel.clone();
        let ipc_active = self.ipc_active.clone();

        let mut idle_count = 0;
        let max_idle = 5; // After 5 idle checks, start sleeping

        loop {
            if shutdown_flag.load(Ordering::Relaxed) {
                info!("RetransmissionManager shutting down gracefully");
                break;
            }

            if !ipc_active.load(Ordering::SeqCst) {
                info!("IPC is not active, skipping request processing");
                thread::sleep(Duration::from_millis(100)); // Sleep briefly if IPC is inactive
                continue;
            }

            let mut got_message = false;

            if let Ok(msg) = requests_channel.try_recv() {
                info!("Processing request from | DECODER |  {:?}", msg);

                self.ipc_channels.0.send(msg).unwrap_or_else(|e| {
                    error!("Failed to send message to IPC channel: {}", e);
                });

                got_message = true;
            }

            if got_message {
                idle_count = 0; // reset idle counter if we got something
            } else {
                idle_count += 1;

                if idle_count > max_idle {
                    // Exponential backoff sleep
                    let sleep_ms = (idle_count - max_idle).min(50); // cap at 50ms
                    thread::sleep(Duration::from_millis(sleep_ms as u64));
                } else {
                    // Short yield if still within fast-polling threshold
                    std::thread::yield_now();
                }
            }
        }
    }
}
