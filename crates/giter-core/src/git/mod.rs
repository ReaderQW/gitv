pub mod change;
pub mod commit;
pub mod scanner;
pub mod snapshot;

use crate::error::CoreResult;
use chrono::{DateTime, FixedOffset, Utc};
use git2::{Repository, Time};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub datetime: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
}

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn open(path: &str) -> CoreResult<Self> {
        let repo = Repository::open(path)?;
        Ok(Self { repo })
    }

    pub fn init(path: &str) -> CoreResult<Self> {
        let repo = Repository::init(path)?;
        Ok(Self { repo })
    }

    pub fn branches(&self) -> CoreResult<Vec<BranchInfo>> {
        let mut branches = Vec::new();
        for branch in self.repo.branches(None)? {
            let (branch, branch_type) = branch?;
            let name = branch.name()?.unwrap_or("unknown").to_string();
            let is_head = branch.is_head();
            let is_remote = match branch_type {
                git2::BranchType::Remote => true,
                _ => false,
            };
            branches.push(BranchInfo {
                name,
                is_head,
                is_remote,
            });
        }
        Ok(branches)
    }

    pub fn commits(&self, max_count: Option<usize>) -> CoreResult<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if let Some(max) = max_count {
                if i >= max {
                    break;
                }
            }
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let time = commit.time();
            let datetime = git_time_to_chrono(&time)
                .unwrap_or_else(|| Utc::now().into())
                .to_rfc3339();

            let author = commit.author();
            let author_name = author.name().unwrap_or("unknown").to_string();
            let author_email = author.email().unwrap_or("unknown").to_string();
            let message = commit.message().unwrap_or("").trim().to_string();

            commits.push(CommitInfo {
                hash: oid.to_string(),
                author_name,
                author_email,
                datetime,
                message,
            });
        }
        Ok(commits)
    }

    pub fn head_commit(&self) -> CoreResult<Option<CommitInfo>> {
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(None),
        };
        let commit = head.peel_to_commit()?;
        let time = commit.time();
        let datetime = git_time_to_chrono(&time)
            .unwrap_or_else(|| Utc::now().into())
            .to_rfc3339();

        let author = commit.author();
        let author_name = author.name().unwrap_or("unknown").to_string();
        let author_email = author.email().unwrap_or("unknown").to_string();
        let message = commit.message().unwrap_or("").trim().to_string();

        Ok(Some(CommitInfo {
            hash: commit.id().to_string(),
            author_name,
            author_email,
            datetime,
            message,
        }))
    }
}

fn git_time_to_chrono(time: &Time) -> Option<DateTime<FixedOffset>> {
    let offset = FixedOffset::east_opt(time.offset_minutes() * 60)?;
    let datetime = DateTime::from_timestamp(time.seconds(), 0)?.naive_utc();
    Some(DateTime::<FixedOffset>::from_naive_utc_and_offset(
        datetime, offset,
    ))
}