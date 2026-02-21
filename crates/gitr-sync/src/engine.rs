use std::path::Path;
use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;

use gitr_core::models::repo::Repo;
use gitr_core::models::sync_link::MergeStrategy;

use crate::fork_sync::{sync_fork, ForkSyncResult};

/// Sync engine that runs fork syncs in parallel with a concurrency limit.
pub struct SyncEngine {
    concurrency: usize,
}

impl SyncEngine {
    pub fn new(concurrency: usize) -> Self {
        Self { concurrency }
    }

    /// Sync all forks in parallel. Each repo needs its upstream clone URL.
    pub async fn sync_all_forks(
        &self,
        repos: Vec<(Repo, String)>, // (repo, upstream_clone_url)
        clone_base_dir: &Path,
        strategy: &MergeStrategy,
        dry_run: bool,
    ) -> Vec<ForkSyncResult> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let multi = MultiProgress::new();
        let style = ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");

        let clone_base = clone_base_dir.to_path_buf();
        let strat = strategy.clone();

        let handles: Vec<_> = repos
            .into_iter()
            .map(|(repo, upstream_url)| {
                let sem = semaphore.clone();
                let pb = multi.add(ProgressBar::new_spinner());
                pb.set_style(style.clone());
                pb.set_message(format!("syncing {}", repo.full_name));
                let base = clone_base.clone();
                let s = strat.clone();

                tokio::task::spawn_blocking(move || {
                    let _permit = sem.acquire_owned();
                    let result = sync_fork(&repo, &upstream_url, &base, &s, dry_run);
                    pb.finish_with_message(format!(
                        "{}: {}",
                        result.repo_full_name,
                        result.record.status
                    ));
                    result
                })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        results
    }
}
