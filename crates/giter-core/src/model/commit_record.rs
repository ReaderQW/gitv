use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    pub repo_name: String,
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub datetime: DateTime<Utc>,
}
