use std::path::Path;

use crate::model::SnapshotRecord;

/// Count snapshots by file extension, returning (extension, count) sorted descending by count.
///
/// Files without an extension use `""` as the key.
pub fn file_type_distribution(snapshots: &[SnapshotRecord]) -> Vec<(String, u64)> {
    let mut counts: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for snapshot in snapshots {
        let ext = Path::new(&snapshot.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        *counts.entry(ext).or_insert(0) += 1;
    }
    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SnapshotRecord;

    fn make_snapshot(path: &str) -> SnapshotRecord {
        SnapshotRecord {
            commit_hash: "abc".into(),
            file_path: path.into(),
            code_lines: 10,
            comment_lines: 2,
            blank_lines: 1,
        }
    }

    #[test]
    fn test_file_types_empty() {
        let result = file_type_distribution(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_file_types_single() {
        let snapshots = vec![make_snapshot("main.rs")];
        let result = file_type_distribution(&snapshots);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ("rs".to_string(), 1));
    }

    #[test]
    fn test_file_types_multiple() {
        let snapshots = vec![
            make_snapshot("main.rs"),
            make_snapshot("lib.rs"),
            make_snapshot("index.html"),
            make_snapshot("style.css"),
            make_snapshot("main.rs"),
        ];
        let result = file_type_distribution(&snapshots);
        assert_eq!(result[0], ("rs".to_string(), 3));
        assert_eq!(result[1], ("css".to_string(), 1));
        assert_eq!(result[2], ("html".to_string(), 1));
    }

    #[test]
    fn test_file_types_no_extension() {
        let snapshots = vec![
            make_snapshot("Makefile"),
            make_snapshot("Dockerfile"),
        ];
        let result = file_type_distribution(&snapshots);
        // no extension -> ""
        for (ext, _) in &result {
            assert_eq!(ext, "", "files without extension should use empty string key");
        }
        assert_eq!(result.iter().map(|(_, c)| c).sum::<u64>(), 2);
    }

    #[test]
    fn test_file_types_case_insensitive() {
        let snapshots = vec![make_snapshot("file.Rs"), make_snapshot("file2.rs")];
        let result = file_type_distribution(&snapshots);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ("rs".to_string(), 2));
    }
}
