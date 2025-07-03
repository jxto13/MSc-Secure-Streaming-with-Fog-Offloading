// reinforcement_learning/state.rs

use serde::{Deserialize, Serialize};

/// Represents the discretized packet loss state
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub loss_bin: u8, // 0: no loss, 1: low loss, ..., N: high loss
}

impl State {
    pub fn from_loss(loss: f32) -> Self {
        let loss_bin = match loss {
            l if l < 1.0 => 0,  // 0–1%: Excellent
            l if l < 5.0 => 1,  // 1–5%: Good
            l if l < 10.0 => 2, // 5–10%: Moderate
            l if l < 20.0 => 3, // 10–20%: Poor
            _ => 4,             // 20%+: Bad
        };

        Self { loss_bin }
    }
}
