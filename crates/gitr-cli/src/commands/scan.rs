use clap::Args;
use gitr_auth::{CredentialStore, KeyringStore};
use gitr_core::config::GitrConfig;
use gitr_core::models::repo::{DiscoverySource, Repo};
use gitr_discover::reconcile::RepoMatch;

#[derive(Args)]
pub struct ScanArgs {
    /// Directory to scan for local repos
    #[arg(long)]
    path: Option<String>,
    /// Only scan for a specific host
    #[arg(long)]
    host: Option<String>,
}

pub async fn run(args: ScanArgs) -> anyhow::Result<()> {
    let config = GitrConfig::load()?;
    let db_path = GitrConfig::db_path()?;
    let conn = gitr_db::open_db(&db_path)?;

    // Determine scan paths
    let scan_paths: Vec<std::path::PathBuf> = match args.path {
        Some(p) => vec![p.into()],
        None => {
            if config.scan_paths.is_empty() {
                println!("No scan paths configured. Use --path or add scan_paths to config.");
                return Ok(());
            }
            config.scan_paths.clone()
        }
    };

    // Determine which hosts to scan
    let hosts = if let Some(label) = &args.host {
        let host = gitr_db::ops::get_host_by_label(&conn, label)?
            .ok_or_else(|| anyhow::anyhow!("Host '{}' not found", label))?;
        vec![host]
    } else {
        gitr_db::ops::list_hosts(&conn)?
    };

    if hosts.is_empty() {
        println!("No hosts registered. Use `gitr host add` to register one.");
        return Ok(());
    }

    let cred_store = KeyringStore::new();

    for host in &hosts {
        println!("\nScanning host: {} ({})", host.label, host.kind);

        let token = cred_store
            .get(&host.credential_key)?
            .ok_or_else(|| anyhow::anyhow!("No token for host '{}'", host.label))?;

        let provider =
            gitr_host::create_provider(&host.kind, &host.api_url, &token, &host.username)?;

        let result =
            gitr_discover::discover(host, provider.as_ref(), &scan_paths, config.max_scan_depth)
                .await?;

        // Print reconciliation results
        println!(
            "  Matched: {}  |  Local-only: {}  |  Remote-only: {}",
            result.matched_count(),
            result.local_only_count(),
            result.remote_only_count()
        );

        // Track remote-only repos in DB
        let mut tracked = 0u32;
        for m in &result.matches {
            match m {
                RepoMatch::RemoteOnly(remote) => {
                    // Check if already tracked
                    if gitr_db::ops::get_repo_by_full_name(&conn, &host.id, &remote.full_name)?
                        .is_some()
                    {
                        continue;
                    }

                    let mut repo = Repo::new(
                        remote.full_name.clone(),
                        host.id.clone(),
                        remote.clone_url.clone(),
                        remote.default_branch.clone(),
                        DiscoverySource::Api,
                    );
                    repo.is_fork = remote.is_fork;
                    repo.upstream_full_name = remote.upstream_full_name.clone();

                    gitr_db::ops::insert_repo(&conn, &repo)?;
                    tracked += 1;
                }
                RepoMatch::Matched { local, remote } => {
                    // Upsert — ensure tracked with local path
                    if gitr_db::ops::get_repo_by_full_name(&conn, &host.id, &remote.full_name)?
                        .is_none()
                    {
                        let mut repo = Repo::new(
                            remote.full_name.clone(),
                            host.id.clone(),
                            remote.clone_url.clone(),
                            remote.default_branch.clone(),
                            DiscoverySource::Filesystem,
                        );
                        repo.is_fork = remote.is_fork;
                        repo.upstream_full_name = remote.upstream_full_name.clone();
                        repo.local_path = Some(local.path.clone());
                        gitr_db::ops::insert_repo(&conn, &repo)?;
                        tracked += 1;
                    }
                }
                RepoMatch::LocalOnly(_) => {}
            }
        }

        if tracked > 0 {
            println!("  Tracked {tracked} new repos.");
        }
    }

    let total = gitr_db::ops::list_repos(&conn)?.len();
    println!("\nTotal tracked repos: {total}");

    Ok(())
}
