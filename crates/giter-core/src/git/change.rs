use std::path::Path;

use chrono::{TimeZone, Utc};

use crate::error::CoreResult;
use crate::model::ChangeRecord;

pub fn scan_changes(
    repo: &git2::Repository,
    commit: &git2::Commit,
) -> CoreResult<Vec<ChangeRecord>> {
    let commit_tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;

    let commit_hash = commit.id().to_string();
    let commit_time = commit.time();
    let datetime = Utc
        .timestamp_opt(commit_time.seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now);
    let mut changes = Vec::new();

    for (i, delta) in diff.deltas().enumerate() {
        let new_file = delta.new_file();

        if new_file.is_binary() {
            continue;
        }

        let file_path = new_file
            .path()
            .unwrap_or_else(|| Path::new(""))
            .to_string_lossy()
            .to_string();

        let ext = Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        let patch = git2::Patch::from_diff(&diff, i)?;
        let mut insertions = 0u32;
        let mut deletions = 0u32;

        if let Some(patch) = patch {
            for h in 0..patch.num_hunks() {
                for l in 0..patch.num_lines_in_hunk(h)? {
                    let line = patch.line_in_hunk(h, l)?;
                    match line.origin() {
                        '+' => insertions += 1,
                        '-' => deletions += 1,
                        _ => {}
                    }
                }
            }
        }

        changes.push(ChangeRecord {
            commit_hash: commit_hash.clone(),
            file_path,
            ext,
            insertions,
            deletions,
            datetime,
        });
    }

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn init_test_repo() -> (TempDir, PathBuf, git2::Repository) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = dir.path().join("test_repo");

        let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");

        {
            let mut config = repo.config().expect("Failed to get config");
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        let file_path = repo_path.join("main.rs");
        fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}\n")
            .expect("Failed to write file");

        let tree_id = {
            let mut index = repo.index().expect("Failed to get index");
            index.add_path(Path::new("main.rs")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )
            .expect("Failed to create initial commit");
        }

        fs::write(
            &file_path,
            "fn main() {\n    println!(\"hello world\");\n    let x = 1;\n}\n",
        )
        .expect("Failed to write file");

        let tree_id = {
            let mut index = repo.index().expect("Failed to get index");
            index.add_path(Path::new("main.rs")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        {
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
        }

        (dir, repo_path, repo)
    }

    #[test]
    fn test_scan_changes_basic() {
        let (_dir, _repo_path, repo) = init_test_repo();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        let changes = scan_changes(&repo, &head_commit).expect("Failed to scan changes");

        assert_eq!(changes.len(), 1, "Should have one changed file");

        let change = &changes[0];
        assert_eq!(change.file_path, "main.rs");
        assert_eq!(change.ext, "rs");
        assert_eq!(change.commit_hash, head_commit.id().to_string());
        assert!(change.insertions > 0, "Should have insertions");
        assert!(change.deletions > 0, "Should have deletions");
    }

    #[test]
    fn test_scan_changes_initial_commit() {
        let (_dir, _repo_path, repo) = init_test_repo();

        let mut revwalk = repo.revwalk().unwrap();
        revwalk.push_head().unwrap();

        let first_oid = revwalk.last().unwrap().unwrap();
        let first_commit = repo.find_commit(first_oid).unwrap();

        let changes = scan_changes(&repo, &first_commit).expect("Failed to scan changes");
        assert_eq!(changes.len(), 1, "Initial commit should have one file");
        assert_eq!(
            changes[0].deletions, 0,
            "Initial commit should have no deletions"
        );
        assert!(
            changes[0].insertions > 0,
            "Initial commit should have insertions"
        );
    }
}