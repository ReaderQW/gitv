use crate::error::CoreResult;
use serde::Serialize;
use std::path::Path;

/// Write a serializable value to a pretty-printed JSON file.
///
/// # Errors
///
/// Returns [`CoreError::Io`] if the file cannot be created, or
/// [`CoreError::Serde`] if serialization fails.
pub fn to_file<T: Serialize>(data: &T, path: &Path) -> CoreResult<()> {
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CommitRecord;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn test_json_export_empty_array() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("out.json");
        let data: Vec<CommitRecord> = vec![];
        to_file(&data, &path).expect("export empty JSON");
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).expect("read JSON");
        assert_eq!(content.trim(), "[]");
    }

    #[test]
    fn test_json_export_and_read_back() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("commits.json");
        let data = vec![CommitRecord {
            repo_name: "test".into(),
            hash: "abc123".into(),
            author_name: "Alice".into(),
            author_email: "alice@example.com".into(),
            datetime: Utc::now(),
            message: "Initial commit".into(),
        }];
        to_file(&data, &path).expect("export JSON");

        let content = std::fs::read_to_string(&path).expect("read JSON");
        let decoded: Vec<CommitRecord> = serde_json::from_str(&content).expect("deserialize JSON");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].author_name, "Alice");
    }
}
