use std::path::Path;

use git2::Repository;

use crate::error::{CoreError, CoreResult};
use crate::git::change::scan_changes;
use crate::git::commit::scan_commits;
use crate::git::snapshot::scan_snapshot;
use crate::model::RepoRawData;

pub fn scan_repository(repo_path: &Path) -> CoreResult<RepoRawData> {
    let commits = scan_commits(repo_path)?;

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

    let mut all_changes = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let changes = scan_changes(&repo, &commit)?;
        all_changes.extend(changes);
    }

    // Snapshot the HEAD tree for language distribution analysis
    let snapshots = scan_snapshot(&repo).unwrap_or_default();

    Ok(RepoRawData {
        repo_name,
        commits,
        changes: all_changes,
        snapshots,
    })
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

        let file_path = repo_path.join("main.rs");
        fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}\n")
            .expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(Path::new("main.rs")).unwrap();
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

        fs::write(
            &file_path,
            "fn main() {\n    let x = 1;\n    println!(\"hello\");\n    println!(\"{}\", x);\n}\n",
        )
        .expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(Path::new("main.rs")).unwrap();
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
    fn test_scan_repository_commits_count() {
        let (_dir, repo_path) = init_test_repo();
        let data = scan_repository(&repo_path).expect("Failed to scan repository");

        assert_eq!(data.repo_name, "test_repo");
        assert_eq!(data.commits.len(), 2, "Should have 2 commits");
    }

    #[test]
    fn test_scan_repository_changes_count() {
        let (_dir, repo_path) = init_test_repo();
        let data = scan_repository(&repo_path).expect("Failed to scan repository");

        assert!(
            !data.changes.is_empty(),
            "Should have changes from the second commit"
        );

        for change in &data.changes {
            assert!(
                !change.commit_hash.is_empty(),
                "Commit hash should not be empty"
            );
            assert!(
                !change.file_path.is_empty(),
                "File path should not be empty"
            );
        }
    }

    #[test]
    fn test_scan_repository_not_a_git_repo() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let result = scan_repository(dir.path());
        assert!(result.is_err());
        match result {
            Err(CoreError::NotAGitRepository(_)) => {}
            _ => panic!("Expected NotAGitRepository error"),
        }
    }
}
