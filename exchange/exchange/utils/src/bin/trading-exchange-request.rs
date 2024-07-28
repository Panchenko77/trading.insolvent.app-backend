use clap::{Parser, Subcommand};
use common::{load_env_recursively, setup_logs, LogLevel};
use tracing::error;
use trading_exchange_utils::leverage::{set_leverage, SetLeverageArgs};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    SetLeverage(SetLeverageArgs),
}
#[tokio::main]
async fn main() -> eyre::Result<()> {
    load_env_recursively()?;

    let cli = Cli::parse();
    setup_logs(LogLevel::Debug)?;
    let result = match cli.command {
        Command::SetLeverage(args) => set_leverage(args).await,
    };
    if let Err(err) = result {
        error!("{:?}", err);
    }
    Ok(())
}
