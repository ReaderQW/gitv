use chrono::NaiveDate;

use crate::model::ChangeRecord;

/// Group code churn (insertions, deletions) by day.
///
/// Returns sorted `(date, insertions, deletions)` tuples aggregated per day.
pub fn code_churn_over_time(changes: &[ChangeRecord]) -> Vec<(NaiveDate, i32, i32)> {
    let mut churn: std::collections::BTreeMap<NaiveDate, (i32, i32)> =
        std::collections::BTreeMap::new();
    for change in changes {
        let date = change.datetime.date_naive();
        let entry = churn.entry(date).or_insert((0, 0));
        entry.0 += change.insertions as i32;
        entry.1 += change.deletions as i32;
    }
    churn
        .into_iter()
        .map(|(date, (ins, del))| (date, ins, del))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ChangeRecord;
    use chrono::{DateTime, Utc};

    fn make_change(days_ago: i64, insertions: u32, deletions: u32) -> ChangeRecord {
        let secs = Utc::now().timestamp() - days_ago * 86400;
        ChangeRecord {
            commit_hash: "abc".into(),
            file_path: "test.rs".into(),
            ext: "rs".into(),
            insertions,
            deletions,
            datetime: DateTime::from_timestamp(secs, 0).unwrap(),
        }
    }

    #[test]
    fn test_churn_empty() {
        let result = code_churn_over_time(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_churn_single() {
        let changes = vec![make_change(0, 10, 2)];
        let result = code_churn_over_time(&changes);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 10);
        assert_eq!(result[0].2, 2);
    }

    #[test]
    fn test_churn_aggregates_same_day() {
        let changes = vec![
            make_change(0, 10, 2),
            make_change(0, 5, 1),
            make_change(1, 20, 3),
        ];
        let result = code_churn_over_time(&changes);
        assert_eq!(result.len(), 2);

        // result sorted ascending by date: [yesterday, today]
        // yesterday should have 20 insertions, 3 deletions
        assert_eq!(result[0].1, 20);
        assert_eq!(result[0].2, 3);
        // today should have 15 insertions, 3 deletions
        assert_eq!(result[1].1, 15);
        assert_eq!(result[1].2, 3);
    }

    #[test]
    fn test_churn_sorted_by_date() {
        let changes = vec![make_change(5, 1, 0), make_change(1, 2, 0), make_change(10, 3, 0)];
        let result = code_churn_over_time(&changes);
        for w in result.windows(2) {
            assert!(w[0].0 <= w[1].0, "dates should be sorted ascending");
        }
    }
}
