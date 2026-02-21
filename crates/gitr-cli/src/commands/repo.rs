use clap::Subcommand;
use gitr_core::config::GitrConfig;

#[derive(Subcommand)]
pub enum RepoAction {
    /// List tracked repos
    List {
        /// Show only forks
        #[arg(long)]
        forks: bool,
        /// Filter by host label
        #[arg(long)]
        host: Option<String>,
    },
    /// Show details of a repo
    Info {
        /// Full name (owner/repo) or repo name
        name: String,
    },
}

pub fn run(action: RepoAction) -> anyhow::Result<()> {
    let db_path = GitrConfig::db_path()?;
    let conn = gitr_db::open_db(&db_path)?;

    match action {
        RepoAction::List { forks, host } => {
            let repos = if forks {
                gitr_db::ops::list_fork_repos(&conn)?
            } else if let Some(label) = host {
                let h = gitr_db::ops::get_host_by_label(&conn, &label)?
                    .ok_or_else(|| anyhow::anyhow!("Host '{}' not found", label))?;
                gitr_db::ops::list_repos_for_host(&conn, &h.id)?
            } else {
                gitr_db::ops::list_repos(&conn)?
            };

            if repos.is_empty() {
                println!("No repos tracked. Use `gitr scan` to discover repos.");
                return Ok(());
            }

            println!(
                "{:<40} {:<8} {:<8} {:<10} {}",
                "REPO", "FORK", "BRANCH", "SOURCE", "LAST SYNC"
            );
            for repo in &repos {
                let fork_str = if repo.is_fork { "yes" } else { "no" };
                let last_sync = repo
                    .last_synced_at
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "never".to_string());
                println!(
                    "{:<40} {:<8} {:<8} {:<10} {}",
                    repo.full_name, fork_str, repo.default_branch, repo.discovery_source, last_sync
                );
            }
            println!("\n{} repos total", repos.len());
            Ok(())
        }
        RepoAction::Info { name } => {
            let repos = gitr_db::ops::list_repos(&conn)?;
            let repo = repos
                .iter()
                .find(|r| r.full_name == name || r.name == name)
                .ok_or_else(|| anyhow::anyhow!("Repo '{}' not found", name))?;

            println!("Full name:       {}", repo.full_name);
            println!("Owner:           {}", repo.owner);
            println!("Name:            {}", repo.name);
            println!("Clone URL:       {}", repo.clone_url);
            println!("Default branch:  {}", repo.default_branch);
            println!("Fork:            {}", repo.is_fork);
            if let Some(ref upstream) = repo.upstream_full_name {
                println!("Upstream:        {}", upstream);
            }
            if let Some(ref path) = repo.local_path {
                println!("Local path:      {}", path.display());
            }
            println!("Discovery:       {}", repo.discovery_source);
            println!(
                "Last synced:     {}",
                repo.last_synced_at
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "never".to_string())
            );

            // Show branch snapshots
            let snapshots = gitr_db::ops::get_branch_snapshots(&conn, &repo.id)?;
            if !snapshots.is_empty() {
                println!("\nBranches:");
                for snap in &snapshots {
                    println!(
                        "  {}: behind={} ahead={}",
                        snap.branch, snap.behind_count, snap.ahead_count
                    );
                }
            }

            Ok(())
        }
    }
}
