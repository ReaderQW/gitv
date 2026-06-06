use serde::{Deserialize, Serialize};

use super::{ChangeRecord, CommitRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRawData {
    pub repo_name: String,
    pub commits: Vec<CommitRecord>,
    pub changes: Vec<ChangeRecord>,
}
