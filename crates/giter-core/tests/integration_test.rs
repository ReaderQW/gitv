use std::fs;
use std::path::{Path, PathBuf};

use giter_core::export;
use giter_core::git::scanner;
use giter_core::model::{ChangeRecord, CommitRecord, RepoRawData};
use tempfile::TempDir;

/// Create a temporary git repository with two commits and return (TempDir, repo_path).
fn init_test_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = dir.path().join("test_repo");

    let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");

    {
        let mut config = repo.config().expect("Failed to get config");
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }

    // --- First commit ---
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
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "First commit", &tree, &[])
            .expect("Failed to create first commit");
    }

    // --- Second commit (modify file) ---
    fs::write(
        &file_path,
        "fn main() {\n    let x = 1;\n    println!(\"hello\");\n    println!(\"{}\", x);\n}\n",
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
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Second commit", &tree, &[&parent])
            .expect("Failed to create second commit");
    }

    (dir, repo_path)
}

#[test]
fn test_scan_then_export_csv_commits() {
    let (_dir, repo_path) = init_test_repo();
    let RepoRawData { commits, .. } =
        scanner::scan_repository(&repo_path, None, &|_, _| {}).expect("scan repository");

    let csv_path = repo_path.parent().unwrap().join("commits.csv");
    export::csv::to_file(&commits, &csv_path).expect("export commits CSV");

    let mut reader = csv::Reader::from_path(&csv_path).expect("open CSV");
    let mut count = 0;
    for result in reader.deserialize() {
        let record: CommitRecord = result.expect("deserialize CSV row");
        count += 1;
        assert!(!record.hash.is_empty());
        assert_eq!(record.author_name, "Test User");
    }
    assert_eq!(count, 2, "should have 2 commit records");
}

#[test]
fn test_scan_then_export_csv_changes() {
    let (_dir, repo_path) = init_test_repo();
    let RepoRawData { changes, .. } =
        scanner::scan_repository(&repo_path, None, &|_, _| {}).expect("scan repository");

    let csv_path = repo_path.parent().unwrap().join("changes.csv");
    export::csv::to_file(&changes, &csv_path).expect("export changes CSV");

    let mut reader = csv::Reader::from_path(&csv_path).expect("open CSV");
    let mut count = 0;
    for result in reader.deserialize() {
        let record: ChangeRecord = result.expect("deserialize CSV row");
        count += 1;
        assert_eq!(record.file_path, "main.rs");
        assert_eq!(record.ext, "rs");
    }
    // Initial commit: 1 addition file (main.rs with 3 lines)
    // Second commit: 1 modified file
    assert!(count >= 1, "should have at least 1 change");
}

#[test]
fn test_scan_then_export_json() {
    let (_dir, repo_path) = init_test_repo();
    let raw = scanner::scan_repository(&repo_path, None, &|_, _| {}).expect("scan repository");

    let json_path = repo_path.parent().unwrap().join("repo.json");
    export::json::to_file(&raw, &json_path).expect("export JSON");

    let content = std::fs::read_to_string(&json_path).expect("read JSON");
    let decoded: RepoRawData = serde_json::from_str(&content).expect("deserialize JSON");

    assert_eq!(decoded.repo_name, "test_repo");
    assert_eq!(decoded.commits.len(), 2);
    assert!(!decoded.changes.is_empty());
}

#[test]
fn test_scan_then_export_to_both_formats() {
    let (_dir, repo_path) = init_test_repo();
    let raw = scanner::scan_repository(&repo_path, None, &|_, _| {}).expect("scan repository");

    let out_dir = repo_path.parent().unwrap();

    // Export commits to CSV
    export::csv::to_file(&raw.commits, &out_dir.join("commits.csv")).expect("export commits CSV");

    // Export changes to CSV
    export::csv::to_file(&raw.changes, &out_dir.join("changes.csv")).expect("export changes CSV");

    // Export everything to JSON
    export::json::to_file(&raw, &out_dir.join("repo.json")).expect("export repo JSON");

    // Read back and cross-validate counts
    let csv_commits = csv::Reader::from_path(&out_dir.join("commits.csv")).expect("open CSV");
    assert_eq!(csv_commits.into_records().count(), 2);

    let csv_changes = csv::Reader::from_path(&out_dir.join("changes.csv")).expect("open CSV");
    assert!(csv_changes.into_records().count() >= 1);

    let json_content = std::fs::read_to_string(&out_dir.join("repo.json")).expect("read JSON");
    let decoded: RepoRawData = serde_json::from_str(&json_content).expect("deserialize JSON");
    assert_eq!(decoded.commits.len(), 2);
}

/// Verify that exporting an empty slice produces a valid (possibly empty) CSV file.
#[test]
fn test_csv_export_empty() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("empty.csv");
    let data: Vec<CommitRecord> = vec![];
    export::csv::to_file(&data, &path).expect("export empty CSV");
    assert!(path.exists());
}
