
use serde::Deserialize;
use crate::market::ticker::KucoinBookTicker;
#[derive(Deserialize)]
#[serde(tag = "e", rename_all = "camelCase")]
pub enum KucoinMarketFeedMessage {
    BookTicker(KucoinBookTicker),

}








#[derive(Deserialize)]
pub struct KucoinErrorMessage {
    pub id: String,
    pub code: i64,
    pub data: String,
}
