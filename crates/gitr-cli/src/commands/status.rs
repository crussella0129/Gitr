use clap::Args;
use comfy_table::{Cell, Color, Table};
use gitr_core::config::GitrConfig;

#[derive(Args)]
pub struct StatusArgs {
    /// Filter by host label
    #[arg(long)]
    host: Option<String>,
}

pub fn run(args: StatusArgs) -> anyhow::Result<()> {
    let db_path = GitrConfig::db_path()?;
    let conn = gitr_db::open_db(&db_path)?;

    let hosts = if let Some(label) = &args.host {
        let h = gitr_db::ops::get_host_by_label(&conn, &label)?
            .ok_or_else(|| anyhow::anyhow!("Host '{}' not found", label))?;
        vec![h]
    } else {
        gitr_db::ops::list_hosts(&conn)?
    };

    if hosts.is_empty() {
        println!("No hosts registered. Use `gitr host add` to register one.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec![
        "HOST / REPO",
        "BRANCH",
        "BEHIND",
        "AHEAD",
        "STRATEGY",
        "LAST SYNC",
        "STATUS",
    ]);

    let mut total_synced = 0u32;
    let mut total_behind = 0u32;
    let mut total_ahead = 0u32;
    let total_errors = 0u32;

    for host in &hosts {
        let repos = gitr_db::ops::list_repos_for_host(&conn, &host.id)?;

        // Host header row
        table.add_row(vec![
            Cell::new(format!("{} ({})", host.label, host.kind)).fg(Color::Cyan),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
        ]);

        for repo in &repos {
            let snapshots = gitr_db::ops::get_branch_snapshots(&conn, &repo.id)?;
            let default_snap = snapshots.iter().find(|s| s.branch == repo.default_branch);

            let behind = default_snap.map(|s| s.behind_count).unwrap_or(0);
            let ahead = default_snap.map(|s| s.ahead_count).unwrap_or(0);

            let last_sync = repo
                .last_synced_at
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_else(|| "—".to_string());

            let (status_str, status_color) = if !repo.is_fork {
                ("tracked", Color::White)
            } else if behind == 0 && ahead == 0 && repo.last_synced_at.is_some() {
                total_synced += 1;
                ("synced", Color::Green)
            } else if behind > 0 {
                total_behind += 1;
                ("behind", Color::Yellow)
            } else if ahead > 0 {
                total_ahead += 1;
                ("ahead", Color::Blue)
            } else {
                ("unknown", Color::White)
            };

            let strategy = if repo.is_fork { "ff" } else { "—" };

            table.add_row(vec![
                Cell::new(format!("  {}", repo.name)),
                Cell::new(&repo.default_branch),
                Cell::new(behind.to_string()),
                Cell::new(ahead.to_string()),
                Cell::new(strategy),
                Cell::new(&last_sync),
                Cell::new(status_str).fg(status_color),
            ]);
        }
    }

    println!("{table}");
    println!(
        "Summary: {} synced | {} behind | {} ahead | {} errors",
        total_synced, total_behind, total_ahead, total_errors
    );

    Ok(())
}
