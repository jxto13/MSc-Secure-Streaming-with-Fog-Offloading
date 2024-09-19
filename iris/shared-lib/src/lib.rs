pub mod crypto_utils {

    use openssl::symm::{Cipher, Crypter, Mode};
    
    fn pad_to_size(data: &[u8], target_size: usize) -> Vec<u8> {
        let padding_len = target_size - data.len();
        let mut padded_data = data.to_vec();
        padded_data.extend(vec![padding_len as u8; padding_len]);
        padded_data
    }
    
    pub fn encrypt(plaintext: &[u8], key: &[u8], iv: &[u8], target_size: usize) -> Vec<u8> {
        let cipher = Cipher::aes_256_cbc();
        let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).unwrap();
        crypter.pad(false); // Disable PKCS7 padding
            
        assert!(target_size > plaintext.len());
        
        // Pad plaintext to target size
        let padded_plaintext = pad_to_size(plaintext, target_size);
    
        let mut ciphertext = vec![0; target_size + cipher.block_size()];
    
        let mut count = crypter.update(&padded_plaintext, &mut ciphertext).unwrap();
        count += crypter.finalize(&mut ciphertext[count..]).unwrap();
    
        ciphertext.truncate(count);
        ciphertext
    }
    
    pub fn decrypt(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
        let cipher = Cipher::aes_256_cbc();
        let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).unwrap();
        crypter.pad(false); // Disable PKCS7 padding
    
        let mut plaintext = vec![0; ciphertext.len() + cipher.block_size()];
    
        let mut count = crypter.update(ciphertext, &mut plaintext).unwrap();
        count += crypter.finalize(&mut plaintext[count..]).unwrap();
    
        plaintext.truncate(count);
    
        // Remove custom padding
        let padding_len = *plaintext.last().unwrap() as usize;
        plaintext.truncate(plaintext.len() - padding_len);
    
        plaintext
    }
}
