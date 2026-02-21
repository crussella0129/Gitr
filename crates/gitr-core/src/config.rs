use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::GitrError;
use crate::models::sync_link::MergeStrategy;

/// Top-level Gitr configuration, stored at `~/.gitr/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitrConfig {
    /// Default merge strategy for fork syncing.
    #[serde(default = "default_merge_strategy")]
    pub default_merge_strategy: MergeStrategy,

    /// Maximum concurrent sync operations.
    #[serde(default = "default_concurrency")]
    pub sync_concurrency: usize,

    /// Default directories to scan for local repos.
    #[serde(default)]
    pub scan_paths: Vec<PathBuf>,

    /// Maximum directory depth for filesystem scanning.
    #[serde(default = "default_max_scan_depth")]
    pub max_scan_depth: usize,
}

fn default_merge_strategy() -> MergeStrategy {
    MergeStrategy::FastForward
}

fn default_concurrency() -> usize {
    8
}

fn default_max_scan_depth() -> usize {
    4
}

impl Default for GitrConfig {
    fn default() -> Self {
        Self {
            default_merge_strategy: MergeStrategy::FastForward,
            sync_concurrency: 8,
            scan_paths: Vec::new(),
            max_scan_depth: 4,
        }
    }
}

impl GitrConfig {
    /// Returns the Gitr home directory (`~/.gitr/`).
    pub fn home_dir() -> Result<PathBuf, GitrError> {
        let base = dirs::home_dir().ok_or_else(|| GitrError::Config {
            message: "could not determine home directory".into(),
        })?;
        Ok(base.join(".gitr"))
    }

    /// Returns the path to the config file.
    pub fn config_path() -> Result<PathBuf, GitrError> {
        Ok(Self::home_dir()?.join("config.toml"))
    }

    /// Returns the path to the database file.
    pub fn db_path() -> Result<PathBuf, GitrError> {
        Ok(Self::home_dir()?.join("gitr.db"))
    }

    /// Load config from the default location, or return defaults if not found.
    pub fn load() -> Result<Self, GitrError> {
        let path = Self::config_path()?;
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> Result<Self, GitrError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| GitrError::Serialization(e.to_string()))
    }

    /// Save config to the default location.
    pub fn save(&self) -> Result<(), GitrError> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<(), GitrError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| GitrError::Serialization(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Initialize the Gitr home directory with default config.
    pub fn init() -> Result<PathBuf, GitrError> {
        let home = Self::home_dir()?;
        std::fs::create_dir_all(&home)?;

        let config_path = Self::config_path()?;
        if !config_path.exists() {
            Self::default().save_to(&config_path)?;
        }

        Ok(home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_roundtrip() {
        let config = GitrConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: GitrConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            config.default_merge_strategy,
            deserialized.default_merge_strategy
        );
        assert_eq!(config.sync_concurrency, deserialized.sync_concurrency);
    }
}
