use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A repo found on the local filesystem.
#[derive(Debug, Clone)]
pub struct ScannedRepo {
    pub path: PathBuf,
    pub remotes: Vec<ScannedRemote>,
}

/// A git remote parsed from a local repo's config.
#[derive(Debug, Clone)]
pub struct ScannedRemote {
    pub name: String,
    pub url: String,
}

/// Directories to skip during scanning.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    ".git",
    "__pycache__",
    ".venv",
    "venv",
];

/// Scan a directory tree for git repos, up to `max_depth` levels deep.
pub fn scan_directory(root: &Path, max_depth: usize) -> Vec<ScannedRepo> {
    let mut repos = Vec::new();

    let walker = WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if !entry.file_type().is_dir() {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            !SKIP_DIRS.contains(&name.as_ref())
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        let git_dir = entry.path().join(".git");
        if git_dir.is_dir() {
            let config_path = git_dir.join("config");
            if config_path.exists() {
                let remotes = parse_git_config(&config_path);
                repos.push(ScannedRepo {
                    path: entry.path().to_path_buf(),
                    remotes,
                });
            }
        }
    }

    repos
}

/// Parse remote URLs from a .git/config file.
fn parse_git_config(config_path: &Path) -> Vec<ScannedRemote> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut remotes = Vec::new();
    let mut current_remote: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[remote \"") && trimmed.ends_with("\"]") {
            let name = trimmed
                .strip_prefix("[remote \"")
                .and_then(|s| s.strip_suffix("\"]"))
                .unwrap_or("")
                .to_string();
            current_remote = Some(name);
        } else if trimmed.starts_with('[') {
            current_remote = None;
        } else if let Some(ref remote_name) = current_remote {
            if let Some(url) = trimmed.strip_prefix("url = ") {
                remotes.push(ScannedRemote {
                    name: remote_name.clone(),
                    url: url.trim().to_string(),
                });
            }
        }
    }

    remotes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_config() {
        let content = r#"[core]
	repositoryformatversion = 0
	bare = false
[remote "origin"]
	url = https://github.com/user/repo.git
	fetch = +refs/heads/*:refs/remotes/origin/*
[remote "upstream"]
	url = https://github.com/upstream/repo.git
	fetch = +refs/heads/*:refs/remotes/upstream/*
[branch "main"]
	remote = origin
"#;
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config");
        std::fs::write(&config_path, content).unwrap();

        let remotes = parse_git_config(&config_path);
        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert!(remotes[0].url.contains("user/repo"));
        assert_eq!(remotes[1].name, "upstream");
    }
}
