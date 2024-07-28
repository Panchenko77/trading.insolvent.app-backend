use crate::{Exchange, Price, Quantity, Symbol, TimeStampNs};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct QtyPx {
    pub quantity: Quantity,
    pub price: Price,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceVolume {
    pub exchange: Exchange,
    pub symbol: Symbol,
    pub bid: QtyPx,
    pub ask: QtyPx,
    pub exchange_tm: TimeStampNs,
}
