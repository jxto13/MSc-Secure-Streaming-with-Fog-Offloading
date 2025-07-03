use std::collections::HashMap;

use rurel::mdp::{Agent, State};
use rurel::strategy::explore::{ExplorationStrategy, RandomExploration};
use rurel::strategy::learn::QLearning;
use rurel::strategy::terminate::FixedIterations;
use rurel::AgentTrainer;

use crate::membership_manager::client_sessions::ClientSessions;
use crate::DYNAMIC_VALUES;

const MAX_FEC_ROWS: i32 = 5;
const MIN_FEC_ROWS: i32 = 0;
const MAX_FEC_COLS: i32 = 5;
const MIN_FEC_COLS: i32 = 0;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct MyState {
    fec_rows: i32,
    fec_cols: i32,
    packet_loss_bin: u8, // Discrete packet loss bin (e.g. 0,1,2,3)
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum MyAction {
    IncreaseRows,
    DecreaseRows,
    IncreaseCols,
    DecreaseCols,
    NoChange,
}
impl State for MyState {
    type A = MyAction;

    fn reward(&self) -> f64 {
        let fec_total = self.fec_rows + self.fec_cols;

        match self.packet_loss_bin {
            0 => {
                // No loss: reward low FEC, penalize high FEC
                if fec_total <= 1 {
                    10.0
                } else {
                    10.0 - (fec_total as f64 * 3.0)
                }
            }
            1 => {
                // Low loss: optimal FEC around 2-3
                let optimal = 2.5;
                let distance = (fec_total as f64 - optimal).abs();
                5.0 - distance
            }
            2 => {
                // Medium loss: optimal FEC around 4-5
                let optimal = 4.5;
                let distance = (fec_total as f64 - optimal).abs();
                3.0 - distance * 0.5
            }
            3 => {
                // High loss: reward higher FEC
                let optimal = 7.0;
                let distance = (fec_total as f64 - optimal).abs();
                1.0 - distance * 0.3
            }
            _ => {
                // Very high loss: reward maximum FEC
                let optimal = 10.0;
                let distance = (fec_total as f64 - optimal).abs();
                -distance * 0.2
            }
        }
    }

    fn actions(&self) -> Vec<MyAction> {
        vec![
            MyAction::IncreaseRows,
            MyAction::DecreaseRows,
            MyAction::IncreaseCols,
            MyAction::DecreaseCols,
            MyAction::NoChange,
        ]
    }
}
struct MyAgent {
    state: MyState,
}

impl Agent<MyState> for MyAgent {
    fn current_state(&self) -> &MyState {
        &self.state
    }

    fn take_action(&mut self, action: &MyAction) {
        let dynamic_values = DYNAMIC_VALUES.get().unwrap();

        let mut dynamic_values_lock = dynamic_values.write().unwrap();

        match action {
            MyAction::IncreaseRows => {
                // Increase rows, but do not exceed max
                self.state.fec_rows = (self.state.fec_rows + 1).min(MAX_FEC_ROWS);
            }
            MyAction::DecreaseRows => {
                self.state.fec_rows = (self.state.fec_rows - 1).max(MIN_FEC_ROWS);
            }
            MyAction::IncreaseCols => {
                self.state.fec_cols = (self.state.fec_cols + 1).min(MAX_FEC_COLS);
            }
            MyAction::DecreaseCols => {
                self.state.fec_cols = (self.state.fec_cols - 1).max(MIN_FEC_COLS);
            }
            MyAction::NoChange => {
                // no update
            }
        }

        // Update dynamic values
        dynamic_values_lock.fec_row_redundancy = self.state.fec_rows as usize;
        dynamic_values_lock.fec_col_redundancy = self.state.fec_cols as usize;
    }
}

struct OnlineQTrainer<S: State> {
    q: HashMap<S, HashMap<S::A, f64>>,
    alpha: f64,
    gamma: f64,
}

impl<S: State> OnlineQTrainer<S> {
    fn new(alpha: f64, gamma: f64) -> Self {
        Self {
            q: HashMap::new(),
            alpha,
            gamma,
        }
    }

    fn get_q(&self, state: &S, action: &S::A) -> f64 {
        self.q
            .get(state)
            .and_then(|m| m.get(action))
            .copied()
            .unwrap_or(0.0)
    }

    fn update(&mut self, s: &S, a: &S::A, reward: f64, s_next: &S, actions_next: &[S::A]) {
        let max_q_next = actions_next
            .iter()
            .map(|an| self.get_q(s_next, an))
            .fold(f64::NEG_INFINITY, f64::max);

        let q_sa = self.get_q(s, a);
        let new_q = q_sa + self.alpha * (reward + self.gamma * max_q_next - q_sa);

        self.q
            .entry(s.clone())
            .or_default()
            .insert(a.clone(), new_q);
    }

    fn best_action(&self, state: &S, actions: &[S::A]) -> Option<S::A> {
        actions
            .iter()
            .max_by(|a, b| {
                self.get_q(state, a)
                    .partial_cmp(&self.get_q(state, b))
                    .unwrap()
            })
            .cloned()
    }
}

// Replace your start() function with this:
// pub fn start(client_sessions: &ClientSessions) {
//     let mut agent = MyAgent {
//         state: MyState {
//             fec_rows: 0,
//             fec_cols: 0,
//             packet_loss_bin: 0,
//         },
//     };

//     let mut trainer = OnlineQTrainer::new(0.2, 0.95);
//     let mut explorer = RandomExploration::new();

//     loop {
//         let packet_loss = client_sessions.calculate_packet_loss();
//         let packet_loss_bin = match packet_loss {
//             loss if loss < 1.0 => 0,
//             loss if loss < 5.0 => 1,
//             loss if loss < 10.0 => 2,
//             loss if loss < 20.0 && loss > 30.0 => 3,
//             _ => 4,
//         };

//         // Update agent's observed state
//         agent.state.packet_loss_bin = packet_loss_bin as u8;
//         let state = agent.current_state().clone();

//         // Choose action (epsilon-greedy: random or best)
//         let actions = state.actions();
//         let action = if rand::random::<f64>() < 0.1 {
//             // 10% random
//             explorer.pick_action(&mut agent)
//         } else {
//             // 90% greedy
//             trainer.best_action(&state, &actions).unwrap_or_else(|| explorer.pick_action(&mut agent))
//         };

//         // Take action
//         agent.take_action(&action);

//         // Observe new state and reward
//         let next_state = agent.current_state().clone();
//         let reward = next_state.reward();
//         let next_actions = next_state.actions();

//         // Q-learning update
//         trainer.update(&state, &action, reward, &next_state, &next_actions);

//         // Optionally, print or log
//         println!(
//             "State: {:?}, Action: {:?}, Reward: {}, Next: {:?}",
//             state, action, reward, next_state
//         );

//         // Optionally, sleep or break
//         std::thread::sleep(std::time::Duration::from_millis(1000));
//     }
// }

pub fn start(client_sessions: &ClientSessions) {
    let mut agent = MyAgent {
        state: MyState {
            fec_rows: 0,
            fec_cols: 0,
            packet_loss_bin: 0,
        },
    };

    let mut trainer = OnlineQTrainer::new(0.1, 0.9); // Lower learning rate for stability
    let mut explorer = RandomExploration::new();

    loop {
        // Get current state first
        let current_state = agent.current_state().clone();
        let actions = current_state.actions();

        // Choose action based on current state
        let action = if rand::random::<f64>() < 0.2 {
            // Increased exploration
            explorer.pick_action(&mut agent)
        } else {
            trainer
                .best_action(&current_state, &actions)
                .unwrap_or_else(|| explorer.pick_action(&mut agent))
        };

        // Take the action
        agent.take_action(&action);

        // Now observe the environment and update packet loss
        let packet_loss = client_sessions.calculate_packet_loss();
        let packet_loss_bin = match packet_loss {
            loss if loss < 1.0 => 0,
            loss if loss < 5.0 => 1,
            loss if loss < 10.0 => 2,
            loss if loss < 20.0 && loss > 30.0 => 3,
            _ => 4,
        };

        // Update the environment observation
        agent.state.packet_loss_bin = packet_loss_bin as u8;

        // Get next state and calculate reward
        let next_state = agent.current_state().clone();
        let reward = next_state.reward();
        let next_actions = next_state.actions();

        // Q-learning update with proper state transition
        trainer.update(&current_state, &action, reward, &next_state, &next_actions);

        println!(
            "Loss: {:.2}% | Prev: {:?} | Action: {:?} | Reward: {:.2} | Current: {:?}",
            packet_loss, current_state, action, reward, next_state
        );

        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
