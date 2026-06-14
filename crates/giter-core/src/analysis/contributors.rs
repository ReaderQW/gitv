use crate::model::CommitRecord;

/// Group commits by author name, returning (name, count) sorted descending by count.
pub fn contributor_commits(commits: &[CommitRecord]) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for commit in commits {
        *counts.entry(commit.author_name.clone()).or_insert(0) += 1;
    }
    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

/// Group commits by email domain, returning (domain, count) sorted descending by count.
pub fn contributor_commits_by_email_domain(commits: &[CommitRecord]) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for commit in commits {
        let domain = commit
            .author_email
            .split('@')
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(domain).or_insert(0) += 1;
    }
    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CommitRecord;
    use chrono::Utc;

    fn make_commit(name: &str, email: &str) -> CommitRecord {
        CommitRecord {
            repo_name: "test".into(),
            hash: "abc".into(),
            author_name: name.into(),
            author_email: email.into(),
            datetime: Utc::now(),
            message: "commit".into(),
        }
    }

    #[test]
    fn test_contributor_commits_empty() {
        let result = contributor_commits(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_contributor_commits_single() {
        let commits = vec![make_commit("alice", "a@b.com")];
        let result = contributor_commits(&commits);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ("alice".to_string(), 1));
    }

    #[test]
    fn test_contributor_commits_multi() {
        let commits = vec![
            make_commit("alice", "a@b.com"),
            make_commit("bob", "b@c.com"),
            make_commit("alice", "a@b.com"),
        ];
        let result = contributor_commits(&commits);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("alice".to_string(), 2));
        assert_eq!(result[1], ("bob".to_string(), 1));
    }

    #[test]
    fn test_email_domain_empty() {
        let result = contributor_commits_by_email_domain(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_email_domain_groups() {
        let commits = vec![
            make_commit("alice", "a@gmail.com"),
            make_commit("bob", "b@gmail.com"),
            make_commit("carol", "carol@company.com"),
        ];
        let result = contributor_commits_by_email_domain(&commits);
        assert_eq!(result.len(), 2);
        for (domain, count) in &result {
            match domain.as_str() {
                "gmail.com" => assert_eq!(*count, 2),
                "company.com" => assert_eq!(*count, 1),
                _ => panic!("unexpected domain: {}", domain),
            }
        }
    }
}
