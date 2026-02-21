use clap::Subcommand;
use gitr_auth::{CredentialStore, KeyringStore};
use gitr_core::config::GitrConfig;
use gitr_core::models::host::{Host, HostKind};

#[derive(Subcommand)]
pub enum HostAction {
    /// Register a new hosting service
    Add {
        /// Label for this host (e.g. "gh", "work-gl")
        name: String,
        /// Provider type
        #[arg(long)]
        provider: String,
        /// Username on the host
        #[arg(long)]
        user: String,
        /// API token (will prompt if not provided)
        #[arg(long)]
        token: Option<String>,
    },
    /// List registered hosts
    List,
    /// Show details of a host
    Info {
        /// Host label
        name: String,
    },
    /// Verify credentials for a host
    Verify {
        /// Host label
        name: String,
    },
    /// Remove a registered host
    Remove {
        /// Host label
        name: String,
    },
}

pub async fn run(action: HostAction) -> anyhow::Result<()> {
    match action {
        HostAction::Add {
            name,
            provider,
            user,
            token,
        } => {
            let db_path = GitrConfig::db_path()?;
            let conn = gitr_db::open_db(&db_path)?;

            // Check if host already exists
            if gitr_db::ops::get_host_by_label(&conn, &name)?.is_some() {
                anyhow::bail!("Host '{}' already exists", name);
            }

            let kind: HostKind = provider
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;

            // Get token
            let token = match token {
                Some(t) => t,
                None => {
                    eprint!("Enter API token for {name}: ");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };

            if token.is_empty() {
                anyhow::bail!("Token cannot be empty");
            }

            let host = Host::new(name.clone(), kind, user);

            // Store token in keychain
            let cred_store = KeyringStore::new();
            cred_store.store(&host.credential_key, &token)?;

            // Save host to DB
            gitr_db::ops::insert_host(&conn, &host)?;

            println!("Host '{}' added ({}, user: {})", name, host.kind, host.username);
            println!("Token stored in OS keychain as '{}'", host.credential_key);
            Ok(())
        }
        HostAction::List => {
            let db_path = GitrConfig::db_path()?;
            let conn = gitr_db::open_db(&db_path)?;
            let hosts = gitr_db::ops::list_hosts(&conn)?;

            if hosts.is_empty() {
                println!("No hosts registered. Use `gitr host add` to register one.");
                return Ok(());
            }

            println!("{:<12} {:<10} {:<20} {}", "LABEL", "PROVIDER", "USERNAME", "API URL");
            for host in &hosts {
                println!(
                    "{:<12} {:<10} {:<20} {}",
                    host.label, host.kind, host.username, host.api_url
                );
            }
            Ok(())
        }
        HostAction::Info { name } => {
            let db_path = GitrConfig::db_path()?;
            let conn = gitr_db::open_db(&db_path)?;
            let host = gitr_db::ops::get_host_by_label(&conn, &name)?
                .ok_or_else(|| anyhow::anyhow!("Host '{}' not found", name))?;

            println!("Label:          {}", host.label);
            println!("Provider:       {}", host.kind);
            println!("Username:       {}", host.username);
            println!("API URL:        {}", host.api_url);
            println!("Credential key: {}", host.credential_key);

            let repos = gitr_db::ops::list_repos_for_host(&conn, &host.id)?;
            println!("Tracked repos:  {}", repos.len());
            let forks = repos.iter().filter(|r| r.is_fork).count();
            println!("  Forks:        {}", forks);
            Ok(())
        }
        HostAction::Verify { name } => {
            let db_path = GitrConfig::db_path()?;
            let conn = gitr_db::open_db(&db_path)?;
            let host = gitr_db::ops::get_host_by_label(&conn, &name)?
                .ok_or_else(|| anyhow::anyhow!("Host '{}' not found", name))?;

            let cred_store = KeyringStore::new();
            let token = cred_store
                .get(&host.credential_key)?
                .ok_or_else(|| anyhow::anyhow!("No token found in keychain for '{}'", name))?;

            let provider = gitr_host::create_provider(&host.kind, &host.api_url, &token, &host.username)?;
            let valid = provider.validate_credentials().await?;

            if valid {
                println!("Credentials for '{}' are valid", name);

                let rl = provider.rate_limit_status().await?;
                println!(
                    "Rate limit: {}/{} remaining (resets {})",
                    rl.remaining, rl.limit, rl.reset_at
                );
            } else {
                println!("Credentials for '{}' are INVALID", name);
            }
            Ok(())
        }
        HostAction::Remove { name } => {
            let db_path = GitrConfig::db_path()?;
            let conn = gitr_db::open_db(&db_path)?;
            let host = gitr_db::ops::get_host_by_label(&conn, &name)?
                .ok_or_else(|| anyhow::anyhow!("Host '{}' not found", name))?;

            // Delete token from keychain
            let cred_store = KeyringStore::new();
            let _ = cred_store.delete(&host.credential_key);

            // Delete from DB (cascades to repos)
            gitr_db::ops::delete_host(&conn, &host.id)?;

            println!("Host '{}' removed", name);
            Ok(())
        }
    }
}
