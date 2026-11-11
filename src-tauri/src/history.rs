//! Recent dictations, stored only on this device.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LIMIT: usize = 200;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub text: String,
    /// Unix time in milliseconds.
    pub ts: i64,
    pub duration: f32,
}

pub struct History {
    path: PathBuf,
    pub items: Vec<Item>,
}

impl History {
    pub fn load(path: PathBuf) -> Self {
        let items = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, items }
    }

    pub fn add(&mut self, item: Item) {
        self.items.insert(0, item);
        self.items.truncate(LIMIT);
        self.persist();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.persist();
    }

    fn persist(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(&self.items) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}
