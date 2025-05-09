use crate::CONFIG;

use std::sync::{Arc, RwLock};

use crate::proto;

use proto::key_service_server::{KeyService, KeyServiceServer};
use proto::{KeyResponse, SessionKey};

use openssl::x509::X509;
use tonic::{Request, Response, Status};

use super::key_manager::KeyManager;

pub struct MyKeyService {
    pub key_manager: Arc<RwLock<KeyManager>>,  // Wrap KeyManager in Arc<Mutex<>>
    ec_certificate: X509
}

impl MyKeyService {
    pub fn new(key_manager: Arc<RwLock<KeyManager>>, ec_certificate: X509) -> Self {
        MyKeyService {
            key_manager,  // Use the provided Arc<Mutex<KeyManager>>
            ec_certificate
        }
    }

    pub fn get_current_key(&self) -> SessionKey {
        let key_manager = self.key_manager.read().unwrap();  // Lock the Mutex
        key_manager.get_current_key().clone()  // Access KeyManager safely
    }

    pub fn get_next_key(&self) -> SessionKey {
        let key_manager = self.key_manager.read().unwrap();  // Lock the Mutex
        key_manager.get_next_key().clone()  // Access KeyManager safely
    }

    pub fn get_certificate(&self) -> Vec<u8> {
        // Convert the X509 certificate to a PEM-encoded string
        let cert_pem = self.ec_certificate.to_pem().expect("Failed to convert certificate to PEM");
        cert_pem
    }
}

#[tonic::async_trait]
impl KeyService for MyKeyService {
    async fn request_current_key(
        &self,
        _: Request<proto::RequestKey>,
    ) -> Result<Response<proto::KeyResponse>, Status> {
        let current_key = self.get_current_key();
        let next_key = self.get_next_key();
        let certificate = self.get_certificate();

        Ok(Response::new(KeyResponse {
            current_key: Some(current_key),
            next_key: Some(next_key),
            certificate,
            epoch: CONFIG.get().unwrap().multicast.epoch as u32,
            reports_interval: CONFIG.get().unwrap().settings.transmissions_interval as u32,
        }))
    }
}

pub fn get_service(key_manager: Arc<RwLock<KeyManager>>, ec_certificate :X509) -> KeyServiceServer<MyKeyService> {
    KeyServiceServer::new(MyKeyService::new(key_manager,ec_certificate))
}
