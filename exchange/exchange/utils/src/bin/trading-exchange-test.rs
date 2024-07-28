use clap::{Parser, Subcommand};
use common::{load_env_recursively, setup_logs, LogLevel};
use tracing::error;
use trading_exchange_utils::latency::{test_latency, TestLatency};
use trading_exchange_utils::orders::{test_orders, TestOrdersArgs};
use trading_exchange_utils::portfolio::{test_portfolio, TestPortfolioArgs};
use trading_exchange_utils::symbols::{test_symbols, TestSymbolsArgs};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Orders(TestOrdersArgs),
    Portfolio(TestPortfolioArgs),
    Symbols(TestSymbolsArgs),
    Latency(TestLatency),
}
#[tokio::main]
async fn main() -> eyre::Result<()> {
    load_env_recursively()?;

    let cli = Cli::parse();
    setup_logs(LogLevel::Debug)?;
    let result = match cli.command {
        Command::Orders(args) => test_orders(args).await,
        Command::Portfolio(args) => test_portfolio(args).await,
        Command::Symbols(args) => test_symbols(args).await,
        Command::Latency(args) => test_latency(args).await,
    };
    if let Err(err) = result {
        error!("{:?}", err);
    }
    Ok(())
}
