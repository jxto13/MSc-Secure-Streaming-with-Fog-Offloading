// reinforcement_learning/action.rs

use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Increase,
    Decrease,
    NoChange,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FecDataAction {
    pub fec_row: Direction,
    pub fec_col: Direction,
}
