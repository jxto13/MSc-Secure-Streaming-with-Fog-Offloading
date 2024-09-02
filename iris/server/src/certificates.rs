use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::X509;
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use log::{error, info};
use std::path::PathBuf;
use std::fs;
use std::fs::create_dir_all;

use crate::crypto::utils::{save_certificate_to_file, save_private_key_to_file};
use crate::crypto::utils::{mk_request, sign_request_with_ca, mk_ca_cert, file_exists, load_private_key_from_file, load_certificate_from_file};

pub struct Certificates {
    pub ca_cert: X509,
    pub ca_private_key: PKey<Private>,
    pub server_cert: String,
    pub server_key: String,
    pub ec_cert: String,
    pub ec_key: EcKey<Private>,
}

pub fn load_or_create_certificates() -> Result<Certificates, Box<dyn std::error::Error>> {
    // Ensure the certs directory exists
    let certs_dir = PathBuf::from("./certs");
    if !certs_dir.exists() {
        create_dir_all(&certs_dir)?;
    }

    // Certificates file path
    let cert_file_path = certs_dir.join("ca_certificate.pem");
    let key_file_path = certs_dir.join("ca_private_key.pem");

    let (ca_cert, ca_private_key): (X509, PKey<Private>);
    
    // Generate or load CA Certificates 
    if crate::crypto::utils::file_exists(cert_file_path.to_str().unwrap()) && file_exists(key_file_path.to_str().unwrap()) {
        // Load the certificate and private key from the files
        match (load_certificate_from_file(cert_file_path.to_str().unwrap()), load_private_key_from_file(key_file_path.to_str().unwrap())) {
            (Ok(cert), Ok(pkey)) => {
                info!("Loaded certificate and private key from files:");
                ca_cert = cert;
                ca_private_key = pkey;
            }
            (Err(e), _) => {
                error!("Failed to load certificate: {}", e);
                return Err(Box::new(e));
            }
            (_, Err(e)) => {
                error!("Failed to load private key: {}", e);
                return Err(Box::new(e));
            }
        }
    } else {
        let (cert, key_pair) = mk_ca_cert()?;
        save_certificate_to_file(&cert, cert_file_path.to_str().unwrap())?;
        save_private_key_to_file(&key_pair, key_file_path.to_str().unwrap())?;
        ca_cert = cert;
        ca_private_key = key_pair;
    }

    // Server Certificates
    let server_cert_file_path = certs_dir.join("rsa_server_certificate.pem");
    let server_private_file_path = certs_dir.join("rsa_server_private_key.pem");

    let rsa = Rsa::generate(2048).unwrap();
    let pkey = PKey::from_rsa(rsa).unwrap();

    let request = mk_request(&pkey).unwrap();
    let signed_cert = sign_request_with_ca(&request, &ca_cert, &ca_private_key)?;

    save_certificate_to_file(&signed_cert, server_cert_file_path.to_str().unwrap())?;
    save_private_key_to_file(&pkey, server_private_file_path.to_str().unwrap())?;

    let cert = fs::read_to_string(server_cert_file_path.to_str().unwrap())?;
    let key = fs::read_to_string(server_private_file_path.to_str().unwrap())?;

    // EC Certificates
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::generate(&group).unwrap();

    let pkey = PKey::from_ec_key(ec_key.clone()).unwrap();
    let request = mk_request(&pkey).unwrap();
    let signed_cert = sign_request_with_ca(&request, &ca_cert, &ca_private_key)?;

    let ec_cert_file_path = certs_dir.join("ec_server_certificate.pem");
    let ec_key_file_path = certs_dir.join("ec_server_private_key.pem");

    save_certificate_to_file(&signed_cert, ec_cert_file_path.to_str().unwrap())?;
    save_private_key_to_file(&pkey, ec_key_file_path.to_str().unwrap())?;

    Ok(Certificates {
        ca_cert,
        ca_private_key,
        server_cert: cert,
        server_key: key,
        ec_cert: fs::read_to_string(ec_cert_file_path.to_str().unwrap())?,
        ec_key,
    })
}
