use iris::streaming_server::proto::SessionKey;

use openssl::ec::EcKey;
use openssl::pkey::Private;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};

pub struct KeyManager {
    current_key: SessionKey,
    next_key: SessionKey,
    ec_key: EcKey<Private>
}

impl KeyManager {
    pub fn new(ec_key: EcKey<Private>) -> Self {
        KeyManager {
            current_key: Self::generate_new_key(),
            next_key: Self::generate_new_key(),
            ec_key
        }
    }
    
    fn generate_new_key() -> SessionKey {
        let key = Self::generate_random_bytes(32); // 256 bits for AES-256
        let iv = Self::generate_random_bytes(16);  // 128 bits for AES block size
        SessionKey {
            key,
            iv,
        }
    }

    fn generate_random_bytes(size: usize) -> Vec<u8> {
        let mut buf = vec![0; size];
        rand_bytes(&mut buf).expect("Failed to generate random bytes");
        buf
    }

    pub fn rotate_key(&mut self) {
       self.current_key = self.next_key.clone();
       self.next_key = Self::generate_new_key();
    }

    pub fn get_current_key(&self) -> &SessionKey {
        &self.current_key
    }

    pub fn get_next_key(&self) -> &SessionKey {
        &self.next_key
    }

    pub fn get_ec_key(&self) -> &EcKey<Private> {
        &self.ec_key
    }

}
