use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Debug)]
pub struct ConcurrentVector<K, V> {
    data: Vec<(K, RwLock<V>)>,
}

impl<K: PartialEq, V> ConcurrentVector<K, V> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.data.push((key, RwLock::new(value)));
    }

    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        for (k, v) in &self.data {
            if k == key {
                let value = v.read().unwrap(); // Read lock
                return Some(value.clone());
            }
        }
        None
    }

    pub fn update(&self, key: &K, new_value: V) -> bool {
        for (k, v) in &self.data {
            if k == key {
                let mut value = v.write().unwrap(); // Write lock
                *value = new_value;
                return true;
            }
        }
        false
    }

    pub fn values(&self) -> Vec<V>
    where
        V: Clone,
    {
        self.data.iter()
            .map(|(_, v)| v.read().unwrap().clone())
            .collect()
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &V) -> bool,
        V: Clone,
    {
        self.data.retain(|(k, v)| f(k, &v.read().unwrap()));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &RwLock<V>)> {
        self.data.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub mod crypto_utils {

    use openssl::symm::{Cipher, Crypter, Mode};

    pub fn encrypt(
        data: &[u8],
        key: &[u8],
        iv: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cipher = Cipher::aes_256_cbc();
        let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
        let mut ciphertext = vec![0; data.len() + cipher.block_size()];
        let mut count = crypter.update(data, &mut ciphertext)?;
        count += crypter.finalize(&mut ciphertext[count..])?;
        ciphertext.truncate(count);
        Ok(ciphertext)
    }

    pub fn decrypt(
        data: &[u8],
        key: &[u8],
        iv: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

pub mod xor {
    pub fn xor_with_padding(vec1: Vec<u8>, vec2: Vec<u8>) -> Vec<u8> {
        let max_len = vec1.len().max(vec2.len());
        let mut result = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let byte1 = *vec1.get(i).unwrap_or(&0);
            let byte2 = *vec2.get(i).unwrap_or(&0);
            result.push(byte1 ^ byte2);
        }

        result
    }

    pub fn depad_vector(vec: Vec<u8>) -> Vec<u8> {
        let mut end = vec.len();
        while end > 0 && vec[end - 1] == 0 {
            end -= 1;
        }
        vec[..end].to_vec()
    }
}

pub mod nc_type {
    use grid::Grid;
    use std::collections::{HashSet};

    // Represents a retransmission strategy
    #[derive(Debug)]
    pub enum Transmission {
        Unicast(usize, usize),      // (Receiver, Packet)
        XorMulticast(usize, usize), // (Packet1, Packet2)
    }

    // Function to compute the best retransmission strategy
    pub fn retransmit(
        loss_matrix: &Grid<u8>, // Loss matrix: 1 = packet missing, 0 = received
        multicast_cost: f64,
        unicast_cost: f64,
    ) -> Vec<Transmission> {
        let num_receivers = loss_matrix.rows();
        let num_packets = loss_matrix.cols();
        let mut best_transmissions: Vec<Transmission> = Vec::new();

        // Precompute receiver sets for each packet
        let mut receiver_sets: Vec<HashSet<usize>> = vec![HashSet::new(); num_packets];
        for receiver in 0..num_receivers {
            for packet in 0..num_packets {
                if *loss_matrix.get(receiver, packet).unwrap() == 1 {
                    receiver_sets[packet].insert(receiver);
                }
            }
        }

        // Track uncovered receivers
        let mut uncovered_packets: HashSet<(usize, usize)> = HashSet::new();
        for packet in 0..num_packets {
            for &receiver in &receiver_sets[packet] {
                uncovered_packets.insert((receiver, packet));
            }
        }

        while !uncovered_packets.is_empty() {
            let mut best_cost = f64::MAX;
            let mut best_strategy: Option<Transmission> = None;

            // Evaluate all unicast options
            for &(receiver, packet) in &uncovered_packets {
                let cost = unicast_cost;
                if cost < best_cost {
                    best_cost = cost;
                    best_strategy = Some(Transmission::Unicast(receiver, packet));
                }
            }

            // Evaluate all XOR multicast options
            for packet1 in 0..num_packets {
                for packet2 in packet1 + 1..num_packets {
                    let union_set: HashSet<_> = receiver_sets[packet1]
                        .union(&receiver_sets[packet2])
                        .cloned()
                        .collect();
                    let cost = multicast_cost; // Cost of XOR multicast
                    if cost < best_cost {
                        best_cost = cost;
                        best_strategy = Some(Transmission::XorMulticast(packet1, packet2));
                    }
                }
            }

            // Apply the best strategy
            if let Some(strategy) = best_strategy {
                match strategy {
                    Transmission::Unicast(receiver, packet) => {
                        best_transmissions.push(Transmission::Unicast(receiver, packet));
                        uncovered_packets.retain(|&(r, p)| !(r == receiver && p == packet));
                    }
                    Transmission::XorMulticast(packet1, packet2) => {
                        best_transmissions.push(Transmission::XorMulticast(packet1, packet2));
                        let union_set: HashSet<_> = receiver_sets[packet1]
                            .union(&receiver_sets[packet2])
                            .cloned()
                            .collect();
                        for receiver in union_set {
                            uncovered_packets.retain(|&(r, p)| {
                                !(r == receiver && (p == packet1 || p == packet2))
                            });
                        }
                    }
                }
            }
        }

        best_transmissions
    }
}

pub mod bandwidth_utils {
    use std::thread;
    use std::time::{Duration, Instant};

    /// A struct representing a simple bandwidth manager.
    pub struct BandwidthManager {
        bitrate: usize,                     // Maximum bits per second, in kbit/s
        bytes_sent: usize,                  // Bytes sent in the current interval
        multicast_packets_sent: usize,      // Number of multicast packets sent
        retransmission_packets_sent: usize, // Number of retransmission packets sent
        interval_start: Instant,            // Tracks the start of the interval
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
                let sleep_duration =
                    Duration::from_secs_f64(excess_bytes as f64 / allowed_bytes_per_second as f64);
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

pub mod rlnc {

    use rand::Rng;
    use std::collections::HashMap;

    #[derive(Debug)]
    pub struct RLNC {
        pub generation_size: usize,
        pub packet_size: usize,
        pub coefficients: Vec<Vec<u8>>,
        pub encoded_packets: Vec<Vec<u8>>,
    }

    impl RLNC {
        pub fn new(generation_size: usize, packet_size: usize) -> Self {
            RLNC {
                generation_size,
                packet_size,
                coefficients: Vec::new(),
                encoded_packets: Vec::new(),
            }
        }

        pub fn encode(&mut self, packets: Vec<Vec<u8>>) {
            let mut rng = rand::thread_rng();
            for _ in 0..self.generation_size {
                let mut coeffs = vec![0u8; self.generation_size];
                for coeff in &mut coeffs {
                    *coeff = rng.gen();
                }
                self.coefficients.push(coeffs.clone());

                let mut encoded_packet = vec![0u8; self.packet_size];
                for (i, packet) in packets.iter().enumerate() {
                    for j in 0..self.packet_size {
                        encoded_packet[j] ^= coeffs[i] * packet[j];
                    }
                }
                self.encoded_packets.push(encoded_packet);
            }
        }

        pub fn decode(
            &self,
            received_packets: Vec<Vec<u8>>,
            received_coeffs: Vec<Vec<u8>>,
        ) -> Vec<Vec<u8>> {
            let mut decoded_packets = vec![vec![0u8; self.packet_size]; self.generation_size];
            let mut coeff_matrix = vec![vec![0u8; self.generation_size]; self.generation_size];

            for i in 0..self.generation_size {
                for j in 0..self.generation_size {
                    coeff_matrix[i][j] = received_coeffs[i][j];
                }
            }

            for i in 0..self.generation_size {
                for j in 0..self.packet_size {
                    decoded_packets[i][j] = received_packets[i][j];
                }
            }

            // Gaussian elimination to solve the linear system
            for i in 0..self.generation_size {
                let pivot = coeff_matrix[i][i];
                for j in 0..self.generation_size {
                    coeff_matrix[i][j] /= pivot;
                }
                for j in 0..self.packet_size {
                    decoded_packets[i][j] /= pivot;
                }

                for k in 0..self.generation_size {
                    if k != i {
                        let factor = coeff_matrix[k][i];
                        for j in 0..self.generation_size {
                            coeff_matrix[k][j] ^= factor * coeff_matrix[i][j];
                        }
                        for j in 0..self.packet_size {
                            decoded_packets[k][j] ^= factor * decoded_packets[i][j];
                        }
                    }
                }
            }

            decoded_packets
        }
    }
}

pub mod source_coding {

    pub type Matrix = Vec<Vec<u8>>;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Transmission {
        Single(usize),
        Xor(Vec<usize>), // Supports XOR of 2 or 3 packets
    }

    // Check if client can resolve a missing packet from an XOR combination
    fn can_resolve_xor(client: &Vec<u8>, combo: &Vec<usize>) -> bool {
        let known = combo.iter().filter(|&&i| client[i] == 1).count();
        let unknown = combo.iter().filter(|&&i| client[i] == 0).count();
        known + 1 == combo.len() && unknown == 1
    }

    // Apply an XOR transmission to the state
    fn apply_xor(state: &mut Matrix, combo: &Vec<usize>) {
        for row in state.iter_mut() {
            let known: Vec<usize> = combo.iter().cloned().filter(|&i| row[i] == 1).collect();
            let unknown: Vec<usize> = combo.iter().cloned().filter(|&i| row[i] == 0).collect();
            if known.len() + 1 == combo.len() && unknown.len() == 1 {
                row[unknown[0]] = 1;
            }
        }
    }

    // Apply a single packet transmission
    fn apply_single(state: &mut Matrix, packet: usize) {
        for row in state.iter_mut() {
            row[packet] = 1;
        }
    }

    // Generate all combinations of `k` packets out of `n`
    fn all_combinations(k: usize, n: usize) -> Vec<Vec<usize>> {
        fn combine(
            result: &mut Vec<Vec<usize>>,
            current: &mut Vec<usize>,
            n: usize,
            start: usize,
            k: usize,
        ) {
            if current.len() == k {
                result.push(current.clone());
                return;
            }
            for i in start..n {
                current.push(i);
                combine(result, current, n, i + 1, k);
                current.pop();
            }
        }

        let mut result = vec![];
        combine(&mut result, &mut vec![], n, 0, k);
        result
    }

    // Greedy strategy that picks the best packet or XOR combo in each step
    pub fn find_optimal_retransmissions(mut state: Matrix) -> Vec<Transmission> {
        let num_packets = state[0].len();
        let mut transmissions = Vec::new();

        while state.iter().any(|row| row.contains(&0)) {
            let mut best_gain = 0;
            let mut best_transmission = None;

            // Try all single packets
            for j in 0..num_packets {
                let gain = state.iter().filter(|row| row[j] == 0).count();
                if gain > best_gain {
                    best_gain = gain;
                    best_transmission = Some(Transmission::Single(j));
                }
            }

            // Try XOR combinations of 2 and 3 packets
            for k in 2..=3 {
                for combo in all_combinations(k, num_packets) {
                    let gain = state
                        .iter()
                        .filter(|row| can_resolve_xor(row, &combo))
                        .count();
                    if gain > best_gain {
                        best_gain = gain;
                        best_transmission = Some(Transmission::Xor(combo));
                    }
                }
            }

            // Apply the best transmission
            match best_transmission {
                Some(Transmission::Single(j)) => {
                    apply_single(&mut state, j);
                    transmissions.push(Transmission::Single(j));
                }
                Some(Transmission::Xor(combo)) => {
                    apply_xor(&mut state, &combo);
                    transmissions.push(Transmission::Xor(combo));
                }
                None => break, // Nothing to improve
            }
        }

        transmissions
    }

    pub fn run_test(matrix: Matrix, expected: Vec<Transmission>) {
        let result = find_optimal_retransmissions(matrix);
        assert_eq!(result.len(), expected.len(), "Transmission count mismatch.");
        for (i, (res, exp)) in result.iter().zip(expected.iter()).enumerate() {
            assert_eq!(res, exp, "Mismatch at step {}: {:?} != {:?}", i, res, exp);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::rlnc::RLNC;
    use crate::crypto_utils::{decrypt, encrypt};
    use crate::padding::{depad, pad};
    use super::source_coding::{Transmission, run_test};

    #[test]
    fn test_simple_case() {
        let matrix = vec![
            vec![1, 0, 0, 1],
            vec![1, 1, 0, 1],
            vec![1, 0, 0, 1],
            vec![1, 1, 0, 0],
        ];
        let expected = vec![
            Transmission::Single(2),
            Transmission::Xor(vec![1, 3]),
        ];
        run_test(matrix, expected);
    }

    #[test]
    fn test_all_missing_same() {
        let matrix = vec![
            vec![1, 0, 1],
            vec![1, 0, 1],
            vec![1, 0, 1],
        ];
        let expected = vec![Transmission::Single(1)];
        run_test(matrix, expected);
    }

    #[test]
    fn test_can_use_triple_xor() {
        let matrix = vec![
            vec![0, 1, 1],
            vec![1, 0, 1],
            vec![1, 1, 0],
        ];
        let expected = vec![Transmission::Xor(vec![0, 1, 2])];
        run_test(matrix, expected);
    }

    #[test]
    fn test_double_xor_works() {
        let matrix = vec![
            vec![1, 0],
            vec![0, 1],
        ];
        let expected = vec![Transmission::Xor(vec![0, 1])];
        run_test(matrix, expected);
    }

    #[test]
    fn test_individual_sends() {
        let matrix = vec![
            vec![0, 0, 1],
            vec![0, 1, 0],
            vec![1, 0, 0],
        ];

        let expected = vec![
            Transmission::Single(0),
            Transmission::Single(1),
            Transmission::Single(2),
        ];
        run_test(matrix, expected);
    }


    #[test]
    fn test_rlnc() {
        let generation_size = 4;
        let packet_size = 8;
        let packets = vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![8, 7, 6, 5, 4, 3, 2, 1],
            vec![1, 1, 1, 1, 1, 1, 1, 1],
            vec![2, 2, 2, 2, 2, 2, 2, 2],
        ];

        let mut rlnc = RLNC::new(generation_size, packet_size);
        rlnc.encode(packets.clone());

        let decoded_packets = rlnc.decode(rlnc.encoded_packets.clone(), rlnc.coefficients.clone());

        assert_eq!(packets, decoded_packets);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let data = b"Hello, world!";
        let key = b"an example very very secret key.";
        let iv = b"unique nonce 123"; // Updated IV to be 16 bytes long

        let encrypted_data = encrypt(data, key, iv).expect("Encryption failed");
        let decrypted_data = decrypt(&encrypted_data, key, iv).expect("Decryption failed");
        println!("{:?}", encrypted_data);

        println!("{:?} {:?}", data, decrypted_data);

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
