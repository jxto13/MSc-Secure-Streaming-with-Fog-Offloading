use crate::membership_manager::client_sessions::ClientSessions;
use crate::reinforcement_learning::action::{Direction, FecDataAction};
use crate::reinforcement_learning::agent::QLearningAgent;
use crate::reinforcement_learning::state::State;
use crate::{CONFIG, DYNAMIC_VALUES};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, RwLock};

pub fn run_learning_loop(client_sessions: Arc<ClientSessions>) {
    let directions = [
        Direction::Increase,
        Direction::Decrease,
        Direction::NoChange,
    ];

    let mut actions = Vec::new();
    let dynamic_values = DYNAMIC_VALUES.get().unwrap();
    let config = CONFIG.get().unwrap();

    let data_row = config.multicast.row;
    let data_col = config.multicast.col;

    let current = dynamic_values.read().unwrap();
    let mut current_fec_row = current.fec_row_redundancy;
    let mut current_fec_col = current.fec_col_redundancy;
    drop(current);

    for &fec_row_dir in &directions {
        for &fec_col_dir in &directions {
            let new_fec_row = adjust_size(current_fec_row, fec_row_dir);
            let new_fec_col = adjust_size(current_fec_col, fec_col_dir);
            let overhead_ratio = (new_fec_row + new_fec_col) as f64 / (data_row * data_col) as f64;
            if overhead_ratio <= 0.4 {
                actions.push(FecDataAction {
                    fec_row: fec_row_dir,
                    fec_col: fec_col_dir,
                });
            }
        }
    }

    let learning_rate = config.reinforcement_learning.learning_rate;
    let discount_factor = config.reinforcement_learning.discount_factor;
    let exploration_rate = config.reinforcement_learning.exploration_rate;
    let agent_data_path = config.reinforcement_learning.agent_data_path.clone();

    std::fs::create_dir_all(&agent_data_path).ok();

    let mut agent = QLearningAgent::load_from_dir(
        &agent_data_path,
        actions.clone(),
        learning_rate,
        discount_factor,
        exploration_rate,
    )
    .unwrap();

    // Filename for the Q-learning table based on parameters
    let filename = format!(
        "q_table_lr{:.4}_df{:.4}_er{:.4}.json",
        learning_rate, discount_factor, exploration_rate
    );
    let table_path = std::path::Path::new(&agent_data_path).join(&filename);

    loop {
        let state = client_sessions.get_session_state();
        let action = agent.choose_action(state.clone());

        let reward = measure_reward(
            &state,
            &action,
            current_fec_row,
            current_fec_col,
            data_row,
            data_col,
        );

        let new_fec_row = adjust_size(current_fec_row, action.fec_row);
        let new_fec_col = adjust_size(current_fec_col, action.fec_col);

        {
            let mut dynamic_values = DYNAMIC_VALUES.get().unwrap().write().unwrap();
            dynamic_values.fec_row_redundancy = new_fec_row;
            dynamic_values.fec_col_redundancy = new_fec_col;
        }

        // log::debug!(
        //     "State: {:?}, Action: {:?}, Reward: {}",
        //     state,
        //     action,
        //     reward
        // );

        std::thread::sleep(std::time::Duration::from_secs(2));

        let next_state = client_sessions.get_session_state();
        agent.update(state, action, reward, next_state);

        // Update local current state
        current_fec_row = new_fec_row;
        current_fec_col = new_fec_col;

        if let Err(e) = agent.save(table_path.to_str().unwrap()) {
            eprintln!("Failed to save Q-learning table: {}", e);
        }
    }
}

fn adjust_size(size: usize, dir: Direction) -> usize {
    match dir {
        Direction::Increase => size + 1,
        Direction::Decrease => size.saturating_sub(1),
        Direction::NoChange => size,
    }
}

fn measure_reward(
    state: &State,
    action: &FecDataAction,
    current_fec_row: usize,
    current_fec_col: usize,
    data_row: usize,
    data_col: usize,
) -> f64 {
    let new_fec_row = adjust_size(current_fec_row, action.fec_row);
    let new_fec_col = adjust_size(current_fec_col, action.fec_col);
    let overhead_ratio = (new_fec_row + new_fec_col) as f64 / (data_row * data_col) as f64;
    let cost = overhead_ratio * 10.0;

    -1.5 * (state.loss_bin as f64) - cost
}
