pub mod config;
pub mod history;
pub mod host;
pub mod repo;
pub mod scan;
pub mod status;
pub mod sync;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Initialize and manage Gitr configuration
    Config {
        #[command(subcommand)]
        action: config::ConfigAction,
    },
    /// Manage git hosting services
    Host {
        #[command(subcommand)]
        action: host::HostAction,
    },
    /// Scan for repos on disk and via API
    Scan(scan::ScanArgs),
    /// Manage tracked repos
    Repo {
        #[command(subcommand)]
        action: repo::RepoAction,
    },
    /// Sync repos with upstream
    Sync(sync::SyncArgs),
    /// Show status of all tracked repos
    Status(status::StatusArgs),
    /// Show sync history
    History(history::HistoryArgs),
}

pub async fn run(cmd: Command) -> anyhow::Result<()> {
    match cmd {
        Command::Config { action } => config::run(action),
        Command::Host { action } => host::run(action).await,
        Command::Scan(args) => scan::run(args).await,
        Command::Repo { action } => repo::run(action),
        Command::Sync(args) => sync::run(args).await,
        Command::Status(args) => status::run(args),
        Command::History(args) => history::run(args),
    }
}
