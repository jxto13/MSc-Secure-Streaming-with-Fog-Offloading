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
    fn tst_signature(){
        use openssl::{ec::EcKey, nid::Nid};
        use openssl::sign::{Signer, Verifier};
        use openssl::rsa::Rsa;
        use openssl::pkey::PKey;
        use openssl::hash::MessageDigest;
        use openssl::ec::EcGroup;
        use openssl::ecdsa::EcdsaSig;

        // Generate a keypair
        let keypair = Rsa::generate(2048).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();
        
        let data = b"hello, world!";
        let data2 = b"hola, mundo!";

        // // Create EC key from a specific curve (e.g., NID_secp256k1 for Bitcoin)
        // let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        // let key = EcKey::generate(&group).unwrap();

        // // Sign the message
        // let digest = MessageDigest::sha256();
        // let signature = EcdsaSig::sign(data, &key).unwrap();

        // // Sign the data
        let mut signer = Signer::new(MessageDigest::sha256(), &keypair).unwrap();
        signer.update(data).unwrap();
        signer.update(data2).unwrap();
        let signature = signer.sign_to_vec().unwrap();
        println!("{:?}", signature.len());
        
        
        // Verify the data
        let mut verifier = Verifier::new(MessageDigest::sha256(), &keypair).unwrap();
        verifier.update(data).unwrap();
        verifier.update(data2).unwrap();
        assert!(verifier.verify(&signature).unwrap());
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
        
        let key = generate_random_bytes(32); // 256 bits for AES-256
        let iv = generate_random_bytes(16);  // 128 bits for AES block size
        
        let encrypted_data = encrypt(Cipher::aes_256_cbc(), data, &key, &iv)
            .expect("Failed to encrypt data");
        
        
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
