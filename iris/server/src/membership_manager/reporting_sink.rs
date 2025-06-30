use crate::{proto, CONFIG};

use std::net::SocketAddr;
use std::sync::Arc;

use crossbeam_channel::Sender;
use iris_shared::compression::decompress_data;
use log::error;
use openssl::ecdsa::EcdsaSig;
use openssl::sha::sha256;
use prost::Message;
use proto::ClientReport;

use super::client_sessions::ClientSessions;

use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use std::io;

const UDP_SOCKET: Token = Token(0);


#[derive(Debug)]
pub struct MyReportService {
}

impl MyReportService {

    pub fn receiver(client_sessions: Arc<ClientSessions>, reports_channel: Sender<(SocketAddr, ClientReport)>) {
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

                                // Verify the signature using the public key from the client certificate
                                let local_hash = sha256(&report.encode_to_vec());

                                // Get the client certificate from the client_sessions
                                let client_cert = match client_sessions.get_client_cert(&report.client_id) {
                                    Some(cert) => cert,
                                    None => {
                                        error!("Client certificate not found for {}", source_address.ip());
                                        continue;
                                    }
                                    
                                };

                                // Verify the signature using the public key from the client certificate
                                if !signature.verify(&local_hash, &client_cert.public_key().unwrap().ec_key().unwrap()).unwrap() {
                                    error!("Signature verification failed");
                                    continue;
                                }
                                
                                
                                // If the signature is valid, we can process the report. 
                                // Also because is a valid report, we pass the ip address to the client session to update if
                                // needed. It can be a client roaming 
                                
                                // Send the report to the reports channel
                                if let Err(_) = reports_channel.send((source_address, report)) {
                                    error!("Failed to send report to channel");
                                }
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
        
    }
}
