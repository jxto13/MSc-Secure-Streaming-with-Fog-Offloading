use crate::reinforcement_learning::action::FecDataAction;
use crate::reinforcement_learning::state::State;
use log::info;
use rand::{prelude::*, rng};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::marker::PhantomData;
use std::path::Path;

// Custom serialization for q_table
fn serialize_q_table<S>(
    q_table: &HashMap<(State, FecDataAction), f64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(q_table.len()))?;
    for ((state, action), value) in q_table {
        let key = format!(
            "{}|{}",
            serde_json::to_string(state).unwrap(),
            serde_json::to_string(action).unwrap()
        );
        map.serialize_entry(&key, value)?;
    }
    map.end()
}

fn deserialize_q_table<'de, D>(
    deserializer: D,
) -> Result<HashMap<(State, FecDataAction), f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct QTableVisitor {
        marker: PhantomData<fn() -> HashMap<(State, FecDataAction), f64>>,
    }

    impl<'de> Visitor<'de> for QTableVisitor {
        type Value = HashMap<(State, FecDataAction), f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map from stringified (State, FecDataAction) to f64")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some((key, value)) = access.next_entry::<String, f64>()? {
                let mut parts = key.splitn(2, '|');
                let state_str = parts
                    .next()
                    .ok_or_else(|| de::Error::custom("Missing state"))?;
                let action_str = parts
                    .next()
                    .ok_or_else(|| de::Error::custom("Missing action"))?;
                let state: State = serde_json::from_str(state_str).map_err(de::Error::custom)?;
                let action: FecDataAction =
                    serde_json::from_str(action_str).map_err(de::Error::custom)?;
                map.insert((state, action), value);
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(QTableVisitor {
        marker: PhantomData,
    })
}

#[derive(Serialize, Deserialize)]
pub struct QLearningAgent {
    #[serde(
        serialize_with = "serialize_q_table",
        deserialize_with = "deserialize_q_table"
    )]
    pub q_table: HashMap<(State, FecDataAction), f64>,
    pub actions: Vec<FecDataAction>,
    pub alpha: f64,
    pub gamma: f64,
    pub epsilon: f64,
}

impl QLearningAgent {
    pub fn new(actions: Vec<FecDataAction>, alpha: f64, gamma: f64, epsilon: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            actions,
            alpha,
            gamma,
            epsilon,
        }
    }

    pub fn choose_action(&self, state: State) -> FecDataAction {
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

    pub fn update(&mut self, state: State, action: FecDataAction, reward: f64, next_state: State) {
        let max_next_q = self
            .actions
            .iter()
            .map(|a| self.q_value(&next_state, a))
            .fold(f64::NEG_INFINITY, f64::max);

        let q = self
            .q_table
            .entry((state.clone(), action.clone()))
            .or_insert(0.0);

        *q += self.alpha * (reward + self.gamma * max_next_q - *q);

        info!("Updated Q-value for ({:?}, {:?}) -> {}", state, action, *q);
    }

    fn q_value(&self, state: &State, action: &FecDataAction) -> f64 {
        *self
            .q_table
            .get(&(state.clone(), action.clone()))
            .unwrap_or(&0.0)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tmp_path = format!("{}.tmp", path);
        let file = BufWriter::new(File::create(&tmp_path)?);

        if let Err(e) = serde_json::to_writer(file, self) {
            eprintln!("Error serializing QLearningAgent: {}", e);
            return Err(Box::new(e));
        }

        std::fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn load(
        path: &str,
        alpha: f64,
        gamma: f64,
        epsilon: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let file = BufReader::new(File::open(path)?);
        let mut agent: Self = serde_json::from_reader(file)?;
        agent.alpha = alpha;
        agent.gamma = gamma;
        agent.epsilon = epsilon;
        log::info!("QLearningAgent loaded successfully from {}", path); // <-- Added log
        Ok(agent)
    }

    pub fn load_from_dir(
        dir: &str,
        actions: Vec<FecDataAction>,
        alpha: f64,
        gamma: f64,
        epsilon: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pattern = format!(
            "q_table_lr{:.4}_df{:.4}_er{:.4}.json",
            alpha, gamma, epsilon
        );
        let dir_path = Path::new(dir);
        if dir_path.exists() {
            for entry in std::fs::read_dir(dir_path)? {
                let entry = entry?;
                let fname = entry.file_name().into_string().unwrap_or_default();
                if fname == pattern {
                    let path = entry.path();
                    return Self::load(path.to_str().unwrap(), alpha, gamma, epsilon);
                }
            }
        }
        Ok(Self::new(actions, alpha, gamma, epsilon))
    }
}
