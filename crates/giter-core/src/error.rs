use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Not a git repository: {0}")]
    NotAGitRepository(PathBuf),

    #[error("JSON error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;
