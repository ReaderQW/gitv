use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application-wide settings persisted to disk as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Recently opened repository paths (most recent first).
    pub recent_repos: Vec<PathBuf>,
    /// Maximum number of recent repos to keep.
    pub max_recent: usize,
    /// Maximum number of commits to scan per repository (0 = unlimited).
    pub max_commits: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            recent_repos: Vec::new(),
            max_recent: 10,
            max_commits: 5000,
        }
    }
}

impl AppSettings {
    /// Load settings from the default config file path.
    ///
    /// Returns [`Default`] when no config file exists or it cannot be read.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to the default config file path.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Add a repo path to the front of the recent list (deduplicated, trimmed).
    pub fn add_recent(&mut self, path: PathBuf) {
        self.recent_repos.retain(|p| p != &path);
        self.recent_repos.insert(0, path);
        self.recent_repos.truncate(self.max_recent);
    }

    /// Compute the config file path.
    ///
    /// Uses `$HOME/.config/giter/settings.json` (Unix) or
    /// `%APPDATA%/giter/settings.json` (Windows).
    fn config_path() -> PathBuf {
        let base = if cfg!(target_os = "windows") {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        } else {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| PathBuf::from("."))
        };
        base.join("giter").join("settings.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_recent_dedup() {
        let mut s = AppSettings::default();
        s.add_recent(PathBuf::from("/a"));
        s.add_recent(PathBuf::from("/b"));
        s.add_recent(PathBuf::from("/a")); // duplicate
        assert_eq!(s.recent_repos.len(), 2);
        assert_eq!(s.recent_repos[0], PathBuf::from("/a"));
    }

    #[test]
    fn test_add_recent_truncate() {
        let mut s = AppSettings {
            max_recent: 3,
            ..Default::default()
        };
        s.add_recent(PathBuf::from("/a"));
        s.add_recent(PathBuf::from("/b"));
        s.add_recent(PathBuf::from("/c"));
        s.add_recent(PathBuf::from("/d"));
        assert_eq!(s.recent_repos.len(), 3);
    }

    #[test]
    fn test_save_roundtrip() {
        // Test serde roundtrip directly.
        let mut s = AppSettings::default();
        s.add_recent(PathBuf::from("/repo1"));
        s.add_recent(PathBuf::from("/repo2"));

        let json = serde_json::to_string(&s).expect("serialize");
        let loaded: AppSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(loaded.recent_repos.len(), 2);
    }
}
