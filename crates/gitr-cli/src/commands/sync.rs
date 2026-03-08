use std::sync::Arc;

use clap::Args;
use gitr_auth::{CredentialStore, KeyringStore};
use gitr_core::config::GitrConfig;
use gitr_core::models::sync_link::MergeStrategy;
use gitr_core::models::sync_state::SyncStatus;
use gitr_sync::engine::SyncEngine;
use gitr_sync::fork_sync;
use tokio::task::JoinSet;

#[derive(Args)]
pub struct SyncArgs {
    /// Repo name, full name (owner/repo), or "all" to sync all forks
    target: String,
    /// Dry run — show what would be done without making changes
    #[arg(long)]
    dry_run: bool,
    /// Override merge strategy (ff, merge, rebase)
    #[arg(long)]
    strategy: Option<String>,
    /// Use the host's server-side merge-upstream API instead of local git operations.
    /// Faster for bulk updates; no local clone required.
    #[arg(long)]
    api: bool,
}

pub async fn run(args: SyncArgs) -> anyhow::Result<()> {
    let config = GitrConfig::load()?;
    let db_path = GitrConfig::db_path()?;
    let conn = gitr_db::open_db(&db_path)?;

    let strategy = match args.strategy {
        Some(ref s) => s
            .parse::<MergeStrategy>()
            .map_err(|e| anyhow::anyhow!(e))?,
        None => config.default_merge_strategy.clone(),
    };

    let clone_base = GitrConfig::home_dir()?.join("repos");
    std::fs::create_dir_all(&clone_base)?;

    if args.target == "all" {
        let forks = gitr_db::ops::list_fork_repos(&conn)?;
        if forks.is_empty() {
            println!("No forks tracked. Use `gitr scan` to discover repos.");
            return Ok(());
        }

        println!("Syncing {} forks...", forks.len());

        if args.api {
            // ── API sync path ─────────────────────────────────────────────────
            // Uses GitHub's POST /repos/{owner}/{repo}/merge-upstream endpoint —
            // fully server-side, no local clone needed.
            let cred_store = KeyringStore::new();
            let sem = Arc::new(tokio::sync::Semaphore::new(10));
            let mut join_set: JoinSet<anyhow::Result<bool>> = JoinSet::new();

            for fork in forks {
                let host = match gitr_db::ops::get_host_by_id(&conn, &fork.host_id)? {
                    Some(h) => h,
                    None => {
                        eprintln!("  Skipping {} — host not found", fork.full_name);
                        continue;
                    }
                };
                let token = match cred_store.get(&host.credential_key)? {
                    Some(t) => t,
                    None => {
                        eprintln!("  Skipping {} — no token for host", fork.full_name);
                        continue;
                    }
                };

                if args.dry_run {
                    println!("  [dry-run] would API-sync {}", fork.full_name);
                    continue;
                }

                let sem = sem.clone();
                let owner = fork.owner.clone();
                let name = fork.name.clone();
                let branch = fork.default_branch.clone();
                let api_url = host.api_url.clone();
                let username = host.username.clone();
                let kind = host.kind.clone();

                join_set.spawn(async move {
                    let Ok(_permit) = sem.acquire_owned().await else {
                        anyhow::bail!("semaphore closed");
                    };
                    let provider =
                        gitr_host::create_provider(&kind, &api_url, &token, &username)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let synced = provider
                        .sync_fork_upstream(&owner, &name, &branch)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    Ok(synced)
                });
            }

            let (mut synced, mut skipped, mut failed) = (0u32, 0u32, 0u32);
            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(Ok(true)) => synced += 1,
                    Ok(Ok(false)) => skipped += 1,
                    Ok(Err(e)) => {
                        failed += 1;
                        eprintln!("  error: {e}");
                    }
                    Err(e) => {
                        failed += 1;
                        eprintln!("  task error: {e}");
                    }
                }
            }

            println!("\nAPI sync complete: {synced} synced | {skipped} skipped/diverged | {failed} failed");
            return Ok(());
        }

        // ── Git sync path (local clone) ───────────────────────────────────────
        let cred_store = KeyringStore::new();
        let mut repo_pairs = Vec::new();

        for fork in &forks {
            let upstream_url = match &fork.upstream_clone_url {
                // Fast path: URL already stored in DB from scan
                Some(url) => url.clone(),
                None => match &fork.upstream_full_name {
                    Some(upstream_name) => {
                        // Fall back to an API call to resolve the clone URL
                        let host = gitr_db::ops::get_host_by_id(&conn, &fork.host_id)?;
                        if let Some(host) = host {
                            let token = cred_store.get(&host.credential_key)?;
                            if let Some(token) = token {
                                let parts: Vec<&str> = upstream_name.splitn(2, '/').collect();
                                if parts.len() == 2 {
                                    let provider = gitr_host::create_provider(
                                        &host.kind,
                                        &host.api_url,
                                        &token,
                                        &host.username,
                                    )?;
                                    match provider.get_repo(parts[0], parts[1]).await? {
                                        Some(r) => r.clone_url,
                                        None => format!("https://github.com/{upstream_name}.git"),
                                    }
                                } else {
                                    format!("https://github.com/{upstream_name}.git")
                                }
                            } else {
                                format!("https://github.com/{upstream_name}.git")
                            }
                        } else {
                            format!("https://github.com/{upstream_name}.git")
                        }
                    }
                    None => {
                        println!("  Skipping {} — no upstream known", fork.full_name);
                        continue;
                    }
                },
            };
            repo_pairs.push((fork.clone(), upstream_url));
        }

        let engine = SyncEngine::new(config.sync_concurrency);
        let results = engine
            .sync_all_forks(repo_pairs, &clone_base, &strategy, args.dry_run)
            .await;

        let success = results
            .iter()
            .filter(|r| r.record.status == SyncStatus::Success)
            .count();
        let failed = results
            .iter()
            .filter(|r| r.record.status == SyncStatus::Failed)
            .count();
        let skipped = results
            .iter()
            .filter(|r| r.record.status == SyncStatus::Skipped)
            .count();

        println!("\nSync complete: {success} synced | {failed} failed | {skipped} skipped");

        if !args.dry_run {
            for result in &results {
                gitr_db::ops::insert_sync_record(&conn, &result.record)?;
                if result.record.status == SyncStatus::Success {
                    gitr_db::ops::update_repo_last_synced(
                        &conn,
                        &result.record.repo_id,
                        &result.record.finished_at,
                    )?;
                }
            }
        }

        for result in &results {
            if !result.record.errors.is_empty() {
                println!("\nErrors for {}:", result.repo_full_name);
                for err in &result.record.errors {
                    println!("  {err}");
                }
            }
        }
    } else {
        // ── Single repo sync ──────────────────────────────────────────────────
        let repos = gitr_db::ops::list_repos(&conn)?;
        let repo = repos
            .iter()
            .find(|r| r.full_name == args.target || r.name == args.target)
            .ok_or_else(|| anyhow::anyhow!("Repo '{}' not found", args.target))?;

        if !repo.is_fork {
            anyhow::bail!("{} is not a fork", repo.full_name);
        }

        let upstream_name = repo
            .upstream_full_name
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No upstream known for {}", repo.full_name))?;

        println!("Syncing {} (strategy: {strategy})...", repo.full_name);
        if args.dry_run {
            println!("  (dry run)");
        }

        if args.api {
            // API-based single-repo sync
            let host = gitr_db::ops::get_host_by_id(&conn, &repo.host_id)?
                .ok_or_else(|| anyhow::anyhow!("Host not found for {}", repo.full_name))?;
            let cred_store = KeyringStore::new();
            let token = cred_store
                .get(&host.credential_key)?
                .ok_or_else(|| anyhow::anyhow!("No token for host '{}'", host.label))?;
            let provider =
                gitr_host::create_provider(&host.kind, &host.api_url, &token, &host.username)?;

            if args.dry_run {
                println!("  [dry-run] would API-sync {}", repo.full_name);
                return Ok(());
            }

            let synced = provider
                .sync_fork_upstream(&repo.owner, &repo.name, &repo.default_branch)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            if synced {
                println!("  API-synced {} ← {upstream_name}", repo.full_name);
            } else {
                println!("  Skipped {} — already up-to-date or diverged", repo.full_name);
            }
            return Ok(());
        }

        // Git-based single-repo sync
        let upstream_url = match &repo.upstream_clone_url {
            Some(url) => url.clone(),
            None => format!("https://github.com/{upstream_name}.git"),
        };

        let result =
            fork_sync::sync_fork(repo, &upstream_url, &clone_base, &strategy, args.dry_run);

        match result.record.status {
            SyncStatus::Success => {
                println!(
                    "  Synced: {} commits transferred on {}",
                    result.record.commits_transferred, repo.default_branch
                );
                if !args.dry_run {
                    gitr_db::ops::insert_sync_record(&conn, &result.record)?;
                    gitr_db::ops::update_repo_last_synced(
                        &conn,
                        &result.record.repo_id,
                        &result.record.finished_at,
                    )?;
                }
            }
            SyncStatus::Skipped => {
                println!(
                    "  [dry-run] {} commits behind on {}",
                    result.record.commits_transferred, repo.default_branch
                );
            }
            SyncStatus::Failed => {
                println!("  Failed:");
                for err in &result.record.errors {
                    println!("    {err}");
                }
            }
            SyncStatus::PartialSuccess => {
                println!("  Partial success");
            }
        }
    }

    Ok(())
}
