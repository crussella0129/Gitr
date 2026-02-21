mod commands;

use clap::Parser;

#[derive(Parser)]
#[command(name = "gitr", version, about = "Git repo sync & management across hosting services")]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    commands::run(cli.command).await
}
