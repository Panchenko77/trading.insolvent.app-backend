use tracing::info;

use trading_exchange::model::ExecutionResponse;
use trading_exchange::model::{ExecutionService, Portfolio};
use trading_exchange::select::SelectExecution;

pub async fn next_update_display(
    execution: &mut SelectExecution,
    portfolio: &mut Portfolio,
) -> eyre::Result<ExecutionResponse> {
    let response = execution.next().await?;
    response.update_portfolio(portfolio)?;
    info!("Portfolio: {}", portfolio);
    Ok(response)
}
