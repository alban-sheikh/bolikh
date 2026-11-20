//! Recent dictations and lifetime totals, stored only on this device.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LIMIT: usize = 1000;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub text: String,
    /// Unix time in milliseconds.
    pub ts: i64,
    pub duration: f32,
}

/// Running totals for the Home page. Kept apart from the list so that clearing history,
/// automatic deletion and the list's size limit don't reset them. Holds counts only, no text.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Totals {
    pub words: u64,
    pub dictations: u64,
    pub seconds: f64,
}

pub struct History {
    path: PathBuf,
    totals_path: PathBuf,
    pub items: Vec<Item>,
    pub totals: Totals,
}

fn words(text: &str) -> u64 {
    text.split_whitespace().count() as u64
}

impl History {
    pub fn load(path: PathBuf) -> Self {
        let items: Vec<Item> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let totals_path = path.with_file_name("totals.json");
        // First run with totals: start from what the list already holds.
        let totals = std::fs::read_to_string(&totals_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| Totals {
                words: items.iter().map(|i| words(&i.text)).sum(),
                dictations: items.len() as u64,
                seconds: items.iter().map(|i| i.duration as f64).sum(),
            });
        Self { path, totals_path, items, totals }
    }

    /// Counts a dictation in the totals and, when `keep` is set, adds it to the list.
    pub fn add(&mut self, item: Item, keep: bool) {
        self.totals.words += words(&item.text);
        self.totals.dictations += 1;
        self.totals.seconds += item.duration as f64;
        if keep {
            self.items.insert(0, item);
            self.items.truncate(LIMIT);
        }
        self.persist();
    }

    /// Removes dictations older than `days` (0 keeps everything). Returns whether any were removed.
    pub fn prune(&mut self, days: u32, now_ms: i64) -> bool {
        if days == 0 {
            return false;
        }
        let cutoff = now_ms - days as i64 * 86_400_000;
        let before = self.items.len();
        self.items.retain(|i| i.ts >= cutoff);
        let removed = self.items.len() != before;
        if removed {
            self.persist();
        }
        removed
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
        if let Ok(json) = serde_json::to_string(&self.totals) {
            let _ = std::fs::write(&self.totals_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_survive_clearing_and_pruning() {
        let dir = std::env::temp_dir().join(format!("bolikh-history-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = History::load(dir.join("history.json"));
        let day = 86_400_000;
        h.add(Item { text: "one two three".into(), ts: 10 * day, duration: 2.0 }, true);
        h.add(Item { text: "four five".into(), ts: 20 * day, duration: 1.0 }, true);
        h.add(Item { text: "not kept".into(), ts: 20 * day, duration: 1.0 }, false);
        assert_eq!(h.items.len(), 2);
        assert_eq!(h.totals, Totals { words: 7, dictations: 3, seconds: 4.0 });

        assert!(h.prune(7, 21 * day));
        assert_eq!(h.items.len(), 1);
        assert!(!h.prune(0, 999 * day));
        h.clear();
        assert_eq!(History::load(dir.join("history.json")).totals.words, 7);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
