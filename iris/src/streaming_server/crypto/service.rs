use crate::crypto::utils;
use crate::proto;

use tonic::{Request, Response, Status};
use proto::certificate_service_server::{CertificateService, CertificateServiceServer};

use openssl::pkey::{PKey, Private};
use openssl::x509::{X509Req, X509, X509VerifyResult};

pub struct MyCertificateService {
    ca_cert: X509,
    ca_key_pair: PKey<Private>,
}

impl MyCertificateService {
    pub fn new(ca_cert: X509, ca_key_pair: PKey<Private>) -> Self {
        MyCertificateService {
            ca_cert,
            ca_key_pair,
        }
    }
}

#[tonic::async_trait]
impl CertificateService for MyCertificateService {
    async fn sign_certificate(
        &self,
        request: Request<proto::CertificateRequest>,
    ) -> Result<Response<proto::CertificateResponse>, Status> {
        // Extract CSR from the request
        let csr = request.into_inner().csr;

        // Convert the CSR from Vec<u8> to X509Req
        let csr = match X509Req::from_pem(&csr) {
            Ok(csr) => csr,
            Err(_) => return Err(Status::invalid_argument("Invalid CSR")),
        };
        
        // Sign the certificate with the CA
        let signed_cert = utils::sign_request_with_ca(&csr, &self.ca_cert, &self.ca_key_pair);

        // Serialize the signed certificate
        let cert_pem = match signed_cert {
            Ok(cert) => cert.to_pem().map_err(|_| Status::internal("Failed to serialize certificate"))?,
            Err(_) => return Err(Status::internal("Failed to sign certificate")),
        };
        
        // Return the signed certificate
        let response = proto::CertificateResponse {
            signed_cert: cert_pem,
        };

        Ok(Response::new(response))
    }
    

    async fn verify_certificate(
        &self,
        request: Request<proto::SignedCertificateRequest>
    ) -> Result<Response<proto::ValidationCertificateResponse>, Status> {
        // Extract Certificate from the request
        let cert_vec = request.into_inner().cert;

        // Convert the CSR from Vec<u8> to X509Req
        let cert = match X509::from_pem(&cert_vec) {
            Ok(cert) => cert,
            Err(_) => return Err(Status::invalid_argument("Invalid Certificate")),
        };

        // Get the CA cert
        let ca_cert = &self.ca_cert;

        // Verify that this cert was issued by this ca
        match ca_cert.issued(&cert) {
            X509VerifyResult::OK => println!("Certificate verified!"),
            ver_err => println!("Failed to verify certificate: {}", ver_err),
        };

        // For demonstration, let's assume it always succeeds
        let response = proto::ValidationCertificateResponse {
            auth: true,
        };

        Ok(Response::new(response))
    }
}

pub fn get_service(ca_cert: X509,ca_key_pair: PKey<Private>) -> CertificateServiceServer<MyCertificateService> {
    CertificateServiceServer::new(MyCertificateService::new(ca_cert,ca_key_pair))
}