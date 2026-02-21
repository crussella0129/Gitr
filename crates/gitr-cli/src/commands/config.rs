use clap::Subcommand;
use gitr_core::config::GitrConfig;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Initialize ~/.gitr/ directory with default config and database
    Init,
    /// Show current configuration
    Show,
}

pub fn run(action: ConfigAction) -> anyhow::Result<()> {
    match action {
        ConfigAction::Init => {
            let home = GitrConfig::init()?;
            let db_path = GitrConfig::db_path()?;

            // Ensure database is created with schema
            gitr_db::open_db(&db_path)?;

            println!("Initialized gitr at {}", home.display());
            println!("  config: {}", GitrConfig::config_path()?.display());
            println!("  database: {}", db_path.display());
            Ok(())
        }
        ConfigAction::Show => {
            let config = GitrConfig::load()?;
            let toml_str = toml::to_string_pretty(&config)?;
            println!("{toml_str}");
            Ok(())
        }
    }
}
