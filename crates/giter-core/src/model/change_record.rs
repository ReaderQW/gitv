use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub commit_hash: String,
    pub file_path: String,
    pub ext: String,
    pub insertions: u32,
    pub deletions: u32,
    pub datetime: DateTime<Utc>,
}
