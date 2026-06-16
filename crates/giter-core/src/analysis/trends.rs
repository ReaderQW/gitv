use chrono::{Datelike, NaiveDate};

use crate::error::{CoreError, CoreResult};
use crate::model::CommitRecord;

/// Group commits by day, returning sorted (date, count) pairs.
pub fn commit_trend_by_day(commits: &[CommitRecord]) -> Vec<(NaiveDate, usize)> {
    let mut counts: std::collections::BTreeMap<NaiveDate, usize> = std::collections::BTreeMap::new();
    for commit in commits {
        let date = commit.datetime.date_naive();
        *counts.entry(date).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

/// Group commits by ISO week (starting Monday), returning sorted (week_start, count) pairs.
pub fn commit_trend_by_week(commits: &[CommitRecord]) -> Vec<(NaiveDate, usize)> {
    let mut counts: std::collections::BTreeMap<NaiveDate, usize> = std::collections::BTreeMap::new();
    for commit in commits {
        let date = commit.datetime.date_naive();
        let dow = date.weekday().num_days_from_monday();
        let week_start = date - chrono::Duration::days(dow as i64);
        *counts.entry(week_start).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

/// Group commits by month, returning sorted (first_of_month, count) pairs.
pub fn commit_trend_by_month(commits: &[CommitRecord]) -> CoreResult<Vec<(NaiveDate, usize)>> {
    let mut counts: std::collections::BTreeMap<(i32, u32), usize> = std::collections::BTreeMap::new();
    for commit in commits {
        let date = commit.datetime.date_naive();
        *counts.entry((date.year(), date.month())).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|((y, m), c)| {
            NaiveDate::from_ymd_opt(y, m, 1)
                .map(|d| (d, c))
                .ok_or(CoreError::InvalidDate(y, m))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CommitRecord;
    use chrono::{DateTime, Utc};

    fn make_commit(days_ago: i64, author: &str) -> CommitRecord {
        let secs = Utc::now().timestamp() - days_ago * 86400;
        let datetime = DateTime::from_timestamp(secs, 0).unwrap();
        CommitRecord {
            repo_name: "test".into(),
            hash: "abc".into(),
            author_name: author.into(),
            author_email: "a@b.com".into(),
            datetime,
            message: "commit".into(),
        }
    }

    #[test]
    fn test_trend_by_day_empty() {
        let result = commit_trend_by_day(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_trend_by_day_single() {
        let commits = vec![make_commit(0, "alice")];
        let result = commit_trend_by_day(&commits);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 1);
    }

    #[test]
    fn test_trend_by_day_multiple_same_day() {
        let commits = vec![
            make_commit(0, "alice"),
            make_commit(0, "bob"),
            make_commit(0, "alice"),
        ];
        let result = commit_trend_by_day(&commits);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 3);
    }

    #[test]
    fn test_trend_by_day_different_days() {
        let commits = vec![make_commit(0, "alice"), make_commit(1, "bob"), make_commit(2, "alice")];
        let result = commit_trend_by_day(&commits);
        assert_eq!(result.len(), 3);
        // should be sorted ascending
        for w in result.windows(2) {
            assert!(w[0].0 <= w[1].0, "days should be sorted");
        }
    }

    #[test]
    fn test_trend_by_week_groups_correctly() {
        // Use large gaps to avoid flakiness from current weekday
        let commits = vec![make_commit(0, "a"), make_commit(8, "b"), make_commit(20, "c")];
        let result = commit_trend_by_week(&commits);
        assert!(result.len() >= 2, "should span at least 2 weeks, got {}", result.len());
    }

    #[test]
    fn test_trend_by_month_empty() {
        let result = commit_trend_by_month(&[]).expect("empty trend");
        assert!(result.is_empty());
    }

    #[test]
    fn test_trend_by_month_groups() {
        let commits = vec![make_commit(0, "a"), make_commit(32, "b")]; // ~1 month apart
        let result = commit_trend_by_month(&commits).expect("month trend");
        assert!(result.len() >= 1);
        // each entry should be the 1st of some month
        for (date, _) in &result {
            assert_eq!(date.day(), 1, "month trend key should be first of month");
        }
    }
}
