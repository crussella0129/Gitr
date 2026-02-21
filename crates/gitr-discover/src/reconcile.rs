use gitr_host::RemoteRepo;

use crate::scanner::ScannedRepo;

/// Classification of a repo during reconciliation.
#[derive(Debug, Clone)]
pub enum RepoMatch {
    /// Found both locally and on the remote host.
    Matched {
        local: ScannedRepo,
        remote: RemoteRepo,
    },
    /// Found locally but not on the remote host.
    LocalOnly(ScannedRepo),
    /// Found on the remote host but not locally.
    RemoteOnly(RemoteRepo),
}

/// Result of reconciling local and remote repos.
#[derive(Debug)]
pub struct ReconcileResult {
    pub host_label: String,
    pub matches: Vec<RepoMatch>,
}

impl ReconcileResult {
    pub fn matched_count(&self) -> usize {
        self.matches
            .iter()
            .filter(|m| matches!(m, RepoMatch::Matched { .. }))
            .count()
    }

    pub fn local_only_count(&self) -> usize {
        self.matches
            .iter()
            .filter(|m| matches!(m, RepoMatch::LocalOnly(_)))
            .count()
    }

    pub fn remote_only_count(&self) -> usize {
        self.matches
            .iter()
            .filter(|m| matches!(m, RepoMatch::RemoteOnly(_)))
            .count()
    }
}

/// Reconcile scanned local repos with remote repos by normalizing URLs.
pub fn reconcile(
    local: &[ScannedRepo],
    remote: &[RemoteRepo],
    host_label: &str,
) -> ReconcileResult {
    let mut matches = Vec::new();
    let mut matched_remote_indices = std::collections::HashSet::new();

    for local_repo in local {
        let mut found = false;
        for (idx, remote_repo) in remote.iter().enumerate() {
            if urls_match(local_repo, remote_repo) {
                matches.push(RepoMatch::Matched {
                    local: local_repo.clone(),
                    remote: remote_repo.clone(),
                });
                matched_remote_indices.insert(idx);
                found = true;
                break;
            }
        }
        if !found {
            matches.push(RepoMatch::LocalOnly(local_repo.clone()));
        }
    }

    for (idx, remote_repo) in remote.iter().enumerate() {
        if !matched_remote_indices.contains(&idx) {
            matches.push(RepoMatch::RemoteOnly(remote_repo.clone()));
        }
    }

    ReconcileResult {
        host_label: host_label.to_string(),
        matches,
    }
}

/// Check if any of the local repo's remote URLs match the remote repo's clone/SSH URL.
fn urls_match(local: &ScannedRepo, remote: &RemoteRepo) -> bool {
    for scanned_remote in &local.remotes {
        let local_normalized = normalize_url(&scanned_remote.url);
        if local_normalized == normalize_url(&remote.clone_url)
            || local_normalized == normalize_url(&remote.ssh_url)
        {
            return true;
        }
    }
    false
}

/// Normalize a git URL for comparison.
/// Strips protocol, trailing .git, and converts SSH to HTTPS-style path.
fn normalize_url(url: &str) -> String {
    let mut s = url.to_lowercase();

    // Strip protocol
    for prefix in &["https://", "http://", "ssh://", "git://"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }

    // Strip user@ (e.g. git@)
    if let Some(at_pos) = s.find('@') {
        // Only strip if @ comes before the first /
        let slash_pos = s.find('/').unwrap_or(s.len());
        if at_pos < slash_pos {
            s = s[at_pos + 1..].to_string();
        }
    }

    // SSH format: host:path → host/path
    if let Some(colon_pos) = s.find(':') {
        if !s[..colon_pos].contains('/') {
            s = format!("{}/{}", &s[..colon_pos], &s[colon_pos + 1..]);
        }
    }

    // Strip trailing .git
    if let Some(stripped) = s.strip_suffix(".git") {
        s = stripped.to_string();
    }

    // Strip trailing slash
    s = s.trim_end_matches('/').to_string();

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://github.com/user/repo.git"),
            "github.com/user/repo"
        );
        assert_eq!(
            normalize_url("git@github.com:user/repo.git"),
            "github.com/user/repo"
        );
        assert_eq!(
            normalize_url("ssh://git@github.com/user/repo"),
            "github.com/user/repo"
        );
    }

    #[test]
    fn test_reconcile_match() {
        let local = vec![ScannedRepo {
            path: "/home/user/repos/myrepo".into(),
            remotes: vec![crate::scanner::ScannedRemote {
                name: "origin".to_string(),
                url: "https://github.com/user/myrepo.git".to_string(),
            }],
        }];
        let remote = vec![RemoteRepo {
            full_name: "user/myrepo".to_string(),
            owner: "user".to_string(),
            name: "myrepo".to_string(),
            clone_url: "https://github.com/user/myrepo.git".to_string(),
            ssh_url: "git@github.com:user/myrepo.git".to_string(),
            default_branch: "main".to_string(),
            is_fork: false,
            upstream_full_name: None,
            upstream_clone_url: None,
            description: None,
            is_private: false,
            is_archived: false,
            updated_at: None,
        }];

        let result = reconcile(&local, &remote, "gh");
        assert_eq!(result.matched_count(), 1);
        assert_eq!(result.local_only_count(), 0);
        assert_eq!(result.remote_only_count(), 0);
    }
}
