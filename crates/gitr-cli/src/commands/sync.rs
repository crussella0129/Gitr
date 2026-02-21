use clap::Args;
use gitr_auth::{CredentialStore, KeyringStore};
use gitr_core::config::GitrConfig;
use gitr_core::models::sync_link::MergeStrategy;
use gitr_core::models::sync_state::SyncStatus;
use gitr_sync::engine::SyncEngine;
use gitr_sync::fork_sync;

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
        // Sync all forks
        let forks = gitr_db::ops::list_fork_repos(&conn)?;
        if forks.is_empty() {
            println!("No forks tracked. Use `gitr scan` to discover repos.");
            return Ok(());
        }

        println!("Syncing {} forks...", forks.len());

        // Build (repo, upstream_url) pairs
        let cred_store = KeyringStore::new();
        let mut repo_pairs = Vec::new();

        for fork in &forks {
            let upstream_url = match &fork.upstream_full_name {
                Some(upstream_name) => {
                    // Try to get upstream clone URL from the API
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
                                    None => {
                                        format!("https://github.com/{upstream_name}.git")
                                    }
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
            };
            repo_pairs.push((fork.clone(), upstream_url));
        }

        let engine = SyncEngine::new(config.sync_concurrency);
        let results = engine
            .sync_all_forks(repo_pairs, &clone_base, &strategy, args.dry_run)
            .await;

        // Print summary
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

        // Record results in DB
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

        // Print errors
        for result in &results {
            if !result.record.errors.is_empty() {
                println!("\nErrors for {}:", result.repo_full_name);
                for err in &result.record.errors {
                    println!("  {err}");
                }
            }
        }
    } else {
        // Sync a single repo
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

        // Get upstream URL
        let upstream_url = format!("https://github.com/{upstream_name}.git");

        println!("Syncing {} (strategy: {strategy})...", repo.full_name);
        if args.dry_run {
            println!("  (dry run)");
        }

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
