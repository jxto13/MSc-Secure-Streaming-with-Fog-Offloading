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
    use std::{net::SocketAddr, vec};

    #[test]
    fn test_multicast_simulation_various_matrices() {
        fn multicast_simulation(mut clients: Vec<Vec<u8>>) -> (bool, Vec<String>) {
            let num_clients = clients.len();
            let num_packets = clients[0].len();
            let mut sent_packets = Vec::new();

            loop {
                // Count missing packets per column
                let mut missing_count = vec![0; num_packets];
                for client in &clients {
                    for (j, &p) in client.iter().enumerate() {
                        if p == 0 {
                            missing_count[j] += 1;
                        }
                    }
                }

                // If all packets are received, break
                if missing_count.iter().all(|&x| x == 0) {
                    break;
                }

                // Find the packet with the most missing
                let (max_idx, &max_val) = missing_count
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_, &val)| val)
                    .unwrap();

                // If only one type of packet is missing, send it
                if missing_count.iter().filter(|&&x| x > 0).count() == 1 {
                    sent_packets.push(format!("Send packet {}", max_idx));
                    for client in clients.iter_mut() {
                        client[max_idx] = 1;
                    }
                    continue;
                }

                // Otherwise, try to find a pair of packets that are missing together
                let mut found_pair = false;
                for i in 0..num_packets {
                    for j in (i + 1)..num_packets {
                        // Check if all remaining missing packets are in columns i and j
                        let mut only_i_j = true;
                        for client in &clients {
                            for k in 0..num_packets {
                                if client[k] == 0 && k != i && k != j {
                                    only_i_j = false;
                                }
                            }
                        }
                        if only_i_j && (missing_count[i] > 0 && missing_count[j] > 0) {
                            // Send XOR of i and j
                            sent_packets.push(format!("Send packet {}⊕{}", i, j));
                            for client in clients.iter_mut() {
                                if client[i] == 0 || client[j] == 0 {
                                    client[i] = 1;
                                    client[j] = 1;
                                }
                            }
                            found_pair = true;
                            break;
                        }
                    }
                    if found_pair {
                        break;
                    }
                }

                // If no pair found, send the most-missing packet
                if !found_pair {
                    sent_packets.push(format!("Send packet {}", max_idx));
                    for client in clients.iter_mut() {
                        client[max_idx] = 1;
                    }
                }
            }

            // Check if all packets are received
            let all_received = clients.iter().all(|c| c.iter().all(|&p| p == 1));
            (all_received, sent_packets)
        }

        // Test with various matrices
        let matrices = vec![
            vec![
                vec![1, 0, 0, 1],
                vec![1, 1, 0, 1],
                vec![1, 0, 0, 1],
                vec![1, 1, 0, 0],
            ],
            vec![vec![0, 0, 1], vec![1, 0, 0], vec![0, 1, 0]],
            vec![vec![1, 1, 1], vec![1, 1, 1]],
            vec![vec![0, 0, 0], vec![0, 0, 0]],
            // Case: diagonal missing
            vec![vec![0, 1, 1], vec![1, 0, 1], vec![1, 1, 0]],
            // Case: one client missing all
            vec![vec![0, 0, 0, 0], vec![1, 1, 1, 1], vec![1, 1, 1, 1]],
            // Case: two clients, alternate missing
            vec![vec![1, 0, 1, 0], vec![0, 1, 0, 1]],
            // Case: all but one packet missing for all
            vec![vec![1, 0, 0, 0], vec![1, 0, 0, 0], vec![1, 0, 0, 0]],
            // Case: random missing
            vec![vec![1, 0, 1, 1], vec![0, 1, 0, 1], vec![1, 1, 0, 0]],
        ];

        for (i, matrix) in matrices.into_iter().enumerate() {
            let (all_received, sent_packets) = multicast_simulation(matrix);
            println!("Matrix {}:", i + 1);
            for pkt in sent_packets {
                println!("  {}", pkt);
            }
            println!("  All packets received? {}\n", all_received);
        }
    }

    #[test]
    fn test_dash_map_capacity() {
        #[derive(Debug, Clone)]
        struct Matrix {
            // Assuming a 2D matrix (you can modify this as needed)
            data: Vec<Vec<i32>>,
            index: usize, // Index associated with the matrix
        }

        struct MatrixHistory {
            matrices: Vec<Option<Matrix>>, // Vec of Option to handle empty slots
            capacity: usize,
            next_index: usize, // Index where the next matrix will go
            size: usize,       // Current number of matrices in the buffer
        }

        impl MatrixHistory {
            // Constructor to create a new MatrixHistory
            fn new(capacity: usize) -> Self {
                MatrixHistory {
                    matrices: vec![None; capacity], // Initialize with None values
                    capacity,
                    next_index: 0,
                    size: 0,
                }
            }

            // Add a new matrix to the history
            fn add_matrix(&mut self, matrix: Matrix) {
                if self.size < self.capacity {
                    // If there is space, simply add
                    self.matrices[self.next_index] = Some(matrix);
                    self.size += 1;
                } else {
                    // If buffer is full, overwrite the oldest matrix
                    self.matrices[self.next_index] = Some(matrix);
                }

                // Update next_index (circular buffer)
                self.next_index = (self.next_index + 1) % self.capacity;
            }

            // Get a matrix by its index
            fn get_matrix(&self, index: usize) -> Option<&Matrix> {
                // Search for the matrix with the given index
                for matrix_option in &self.matrices {
                    if let Some(matrix) = matrix_option {
                        if matrix.index == index {
                            return Some(matrix);
                        }
                    }
                }
                None
            }
        }

        let mut history = MatrixHistory::new(3); // Store the last 3 matrices

        // Add matrices with specific index numbers
        history.add_matrix(Matrix {
            data: vec![vec![1, 2], vec![3, 4]],
            index: 101,
        });
        history.add_matrix(Matrix {
            data: vec![vec![5, 6], vec![7, 8]],
            index: 102,
        });
        history.add_matrix(Matrix {
            data: vec![vec![9, 10], vec![11, 12]],
            index: 103,
        });

        // Adding a new matrix will remove the oldest one
        history.add_matrix(Matrix {
            data: vec![vec![13, 14], vec![15, 16]],
            index: 104,
        });

        // Access matrices by their index
        if let Some(matrix) = history.get_matrix(101) {
            println!("Matrix with index 101: {:?}", matrix);
        }

        if let Some(matrix) = history.get_matrix(104) {
            println!("Matrix with index 104: {:?}", matrix);
        }

        // Try to access a matrix that doesn't exist
        if let Some(matrix) = history.get_matrix(999) {
            println!("Matrix with index 999: {:?}", matrix);
        } else {
            println!("No matrix found with index 999");
        }
    }

    #[test]
    fn sort_ips() {
        use std::net::IpAddr;
        let mut ips = vec![
            "10.0.0.1".parse::<IpAddr>().unwrap(),
            "172.16.0.1".parse::<IpAddr>().unwrap(),
            "192.168.0.1".parse::<IpAddr>().unwrap(),
        ];

        // The list must already be sorted
        let new_ip = "8.8.8.8".parse::<IpAddr>().unwrap();

        match ips.binary_search(&new_ip) {
            Ok(_) => {
                // Already exists; do nothing or handle duplicates
                println!("IP already in the list");
            }
            Err(pos) => {
                // Insert at the position to keep the list sorted
                ips.insert(pos, new_ip);
                println!("Inserted at position {}", pos);
            }
        }

        for ip in &ips {
            println!("{}", ip);
        }
    }

    #[test]
    fn compression_test() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        fn compress_data(data: &[u8]) -> Vec<u8> {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        }

        fn decompress_data(data: &[u8]) -> Vec<u8> {
            use flate2::read::GzDecoder;
            use std::io::Read;

            let mut decoder = GzDecoder::new(data);
            let mut decompressed_data = Vec::new();
            decoder.read_to_end(&mut decompressed_data).unwrap();
            decompressed_data
        }

        let data = b"Hello, Hello, Hello, Hello, Hello!";
        println!("Original size: {}", data.len());
        let compressed = compress_data(data);
        println!("Compressed size: {}", compressed.len());
        let decompressed = decompress_data(&compressed);
        println!("Decompressed size: {}", decompressed.len());
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_xor_with_padding() {
        fn xor_with_padding(vec1: Vec<u8>, vec2: Vec<u8>) -> Vec<u8> {
            let max_len = vec1.len().max(vec2.len());
            let mut result = Vec::with_capacity(max_len);

            for i in 0..max_len {
                let byte1 = *vec1.get(i).unwrap_or(&0);
                let byte2 = *vec2.get(i).unwrap_or(&0);
                result.push(byte1 ^ byte2);
            }

            result
        }

        fn depad_vector(vec: Vec<u8>) -> Vec<u8> {
            let mut end = vec.len();
            while end > 0 && vec[end - 1] == 0 {
                end -= 1;
            }
            vec[..end].to_vec()
        }

        let vec1 = vec![1, 2, 3];
        let vec2 = vec![4, 5];
        let expected_xor_result = vec![5, 7, 3];

        let xor_result = xor_with_padding(vec1.clone(), vec2.clone());
        assert_eq!(xor_result, expected_xor_result);

        let recovered_vec1 = xor_with_padding(xor_result.clone(), vec2.clone());
        let recovered_vec2 = xor_with_padding(xor_result.clone(), vec1.clone());

        println!("{:?}", recovered_vec1);
        println!("{:?}", recovered_vec2);
        assert_eq!(depad_vector(recovered_vec1), vec1);
        assert_eq!(depad_vector(recovered_vec2), vec2);
    }

    #[test]
    fn test_nc_3() {
        use grid::Grid;
        use std::collections::{HashMap, HashSet};

        // Represents a retransmission strategy
        #[derive(Debug)]
        enum Transmission {
            Unicast(usize, usize),      // (Receiver, Packet)
            Multicast(usize),           // (Packet)
            XorMulticast(usize, usize), // (Packet1, Packet2)
        }

        // Function to compute the best retransmission strategy
        fn retransmit(
            loss_matrix: Grid<u8>, // Loss matrix: 1 = packet missing, 0 = received
            multicast_cost: f64,
            unicast_cost: f64,
        ) -> (Vec<Transmission>, Grid<u8>) {
            let num_receivers = loss_matrix.rows();
            let num_packets = loss_matrix.cols();
            let mut best_transmissions: Vec<Transmission> = Vec::new();

            // Precompute receiver sets for each packet
            let mut receiver_sets: Vec<HashSet<usize>> = Vec::new();
            for packet in 0..num_packets {
                let mut receivers = HashSet::new();
                for receiver in 0..num_receivers {
                    if *loss_matrix.get(receiver, packet).unwrap() == 1 {
                        receivers.insert(receiver);
                    }
                }
                receiver_sets.push(receivers);
            }

            // Track covered receivers and which packets they've recovered
            let mut uncovered: Vec<HashSet<usize>> = receiver_sets.clone();
            let mut updated_loss_matrix = loss_matrix.clone(); // To track recovered packets

            // Calculate miss counts and sort packets by most missed
            let mut packets_with_miss_counts: Vec<(usize, usize)> = receiver_sets
                .iter()
                .enumerate()
                .map(|(packet, receivers)| (packet, receivers.len())) // (packet_index, miss_count)
                .collect();

            packets_with_miss_counts.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by miss_count descending

            println!("Packets with miss counts: {:?}", packets_with_miss_counts);

            // Process packets in priority order
            for (packet_index, _miss_count) in packets_with_miss_counts {
                // Skip if all receivers have the packet
                if receiver_sets[packet_index].is_empty() {
                    continue;
                }

                // Unicast cost for this packet
                let unicast_cost_i = receiver_sets[packet_index].len() as f64 * unicast_cost;

                // Multicast cost for this packet
                let multicast_cost_i = multicast_cost;

                // Evaluate XOR pairing
                let mut best_pair_cost = f64::MAX;
                let mut best_pair: Option<(usize, usize)> = None;
                let mut best_pair_union: HashSet<usize> = HashSet::new();

                for j in packet_index + 1..num_packets {
                    // Compute union of receivers missing P_i and P_j
                    let union_set: HashSet<_> = receiver_sets[packet_index]
                        .union(&receiver_sets[j])
                        .cloned()
                        .collect();
                    let xor_set: HashSet<_> = receiver_sets[packet_index]
                        .symmetric_difference(&receiver_sets[j])
                        .cloned()
                        .collect();

                    // Cost of multicasting XOR pair
                    let pair_cost = multicast_cost;

                    if pair_cost < best_pair_cost {
                        best_pair_cost = pair_cost;
                        best_pair = Some((packet_index, j));
                        best_pair_union = union_set;
                    }
                }

                // Decide the best strategy
                if unicast_cost_i < multicast_cost_i && unicast_cost_i < best_pair_cost {
                    // Unicast
                    for &receiver in &receiver_sets[packet_index] {
                        best_transmissions.push(Transmission::Unicast(receiver, packet_index));
                        uncovered[receiver].remove(&packet_index);
                        *updated_loss_matrix.get_mut(receiver, packet_index).unwrap() = 1;
                    }
                } else if multicast_cost_i < best_pair_cost {
                    // Multicast single packet
                    best_transmissions.push(Transmission::Multicast(packet_index));
                    for &receiver in &receiver_sets[packet_index] {
                        uncovered[receiver].remove(&packet_index);
                        *updated_loss_matrix.get_mut(receiver, packet_index).unwrap() = 1;
                    }
                } else if let Some((packet1, packet2)) = best_pair {
                    // Multicast XOR pair
                    best_transmissions.push(Transmission::XorMulticast(packet1, packet2));
                    for &receiver in &best_pair_union {
                        if receiver_sets[packet1].contains(&receiver)
                            || receiver_sets[packet2].contains(&receiver)
                        {
                            uncovered[receiver].remove(&packet1);
                            uncovered[receiver].remove(&packet2);
                            *updated_loss_matrix.get_mut(receiver, packet1).unwrap() = 1;
                            *updated_loss_matrix.get_mut(receiver, packet2).unwrap() = 1;
                        }
                    }
                }

                // Debug: Print intermediate states
                println!("After processing packet {}: ", packet_index);
                println!("Best Transmissions: {:?}", best_transmissions);
                println!("Updated Loss Matrix: {:?}", updated_loss_matrix);
                println!("Uncovered: {:?}", uncovered);
            }

            // Handle any remaining uncovered packets with unicast or multicast
            for receiver in 0..num_receivers {
                for &packet in &uncovered[receiver] {
                    best_transmissions.push(Transmission::Unicast(receiver, packet));
                    *updated_loss_matrix.get_mut(receiver, packet).unwrap() = 1;
                }
            }

            (best_transmissions, updated_loss_matrix)
        }

        fn main() {
            // Example loss matrix: rows = receivers, columns = packets
            let loss_matrix = Grid::from_vec(
                vec![
                    vec![1, 0, 0, 1], // Receiver 1
                    vec![1, 1, 0, 1], // Receiver 2
                    vec![1, 0, 0, 1], // Receiver 3
                    vec![1, 1, 0, 0], // Receiver 4
                ]
                .concat(),
                4,
            );

            // Bandwidth costs
            let multicast_cost = 1.0;
            let unicast_cost = 1.5;

            // Compute the best retransmissions and the final loss matrix
            let (transmissions, final_loss_matrix) =
                retransmit(loss_matrix, multicast_cost, unicast_cost);

            // Output the result
            println!("Best Transmissions:");
            for transmission in transmissions {
                println!("{:?}", transmission);
            }

            println!("\nFinal Loss Matrix:");
            for row in 0..final_loss_matrix.rows() {
                let mut row_vec = Vec::new();
                for col in 0..final_loss_matrix.cols() {
                    row_vec.push(*final_loss_matrix.get(row, col).unwrap());
                }
                println!("{:?}", row_vec);
            }
        }

        main();
    }

    #[test]
    fn test_nc_2() {
        use std::collections::{HashMap, HashSet};

        // Represents a retransmission strategy
        #[derive(Debug)]
        enum Transmission {
            Unicast(usize, usize),      // (Receiver, Packet)
            Multicast(usize),           // (Packet)
            XorMulticast(usize, usize), // (Packet1, Packet2)
        }

        // Function to compute the best retransmission strategy
        fn retransmit(
            loss_matrix: Vec<Vec<u8>>, // Loss matrix: 1 = packet missing, 0 = received
            multicast_cost: f64,
            unicast_cost: f64,
        ) -> (Vec<Transmission>, Vec<Vec<u8>>) {
            let num_receivers = loss_matrix.len();
            let num_packets = loss_matrix[0].len();
            let mut best_transmissions: Vec<Transmission> = Vec::new();

            // Precompute receiver sets for each packet
            let mut receiver_sets: Vec<HashSet<usize>> = Vec::new();
            for packet in 0..num_packets {
                let mut receivers = HashSet::new();
                for (receiver, row) in loss_matrix.iter().enumerate() {
                    if row[packet] == 1 {
                        receivers.insert(receiver);
                    }
                }
                receiver_sets.push(receivers);
            }

            // Track covered receivers and which packets they've recovered
            let mut uncovered: Vec<HashSet<usize>> = receiver_sets.clone();
            let mut updated_loss_matrix = loss_matrix.clone(); // To track recovered packets

            for i in 0..num_packets {
                // Skip if all receivers have the packet
                if receiver_sets[i].is_empty() {
                    continue;
                }

                // Unicast cost for this packet
                let unicast_cost_i = receiver_sets[i].len() as f64 * unicast_cost;

                // Multicast cost for this packet
                let multicast_cost_i = multicast_cost;

                // Evaluate XOR pairing
                let mut best_pair_cost = f64::MAX;
                let mut best_pair: Option<(usize, usize)> = None;
                let mut best_pair_union: HashSet<usize> = HashSet::new();

                for j in i + 1..num_packets {
                    // Compute union of receivers missing P_i and P_j
                    let union_set: HashSet<_> =
                        receiver_sets[i].union(&receiver_sets[j]).cloned().collect();
                    let xor_set: HashSet<_> = receiver_sets[i]
                        .symmetric_difference(&receiver_sets[j])
                        .cloned()
                        .collect();

                    // Cost of multicasting XOR pair
                    let pair_cost = multicast_cost;

                    if pair_cost < best_pair_cost {
                        best_pair_cost = pair_cost;
                        best_pair = Some((i, j));
                        best_pair_union = union_set;
                    }
                }

                // Decide the best strategy
                if unicast_cost_i < multicast_cost_i && unicast_cost_i < best_pair_cost {
                    // Unicast
                    for &receiver in &receiver_sets[i] {
                        best_transmissions.push(Transmission::Unicast(receiver, i));
                        uncovered[receiver].remove(&i);
                        updated_loss_matrix[receiver][i] = 1;
                    }
                } else if multicast_cost_i < best_pair_cost {
                    // Multicast single packet
                    best_transmissions.push(Transmission::Multicast(i));
                    for &receiver in &receiver_sets[i] {
                        uncovered[receiver].remove(&i);
                        updated_loss_matrix[receiver][i] = 1;
                    }
                } else if let Some((packet1, packet2)) = best_pair {
                    // Multicast XOR pair
                    best_transmissions.push(Transmission::XorMulticast(packet1, packet2));
                    for &receiver in &best_pair_union {
                        if receiver_sets[packet1].contains(&receiver)
                            || receiver_sets[packet2].contains(&receiver)
                        {
                            uncovered[receiver].remove(&packet1);
                            uncovered[receiver].remove(&packet2);
                            updated_loss_matrix[receiver][packet1] = 1;
                            updated_loss_matrix[receiver][packet2] = 1;
                        }
                    }
                }

                // Debug: Print intermediate states
                println!("After processing packet {}: ", i);
                println!("Best Transmissions: {:?}", best_transmissions);
                println!("Updated Loss Matrix: {:?}", updated_loss_matrix);
                println!("Uncovered: {:?}", uncovered);
            }

            // Handle any remaining uncovered packets with unicast or multicast
            for receiver in 0..num_receivers {
                for &packet in &uncovered[receiver] {
                    best_transmissions.push(Transmission::Unicast(receiver, packet));
                    updated_loss_matrix[receiver][packet] = 1;
                }
            }

            (best_transmissions, updated_loss_matrix)
        }

        fn main() {
            // Example loss matrix: rows = receivers, columns = packets
            let loss_matrix = vec![
                vec![1, 0, 0, 1, 0], // Receiver 1
                vec![1, 0, 1, 0, 1], // Receiver 2
                vec![0, 0, 1, 0, 0], // Receiver 3
                vec![1, 0, 0, 1, 0], // Receiver 4
                vec![0, 0, 0, 0, 1], // Receiver 5
            ];

            // Bandwidth costs
            let multicast_cost = 1.0;
            let unicast_cost = 1.5;

            // Compute the best retransmissions and the final loss matrix
            let (transmissions, final_loss_matrix) =
                retransmit(loss_matrix, multicast_cost, unicast_cost);

            // Output the result
            println!("Best Transmissions:");
            for transmission in transmissions {
                println!("{:?}", transmission);
            }

            println!("\nFinal Loss Matrix:");
            for row in final_loss_matrix {
                println!("{:?}", row);
            }
        }

        main();
    }

    #[test]
    fn test_nc() {
        use std::collections::{HashMap, HashSet};

        // Represents a retransmission strategy
        #[derive(Debug)]
        enum Transmission {
            Unicast(usize, usize),      // (Receiver, Packet)
            Multicast(usize),           // (Packet)
            XorMulticast(usize, usize), // (Packet1, Packet2)
        }

        // Function to compute the best retransmission strategy
        fn retransmit(
            loss_matrix: Vec<Vec<u8>>, // Loss matrix: 1 = packet missing, 0 = received
            multicast_cost: f64,
            unicast_cost: f64,
        ) -> (Vec<Transmission>, Vec<Vec<u8>>) {
            let num_receivers = loss_matrix.len();
            let num_packets = loss_matrix[0].len();
            let mut best_transmissions: Vec<Transmission> = Vec::new();

            // Precompute receiver sets for each packet
            let mut receiver_sets: Vec<HashSet<usize>> = Vec::new();
            for packet in 0..num_packets {
                let mut receivers = HashSet::new();
                for (receiver, row) in loss_matrix.iter().enumerate() {
                    if row[packet] == 1 {
                        receivers.insert(receiver);
                    }
                }
                receiver_sets.push(receivers);
            }

            // Track covered receivers and which packets they've recovered
            let mut uncovered: Vec<HashSet<usize>> = receiver_sets.clone();
            let mut updated_loss_matrix = loss_matrix.clone(); // To track recovered packets

            for i in 0..num_packets {
                // Skip if all receivers have the packet
                if receiver_sets[i].is_empty() {
                    continue;
                }

                // Unicast cost for this packet
                let unicast_cost_i = receiver_sets[i].len() as f64 * unicast_cost;

                // Multicast cost for this packet
                let multicast_cost_i = multicast_cost;

                // Evaluate XOR pairing
                let mut best_pair_cost = f64::MAX;
                let mut best_pair: Option<(usize, usize)> = None;
                let mut best_pair_union: HashSet<usize> = HashSet::new();

                for j in i + 1..num_packets {
                    // Compute union of receivers missing P_i and P_j
                    let union_set: HashSet<_> =
                        receiver_sets[i].union(&receiver_sets[j]).cloned().collect();
                    let xor_set: HashSet<_> = receiver_sets[i]
                        .symmetric_difference(&receiver_sets[j])
                        .cloned()
                        .collect();

                    // Cost of multicasting XOR pair
                    let pair_cost = multicast_cost;

                    if pair_cost < best_pair_cost {
                        best_pair_cost = pair_cost;
                        best_pair = Some((i, j));
                        best_pair_union = union_set;
                    }
                }

                // Decide the best strategy
                if unicast_cost_i < multicast_cost_i && unicast_cost_i < best_pair_cost {
                    // Unicast
                    for &receiver in &receiver_sets[i] {
                        best_transmissions.push(Transmission::Unicast(receiver, i));
                        uncovered[receiver].remove(&i);
                        updated_loss_matrix[receiver][i] = 1;
                    }
                } else if multicast_cost_i < best_pair_cost {
                    // Multicast single packet
                    best_transmissions.push(Transmission::Multicast(i));
                    for &receiver in &receiver_sets[i] {
                        uncovered[receiver].remove(&i);
                        updated_loss_matrix[receiver][i] = 1;
                    }
                } else if let Some((packet1, packet2)) = best_pair {
                    // Multicast XOR pair
                    best_transmissions.push(Transmission::XorMulticast(packet1, packet2));
                    for &receiver in &best_pair_union {
                        if receiver_sets[packet1].contains(&receiver)
                            || receiver_sets[packet2].contains(&receiver)
                        {
                            uncovered[receiver].remove(&packet1);
                            uncovered[receiver].remove(&packet2);
                            updated_loss_matrix[receiver][packet1] = 1;
                            updated_loss_matrix[receiver][packet2] = 1;
                        }
                    }
                }
            }

            // Handle any remaining uncovered packets with unicast or multicast
            for receiver in 0..num_receivers {
                for &packet in &uncovered[receiver] {
                    best_transmissions.push(Transmission::Unicast(receiver, packet));
                    updated_loss_matrix[receiver][packet] = 1;
                }
            }

            (best_transmissions, updated_loss_matrix)
        }

        fn main() {
            // Example loss matrix: rows = receivers, columns = packets
            let loss_matrix = vec![
                vec![1, 0, 0, 1, 0], // Receiver 1
                vec![1, 0, 1, 0, 1], // Receiver 2
                vec![0, 0, 1, 0, 0], // Receiver 3
                vec![1, 0, 0, 1, 0], // Receiver 4
                vec![0, 0, 0, 0, 1], // Receiver 5
            ];

            // Bandwidth costs
            let multicast_cost = 1.0;
            let unicast_cost = 1.5;

            // Compute the best retransmissions and the final loss matrix
            let (transmissions, final_loss_matrix) =
                retransmit(loss_matrix, multicast_cost, unicast_cost);

            // Output the result
            println!("Best Transmissions:");
            for transmission in transmissions {
                println!("{:?}", transmission);
            }

            println!("\nFinal Loss Matrix:");
            for row in final_loss_matrix {
                println!("{:?}", row);
            }
        }

        main();
    }

    #[test]
    fn test_mio_unicast() {
        // You can run this example from the root of the mio repo:
        // cargo run --example udp_server --features="os-poll net"
        use log::warn;
        use mio::{Events, Interest, Poll, Token};
        use std::io;

        // A token to allow us to identify which event is for the `UdpSocket`.
        const UDP_SOCKET: Token = Token(0);

        fn server(rx: crossbeam_channel::Receiver<(Vec<u8>, SocketAddr)>) -> io::Result<()> {
            use mio::net::UdpSocket;

            env_logger::init();

            // Create a poll instance.
            let mut poll = Poll::new()?;
            // Create storage for events. Since we will only register a single socket, a
            // capacity of 1 will do.
            let mut events = Events::with_capacity(1);

            // Setup the UDP socket.
            let addr = "127.0.0.1:9000".parse().unwrap();

            let mut socket = UdpSocket::bind(addr)?;

            // Register our socket with the token defined above and an interest in being
            // `READABLE`.
            poll.registry()
                .register(&mut socket, UDP_SOCKET, Interest::WRITABLE)?;

            println!("You can connect to the server using `nc`:");
            println!(" $ nc -u 127.0.0.1 9000");
            println!("Anything you type will be echoed back to you.");

            // Initialize a buffer for the UDP packet. We use the maximum size of a UDP
            // packet, which is the maximum value of 16 a bit integer.
            let mut buf = [0; 1 << 16];

            // Our event loop.
            loop {
                // Poll to check if we have events waiting for us.
                if let Err(err) = poll.poll(&mut events, None) {
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }

                // Process each event.
                for event in events.iter() {
                    // Validate the token we registered our socket with,
                    // in this example it will only ever be one but we
                    // make sure it's valid none the less.
                    match event.token() {
                        UDP_SOCKET => {
                            let message = rx.recv().unwrap();

                            socket.send_to(&message.0, message.1)?;
                        }
                        _ => {
                            // This should never happen as we only registered our
                            // `UdpSocket` using the `UDP_SOCKET` token, but if it ever
                            // does we'll log it.
                            warn!("Got event for unexpected token: {:?}", event);
                        }
                    }
                }
            }
        }

        // Create a channel for sending messages to clients
        let (tx, rx) = crossbeam_channel::unbounded();

        let messages = vec![("Hello, world!", SocketAddr::from(([127, 0, 0, 1], 9001)))];

        let handler = std::thread::spawn(move || {
            server(rx).unwrap();
        });

        tx.send((messages[0].0.as_bytes().to_vec(), messages[0].1))
            .unwrap();

        handler.join().unwrap();
    }

    #[test]
    fn test_value() {
        fn find_zero_indices(vec: Vec<u8>) -> Vec<usize> {
            vec.iter()
                .enumerate()
                .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
                .collect()
        }

        fn apply_to_tuples(vec_of_tuples: Vec<(i32, Vec<u8>)>) -> Vec<(i32, Vec<usize>)> {
            vec_of_tuples
                .into_iter()
                .map(|(num, vec)| (num, find_zero_indices(vec)))
                .collect()
        }

        let reports = [
            (1, vec![1, 1, 1, 0, 0, 0]),
            (2, vec![0, 0, 0, 0, 0, 0]),
            (3, vec![1, 1, 1, 1, 1, 1]),
        ];

        let result = apply_to_tuples(reports.to_vec());

        println!("{:?}", result);
    }

    #[test]
    fn test_encrpt() {
        use openssl::rand::rand_bytes;
        use openssl::symm::{Cipher, Crypter, Mode};

        fn encrypt_aes_256_cbc(plaintext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
            let cipher = Cipher::aes_256_cbc();
            let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).unwrap();
            crypter.pad(true);

            let block_size = cipher.block_size();
            let mut ciphertext = vec![0; plaintext.len() + block_size];
            let mut count = crypter.update(plaintext, &mut ciphertext).unwrap();
            count += crypter.finalize(&mut ciphertext[count..]).unwrap();
            ciphertext.truncate(count);
            ciphertext
        }

        fn decrypt_aes_256_cbc(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
            let cipher = Cipher::aes_256_cbc();
            let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).unwrap();
            crypter.pad(true);

            let mut plaintext = vec![0; ciphertext.len() + cipher.block_size()];
            let mut count = crypter.update(ciphertext, &mut plaintext).unwrap();
            count += crypter.finalize(&mut plaintext[count..]).unwrap();
            plaintext.truncate(count);
            plaintext
        }

        let key = b"01234567890123456789012345678901"; // 32 bytes key for AES-256
        let iv = b"0123456789012345"; // 16 bytes IV for AES-CBC

        let plaintext = b"Hello, world!";
        let ciphertext = encrypt_aes_256_cbc(plaintext, key, iv);
        println!("Ciphertext: {:?}", ciphertext);

        let decrypted_plaintext = decrypt_aes_256_cbc(&ciphertext, key, iv);
        println!(
            "Decrypted plaintext: {:?}",
            String::from_utf8(decrypted_plaintext).unwrap()
        );

        // fn generate_random_bytes(size: usize) -> Vec<u8> {
        //     use openssl::rand::rand_bytes;

        //     let mut buf = vec![0; size];
        //     rand_bytes(&mut buf).expect("Failed to generate random bytes");
        //     buf
        // }

        // let data = b"Hello, OpenSSL with Rust!";

        // let key = generate_random_bytes(32); // 256 bits for AES-256
        // let iv = generate_random_bytes(16);  // 128 bits for AES block size
        // let encrypted_data = iris_shared::crypto_utils::encrypt(data, &key, &iv, 1280);

        // let decryped_data = iris_shared::crypto_utils::decrypt(&encrypted_data, &key, &iv);

        // assert_eq!(data.to_vec(), decryped_data);
    }

    #[test]
    fn test_encryption_decryption() {
        use openssl::rand::rand_bytes;
        use openssl::symm::{Cipher, Crypter, Mode};

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
        let iv = generate_random_bytes(16); // 16 bytes for AES block size

        let plaintext = b"Hello, OpenSSL in Rust WDAWDAWD AFWA!adwawdaw";

        let ciphertext = encrypt(plaintext, &key, &iv);
        println!("Ciphertext: {:?}", ciphertext.len());

        let decrypted_plaintext = decrypt(&ciphertext, &key, &iv);
        println!(
            "Decrypted plaintext: {:?}",
            String::from_utf8(decrypted_plaintext.clone()).unwrap()
        );

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
        use openssl::hash::MessageDigest;
        use openssl::nid::Nid;
        use openssl::pkey::PKey;
        use openssl::sign::Signer;
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
    fn tst_signature_3() {
        use openssl::ec::{EcGroup, EcKey};
        use openssl::ecdsa::EcdsaSig;
        use openssl::nid::Nid;
        use openssl::pkey::Private;
        use openssl::sha::sha256;

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
    fn tst_signature_2() {
        use openssl::ec::{EcGroup, EcKey};
        use openssl::ecdsa::EcdsaSig;
        use openssl::error::ErrorStack;
        use openssl::hash::MessageDigest;
        use openssl::nid::Nid;
        use openssl::pkey::PKey;
        use openssl::pkey::Private;
        use openssl::sha::sha256;
        use openssl::x509::X509NameBuilder;
        use openssl::x509::X509Req;
        use openssl::x509::X509ReqBuilder;

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
        use openssl::encrypt::{Decrypter, Encrypter};
        use openssl::pkey::PKey;
        use openssl::rsa::{Padding, Rsa};

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
    fn enumerate() {
        let n = 10; // The total count
        let start = 5; // The starting point
        let end = n; // The endpoint

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
            3,        // total number of original shards
            5,        // total number of recovery shards
            original, // all original shards
        )
        .unwrap();

        let restored = reed_solomon_16::decode(
            3, // total number of original shards
            5, // total number of recovery shards
            [
                // provided original shards with indexes
                (1, &original[1]),
            ],
            [
                // provided recovery shards with indexes
                (1, &recovery[1]),
                (4, &recovery[4]),
            ],
        )
        .unwrap();

        for item in restored.clone() {
            println!("{:?}", item.1.len());
        }

        assert_eq!(restored[&0], original[0]);
        assert_eq!(restored[&2], original[2]);
    }

    #[test]
    fn grid_row_iter() {
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
        let col = postion % matrix_size;
        insert_value(&mut matrix, postion, matrix_size, 10);

        assert_eq!(matrix[row][col], 10);
    }

    #[test]
    fn test_encrypt_decrypt() {
        use openssl::rand::rand_bytes;
        use openssl::symm::{Cipher, Crypter, Mode};

        fn generate_random_bytes(size: usize) -> Vec<u8> {
            let mut buf = vec![0; size];
            rand_bytes(&mut buf).expect("Failed to generate random bytes");
            buf
        }

        fn encrypt(
            cipher: Cipher,
            data: &[u8],
            key: &[u8],
            iv: &[u8],
        ) -> Result<Vec<u8>, openssl::error::ErrorStack> {
            let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
            let mut ciphertext = vec![0; data.len() + cipher.block_size()];
            let mut count = crypter.update(data, &mut ciphertext)?;
            count += crypter.finalize(&mut ciphertext[count..])?;
            ciphertext.truncate(count);
            Ok(ciphertext)
        }

        fn decrypt(
            cipher: Cipher,
            data: &[u8],
            key: &[u8],
            iv: &[u8],
        ) -> Result<Vec<u8>, openssl::error::ErrorStack> {
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
        let iv = generate_random_bytes(16); // 128 bits for AES block size

        let encrypted_data =
            encrypt(Cipher::aes_256_cbc(), data, &key, &iv).expect("Failed to encrypt data");

        println!("encrypted data size: {:?}", encrypted_data.len());

        let decrypted_data = decrypt(Cipher::aes_256_cbc(), &encrypted_data, &key, &iv)
            .expect("Failed to decrypt data");

        assert_eq!(data.to_vec(), decrypted_data);
    }

    #[test]
    fn add_row_fec() {
        // Define the input vectors
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]];

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

        let input = vec![vec![1, 4], vec![2, 5], vec![3, 6]];

        // Transpose the rows, appending columns to the matrix
        let transposed_fec: Vec<Vec<_>> = (0..input[0].len())
            .map(|row_index| input.iter().map(|row| row[row_index].clone()).collect())
            .collect();

        // Print the transposed vector of arrays
        for row in &transposed_fec {
            println!("{:?}", row);
        }

        //assert_eq!(dm, dm1);
    }
}
