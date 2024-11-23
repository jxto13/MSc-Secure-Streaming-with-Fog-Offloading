fn insert_value<T>(matrix: &mut Vec<Vec<T>>, position: usize, matrix_size: usize, value: T) {
    let row = position / matrix_size;
    let col = position % matrix_size;

    if row < matrix_size && col < matrix_size {
        matrix[row][col] = value;
     } else {
        println!("Invalid position!");
     }
}


#[cfg(test)]
mod tests {

    #[test]
    fn test_lookup(){
        use std::time::Instant;
        // Create a large vector of integers
        let n = 10_000_000;
        let vec: Vec<i32> = (0..n).collect();

        // Measure the time to iterate over the Vec
        let mut start = Instant::now();

        for _ in &vec {
            println!("Time to iterate over: {:?}",start.elapsed());
            start = Instant::now();
        
        }
    }

    #[test]
    fn test_encryption_decryption() {

        use openssl::symm::{Cipher, Crypter, Mode};
        use openssl::rand::rand_bytes;

        fn generate_random_bytes(length: usize) -> Vec<u8> {
            let mut bytes = vec![0; length];
            rand_bytes(&mut bytes).unwrap();
            bytes
        }

        fn encrypt(plaintext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
            let cipher = Cipher::aes_256_cbc();
            let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).unwrap();
            crypter.pad(true); // Enable PKCS7 padding

            let block_size = cipher.block_size();
            let mut ciphertext = vec![0; plaintext.len() + block_size];

            let mut count = crypter.update(plaintext, &mut ciphertext).unwrap();
            count += crypter.finalize(&mut ciphertext[count..]).unwrap();

            ciphertext.truncate(count);
            ciphertext
        }

        fn decrypt(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
            let cipher = Cipher::aes_256_cbc();
            let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).unwrap();
            crypter.pad(true); // Enable PKCS7 padding

            let block_size = cipher.block_size();
            let mut plaintext = vec![0; ciphertext.len() + block_size];

            let mut count = crypter.update(ciphertext, &mut plaintext).unwrap();
            count += crypter.finalize(&mut plaintext[count..]).unwrap();

