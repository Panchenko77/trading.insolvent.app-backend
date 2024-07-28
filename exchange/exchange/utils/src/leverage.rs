use clap::Parser;
use eyre::{Context, ContextCompat};
use tracing::info;
use trading_exchange::exchange::get_execution_service_builder_manager;
use trading_exchange::model::{ExecutionConfig, ExecutionRequest, RequestUpdateLeverage};
use trading_model::model::{Exchange, InstrumentCategory, InstrumentSymbol, Symbol};
#[derive(Parser)]
pub struct SetLeverageArgs {
    /// Exchange or Symbol to set leverage for
    /// EXCHANGE or EXCHANGE:SYMBOL or EXCHANGE:BASE:CATEGORY
    pub exchange_or_symbol: String,
    #[clap(long)]
    pub leverage: f64,
}

pub async fn set_leverage(args: SetLeverageArgs) -> eyre::Result<()> {
    info!("Setting leverage for {} to {}", args.exchange_or_symbol, args.leverage);
    let manager = get_execution_service_builder_manager();
    let mut split = args.exchange_or_symbol.split(':');
    let exchange: Exchange = split.next().with_context(|| "Exchange")?.parse()?;
    let symbol: Option<Symbol> = split.next().map(|s| s.parse()).transpose().with_context(|| "Symbol")?;
    let category: Option<InstrumentCategory> = split
        .next()
        .map(|s| s.parse())
        .transpose()
        .with_context(|| "Category")?;
    let config = ExecutionConfig {
        exchange,
        enabled: true,
        ..ExecutionConfig::empty()
    };

    let mut conn = manager
        .find_builder(&config)
        .with_context(|| "Builder")?
        .build(&config)
        .await?;

    conn.request(&ExecutionRequest::UpdateLeverage(RequestUpdateLeverage {
        exchange,
        symbol: symbol.map(|s| InstrumentSymbol {
            exchange,
            symbol: s,
            category,
        }),
        leverage: args.leverage,
    }))
    .await?;
    Ok(())
}
