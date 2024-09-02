
use openssl::ec::EcKey;
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::x509::{X509NameBuilder, X509Req, X509ReqBuilder};

use super::proto::SessionKey;

#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    pub current_key: SessionKey,
    pub ec_server_certificate: Vec<u8>,
    pub epoch: i32,
    pub client_ec_certificate: Vec<u8>,
    pub client_ec_key: Option<EcKey<Private>>
}

// impl Default for SessionConfig {
//     fn default() -> Self {
//         Self {
//             current_key: SessionKey::default(),
//             ec_server_certificate: Vec::default(),
//             epoch: 0, // or any other default value you prefer
//             client_ec_certificate: Vec::default(),
//             client_ec_key:
//         }
//     }
// }

/// Make a X509 request with the given private key
pub fn mk_request(key_pair: &PKey<Private>) -> Result<X509Req, ErrorStack> {
    let mut req_builder = X509ReqBuilder::new()?;
    req_builder.set_pubkey(key_pair)?;

    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", "PT")?;
    x509_name.append_entry_by_text("ST", "Porto")?;
    x509_name.append_entry_by_text("O", "DCC")?;
    x509_name.append_entry_by_text("CN", "www.iris.dcc.fc.up.pt")?;
    let x509_name = x509_name.build();
    req_builder.set_subject_name(&x509_name)?;

    req_builder.sign(key_pair, MessageDigest::sha256())?;
    let req = req_builder.build();
    Ok(req)
}