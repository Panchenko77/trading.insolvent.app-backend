use crate::get_bybit_order_lid;
use crate::model::{ResponseDataListed, WsMessage};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use trading_exchange_core::model::{
    AccountId, ExecutionResponse, Order, OrderCid, OrderSid, OrderStatus, OrderType, PositionEffect, SyncOrders,
    TimeInForce,
};
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::Exchange;
use trading_model::{
    InstrumentCode, InstrumentManagerExt, InstrumentSelector, Price, Quantity, SharedInstrumentManager, Side, Symbol,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum BybitOrderType {
    Limit,
    Market,
}

impl Into<OrderType> for BybitOrderType {
    fn into(self) -> OrderType {
        match self {
            Self::Limit => OrderType::Limit,
            Self::Market => OrderType::Market,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BybitTimeInForce {
    GTC,
    IOC,
    FOK,
}

impl Into<TimeInForce> for BybitTimeInForce {
    fn into(self) -> TimeInForce {
        match self {
            Self::GTC => TimeInForce::GoodTilCancel,
            Self::IOC => TimeInForce::ImmediateOrCancel,
            Self::FOK => TimeInForce::FillOrKill,
        }
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct BybitHttpLiveOrder {
    symbol: Symbol,
    orderType: BybitOrderType,
    orderLinkId: OrderCid,
    orderId: OrderSid,
    blockTradeId: String,
    #[serde_as(as = "DisplayFromStr")]
    price: f64,
    #[serde_as(as = "DisplayFromStr")]
    qty: f64,
    side: Side,
    // isLeverage: String,
    // positionIdx: i32,
    orderStatus: BybitOrderStatus,
    // cancelType: String,
    // rejectReason: String,
    // avgPrice: String,
    #[serde_as(as = "DisplayFromStr")]
    leavesQty: f64,
    #[serde_as(as = "DisplayFromStr")]
    leavesValue: f64,
    #[serde_as(as = "DisplayFromStr")]
    cumExecQty: f64,
    #[serde_as(as = "DisplayFromStr")]
    cumExecValue: f64,
    #[serde_as(as = "DisplayFromStr")]
    cumExecFee: f64,
    timeInForce: BybitTimeInForce,
    // stopOrderType: String,
    // orderIv: String,
    #[serde_as(as = "DisplayFromStr")]
    triggerPrice: f64,
    #[serde_as(as = "DisplayFromStr")]
    takeProfit: f64,
    #[serde_as(as = "DisplayFromStr")]
    stopLoss: f64,
    // tpTriggerBy: String,
    // slTriggerBy: String,
    // triggerDirection: i32,
    // triggerBy: String,
    // lastPriceOnCreated: String,
    reduceOnly: bool,
    // closeOnTrigger: bool,
    // smpType: String,
    // smpGroup: i32,
    // smpOrderId: String,
    // tpslMode: String,
    // tpLimitPrice: String,
    // slLimitPrice: String,
    // placeType: String,
    #[serde_as(as = "DisplayFromStr")]
    createdTime: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    updatedTime: TimeStampMs,
}

impl BybitHttpLiveOrder {
    pub fn get_pos_effect(&self) -> PositionEffect {
        if self.reduceOnly {
            PositionEffect::Close
        } else {
            PositionEffect::NA
        }
    }
    pub fn into_order(self, instrument: InstrumentCode) -> Order {
        Order {
            instrument,
            effect: self.get_pos_effect(),
            side: self.side,
            price: self.price,
            size: self.qty,
            filled_size: self.cumExecQty,
            average_filled_price: self.cumExecValue / self.cumExecQty,
            stop_price: self.triggerPrice,
            local_id: get_bybit_order_lid(&self.orderId),
            client_id: self.orderLinkId,
            server_id: self.orderId,
            status: self.orderStatus.into(),
            open_lt: Time::NULL,
            open_tst: Time::from_millis(self.createdTime),
            update_lt: Time::NULL,
            update_est: Time::from_millis(self.updatedTime),
            update_tst: Time::NULL,
            ty: self.orderType.into(),
            tif: self.timeInForce.into(),
            ..Order::empty()
        }
    }
}

pub fn decode_http_open_orders(
    range: InstrumentSelector,
    data: &str,
    manager: Option<SharedInstrumentManager>,
) -> Result<ExecutionResponse, String> {
    let mut sync_orders = SyncOrders::empty();
    sync_orders.range = range;
    let orders: ResponseDataListed<BybitHttpLiveOrder> =
        serde_json::from_str(data).expect("failed to decode_http_open_orders");

    let Some(result) = orders.result.into_option() else {
        return Err(format!(
            "failed to decode http open orders: {}: {}",
            orders.retCode, orders.retMsg
        ));
    };
    for order in result.list {
        let instrument = manager.maybe_lookup_instrument(Exchange::Bybit, order.symbol.clone());
        let order = order.into_order(instrument);
        sync_orders.orders.push(order);
    }
    Ok(ExecutionResponse::SyncOrders(sync_orders))
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BybitWsOrder {
    pub symbol: Symbol,
    #[serde(rename = "orderId")]
    pub order_id: OrderSid,
    pub side: Side,
    #[serde(rename = "orderType")]
    pub order_type: OrderType,
    #[serde(rename = "cancelType")]
    pub cancel_type: String,
    #[serde_as(as = "DisplayFromStr")]
    pub price: Price,
    #[serde_as(as = "DisplayFromStr")]
    pub qty: Quantity,
    #[serde(rename = "orderIv")]
    pub order_iv: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: BybitTimeInForce,
    #[serde(rename = "orderStatus")]
    pub order_status: BybitOrderStatus,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: OrderCid,
    #[serde(rename = "lastPriceOnCreated")]
    pub last_price_on_created: String,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: bool,
    #[serde(rename = "leavesQty")]
    pub leaves_qty: String,
    #[serde(rename = "leavesValue")]
    pub leaves_value: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cumExecQty")]
    pub cum_exec_qty: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cumExecValue")]
    pub cum_exec_value: Quantity,
    #[serde(rename = "avgPrice")]
    #[serde_as(as = "DisplayFromStr")]
    pub avg_price: Price,
    #[serde(rename = "blockTradeId")]
    pub block_trade_id: String,
    #[serde(rename = "positionIdx")]
    pub position_idx: i64,
    #[serde(rename = "cumExecFee")]
    pub cum_exec_fee: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "createdTime")]
    pub created_time: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "updatedTime")]
    pub updated_time: TimeStampMs,
    #[serde(rename = "rejectReason")]
    pub reject_reason: String,
    #[serde(rename = "stopOrderType")]
    pub stop_order_type: String,
    #[serde(rename = "tpslMode")]
    pub tpsl_mode: String,
    #[serde(rename = "triggerPrice")]
    pub trigger_price: String,
    #[serde(rename = "takeProfit")]
    pub take_profit: String,
    #[serde(rename = "stopLoss")]
    pub stop_loss: String,
    #[serde(rename = "tpTriggerBy")]
    pub tp_trigger_by: String,
    #[serde(rename = "slTriggerBy")]
    pub sl_trigger_by: String,
    #[serde(rename = "tpLimitPrice")]
    pub tp_limit_price: String,
    #[serde(rename = "slLimitPrice")]
    pub sl_limit_price: String,
    #[serde(rename = "triggerDirection")]
    pub trigger_direction: i64,
    #[serde(rename = "triggerBy")]
    pub trigger_by: String,
    #[serde(rename = "closeOnTrigger")]
    pub close_on_trigger: bool,
    pub category: String,
    #[serde(rename = "placeType")]
    pub place_type: String,
    #[serde(rename = "smpType")]
    pub smp_type: String,
    #[serde(rename = "smpGroup")]
    pub smp_group: i64,
    #[serde(rename = "smpOrderId")]
    pub smp_order_id: String,
    #[serde(rename = "feeCurrency")]
    pub fee_currency: String,
}

impl BybitWsOrder {
    pub fn get_pos_effect(&self) -> PositionEffect {
        if self.reduce_only {
            PositionEffect::Close
        } else {
            PositionEffect::NA
        }
    }
    pub fn into_order(self, instrument: InstrumentCode) -> Order {
        Order {
            instrument,
            effect: self.get_pos_effect(),
            side: self.side,
            price: self.price,
            size: self.qty,
            filled_size: self.cum_exec_qty,
            average_filled_price: self.cum_exec_value / self.cum_exec_qty,
            stop_price: self.trigger_price.parse().unwrap(),
            local_id: get_bybit_order_lid(&self.order_id),
            client_id: self.order_link_id,
            server_id: self.order_id,
            status: self.order_status.into(),
            open_lt: Time::NULL,
            open_tst: Time::from_millis(self.created_time),
            update_lt: Time::NULL,
            update_est: Time::from_millis(self.updated_time),
            update_tst: Time::NULL,
            ty: self.order_type,
            tif: self.time_in_force.into(),
            ..Order::empty()
        }
    }
}

// open status
//
// New order has been placed successfully
// PartiallyFilled
// Untriggered Conditional orders are created
// closed status
//
// Rejected
// PartiallyFilledCanceled Only spot has this order status
// Filled
// Cancelled In derivatives, orders with this status may have an executed qty
// Triggered instantaneous state for conditional orders from Untriggered to New
// Deactivated UTA: Spot tp/sl order, conditional order, OCO order are cancelled before they are triggered
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BybitOrderStatus {
    New,
    PartiallyFilled,
    Untriggered,
    Rejected,
    PartiallyFilledCanceled,
    Filled,
    Cancelled,
    Triggered,
    Deactivated,
}

impl Into<OrderStatus> for BybitOrderStatus {
    fn into(self) -> OrderStatus {
        match self {
            Self::New => OrderStatus::Open,
            Self::PartiallyFilled => OrderStatus::PartiallyFilled,
            Self::Untriggered => OrderStatus::Untriggered,
            Self::Rejected => OrderStatus::Rejected,
            Self::PartiallyFilledCanceled => OrderStatus::Cancelled,
            Self::Filled => OrderStatus::Filled,
            Self::Cancelled => OrderStatus::Cancelled,
            Self::Triggered => OrderStatus::Triggered,
            Self::Deactivated => OrderStatus::Rejected,
        }
    }
}

pub fn parse_bybit_ws_order(
    account: AccountId,

    msg: WsMessage<BybitWsOrder>,
    manager: Option<SharedInstrumentManager>,
) -> Result<SyncOrders> {
    let mut sync_orders = SyncOrders::new(Exchange::Bybit, None);
    sync_orders.full = false;
    for order_changes in msg.data {
        let instrument = manager.maybe_lookup_instrument(Exchange::Bybit, order_changes.symbol.clone());

        let mut order = order_changes.into_order(instrument);
        order.account = account;

        sync_orders.orders.push(order);
    }
    Ok(sync_orders)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BybitOrderExecution {
    pub category: String,
    pub symbol: String,
    #[serde(rename = "execFee")]
    pub exec_fee: String,
    #[serde(rename = "execId")]
    pub exec_id: String,
    #[serde(rename = "execPrice")]
    pub exec_price: String,
    #[serde(rename = "execQty")]
    pub exec_qty: String,
    #[serde(rename = "execType")]
    pub exec_type: String,
    #[serde(rename = "execValue")]
    pub exec_value: String,
    #[serde(rename = "isMaker")]
    pub is_maker: bool,
    #[serde(rename = "feeRate")]
    pub fee_rate: String,
    #[serde(rename = "tradeIv")]
    pub trade_iv: String,
    #[serde(rename = "markIv")]
    pub mark_iv: String,
    #[serde(rename = "blockTradeId")]
    pub block_trade_id: String,
    #[serde(rename = "markPrice")]
    pub mark_price: String,
    #[serde(rename = "indexPrice")]
    pub index_price: String,
    #[serde(rename = "underlyingPrice")]
    pub underlying_price: String,
    #[serde(rename = "leavesQty")]
    pub leaves_qty: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderLinkId")]
    pub order_link_id: String,
    #[serde(rename = "orderPrice")]
    pub order_price: String,
    #[serde(rename = "orderQty")]
    pub order_qty: String,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "stopOrderType")]
    pub stop_order_type: String,
    pub side: String,
    #[serde(rename = "execTime")]
    pub exec_time: String,
    #[serde(rename = "isLeverage")]
    pub is_leverage: String,
    #[serde(rename = "closedSize")]
    pub closed_size: String,
    pub seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BybitCreateOrder {
    pub order_id: OrderSid,
    pub order_link_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BybitCancleOrder {
    pub order_id: OrderSid,
    pub order_link_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use trading_model::InstrumentSelector;

    #[test]
    fn test_deserialize_http_order() {
        // Example usage:
        let json_response = r#"
        {
            "orderId": "fd4300ae-7847-404e-b947-b46980a4d140",
            "orderLinkId": "test-000005",
            "blockTradeId": "",
            "symbol": "ETHUSDT",
            "price": "1600.00",
            "qty": "0.10",
            "side": "Buy",
            "isLeverage": "",
            "positionIdx": 1,
            "orderStatus": "New",
            "cancelType": "UNKNOWN",
            "rejectReason": "EC_NoError",
            "avgPrice": "0",
            "leavesQty": "0.10",
            "leavesValue": "160",
            "cumExecQty": "0.00",
            "cumExecValue": "0",
            "cumExecFee": "0",
            "timeInForce": "GTC",
            "orderType": "Limit",
            "stopOrderType": "UNKNOWN",
            "orderIv": "",
            "triggerPrice": "0.00",
            "takeProfit": "2500.00",
            "stopLoss": "1500.00",
            "tpTriggerBy": "LastPrice",
            "slTriggerBy": "LastPrice",
            "triggerDirection": 0,
            "triggerBy": "UNKNOWN",
            "lastPriceOnCreated": "",
            "reduceOnly": false,
            "closeOnTrigger": false,
            "smpType": "None",
            "smpGroup": 0,
            "smpOrderId": "",
            "tpslMode": "Full",
            "tpLimitPrice": "",
            "slLimitPrice": "",
            "placeType": "",
            "createdTime": "1684738540559",
            "updatedTime": "1684738540561"
        }
    "#;

        let order: BybitHttpLiveOrder = serde_json::from_str(json_response).expect("Failed to deserialize JSON");
        println!("{:?}", order);
    }

    #[test]
    fn test_decode_http_live_orders() {
        let data = r#"{
  "retCode": 0,
  "retMsg": "OK",
  "result": {
    "nextPageCursor": "1615086401936952320%3A1707269302538%2C1615086401936952320%3A1707269302538",
    "category": "spot",
    "list": [
      {
        "symbol": "ETHUSDT",
        "orderType": "Limit",
        "orderLinkId": "ORD693020000",
        "slLimitPrice": "0",
        "orderId": "1615086401936952320",
        "cancelType": "UNKNOWN",
        "avgPrice": "0.00",
        "stopOrderType": "",
        "lastPriceOnCreated": "",
        "orderStatus": "New",
        "takeProfit": "0",
        "cumExecValue": "0.0000000",
        "smpType": "None",
        "triggerDirection": 0,
        "blockTradeId": "",
        "isLeverage": "0",
        "rejectReason": "EC_NoError",
        "price": "1800.00",
        "orderIv": "",
        "createdTime": "1707269302538",
        "tpTriggerBy": "",
        "positionIdx": 0,
        "timeInForce": "GTC",
        "leavesValue": "17.9820000",
        "updatedTime": "1707269302540",
        "side": "Buy",
        "smpGroup": 0,
        "triggerPrice": "0.00",
        "tpLimitPrice": "0",
        "cumExecFee": "0",
        "leavesQty": "0.00999",
        "slTriggerBy": "",
        "closeOnTrigger": false,
        "placeType": "",
        "cumExecQty": "0.00000",
        "reduceOnly": false,
        "qty": "0.00999",
        "stopLoss": "0",
        "marketUnit": "",
        "smpOrderId": "",
        "triggerBy": ""
      }
    ]
  },
  "retExtInfo": {},
  "time": 1707270044907
}"#;
        let decoded = decode_http_open_orders(InstrumentSelector::All, data, None).unwrap();
        println!("{:?}", decoded);
    }
}
