#![allow(non_camel_case_types)]

use eyre::Result;
use serde::{Deserialize, Serialize};
use trading_model::model;

use trading_model::model::{InstrumentType, Side};
use trading_model::utils::serde::hex2_i64;

use crate::js::DriftJsClient;

#[derive(Serialize, Deserialize)]
pub enum TriggerCondition {
    above {},
}

#[derive(Serialize, Deserialize)]
pub enum Direction {
    long {},
    short {},
}

impl From<Direction> for Side {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::long {} => Side::Buy,
            Direction::short {} => Side::Sell,
        }
    }
}

impl From<Side> for Direction {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => Direction::long {},
            Side::Sell => Direction::short {},
            _ => unimplemented!("unsupported side: {:?}", side),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MarketType {
    spot {},
    perp {},
}

impl From<MarketType> for model::InstrumentCategory {
    fn from(market_type: MarketType) -> Self {
        match market_type {
            MarketType::spot {} => model::InstrumentCategory::Spot,
            MarketType::perp {} => model::InstrumentCategory::Futures,
        }
    }
}

impl From<InstrumentType> for MarketType {
    fn from(value: InstrumentType) -> Self {
        match value {
            InstrumentType::Spot => MarketType::spot {},
            InstrumentType::Perpetual(_) => MarketType::perp {},
            _ => unimplemented!("unsupported instrument type: {:?}", value),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderType {
    limit {},
    market {},
}

impl From<OrderType> for trading_exchange_core::model::OrderType {
    fn from(order_type: OrderType) -> Self {
        match order_type {
            OrderType::limit {} => trading_exchange_core::model::OrderType::Limit,
            OrderType::market {} => trading_exchange_core::model::OrderType::Market,
        }
    }
}

pub(crate) fn get_order_type(ty: trading_exchange_core::model::OrderType) -> (OrderType, PostOnlyParam) {
    match ty {
        trading_exchange_core::model::OrderType::Limit => (OrderType::limit {}, PostOnlyParam::None {}),
        trading_exchange_core::model::OrderType::Market => (OrderType::market {}, PostOnlyParam::None {}),
        trading_exchange_core::model::OrderType::PostOnly => (OrderType::limit {}, PostOnlyParam::MustPostOnly {}),
        _ => unimplemented!("unsupported order type: {:?}", ty),
    }
}

#[derive(Serialize, Deserialize)]
pub enum OrderStatus {
    init {},
    open {},
}

impl From<OrderStatus> for trading_exchange_core::model::OrderStatus {
    fn from(order_status: OrderStatus) -> Self {
        match order_status {
            OrderStatus::init {} => trading_exchange_core::model::OrderStatus::Received,
            OrderStatus::open {} => trading_exchange_core::model::OrderStatus::Open,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftJsOrder {
    // pub slot: String,
    #[serde(with = "hex2_i64")]
    pub price: i64,
    #[serde(with = "hex2_i64")]
    pub base_asset_amount: i64,
    #[serde(with = "hex2_i64")]
    pub base_asset_amount_filled: i64,
    #[serde(with = "hex2_i64")]
    pub quote_asset_amount_filled: i64,
    #[serde(with = "hex2_i64")]
    pub trigger_price: i64,
    // pub auction_start_price: String,
    // pub auction_end_price: String,
    // pub max_ts: String,
    // pub oracle_price_offset: i64,
    pub order_id: u64,
    pub market_index: u32,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub market_type: MarketType,
    pub user_order_id: u8,
    pub existing_position_direction: Direction,
    pub direction: Direction,
    pub reduce_only: bool,
    pub post_only: bool,
    pub immediate_or_cancel: bool,
    // pub trigger_condition: TriggerCondition,
    // pub auction_duration: i64,
    // pub padding: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PostOnlyParam {
    None {},
    MustPostOnly {},
    TryPostOnly {},
    Slide {},
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderParams {
    pub order_type: OrderType,
    pub market_type: MarketType,
    pub user_order_id: u8,
    pub direction: Direction,
    #[serde(with = "hex2_i64")]
    pub base_asset_amount: i64,
    #[serde(with = "hex2_i64")]
    pub price: i64,
    pub market_index: u64,
    pub reduce_only: bool,
    pub post_only: PostOnlyParam,
    pub immediate_or_cancel: bool,
    // pub trigger_price: Option<BigUint>,
    // pub trigger_condition: OrderTriggerCondition,
    // pub oracle_price_offset: Option<i32>,
    // pub auction_duration: Option<u32>,
    // pub max_ts: Option<BigUint>,
    // pub auction_start_price: Option<BigUint>,
    // pub auction_end_price: Option<BigUint>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderParams {
    pub order_id: Option<i32>,
    pub order_user_id: Option<i32>,
    pub market_type: Option<MarketType>,
    pub market_index: Option<i32>,
}

impl DriftJsClient {
    pub async fn get_orders(&self) -> Result<Vec<DriftJsOrder>> {
        self.await_function_call("get_orders").await
    }
    pub async fn place_order(&self, params: &OrderParams) -> Result<String> {
        self.await_function_call_with_params("place_order", params).await
    }
    pub async fn cancel_order(&self, params: &CancelOrderParams) -> Result<String> {
        self.await_function_call_with_params("cancel_order", params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_get_orders() -> Result<()> {
        Ok(())
    }
}
