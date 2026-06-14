pub mod analysis;
pub mod error;
pub mod export;
pub mod git;
pub mod model;

pub use error::{CoreError, CoreResult};
pub use git::{BranchInfo, CommitInfo, GitRepo};
pub use model::{ChangeRecord, CommitRecord, RepoStats, SnapshotRecord};
