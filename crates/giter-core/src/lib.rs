pub mod error;
pub mod git;
pub mod model;

pub use error::{CoreError, CoreResult};
pub use git::{BranchInfo, CommitInfo, GitRepo};
