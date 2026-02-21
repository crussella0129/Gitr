use clap::Args;
use gitr_core::config::GitrConfig;

#[derive(Args)]
pub struct HistoryArgs {
    /// Filter by repo name or full name
    repo: Option<String>,
    /// Number of records to show
    #[arg(long, default_value = "20")]
    limit: u32,
}

pub fn run(args: HistoryArgs) -> anyhow::Result<()> {
    let db_path = GitrConfig::db_path()?;
    let conn = gitr_db::open_db(&db_path)?;

    let repo_id = if let Some(ref name) = args.repo {
        let repos = gitr_db::ops::list_repos(&conn)?;
        let repo = repos
            .iter()
            .find(|r| r.full_name == *name || r.name == *name)
            .ok_or_else(|| anyhow::anyhow!("Repo '{}' not found", name))?;
        Some(repo.id.clone())
    } else {
        None
    };

    let records =
        gitr_db::ops::list_sync_history(&conn, repo_id.as_ref(), args.limit)?;

    if records.is_empty() {
        println!("No sync history found.");
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<8} {:<8} {:<8} {}",
        "STARTED", "STATUS", "SYNCED", "FAILED", "COMMITS", "ERRORS"
    );
    for record in &records {
        let errors_str = if record.errors.is_empty() {
            "—".to_string()
        } else {
            format!("{} error(s)", record.errors.len())
        };
        println!(
            "{:<20} {:<10} {:<8} {:<8} {:<8} {}",
            record.started_at.format("%Y-%m-%d %H:%M:%S"),
            record.status,
            record.branches_synced,
            record.branches_failed,
            record.commits_transferred,
            errors_str,
        );
    }

    Ok(())
}
