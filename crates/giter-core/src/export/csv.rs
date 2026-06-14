use crate::error::CoreResult;
use serde::Serialize;
use std::path::Path;

/// Write a slice of serializable records to a CSV file.
///
/// Each element in `data` is written as one CSV row.  The first row contains
/// the field names derived from the struct definition.
///
/// # Errors
///
/// Returns [`CoreError::Csv`] if the file cannot be created or a record
/// cannot be serialized.
pub fn to_file<T: Serialize>(data: &[T], path: &Path) -> CoreResult<()> {
    let mut writer = csv::Writer::from_path(path)?;
    for record in data {
        writer.serialize(record)?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CommitRecord;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_commits() -> Vec<CommitRecord> {
        vec![
            CommitRecord {
                repo_name: "test".into(),
                hash: "abc123".into(),
                author_name: "Alice".into(),
                author_email: "alice@example.com".into(),
                datetime: Utc::now(),
                message: "First commit".into(),
            },
            CommitRecord {
                repo_name: "test".into(),
                hash: "def456".into(),
                author_name: "Bob".into(),
                author_email: "bob@example.com".into(),
                datetime: Utc::now(),
                message: "Second commit".into(),
            },
        ]
    }

    #[test]
    fn test_csv_export_empty() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("out.csv");
        let data: Vec<CommitRecord> = vec![];
        to_file(&data, &path).expect("export empty CSV");
        assert!(path.exists(), "file should exist");
        // With no records the CSV writer produces a clean (possibly empty) file
    }

    #[test]
    fn test_csv_export_and_read_back() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("commits.csv");
        let data = make_commits();
        to_file(&data, &path).expect("export CSV");

        let mut reader = csv::Reader::from_path(&path).expect("open CSV");
        let mut count = 0;
        for result in reader.deserialize() {
            let record: CommitRecord = result.expect("deserialize CSV row");
            count += 1;
            assert_eq!(record.repo_name, "test");
        }
        assert_eq!(count, 2);
    }
}
