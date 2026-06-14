use giter_core::model::RepoStats;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The state of a scan for a particular repository.
///
/// Serialized as `{"state": "Completed", "data": {...}}` so the frontend
/// can access `result.state` and `result.data` directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", content = "data")]
pub enum ScanState {
    NotScanned,
    Scanning,
    Completed(RepoStats),
    Failed(String),
}

/// Thread-safe store mapping repo paths to their scan states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultStore {
    results: HashMap<String, ScanState>,
}

impl ScanResultStore {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    /// Get the current scan state for a repo path.
    pub fn get(&self, path: &Path) -> &ScanState {
        self.results
            .get(&path.to_string_lossy().to_string())
            .unwrap_or(&ScanState::NotScanned)
    }

    /// Set the scan state for a repo path.
    pub fn set_state(&mut self, path: &str, state: ScanState) {
        self.results.insert(path.to_string(), state);
    }

    /// Remove a repo and its scan result.
    pub fn remove(&mut self, path: &Path) {
        self.results.remove(&path.to_string_lossy().to_string());
    }

    /// Return all (path, state) entries.
    pub fn all(&self) -> Vec<(String, ScanState)> {
        self.results
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Return whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

impl Default for ScanResultStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_stats() -> RepoStats {
        RepoStats {
            repo_path: "/dummy".into(),
            repo_name: "dummy".into(),
            commits: vec![],
            changes: vec![],
            snapshots: vec![],
        }
    }

    #[test]
    fn test_default_is_not_scanned() {
        let store = ScanResultStore::new();
        let p = Path::new("/nonexistent");
        assert!(matches!(store.get(p), ScanState::NotScanned));
    }

    #[test]
    fn test_set_and_get() {
        let mut store = ScanResultStore::new();
        let path = "/repo".to_string();
        store.set_state(&path, ScanState::Scanning);
        assert!(matches!(store.get(Path::new(&path)), ScanState::Scanning));
    }

    #[test]
    fn test_completed_state() {
        let mut store = ScanResultStore::new();
        let path = "/repo".to_string();
        let stats = dummy_stats();
        store.set_state(&path, ScanState::Completed(stats));
        let p = Path::new(&path);
        match store.get(p) {
            ScanState::Completed(s) => assert_eq!(s.repo_name, "dummy"),
            _ => panic!("expected Completed"),
        }
    }

    #[test]
    fn test_failed_state() {
        let mut store = ScanResultStore::new();
        let path = "/repo".to_string();
        store.set_state(&path, ScanState::Failed("error".into()));
        let p = Path::new(&path);
        match store.get(p) {
            ScanState::Failed(msg) => assert_eq!(msg, "error"),
            _ => panic!("expected Failed"),
        }
    }

    #[test]
    fn test_serialization_format_not_scanned() {
        let json = serde_json::to_value(ScanState::NotScanned).expect("serialize");
        assert_eq!(json, serde_json::json!({"state": "NotScanned"}));
    }

    #[test]
    fn test_serialization_format_completed() {
        let state = ScanState::Completed(dummy_stats());
        let json = serde_json::to_value(state).expect("serialize");
        assert_eq!(json["state"], "Completed");
        assert_eq!(json["data"]["repo_name"], "dummy");
    }

    #[test]
    fn test_serialization_format_failed() {
        let state = ScanState::Failed("oops".into());
        let json = serde_json::to_value(state).expect("serialize");
        assert_eq!(json["state"], "Failed");
        assert_eq!(json["data"], "oops");
    }

    #[test]
    fn test_serialization_roundtrip_all_states() {
        let states: Vec<ScanState> = vec![
            ScanState::NotScanned,
            ScanState::Scanning,
            ScanState::Completed(dummy_stats()),
            ScanState::Failed("err".into()),
        ];
        for original in states {
            let json = serde_json::to_value(&original).expect("serialize");
            let decoded: ScanState = serde_json::from_value(json).expect("deserialize");
            match (&original, &decoded) {
                (ScanState::NotScanned, ScanState::NotScanned) => {}
                (ScanState::Scanning, ScanState::Scanning) => {}
                (ScanState::Completed(a), ScanState::Completed(b)) => assert_eq!(a.repo_name, b.repo_name),
                (ScanState::Failed(a), ScanState::Failed(b)) => assert_eq!(a, b),
                _ => panic!("mismatch: {:?} vs {:?}", original, decoded),
            }
        }
    }

    #[test]
    fn test_remove() {
        let mut store = ScanResultStore::new();
        let path = "/repo".to_string();
        store.set_state(&path, ScanState::Scanning);
        store.remove(Path::new(&path));
        assert!(store.is_empty());
    }
}
