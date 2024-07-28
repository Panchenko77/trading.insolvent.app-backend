use crate::market::depth_futures::BinanceFuturesDepthUpdate;
use crate::market::ticker::BinanceBookTicker;
use crate::market::trade::BinanceTrade;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "e", rename_all = "camelCase")]
pub enum BinanceMarketFeedMessage {
    #[serde(rename = "depthUpdate")]
    DepthUpdateFutures(BinanceFuturesDepthUpdate),
    Trade(BinanceTrade),
    BookTicker(BinanceBookTicker),
}

#[derive(Deserialize)]
pub struct BinanceErrorMessage {
    pub code: i64,
    pub msg: String,
}

#[derive(Deserialize)]
pub struct BinanceErrorMessageOuter {
    pub error: BinanceErrorMessage,
    pub id: i64,
}
