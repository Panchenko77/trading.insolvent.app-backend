use serde::{Deserialize, Serialize};
use trading_model::utils::serde::Emptiable;

mod balance;
mod order;
mod position;
mod ws_message;

pub use balance::*;
pub use order::*;
pub use position::*;
pub use ws_message::*;

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ResponseData<T> {
    pub retCode: i32,
    pub retMsg: String,
    pub result: Emptiable<T>,
    pub retExtInfo: serde_json::Value,
    pub time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListedValue<T> {
    pub list: Vec<T>,
}

pub type ResponseDataListed<T> = ResponseData<ListedValue<T>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage<T> {
    pub id: String,
    pub topic: String,
    #[serde(rename = "creationTime")]
    pub creation_time: i64,
    pub data: Vec<T>,
}
