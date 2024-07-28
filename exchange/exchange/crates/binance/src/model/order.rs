#![allow(non_camel_case_types)]

use eyre::bail;
use parse_display::FromStr;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use trading_exchange_core::model::{Order, OrderCid, OrderStatus, OrderType, SyncOrders};
use trading_model::core::{Time, TimeStampMs};
use trading_model::model::{Exchange, InstrumentManagerExt, SharedInstrumentManager, Side, Symbol};
use trading_model::{Price, Quantity};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountUpdateReason {
    DEPOSIT,
    WITHDRAW,
    ORDER,
    ADJUSTMENT,
    INSURANCE_CLEAR,
    ADMIN_DEPOSIT,
    ADMIN_WITHDRAW,
    MARGIN_TRANSFER,
    MARGIN_TYPE_CHANGE,
    ASSET_TRANSFER,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinanceOrderStatus {
    NEW,
    PARTIALLY_FILLED,
    FILLED,
    CANCELED,
    PENDING_CANCEL,
    REJECTED,
    EXPIRED,
    TRADE,
    TRADE_PREVENTION,
}

impl Into<OrderStatus> for BinanceOrderStatus {
    fn into(self) -> OrderStatus {
        match self {
            Self::NEW => OrderStatus::Open,
            Self::PARTIALLY_FILLED => OrderStatus::PartiallyFilled,
            Self::FILLED => OrderStatus::Filled,
            Self::CANCELED => OrderStatus::Cancelled,
            Self::PENDING_CANCEL => OrderStatus::CancelReceived,
            Self::REJECTED => OrderStatus::Rejected,
            Self::EXPIRED => OrderStatus::Expired,
            Self::TRADE => OrderStatus::PartiallyFilled,
            Self::TRADE_PREVENTION => OrderStatus::Expired,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromStr)]
pub enum BinanceSpotOrderType {
    LIMIT,
    MARKET,
    STOP_LOSS,
    STOP_LOSS_LIMIT,
    TAKE_PROFIT,
    TAKE_PROFIT_LIMIT,
    LIMIT_MAKER,
}
impl From<BinanceSpotOrderType> for OrderType {
    fn from(ty: BinanceSpotOrderType) -> Self {
        match ty {
            BinanceSpotOrderType::LIMIT => OrderType::Limit,
            BinanceSpotOrderType::MARKET => OrderType::Market,
            BinanceSpotOrderType::STOP_LOSS => OrderType::StopLossMarket,
            BinanceSpotOrderType::STOP_LOSS_LIMIT => OrderType::StopLossLimit,
            BinanceSpotOrderType::TAKE_PROFIT => OrderType::TakeProfitMarket,
            BinanceSpotOrderType::TAKE_PROFIT_LIMIT => OrderType::TakeProfitLimit,
            BinanceSpotOrderType::LIMIT_MAKER => OrderType::PostOnly,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromStr)]
pub enum BinanceFuturesOrderType {
    LIMIT,
    MARKET,
    STOP,
    STOP_MARKET,
    TAKE_PROFIT,
    TAKE_PROFIT_MARKET,
    TRAILING_STOP_MARKET,
}
impl From<BinanceFuturesOrderType> for OrderType {
    fn from(ty: BinanceFuturesOrderType) -> Self {
        match ty {
            BinanceFuturesOrderType::LIMIT => OrderType::Limit,
            BinanceFuturesOrderType::MARKET => OrderType::Market,
            BinanceFuturesOrderType::STOP => OrderType::StopLossLimit,
            BinanceFuturesOrderType::STOP_MARKET => OrderType::StopLossMarket,
            BinanceFuturesOrderType::TAKE_PROFIT => OrderType::TakeProfitLimit,
            BinanceFuturesOrderType::TAKE_PROFIT_MARKET => OrderType::TakeProfitMarket,
            BinanceFuturesOrderType::TRAILING_STOP_MARKET => OrderType::TriggerMarket,
        }
    }
}

pub fn parse_binance_order_type(e: Exchange, s: &str) -> eyre::Result<OrderType> {
    match e {
        Exchange::BinanceSpot | Exchange::BinanceMargin => {
            let ty: BinanceSpotOrderType = s.parse()?;
            Ok(ty.into())
        }
        Exchange::BinanceFutures => {
            let ty: BinanceFuturesOrderType = s.parse()?;
            Ok(ty.into())
        }
        _ => bail!("unsupported exchange: {}", e),
    }
}

///
/// GET /fapi/v1/openOrders
///   {
///     "avgPrice": "0.00000",              // 平均成交价
///     "clientOrderId": "abc",             // 用户自定义的订单号
///     "cumQuote": "0",                        // 成交金额
///     "executedQty": "0",                 // 成交量
///     "orderId": 1917641,                 // 系统订单号
///     "origQty": "0.40",                  // 原始委托数量
///     "origType": "TRAILING_STOP_MARKET", // 触发前订单类型
///     "price": "0",                   // 委托价格
///     "reduceOnly": false,                // 是否仅减仓
///     "side": "BUY",                      // 买卖方向
///     "positionSide": "SHORT", // 持仓方向
///     "status": "NEW",                    // 订单状态
///     "stopPrice": "9300",                    // 触发价，对`TRAILING_STOP_MARKET`无效
///     "closePosition": false,             // 是否条件全平仓
///     "symbol": "BTCUSDT",                // 交易对
///     "time": 1579276756075,              // 订单时间
///     "timeInForce": "GTC",               // 有效方法
///     "type": "TRAILING_STOP_MARKET",     // 订单类型
///     "activatePrice": "9020", // 跟踪止损激活价格, 仅`TRAILING_STOP_MARKET` 订单返回此字段
///     "priceRate": "0.3", // 跟踪止损回调比例, 仅`TRAILING_STOP_MARKET` 订单返回此字段
///     "updateTime": 1579276756075,        // 更新时间
///     "workingType": "CONTRACT_PRICE", // 条件价格触发类型
///     "priceProtect": false            // 是否开启条件单触发保护
///   }
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HttpLiveOrder {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)]
    pub avg_price: f64,
    pub client_order_id: OrderCid,
    // #[serde_as(as = "DisplayFromStr")]
    // #[serde(default)]
    // pub cum_quote: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub executed_qty: f64,
    pub order_id: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub orig_qty: f64,
    // pub orig_type: String,
    #[serde_as(as = "DisplayFromStr")]
    pub price: f64,
    #[serde(default)]
    pub reduce_only: bool,
    pub side: Side,
    // pub position_side: String,
    pub status: BinanceOrderStatus,
    #[serde_as(as = "DisplayFromStr")]
    pub stop_price: f64,
    #[serde(default)]
    pub close_position: bool,
    pub symbol: Symbol,
    pub time: TimeStampMs,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub update_time: TimeStampMs,
}

pub fn decode_http_open_orders(
    data: &[u8],
    exchange: Exchange,
    manager: Option<SharedInstrumentManager>,
) -> eyre::Result<SyncOrders> {
    let mut sync_orders = SyncOrders::new(exchange, None);
    let orders: Vec<HttpLiveOrder> = serde_json::from_slice(data)?;

    for order in orders {
        let instrument = manager.maybe_lookup_instrument(exchange, order.symbol);
        sync_orders.orders.push(Order {
            instrument,
            ty: parse_binance_order_type(exchange, &order.ty)?,
            server_id: order.order_id.to_string().as_str().into(),
            client_id: order.client_order_id,
            status: order.status.into(),
            side: order.side,
            open_lt: Time::from_millis(order.time),
            price: order.price,
            size: order.orig_qty,
            // update_tst: TimeStamp::from_millis(order.time),
            // update_est: TimeStamp::from_millis(order.update_time),
            update_lt: Time::now(),
            filled_size: order.executed_qty,
            ..Order::empty()
        });
    }

    Ok(sync_orders)
}

/// a very brief struct, used in all Spot/Margin/USDM. be careful when adding new fields
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewOrderResponse {
    pub order_id: u64,
    #[serde(alias = "updateTime")] // update time for BinanceFutures
    pub transact_time: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    pub price: Price,
    #[serde_as(as = "DisplayFromStr")]
    pub orig_qty: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub executed_qty: Quantity,
}