            plaintext.truncate(count);
            plaintext
        }

        let key = generate_random_bytes(32); // 32 bytes for AES-256
        let iv = generate_random_bytes(16);  // 16 bytes for AES block size

        let plaintext = b"Hello, OpenSSL in Rust WDAWDAWD AFWA!adwawdaw";

        let ciphertext = encrypt(plaintext, &key, &iv);
        println!("Ciphertext: {:?}", ciphertext.len());

        let decrypted_plaintext = decrypt(&ciphertext, &key, &iv);
        println!("Decrypted plaintext: {:?}", String::from_utf8(decrypted_plaintext.clone()).unwrap());

        assert_eq!(plaintext.to_vec(), decrypted_plaintext);
    }

    #[test]
    fn slices() {
        let slices: Vec<_> = "-r 30 -video_size 640x480 -i /dev/video0 -c:v libx264 -b:a 2048k -preset ultrafast -tune zerolatency -f mpegts".split_whitespace().collect();
        println!("{:?}", slices);

    }

    #[test]
    fn test_encrypt_decrypt_2() {
        use openssl::symm::{Cipher, Crypter, Mode};

        pub fn pad_to_size(data: &[u8], target_size: usize) -> Vec<u8> {
            let padding_len = target_size - data.len();
            let mut padded_data = data.to_vec();
            padded_data.extend(vec![padding_len as u8; padding_len]);
            padded_data
        }

        pub fn encrypt(plaintext: &[u8], key: &[u8], iv: &[u8], target_size: usize) -> Vec<u8> {
            let cipher = Cipher::aes_256_cbc();
            let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).unwrap();
            crypter.pad(false); // Disable PKCS7 padding

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

        let key = b"an example very very secret key."; // 32 bytes for AES-256
        let iv = b"unique iv!            "; // 16 bytes for AES-256
        let plaintext = b"this is a test plaintext message";

        let target_size = 1408;
        let ciphertext = encrypt(plaintext, key, iv, target_size);

        // Ensure the ciphertext length is 1408 bytes
        assert_eq!(ciphertext.len(), target_size);

        // Decrypt the ciphertext
        let decrypted_plaintext = decrypt(&ciphertext, key, iv);

        // Verify that the decrypted plaintext matches the original plaintext
        assert_eq!(&decrypted_plaintext[..plaintext.len()], plaintext);
    }


    #[test]
    fn test_sign_message() {

        use openssl::ec::{EcGroup, EcKey};
        use openssl::nid::Nid;
        use openssl::sign::Signer;
        use openssl::hash::MessageDigest;
        use openssl::pkey::PKey;
        use openssl::sign::Verifier;

        // Generate EC key pair
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let key = EcKey::generate(&group).unwrap();

        // Message to sign
        let message = b"Hello, world!";

        // Sign the message
        let pkey = PKey::from_ec_key(key.clone()).unwrap();
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
        signer.update(message).unwrap();
        let signature = signer.sign_to_vec().unwrap();

        // Print the length of the signature
        println!("Signature length: {}", signature.len());

        // Verify the signature
        let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey).unwrap();
        verifier.update(message).unwrap();
        let is_valid = verifier.verify(&signature).unwrap();

        // Assert the signature is valid
        assert!(is_valid);
    }

    #[test]
    fn tst_signature_3(){
        use openssl::ecdsa::EcdsaSig;
        use openssl::ec::{EcGroup, EcKey};
        use openssl::nid::Nid;
        use openssl::sha::sha256;
        use openssl::pkey::Private;

        // Generate EC key pair
        let group: EcGroup = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let ec_key: EcKey<Private> = EcKey::generate(&group).unwrap();

        // Extract the public key
        let public_key = EcKey::from_public_key(&group, ec_key.public_key()).unwrap();

        // Message to be signed
        let message = b"Hello, world! awdawfawfawe";

        // Hash the message using SHA-256
        let message_hash = sha256(message);

        // Sign the hashed message
        let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap();

        // Get the r and s components of the signature
        let r = signature.r().to_vec();
        let s = signature.s().to_vec();

        // Ensure r and s are of fixed length (32 bytes for P-256 curve)
        let mut fixed_r = vec![0; 32];
        let mut fixed_s = vec![0; 32];
        fixed_r[32 - r.len()..].copy_from_slice(&r);
        fixed_s[32 - s.len()..].copy_from_slice(&s);

        // Combine r and s into a fixed-size signature vector
        let mut fixed_signature = vec![];
        fixed_signature.extend_from_slice(&fixed_r);
        fixed_signature.extend_from_slice(&fixed_s);

        // Verify the signature
        let signature = EcdsaSig::from_private_components(
            openssl::bn::BigNum::from_slice(&fixed_signature[..32]).unwrap(),
            openssl::bn::BigNum::from_slice(&fixed_signature[32..]).unwrap(),
        )
        .unwrap();

        // Verify the signature
        let is_valid = signature.verify(&message_hash, &public_key).unwrap();

        // Assert that the signature is valid
        assert!(is_valid);
    }

    #[test]
    fn tst_signature_2(){
        use openssl::hash::MessageDigest;
        use openssl::ecdsa::EcdsaSig;
        use openssl::ec::{EcGroup, EcKey};
        use openssl::nid::Nid;
        use openssl::sha::sha256;
        use openssl::error::ErrorStack;
        use openssl::x509::X509Req;
        use openssl::pkey::Private;
        use openssl::pkey::PKey;
        use openssl::x509::X509ReqBuilder;
        use openssl::x509::X509NameBuilder;

        // Generate EC key pair
        let group: EcGroup = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let ec_key: EcKey<Private> = EcKey::generate(&group).unwrap();

        // Extract the public key
        let public_key = EcKey::from_public_key(&group, ec_key.public_key()).unwrap();

        // Message to be signed
        let message = b"Hello, world! awdawfawfawe";

        // Hash the message using SHA-256
        let message_hash = sha256(message);

        /// Make a X509 request with the given private key
        pub fn mk_request(key_pair: &PKey<Private>) -> Result<X509Req, ErrorStack> {
            let mut req_builder = X509ReqBuilder::new()?;
            req_builder.set_pubkey(key_pair)?;

            let mut x509_name = X509NameBuilder::new()?;
            x509_name.append_entry_by_text("C", "US")?;
            x509_name.append_entry_by_text("ST", "TX")?;
            x509_name.append_entry_by_text("O", "Some organization")?;
            x509_name.append_entry_by_text("CN", "www.example.com")?;
            let x509_name = x509_name.build();
            req_builder.set_subject_name(&x509_name)?;

            req_builder.sign(key_pair, MessageDigest::sha256())?;
            let req = req_builder.build();
            Ok(req)
        }

        let pkey = PKey::from_ec_key(ec_key.clone()).unwrap();
        let request = mk_request(&pkey).unwrap();
        println!("{:?}", String::from_utf8(request.to_pem().unwrap()));

        // Sign the hashed message
        let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap();

        // Get the r and s components of the signature
        let r = signature.r().to_vec();
        let s = signature.s().to_vec();

        // Ensure r and s are of fixed length (32 bytes for P-256 curve)
        let mut fixed_r = vec![0; 32];
        let mut fixed_s = vec![0; 32];
        fixed_r[32 - r.len()..].copy_from_slice(&r);
        fixed_s[32 - s.len()..].copy_from_slice(&s);

        // Combine r and s into a fixed-size signature vector
        let mut fixed_signature = vec![];
        fixed_signature.extend_from_slice(&fixed_r);
        fixed_signature.extend_from_slice(&fixed_s);

        // Verify the signature
        let signature = EcdsaSig::from_private_components(
            openssl::bn::BigNum::from_slice(&fixed_signature[..32]).unwrap(),
            openssl::bn::BigNum::from_slice(&fixed_signature[32..]).unwrap(),
        )
        .unwrap();

        // Verify the signature
        let is_valid = signature.verify(&message_hash, &public_key).unwrap();

        // Assert that the signature is valid
        assert!(is_valid);
    }



    #[test]
    fn encrypt() {
        use openssl::encrypt::{Encrypter, Decrypter};
        use openssl::rsa::{Rsa, Padding};
        use openssl::pkey::PKey;

        // Generate a keypair
        let keypair = Rsa::generate(2048).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        let data = b"hello, world!";

        // Encrypt the data with RSA PKCS1
        let mut encrypter = Encrypter::new(&keypair).unwrap();
        encrypter.set_rsa_padding(Padding::PKCS1).unwrap();

        // Create an output buffer
        let buffer_len = encrypter.encrypt_len(data).unwrap();
        let mut encrypted = vec![0; buffer_len];

        // Encrypt and truncate the buffer
        let encrypted_len = encrypter.encrypt(data, &mut encrypted).unwrap();
        encrypted.truncate(encrypted_len);

        // Decrypt the data
        let mut decrypter = Decrypter::new(&keypair).unwrap();
        decrypter.set_rsa_padding(Padding::PKCS1).unwrap();

        // Create an output buffer
        let buffer_len = decrypter.decrypt_len(&encrypted).unwrap();
        let mut decrypted = vec![0; buffer_len];
        // Encrypt and truncate the buffer

        let decrypted_len = decrypter.decrypt(&encrypted, &mut decrypted).unwrap();
        decrypted.truncate(decrypted_len);
        assert_eq!(&*decrypted, data);
    }


    #[test]
    fn enumerate(){
        let n = 10;  // The total count
        let start = 5; // The starting point
        let end = n;  // The endpoint

        // Calculate m dynamically, for example, based on the range from start to end
        let m = end - start;

        for i in 0..n {
            if i < start {
                continue;
            } else {
                if i == 8 {
                    continue;
                }
                let enumerated_value = (i - start) % m;
                println!("i: {}, enumerated_value: {}", i, enumerated_value);
            }
        }
    }

    #[test]
    fn reed_solomon_16_default() {
        let original = [
            b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do ",
            b"eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut e",
            b"nim ad minim veniam, quis nostrud exercitation ullamco laboris n",
        ];

        let recovery = reed_solomon_16::encode(
            3, // total number of original shards
            5, // total number of recovery shards
            original, // all original shards
        ).unwrap();

        let restored = reed_solomon_16::decode(
            3, // total number of original shards
            5, // total number of recovery shards
            [  // provided original shards with indexes
                (1, &original[1]),
            ],
            [  // provided recovery shards with indexes
                (1, &recovery[1]),
                (4, &recovery[4]),
            ],
        ).unwrap();

        for item in restored.clone() {

            println!("{:?}", item.1.len());
        }


        assert_eq!(restored[&0], original[0]);
        assert_eq!(restored[&2], original[2]);
    }

    #[test]
    fn grid_row_iter () {
        use grid::*;
        let grid: Grid<u8> = grid![[1,2][3,4]];
        for ((row, col), i) in grid.indexed_iter() {
            println!("value at row {row} and column {col} is: {i}");
        }
    }

    #[test]
    fn insertion() {
        let mut numbers = vec![1, 2, 3, 4, 5];
        let index_to_add_to = 2; // Index at which you want to add the number
        let number_to_add = 10;

        // Ensure the index is within the bounds of the vector
        if index_to_add_to <= numbers.len() {
            numbers.insert(index_to_add_to, number_to_add);
            println!("Updated vector: {:?}", numbers);
        } else {
            println!("Index out of bounds");
        }
    }

    #[test]
    fn insert_value_valid_position() {
        use super::insert_value;

        let matrix_size = 4;
        let mut matrix = vec![vec![0; matrix_size]; matrix_size];
        let postion = 5;
        // Calculate the row and column indices without subtracting 1
        let row = postion / matrix_size;
        let col = postion  % matrix_size;
        insert_value(&mut matrix, postion, matrix_size, 10);

        assert_eq!(matrix[row][col], 10);
    }

    #[test]
    fn test_encrypt_decrypt() {
        use openssl::symm::{Cipher, Crypter, Mode};
        use openssl::rand::rand_bytes;

        fn generate_random_bytes(size: usize) -> Vec<u8> {
            let mut buf = vec![0; size];
            rand_bytes(&mut buf).expect("Failed to generate random bytes");
            buf
        }

        fn encrypt(cipher: Cipher, data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
            let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
            let mut ciphertext = vec![0; data.len() + cipher.block_size()];
            let mut count = crypter.update(data, &mut ciphertext)?;
            count += crypter.finalize(&mut ciphertext[count..])?;
            ciphertext.truncate(count);
            Ok(ciphertext)
        }

        fn decrypt(cipher: Cipher, data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
            let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))?;
            let mut plaintext = vec![0; data.len() + cipher.block_size()];
            let mut count = crypter.update(data, &mut plaintext)?;
            count += crypter.finalize(&mut plaintext[count..])?;
            plaintext.truncate(count);
            Ok(plaintext)
        }

        let data = b"Hello, OpenSSL with Rust!";

        println!("original data size: {:?}", data.len());

        let key = generate_random_bytes(32); // 256 bits for AES-256
        let iv = generate_random_bytes(16);  // 128 bits for AES block size

        let encrypted_data = encrypt(Cipher::aes_256_cbc(), data, &key, &iv)
            .expect("Failed to encrypt data");

        println!("encrypted data size: {:?}", encrypted_data.len());


        let decrypted_data = decrypt(Cipher::aes_256_cbc(), &encrypted_data, &key, &iv)
            .expect("Failed to decrypt data");

        assert_eq!(data.to_vec(), decrypted_data);
    }

    #[test]
    fn add_row_fec(){

        // Define the input vectors
        let input = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
        ];

        // Transpose the input using map and zip
        let transposed: Vec<Vec<_>> = (0..input[0].len())
            .map(|col_index| input.iter().map(|row| row[col_index]).collect())
            .collect();

        // Print the transposed vector of arrays
        for row in &transposed {
            println!("{:?}", row);
        }
        println!("------");

        // Define the input vectors

        let input = vec![
            vec![1, 4],
            vec![2, 5],
            vec![3, 6],
        ];

        // Transpose the rows, appending columns to the matrix
        let transposed_fec: Vec<Vec<_>> = (0..input[0].len())
        .map(|row_index| {
            input.iter().map(|row| row[row_index].clone()).collect()
        })
        .collect();


        // Print the transposed vector of arrays
        for row in &transposed_fec {
            println!("{:?}", row);
        }


        //assert_eq!(dm, dm1);

    }
}
