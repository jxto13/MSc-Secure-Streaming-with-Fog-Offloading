// reinforcement_learning/agent.rs

use std::collections::HashMap;
use log::info;
use rand::{prelude::*, rng};
use serde::{Serialize, Deserialize};
use std::fs;
use std::hash::Hash;

#[derive(Serialize, Deserialize)]
pub struct QLearningAgent<S, A> where S: Eq + Hash + Clone, A: Eq + Hash + Clone {
    q_table: HashMap<(S, A), f64>,
    actions: Vec<A>,
    alpha: f64,
    gamma: f64,
    epsilon: f64,
}

impl<S, A> QLearningAgent<S, A>
where
    S: Eq + Hash + Clone,
    A: Eq + Hash + Clone + Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(actions: Vec<A>, alpha: f64, gamma: f64, epsilon: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            actions,
            alpha,
            gamma,
            epsilon,
        }
    }

    pub fn choose_action(&self, state: S) -> A {
        let mut rng = rng();

        if rng.random::<f64>() < self.epsilon {
            info!("Choosing random action");
            self.actions.choose(&mut rng).unwrap().clone()
        } else {
            info!("Choosing best action");
            self.actions
                .iter()
                .max_by(|a, b| {
                    self.q_value(&state, a)
                        .partial_cmp(&self.q_value(&state, b))
                        .unwrap()
                })
                .unwrap()
                .clone()
        }
    }

    pub fn update(&mut self, state: S, action: A, reward: f64, next_state: S) {
        let max_next_q = self
            .actions
            .iter()
            .map(|a| self.q_value(&next_state, a))
            .fold(f64::NEG_INFINITY, f64::max);

        let q = self.q_table.entry((state.clone(), action.clone())).or_insert(0.0);
        *q += self.alpha * (reward + self.gamma * max_next_q - *q);
    }

    fn q_value(&self, state: &S, action: &A) -> f64 {
        *self.q_table.get(&(state.clone(), action.clone())).unwrap_or(&0.0)
    }

}
