use serde::{Deserialize, Serialize};

use super::{ChangeRecord, CommitRecord, SnapshotRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStats {
    pub repo_path: String,
    pub repo_name: String,
    pub commits: Vec<CommitRecord>,
    pub changes: Vec<ChangeRecord>,
    pub snapshots: Vec<SnapshotRecord>,
}
