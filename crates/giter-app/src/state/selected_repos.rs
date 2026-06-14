use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A managed collection of user-selected repository paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedRepos {
    repos: Vec<PathBuf>,
}

impl SelectedRepos {
    pub fn new() -> Self {
        Self { repos: Vec::new() }
    }

    /// Add a path (ignored if already present).
    pub fn add(&mut self, path: PathBuf) {
        if !self.repos.contains(&path) {
            self.repos.push(path);
        }
    }

    /// Remove a path by reference.
    pub fn remove(&mut self, path: &Path) {
        self.repos.retain(|p| p != path);
    }

    /// Check whether a path is already selected.
    pub fn contains(&self, path: &Path) -> bool {
        self.repos.iter().any(|p| p == path)
    }

    /// Return all selected paths.
    pub fn all(&self) -> &[PathBuf] {
        &self.repos
    }

    /// Return whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.repos.is_empty()
    }

    /// Return the number of selected repos.
    pub fn len(&self) -> usize {
        self.repos.len()
    }
}

impl Default for SelectedRepos {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_contains() {
        let mut r = SelectedRepos::new();
        let p = PathBuf::from("/repo");
        r.add(p.clone());
        assert!(r.contains(&p));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_add_duplicate() {
        let mut r = SelectedRepos::new();
        let p = PathBuf::from("/repo");
        r.add(p.clone());
        r.add(p.clone());
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut r = SelectedRepos::new();
        let p = PathBuf::from("/repo");
        r.add(p.clone());
        r.remove(&p);
        assert!(!r.contains(&p));
        assert!(r.is_empty());
    }

    #[test]
    fn test_empty() {
        let r = SelectedRepos::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }
}
