use std::collections::HashMap;
use std::fs;
use rand::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct State {
    pub loss_bin: u8,
    pub multicast_retx_bin: u8,
    pub unicast_retx_bin: u8,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Action {
    pub redundancy: u8,
    pub block_size: u8,
}

#[derive(Serialize, Deserialize)]
pub struct QLearningAgent {
    pub q_table: HashMap<(State, Action), f64>,
    pub alpha: f64,
    pub gamma: f64,
    pub epsilon: f64,
    pub actions: Vec<Action>,
}

impl QLearningAgent {
    pub fn new(alpha: f64, gamma: f64, epsilon: f64, actions: Vec<Action>) -> Self {
        Self {
            q_table: HashMap::new(),
            alpha,
            gamma,
            epsilon,
            actions,
        }
    }

    pub fn choose_action(&self, state: State) -> Action {
        if rand::random::<f64>() < self.epsilon {
            *self.actions.choose(&mut rand::thread_rng()).unwrap()
        } else {
            self.actions.iter()
                .max_by(|a, b| {
                    self.q_value(state, **a)
                        .partial_cmp(&self.q_value(state, **b))
                        .unwrap()
                })
                .copied()
                .unwrap()
        }
    }

    fn q_value(&self, state: State, action: Action) -> f64 {
        *self.q_table.get(&(state, action)).unwrap_or(&0.0)
    }

    pub fn update(&mut self, state: State, action: Action, reward: f64, next_state: State) {
        let max_q_next = self.actions.iter()
            .map(|a| self.q_value(next_state, *a))
            .fold(f64::NEG_INFINITY, f64::max);

        let old_q = self.q_value(state, action);
        let new_q = old_q + self.alpha * (reward + self.gamma * max_q_next - old_q);
        self.q_table.insert((state, action), new_q);
    }

    pub fn save(&self, path: &str) {
        let serialized = serde_json::to_string(&self).expect("Serialization failed");
        fs::write(path, serialized).expect("Failed to write Q-table");
    }

    pub fn load(path: &str, actions: Vec<Action>, alpha: f64, gamma: f64, epsilon: f64) -> Self {
        if let Ok(data) = fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or(Self::new(alpha, gamma, epsilon, actions))
        } else {
            Self::new(alpha, gamma, epsilon, actions)
        }
    }
}
