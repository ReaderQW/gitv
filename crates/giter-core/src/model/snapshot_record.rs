use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub commit_hash: String,
    pub file_path: String,
    pub code_lines: u32,
    pub comment_lines: u32,
    pub blank_lines: u32,
}
