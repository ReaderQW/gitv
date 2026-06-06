use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use git2::Repository;

use crate::error::{CoreError, CoreResult};
use crate::model::CommitRecord;

pub fn scan_commits(repo_path: &Path) -> CoreResult<Vec<CommitRecord>> {
    let repo = Repository::open(repo_path)
        .map_err(|_| CoreError::NotAGitRepository(repo_path.to_path_buf()))?;

    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();
        let time = commit.time();
        let datetime = git_time_to_utc(&time);

        commits.push(CommitRecord {
            repo_name: repo_name.clone(),
            hash: oid.to_string(),
            author_name: author.name().unwrap_or("unknown").to_string(),
            author_email: author.email().unwrap_or("unknown").to_string(),
            datetime,
        });
    }

    Ok(commits)
}

fn git_time_to_utc(time: &git2::Time) -> DateTime<Utc> {
    Utc.timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn init_test_repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = dir.path().join("test_repo");

        let repo = Repository::init(&repo_path).expect("Failed to init repo");

        {
            let mut config = repo.config().expect("Failed to get config");
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        let file_path = repo_path.join("README.md");
        fs::write(&file_path, "# Hello\n").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "First commit",
            &tree,
            &[],
        )
        .expect("Failed to create first commit");

        fs::write(&file_path, "# Hello\n\nWorld\n").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
        let parent_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Second commit",
            &tree,
            &[&parent_commit],
        )
        .expect("Failed to create second commit");

        (dir, repo_path)
    }

    #[test]
    fn test_scan_commits_returns_correct_count() {
        let (_dir, repo_path) = init_test_repo();
        let commits = scan_commits(&repo_path).expect("Failed to scan commits");
        assert_eq!(commits.len(), 2, "Should have exactly 2 commits");
    }

    #[test]
    fn test_scan_commits_fields() {
        let (_dir, repo_path) = init_test_repo();
        let commits = scan_commits(&repo_path).expect("Failed to scan commits");

        assert_eq!(commits.len(), 2);

        for commit in &commits {
            assert_eq!(commit.hash.len(), 40, "Hash should be 40 hex chars");
            assert!(
                commit.hash.chars().all(|c| c.is_ascii_hexdigit()),
                "Hash should be hex"
            );
            assert_eq!(commit.author_name, "Test User");
            assert_eq!(commit.author_email, "test@example.com");
            assert_eq!(commit.repo_name, "test_repo");
        }
    }

    #[test]
    fn test_scan_commits_not_a_git_repo() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let result = scan_commits(dir.path());
        assert!(result.is_err());
        match result {
            Err(CoreError::NotAGitRepository(_)) => {}
            _ => panic!("Expected NotAGitRepository error"),
        }
    }
}
