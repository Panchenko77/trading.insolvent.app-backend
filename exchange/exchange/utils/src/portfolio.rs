use clap::Args;

use trading_exchange::model::{ExecutionConfig, ExecutionResource, Portfolio};
use trading_exchange::select::SelectExecution;
use trading_model::model::Exchange;

use crate::utils::next_update_display;

#[derive(Debug, Args)]
pub struct TestPortfolioArgs {
    pub exchange: Exchange,
}

pub async fn test_portfolio(args: TestPortfolioArgs) -> eyre::Result<()> {
    let mut execution_config = vec![];

    let cfg = ExecutionConfig {
        exchange: args.exchange,
        enabled: true,
        resources: vec![ExecutionResource::Accounting],
        ..ExecutionConfig::empty()
    };

    execution_config.push(cfg);
    let mut execution = SelectExecution::new(execution_config).await?;
    let mut portfolio = Portfolio::new(0);

    loop {
        next_update_display(&mut execution, &mut portfolio).await?;
    }
}
