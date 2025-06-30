use std::{
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

use crate::CONFIG;
use crate::{metrics::get_metrics, proto::RetransmissionPacket};
use crate::{membership_manager::client_sessions::ClientSessions, MATRIX_BUFFER};
use grid::Grid;
use iris_shared::source_coding::{find_optimal_retransmissions, Transmission};

use crossbeam_channel::Sender;

// Retransmission manager struct
pub struct RetransmissionManager {}

impl RetransmissionManager {
    // Initialize the manager with receivers for multicast and unicast retransmissions
    pub fn new() -> Self {
        Self {}
    }

    //  Convert from Grid to Vec<Vec<>>
    fn grid_to_vec<T: Clone>(grid: &Grid<T>) -> Vec<Vec<T>> {
        let rows = grid.rows();
        let cols = grid.cols();
        
        let mut result = Vec::with_capacity(rows);
        
        for row in 0..rows {
            let mut row_vec = Vec::with_capacity(cols);
            for col in 0..cols {
                row_vec.push(grid.get(row, col).cloned().unwrap());
            }
            result.push(row_vec);
        }
        
        result
    }

    // Run the retransmission handler that listens to both channels
    pub fn handle_retransmissions(
        client_sessions: Arc<ClientSessions>,
        multicast_tx: Sender<Vec<u8>>,
        unicast_tx: Sender<(Vec<u8>, IpAddr)>,
    ) {
        let config = CONFIG.get().unwrap();
        let retransmission_interval = Duration::from_millis(config.retransmission.interval as u64);
        let matrix_buffer = MATRIX_BUFFER.get().unwrap();

        loop {
            // 1. Generate the network coding matrix for all clients
            let nc_matrix = client_sessions.generate_nc_matrix();

            std::thread::sleep(retransmission_interval);
        }
    }
}
