use trading_exchange::exchange::get_instrument_loader_manager;
use trading_exchange::model::InstrumentsMultiConfig;
use trading_model::model::{Exchange, Network, NetworkSelector};

#[derive(clap::Args)]
pub struct TestSymbolsArgs {
    pub exchange: Exchange,
    #[clap(long, default_value = "Mainnet")]
    pub network: Network,
}

pub async fn test_symbols(args: TestSymbolsArgs) -> eyre::Result<()> {
    let config = InstrumentsMultiConfig::from_exchanges(NetworkSelector::Network(args.network), &[args.exchange]);
    let manager = get_instrument_loader_manager().load_instruments_multi(&config).await?;
    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header(vec![
        "Network",
        "Instrument",
        "Symbol",
        "Base",
        "Base Wire",
        "Quote",
        "Quote Wire",
        "Size",
        "Price",
        "Lot Size",
        "Tick Size",
    ]);
    for instrument in manager.iter() {
        table.add_row(vec![
            instrument.network.to_string(),
            instrument.code_simple.to_string(),
            instrument.instrument_symbol.to_string(),
            instrument.base.asset.to_string(),
            instrument.base.wire.precision.to_string(),
            instrument.quote.asset.to_string(),
            instrument.quote.wire.precision.to_string(),
            instrument.size.precision.to_string(),
            instrument.price.precision.to_string(),
            instrument.lot.size.precision.to_string(),
            instrument.tick.size.precision.to_string(),
        ]);
    }
    println!("{}", table);
    Ok(())
}
