use std::path::Path;

use chrono::Utc;
use gitr_core::error::GitrError;
use gitr_core::models::repo::Repo;
use gitr_core::models::sync_link::MergeStrategy;
use gitr_core::models::sync_state::{SyncRecord, SyncStatus};

use crate::git_ops;

/// Result of syncing a single fork.
#[derive(Debug)]
pub struct ForkSyncResult {
    pub repo_full_name: String,
    pub record: SyncRecord,
    pub dry_run: bool,
}

/// Sync a fork with its upstream.
///
/// Flow:
/// 1. Ensure local clone exists (clone if not)
/// 2. Add upstream remote if missing
/// 3. Fetch upstream
/// 4. Checkout default branch
/// 5. Apply merge strategy
/// 6. Push to origin
/// 7. Return SyncRecord
pub fn sync_fork(
    repo: &Repo,
    upstream_clone_url: &str,
    clone_base_dir: &Path,
    strategy: &MergeStrategy,
    dry_run: bool,
) -> ForkSyncResult {
    let started_at = Utc::now();
    let mut record = SyncRecord::new(repo.id.clone());
    record.started_at = started_at;

    let result = sync_fork_inner(repo, upstream_clone_url, clone_base_dir, strategy, dry_run);

    record.finished_at = Utc::now();

    match result {
        Ok(commits) => {
            record.branches_synced = 1;
            record.commits_transferred = commits;
            record.status = if dry_run {
                SyncStatus::Skipped
            } else {
                SyncStatus::Success
            };
        }
        Err(e) => {
            record.branches_failed = 1;
            record.status = SyncStatus::Failed;
            record.errors.push(e.to_string());
        }
    }

    ForkSyncResult {
        repo_full_name: repo.full_name.clone(),
        record,
        dry_run,
    }
}

fn sync_fork_inner(
    repo: &Repo,
    upstream_clone_url: &str,
    clone_base_dir: &Path,
    strategy: &MergeStrategy,
    dry_run: bool,
) -> Result<u32, GitrError> {
    // Determine local path
    let local_path = match &repo.local_path {
        Some(p) => p.clone(),
        None => clone_base_dir.join(&repo.name),
    };

    // 1. Clone if needed
    if !local_path.join(".git").exists() {
        if dry_run {
            tracing::info!("[dry-run] would clone {} to {}", repo.clone_url, local_path.display());
            return Ok(0);
        }
        tracing::info!("cloning {} to {}", repo.clone_url, local_path.display());
        git_ops::clone(&repo.clone_url, &local_path)?;
    }

    // 2. Add upstream remote if missing
    let remotes = git_ops::remote_list(&local_path)?;
    if !remotes.iter().any(|r| r == "upstream") {
        if dry_run {
            tracing::info!("[dry-run] would add upstream remote: {upstream_clone_url}");
        } else {
            git_ops::remote_add(&local_path, "upstream", upstream_clone_url)?;
        }
    }

    // 3. Fetch upstream
    if !dry_run {
        git_ops::fetch(&local_path, "upstream")?;
    }

    // 4. Check behind count
    let branch = &repo.default_branch;
    let upstream_ref = format!("upstream/{branch}");

    if dry_run {
        // For dry-run, try to get the behind count if we have the refs
        let behind = git_ops::rev_list_count(&local_path, branch, &upstream_ref).unwrap_or(0);
        tracing::info!(
            "[dry-run] {}: {behind} commits behind upstream on {branch}",
            repo.full_name
        );
        return Ok(behind);
    }

    let behind = git_ops::rev_list_count(&local_path, branch, &upstream_ref)?;
    if behind == 0 {
        tracing::info!("{}: already up to date on {branch}", repo.full_name);
        return Ok(0);
    }

    tracing::info!(
        "{}: {behind} commits behind upstream on {branch}, syncing with strategy {}",
        repo.full_name,
        strategy
    );

    // 5. Checkout default branch
    git_ops::checkout(&local_path, branch)?;

    // 6. Apply strategy
    match strategy {
        MergeStrategy::FastForward => git_ops::merge_ff(&local_path, &upstream_ref)?,
        MergeStrategy::Merge => git_ops::merge(&local_path, &upstream_ref)?,
        MergeStrategy::Rebase => git_ops::rebase(&local_path, &upstream_ref)?,
        MergeStrategy::ForcePush => {
            // Reset to upstream and force push
            git_ops::checkout(&local_path, branch)?;
            let out = std::process::Command::new("git")
                .args(["reset", "--hard", &upstream_ref])
                .current_dir(&local_path)
                .output()
                .map_err(|e| GitrError::GitError {
                    message: format!("git reset failed: {e}"),
                })?;
            if !out.status.success() {
                return Err(GitrError::GitError {
                    message: format!(
                        "git reset --hard failed: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    ),
                });
            }
        }
    }

    // 7. Push to origin
    git_ops::push(&local_path, "origin", branch)?;

    Ok(behind)
}
