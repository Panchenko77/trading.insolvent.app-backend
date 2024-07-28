use trading_model::model::Exchange;

pub mod execution;

pub mod market;
pub(crate) mod model;
pub mod rest;
pub mod symbol;
pub mod urls;

pub trait ExchangeIsGateioExt {
    fn is_gateio(&self) -> bool;
}
impl ExchangeIsGateioExt for Exchange {
    fn is_gateio(&self) -> bool {
        matches!(
            self,
            Exchange::GateioSpot | Exchange::GateioMargin | Exchange::GateioPerpetual
        )
    }
}
