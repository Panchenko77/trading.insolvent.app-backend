use serde::{Deserialize, Serialize};

use trading_model::model::{Exchange, InstrumentSymbol};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestUpdateLeverage {
    pub exchange: Exchange,
    pub symbol: Option<InstrumentSymbol>,
    pub leverage: f64,
}
