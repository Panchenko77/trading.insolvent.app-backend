
use serde::{Deserialize, Serialize};
use trading_model::utils::serde::Emptiable;
use trading_model::core::TimeStampMs;
use trading_model::model::InstrumentCategory;
pub mod order;
pub mod ws_message;
pub mod positions;
pub mod balance;
pub use order::*;
pub use positions::*;
pub use balance::*;

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ResponseData<T> {
    pub code: i32,
    pub message: String,
    pub requestTime: i64,
    pub data: Emptiable<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListedValue<T> {
    pub list: Vec<T>,
}

pub type ResponseDataListed<T> = ResponseData<ListedValue<T>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage<T> {
    pub id: String,
        pub action: String,
        pub arg: WsArg,
        pub data: Vec<T>,
        #[serde(rename = "ts")]
        pub creation_time: TimeStampMs,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct WsArg {
 pub instType: InstrumentCategory,
 pub channel: String,
 pub InstId: String, // this would either be a Symbol but when subscribing to position channels
    // the arg is "default"
}
