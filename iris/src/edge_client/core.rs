use crossbeam_channel::{unbounded, Receiver};
use tokio::sync::mpsc;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;

use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;

use proto::certificate_service_client::CertificateServiceClient;
use proto::{CertificateRequest, SignedCertificateRequest};

use tokio::runtime::Runtime;
use tokio::time::sleep;

use tonic::transport::{Certificate, Channel, ClientTlsConfig};

use log::{error, info};

use super::{decoder, proto, utils};
use super::receiver;
use super::session;

pub fn start() -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {

    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());

    // Create a shared Arc<Mutex<Option<CertificateServiceClient>>>
    let client = Arc::new(Mutex::new(None));

    {
        let runtime = Arc::clone(&rt);
        // Clone the Arc for use inside the async block
        let client = Arc::clone(&client);

        // Run async code using the Tokio runtime
        runtime.block_on(async move {
            loop {
                let connection = CertificateServiceClient::connect("http://razer15:50051").await;
                
                match connection {
                    Ok(mut connected_client) => {
                        info!("Connected to GRPC Server");

                        // Generate an RSA key pair
                        let rsa = Rsa::generate(2048).unwrap();
                        // Create a PKey object from the RSA key pair
                        let key_pair = PKey::from_rsa(rsa).unwrap();

                        // Make a X509 request with the given private key
                        let request = utils::mk_request(&key_pair).unwrap();

                        // Serialize the request CSR
                        let csr_vec = request.to_pem().unwrap();

                        // Make a CertificateRequest with the CSR
                        let csr_request = CertificateRequest {
                            csr: csr_vec,
                        };

                        // Send the CSR to the server to get it signed
                        let response = connected_client.sign_certificate(csr_request).await.unwrap();
                        let signed_cert = response.into_inner().signed_cert;

                        // Assuming cert_bytes is the bytes of the received certificate
                        let cert_request = SignedCertificateRequest { cert: signed_cert };

                        // Send the signed certificate to the server for verification
                        let validation_response = connected_client.verify_certificate(cert_request).await.unwrap();
                        let auth = validation_response.into_inner().auth;

                        if auth {
                            info!("Certificate verified successfully!");
                        } else {
                            error!("Certificate verification failed!");
                        }

                        // Save the client for use after the loop
                        let mut client_lock = client.lock().unwrap();
                        *client_lock = Some(connected_client);
                        break; // Exit the loop once connected
                    }
                    Err(e) => {
                        error!("Failed to connect: {}. Retrying in 3 seconds...", e);
                        sleep(Duration::from_secs(3)).await;
                    }
                }
            }
        });
    }

    // If we arrive here we have certificates signed from PKI 

    // Create channels for thread comunication
    let (listener_sender, matrix_decoder_receiver) = unbounded();
    let (decoded_data_sender, decoded_data_receiver) = unbounded();
    let (client_report_sender, client_report_receiver) = mpsc::channel(1000);

    // Create a Barrier for thread synchronization
    let barrier = Arc::new(Barrier::new(2));

    // Clone the Barrier for the listener thread
    let listener_barrier = Arc::clone(&barrier);

    // Creates new thread for multicast receiver
    let listener_handler = std::thread::spawn(move || {
        info!("Starting multicast receiver on address 239.1.1.10:12345.");
        match receiver::multicast_receiver::start(
            "239.1.1.10".to_string(),
            "12345".to_string(),
            listener_sender,
            listener_barrier,
        ) {
            Ok(_) => {
                info!("Multicast receiver exited successfully.");
            }
            Err(e) => {
                error!("Multicast receiver failed with error: {}", e);
            }
        }
    });

    // Clone the Barrier for the decoder thread
    let decoder_barrier = Arc::clone(&barrier);

    // dont need handler because when the listener goes out of scope, i want the decoder to also go out of scoupe
    thread::spawn(move || {
        // Wait for the listener multicast with a barrier before starting the decoder
        decoder_barrier.wait();
        if let Err(e) = decoder::start(matrix_decoder_receiver, decoded_data_sender,client_report_sender) {
            error!("Matrix decoder failed to start: {}", e);
        }
    });

    let runtime = Arc::clone(&rt);
    // Spawn Client Report Thread
    thread::spawn(move || {
        // Wait for the listener multicast with a barrier before starting the decoder
        let mut client_report = session::ClientSession::new(runtime);
        client_report.start_report_client(client_report_receiver);
    });


    // Spawn a separate thread for joining and returning the sender
    thread::spawn(move || {
        // Join all the threads to ensure they complete before returning
        listener_handler.join().expect("listener_handler panicked");
    });

    Ok(decoded_data_receiver)
}
