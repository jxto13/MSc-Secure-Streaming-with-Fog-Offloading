pub mod crypto_utils {

    use openssl::symm::{Cipher, Crypter, Mode};
    
    pub fn encrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cipher = Cipher::aes_256_cbc();
        let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
        let mut ciphertext = vec![0; data.len() + cipher.block_size()];
        let mut count = crypter.update(data, &mut ciphertext)?;
        count += crypter.finalize(&mut ciphertext[count..])?;
        ciphertext.truncate(count);
        Ok(ciphertext)
    }
    
    pub fn decrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cipher = Cipher::aes_256_cbc();
        let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))?;
        let mut plaintext = vec![0; data.len() + cipher.block_size()];
        let mut count = crypter.update(data, &mut plaintext)?;
        count += crypter.finalize(&mut plaintext[count..])?;
        plaintext.truncate(count);
        Ok(plaintext)
    }
    
}

pub mod padding {
    const AES_BLOCK_SIZE: usize = 16;

    /// Pads the input data to a specific size that is a multiple of the AES block size (16 bytes)
    /// and stores the padding length in the last element.
    pub fn pad(data: &mut Vec<u8>, target_size: usize) {
        if target_size % AES_BLOCK_SIZE != 0 {
            panic!("Target size must be a multiple of the AES block size (16 bytes)");
        }
        if target_size < data.len() {
            panic!("Target size must be greater than or equal to the data length");
        }
        let padding_len = target_size - data.len();
        data.extend(vec![0; padding_len - 1]);
        data.push(padding_len as u8);
    }

    /// Removes the padding from the input data and returns a new Vec<u8>.
    pub fn depad(data: &Vec<u8>) -> Result<Vec<u8>, &'static str> {
        if data.is_empty() {
            return Err("Data is empty");
        }
        let padding_len = *data.last().unwrap() as usize;
        if padding_len == 0 || padding_len > data.len() {
            return Err("Invalid padding length");
        }
        Ok(data[..data.len() - padding_len].to_vec())
    }
}

pub mod bandwidth_utils {
    use std::time::{Duration, Instant};
    use std::thread;

    /// A struct representing a simple bandwidth manager.
    pub struct BandwidthManager {
        bitrate: usize,              // Maximum bits per second, in kbit/s
        bytes_sent: usize,           // Bytes sent in the current interval
        multicast_packets_sent: usize,   // Number of multicast packets sent
        retransmission_packets_sent: usize, // Number of retransmission packets sent
        interval_start: Instant,     // Tracks the start of the interval
    }

    impl BandwidthManager {
        /// Creates a new bandwidth manager with a specified bitrate.
        pub fn new(bitrate: usize) -> Self {
            Self {
                bitrate,
                bytes_sent: 0,
                multicast_packets_sent: 0,
                retransmission_packets_sent: 0,
                interval_start: Instant::now(),
            }
        }

        /// Records a multicast packet sent.
        pub fn record_multicast(&mut self, packet_size: usize) {
            self.bytes_sent += packet_size;
            self.multicast_packets_sent += 1;
            self.enforce_rate();
        }

        /// Records a retransmission packet sent.
        pub fn record_retransmission(&mut self, packet_size: usize) {
            self.bytes_sent += packet_size;
            self.retransmission_packets_sent += 1;
            self.enforce_rate();
        }

        /// Enforces the bandwidth rate by introducing delays if needed.
        fn enforce_rate(&mut self) {
            // Calculate the allowed bytes per second based on the bitrate (kbit/s to bytes/sec)
            let allowed_bytes_per_second = (self.bitrate * 1_000) / 8;

            // Calculate the elapsed time since the start of the interval
            let elapsed = self.interval_start.elapsed();
            let elapsed_seconds = elapsed.as_secs_f64();

            // Determine if we are exceeding the allowed bandwidth
            let expected_bytes = (elapsed_seconds * allowed_bytes_per_second as f64) as usize;

            if self.bytes_sent > expected_bytes {
                // Calculate how long to sleep to maintain the CBR
                let excess_bytes = self.bytes_sent - expected_bytes;
                let sleep_duration = Duration::from_secs_f64(excess_bytes as f64 / allowed_bytes_per_second as f64);
                thread::sleep(sleep_duration);
            }

            // Reset interval and bytes if a second has passed
            if elapsed >= Duration::from_secs(1) {
                self.interval_start += Duration::from_secs(1);
                self.bytes_sent = 0;
            }
        }
    }

}
#[cfg(test)]
mod tests {
    use crate::crypto_utils::{encrypt,decrypt};
    use crate::padding::{pad, depad};


    #[test]
    fn test_encrypt_decrypt() {
        let data = b"Hello, world!";
        let key = b"an example very very secret key.";
        let iv = b"unique nonce 123"; // Updated IV to be 16 bytes long

        let encrypted_data = encrypt(data, key, iv).expect("Encryption failed");
        let decrypted_data = decrypt(&encrypted_data, key, iv).expect("Decryption failed");
        println!("{:?}",encrypted_data);

        println!("{:?} {:?}",data, decrypted_data);

        assert_eq!(data.to_vec(), decrypted_data);
    }

    #[test]
    fn test_pad_depad() {
        let mut data = b"YELLOW SUBMARINE".to_vec();
        let block_size = 20;

        pad(&mut data, block_size);
        let depadded_data = depad(&data).expect("Depadding failed");

        assert_eq!(depadded_data, b"YELLOW SUBMARINE".to_vec());
    }

    #[test]
    fn test_invalid_depad() {
        let data = b"INVALID PADDING\x05\x05\x05\x05\x06".to_vec();
        let result = depad(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_depad() {
        let data = vec![];
        let result = depad(&data);
        assert!(result.is_err());
    }
}
