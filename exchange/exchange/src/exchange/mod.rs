use trading_exchange_core::model::{
    ExecutionConfig, ExecutionRequest, ExecutionResponse, InstrumentLoaderManager, OrderCid, ServiceBuilderErased,
    ServiceBuilderManager, ServiceBuilderManagerTrait,
};

#[cfg(feature = "binance")]
pub use trading_exchange_binance as binance;
use trading_exchange_binance::symbol::BINANCE_INSTRUMENT_LOADER;

#[cfg(feature = "bitget")]
pub use trading_exchange_bitget as bitget;
use trading_exchange_bitget::symbol::BITGET_INSTRUMENT_LOADER;
#[cfg(feature = "bybit")]
pub use trading_exchange_bybit as bybit;
use trading_exchange_bybit::symbol::BYBIT_INSTRUMENT_LOADER;
#[cfg(feature = "coinbase")]
pub use trading_exchange_coinbase as coinbase;
use trading_exchange_coinbase::symbol::COINBASE_INSTRUMENT_LOADER;
#[cfg(feature = "drift")]
pub use trading_exchange_drift as drift;
use trading_exchange_drift::symbol::DRIFT_INSTRUMENT_LOADER;
#[cfg(feature = "gateio")]
pub use trading_exchange_gateio as gateio;
use trading_exchange_gateio::symbol::GATEIO_INSTRUMENT_LOADER;
#[cfg(feature = "hyperliquid")]
pub use trading_exchange_hyperliquid as hyperliquid;
use trading_exchange_hyperliquid::HYPERLIQUID_INSTRUMENT_LOADER;
use trading_model::Exchange;

pub struct ExecutionServiceBuilderManagerTrait;
impl ServiceBuilderManagerTrait for ExecutionServiceBuilderManagerTrait {
    type Builder =
        dyn ServiceBuilderErased<Config = ExecutionConfig, Request = ExecutionRequest, Response = ExecutionResponse>;
    type Config = ExecutionConfig;
    type Request = ExecutionRequest;
    type Response = ExecutionResponse;
}
pub type ExecutionServiceBuilderManager = ServiceBuilderManager<ExecutionServiceBuilderManagerTrait>;

pub fn get_execution_service_builder_manager() -> ExecutionServiceBuilderManager {
    let mut manager = ExecutionServiceBuilderManager::new();
    #[cfg(feature = "binance")]
    manager.add(Box::new(binance::execution::BinanceExecutionBuilder::new()));
    #[cfg(feature = "bybit")]
    manager.add(Box::new(bybit::execution::BybitExecutionBuilder::new()));
    // #[cfg(feature = "coinbase")]
    // manager.add(coinbase::CoinbaseExecutionServiceBuilder);
    #[cfg(feature = "drift")]
    manager.add(Box::new(drift::execution::DriftExecutionServiceBuilder::new()));
    #[cfg(feature = "gateio")]
    manager.add(Box::new(gateio::execution::GateioExecutionBuilder::new()));
    #[cfg(feature = "hyperliquid")]
    manager.add(Box::new(
        hyperliquid::execution::HyperliquidExecutionServiceBuilder::new(),
    ));
    //#[cfg(feature = "bitget")]
    //  manager.add(Box::new( bitget::exectution::BinanceExecutionBuilder::new(),
    // ));
    manager
}

pub fn get_instrument_loader_manager() -> InstrumentLoaderManager {
    let mut manager = InstrumentLoaderManager::new();
    manager.add_loader_raw(&BINANCE_INSTRUMENT_LOADER);
    manager.add_loader_raw(&BYBIT_INSTRUMENT_LOADER);
    manager.add_loader_raw(&COINBASE_INSTRUMENT_LOADER);
    manager.add_loader_raw(&HYPERLIQUID_INSTRUMENT_LOADER);
    manager.add_loader_raw(&GATEIO_INSTRUMENT_LOADER);
    manager.add_loader_raw(&DRIFT_INSTRUMENT_LOADER);
    manager.add_loader_raw(&BITGET_INSTRUMENT_LOADER);
    manager
}

pub fn gen_order_cid(exchange: Exchange) -> OrderCid {
    match exchange {
        #[cfg(feature = "binance")]
        _ if exchange.is_binance() => binance::gen_client_id(),
        #[cfg(feature = "hyperliquid")]
        Exchange::Hyperliquid => hyperliquid::gen_client_id(),
        _ => "".into(),
    }
}
