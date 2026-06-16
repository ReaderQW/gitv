use serde::{Deserialize, Serialize};

use super::{ChangeRecord, CommitRecord, SnapshotRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRawData {
    pub repo_name: String,
    pub commits: Vec<CommitRecord>,
    pub changes: Vec<ChangeRecord>,
    pub snapshots: Vec<SnapshotRecord>,
}
