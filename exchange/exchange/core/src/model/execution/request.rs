use derive_from_one::FromOne;
use serde::{Deserialize, Serialize};

use trading_model::{Exchange, InstrumentSelector};

mod cancel_order;
mod new_order;
mod set_leverage;
pub use cancel_order::*;
pub use new_order::*;
pub use set_leverage::*;

#[derive(Debug, Clone, Serialize, Deserialize, FromOne)]
pub enum ExecutionRequest {
    PlaceOrder(RequestPlaceOrder),
    GetPositions(Exchange),
    CancelOrder(RequestCancelOrder),
    CancelAllOrders(Option<Exchange>),
    SyncOrders(InstrumentSelector),
    QueryAssets(Option<Exchange>),
    UpdateLeverage(RequestUpdateLeverage),
}

impl ExecutionRequest {
    pub fn get_exchange(&self) -> Option<Exchange> {
        match self {
            Self::PlaceOrder(req) => req.instrument.get_exchange(),
            Self::GetPositions(exchange) => Some(exchange.clone()),
            Self::CancelOrder(req) => req.instrument.get_exchange(),
            Self::CancelAllOrders(exchange) => exchange.clone(),
            Self::SyncOrders(range) => range.get_exchange(),
            Self::QueryAssets(exchange) => exchange.clone(),
            Self::UpdateLeverage(req) => Some(req.exchange.clone()),
        }
    }
}
