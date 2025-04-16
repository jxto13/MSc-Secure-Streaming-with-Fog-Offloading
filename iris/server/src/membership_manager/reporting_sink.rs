use crate::{proto, CONFIG, MANAGER};

use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use log::{error, info};
use openssl::ecdsa::EcdsaSig;
use openssl::sha::sha256;
use prost::Message;
use proto::TimestampResponse;
use proto::{report_service_server::ReportService, ClientReport};
use tonic::{Request, Response, Status};

use super::client_sessions::ClientSessions;

#[derive(Debug)]
pub struct MyReportService {
    client_sessions: Arc<RwLock<ClientSessions>>,
}

impl MyReportService {
    pub fn new(client_sessions: Arc<RwLock<ClientSessions>>) -> Self {
        Self::udp_receiver(Arc::clone(&client_sessions));
        Self { client_sessions }
    }

    pub fn udp_receiver(client_sessions: Arc<RwLock<ClientSessions>>) {
        // Start the UDP receiver thread

        fn decompress_data(data: &[u8]) -> Vec<u8> {
            use flate2::read::GzDecoder;
            use std::io::Read;

            let mut decoder = GzDecoder::new(data);
            let mut decompressed_data = Vec::new();

            decoder.read_to_end(&mut decompressed_data).unwrap();
            decompressed_data
        }

        std::thread::spawn(move || {
            use mio::net::UdpSocket;
            use mio::{Events, Interest, Poll, Token};
            use std::io;

            const UDP_SOCKET: Token = Token(0);

            // Create a poll instance.
            let mut poll = Poll::new().unwrap();
            // Create storage for events. Since we will only register a single socket, a
            // capacity of 1 will do.
            let mut events = Events::with_capacity(1);

            // Setup the UDP socket.
            let addr = format!("0.0.0.0:{}", CONFIG.get().unwrap().retransmission.port).parse().unwrap();

            let mut socket = UdpSocket::bind(addr).unwrap();

            // Register our socket with the token defined above and an interest in being
            // `READABLE`.
            poll.registry()
                .register(&mut socket, UDP_SOCKET, Interest::READABLE)
                .unwrap();

            // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
            // packet, which is the maximum value of 16 a bit integer.
            let mut buf = [0; 1 << 16];

            // Our event loop.
            loop {
                // Poll to check if we have events waiting for us.
                if let Err(err) = poll.poll(&mut events, None) {
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return;
                }

                // Process each event.
                for event in events.iter() {
                    // Validate the token we registered our socket with,
                    // in this example it will only ever be one but we
                    // make sure it's valid none the less.
                    match event.token() {
                        UDP_SOCKET => loop {
                            // In this loop we receive all packets queued for the socket.
                            match socket.recv_from(&mut buf) {
                                Ok((packet_size, source_address)) => {
                                    // Echo the data.
                                    let mut report = ClientReport::decode(decompress_data(&buf[..packet_size]).as_slice()).unwrap();
                                    
                                    let signature = EcdsaSig::from_der(&report.signature).unwrap();
                                    report.signature = vec![0, 255];

                                    let local_hash = sha256(&report.encode_to_vec());

                                    let client_cert = client_sessions.read().unwrap().certs.get(&source_address.ip()).unwrap().clone();

                                    if !signature.verify(&local_hash, &client_cert.public_key().unwrap().ec_key().unwrap()).unwrap() {
                                        error!("Signature verification failed");
                                        continue;
                                    }
                                    
                                    // Create a new instance of ClientSessions
                                    let mut client_sessions = client_sessions.write().unwrap();
                                    client_sessions.save_client_report(source_address.ip(), report);

                                    // Drop the lock to allow other threads to access it
                                    drop(client_sessions);
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    // If we get a `WouldBlock` error we know our socket
                                    // has no more packets queued, so we can return to
                                    // polling and wait for some more.
                                    break;
                                }
                                Err(_e) => {
                                    // If it was any other kind of error, something went
                                    // wrong and we terminate with an error.
                                    return;
                                }
                            }
                        },
                        _ => {
                            // This should never happen as we only registered our
                            // `UdpSocket` using the `UDP_SOCKET` token, but if it ever
                            // does we'll log it.
                            log::warn!("Got event for unexpected token: {:?}", event);
                        }
                    }
                }
            }
        });
    }
}

// #[tonic::async_trait]
// impl ReportService for MyReportService {
//     async fn report_sink(
//         &self,
//         request: Request<ClientReport>,
//     ) -> Result<Response<TimestampResponse>, Status> {
//         // Get the client address from the request metadata
//         if let Some(client_addr) = request.remote_addr() {

//             let mut client_sessions = self.client_sessions.write().unwrap();
//             // Update the client session
//             client_sessions.save_client_report(client_addr, request.into_inner());
//         } else {
//             error!("Client address not found in request metadata");
//         }

//         Ok(Response::new(TimestampResponse {
//             timestamp: Utc::now().to_rfc3339(),
//         })) // Respond with the current timestamp
//     }
// }
