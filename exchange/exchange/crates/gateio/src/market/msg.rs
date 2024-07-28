use serde::Deserialize;

use crate::market::depth::{GateioPerpetualDepthMessage, GateioSpotDepthMessage};
use crate::market::ticker::GateioBookTicker;
use crate::market::trade::{GateioPerpetualTrade, GateioSpotTrade};

#[derive(Deserialize)]
#[serde(tag = "channel", content = "result", rename_all = "camelCase")]
pub enum GateioMarketFeedMessage {
    #[serde(rename = "spot.order_book")]
    SpotOrderBook(GateioSpotDepthMessage),
    #[serde(rename = "futures.order_book")]
    PerpetualOrderBook(GateioPerpetualDepthMessage),
    #[serde(rename = "spot.trades")]
    SpotTrade(GateioSpotTrade),
    #[serde(rename = "perpetual.trades")]
    PerpetualTrade(GateioPerpetualTrade),
    BookTicker(GateioBookTicker),
}

#[derive(Deserialize)]
pub struct GateioMarketFeedMessageOuter {
    pub time: i64,
    pub time_ms: i64,
    pub event: String,
    #[serde(flatten)]
    pub result: GateioMarketFeedMessage,
}

#[derive(Deserialize)]
pub struct GateioErrorMessage {
    pub code: i64,
    pub msg: String,
}

#[derive(Deserialize)]
pub struct GateioErrorMessageOuter {
    pub error: GateioErrorMessage,
    pub id: i64,
}
