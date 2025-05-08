use std::sync::Arc;

use crate::crypto::utils;
use crate::membership_manager::client_sessions::{self, ClientSessions};
use crate::proto;

use log::info;
use tonic::{Request, Response, Status};
use proto::certificate_service_server::{CertificateService, CertificateServiceServer};
use std::time::Instant;
use client_sessions::ClientData;

use openssl::pkey::{PKey, Private};
use openssl::x509::{X509Req, X509VerifyResult, X509};

pub struct MyCertificateService {
    ca_cert: X509,
    ca_private_key: PKey<Private>,
    client_sessions: Arc<ClientSessions>,
}

impl MyCertificateService {
    pub fn new(ca_cert: X509, ca_key_pair: PKey<Private>, client_sessions: Arc<ClientSessions>) -> Self {
        MyCertificateService {
            ca_cert,
            ca_private_key: ca_key_pair,
            client_sessions
        }
    }
}

#[tonic::async_trait]
impl CertificateService for MyCertificateService {
    async fn sign_certificate(
        &self,
        request: Request<proto::CertificateRequest>,
    ) -> Result<Response<proto::CertificateResponse>, Status> {
        // Extract the remote address from the request
        let request_addr = request.remote_addr().unwrap();
    
        // Extract CSR from the request
        let csr = request.into_inner().csr;

        // Convert the CSR from Vec<u8> to X509Req
        let csr = match X509Req::from_pem(&csr) {
            Ok(csr) => csr,
            Err(_) => return Err(Status::invalid_argument("Invalid CSR")),
        };
        
        // Sign the certificate with the CA
        let signed_cert = utils::sign_request_with_ca(&csr, &self.ca_cert, &self.ca_private_key);

        // Serialize the signed certificate
        let cert_pem = match signed_cert {
            Ok(cert) => cert,
            Err(_) => return Err(Status::internal("Failed to sign certificate")),
        };
        
        // Return the signed certificate
        let response = proto::CertificateResponse {
            signed_cert: cert_pem.to_pem().map_err(|_| Status::internal("Failed to serialize certificate"))?,
        };

        self.client_sessions.add_client(request_addr.ip(), cert_pem.clone());

        // Now it's clear that the client session is removed, lets add a new one
        // self.client_sessions.update_client(request_addr.ip());
        // self.client_sessions.certs.insert(request_addr.ip(), cert_pem);

        Ok(Response::new(response))
    }

    async fn verify_certificate(
        &self,
        mut request: Request<proto::SignedCertificateRequest>
    ) -> Result<Response<proto::ValidationCertificateResponse>, Status> {
        let request_ref = request.get_mut();

        // Extract Certificate from the request
        let cert_vec = &request_ref.cert;

        // Convert the CSR from Vec<u8> to X509Req
        let cert = match X509::from_pem(cert_vec) {
            Ok(cert) => cert,
            Err(_) => return Err(Status::invalid_argument("Invalid Certificate")),
        };

        // Get the CA cert
        let ca_cert = &self.ca_cert;

        // Verify that this cert was issued by this ca
        match ca_cert.issued(&cert) {
            X509VerifyResult::OK => info!("Certificate verified for client: {:?}", request.remote_addr().unwrap()),
            ver_err => info!("Failed to verify certificate for client: {} - {}", request.remote_addr().unwrap(), ver_err),
        };

        // For demonstration, let's assume it always succeeds
        let response = proto::ValidationCertificateResponse {
            auth: true,
        };

        Ok(Response::new(response))
    }
    
}

pub fn get_service(ca_cert: X509,ca_key_pair: PKey<Private>, client_sessions: Arc<ClientSessions>) -> CertificateServiceServer<MyCertificateService> {
    CertificateServiceServer::new(MyCertificateService::new(ca_cert,ca_key_pair,client_sessions))
}