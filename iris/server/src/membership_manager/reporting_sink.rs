use crate::proto;

use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use log::{error, info};
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
            let addr = "192.168.15.1:9000".parse().unwrap();

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
                                    // info!("Received {} bytes from {}", packet_size, source_address);
                                    let report = ClientReport::decode(&buf[..packet_size]).unwrap();

                                    // Create a new instance of ClientSessions
                                    let mut client_sessions = client_sessions.write().unwrap();
                                    client_sessions.save_client_report(source_address, report);

                                    // Drop the lock to allow other threads to access it
                                    drop(client_sessions);
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    // If we get a `WouldBlock` error we know our socket
                                    // has no more packets queued, so we can return to
                                    // polling and wait for some more.
                                    break;
                                }
                                Err(e) => {
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

#[tonic::async_trait]
impl ReportService for MyReportService {
    async fn report_sink(
        &self,
        request: Request<ClientReport>,
    ) -> Result<Response<TimestampResponse>, Status> {
        // Get the client address from the request metadata
        if let Some(client_addr) = request.remote_addr() {

            let mut client_sessions = self.client_sessions.write().unwrap();
            // Update the client session
            client_sessions.save_client_report(client_addr, request.into_inner());
        } else {
            error!("Client address not found in request metadata");
        }

        Ok(Response::new(TimestampResponse {
            timestamp: Utc::now().to_rfc3339(),
        })) // Respond with the current timestamp
    }
}
