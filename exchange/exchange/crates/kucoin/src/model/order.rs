#![allow(non_camel_case_types)]

use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tracing::warn;
use trading_exchange_core::model::{
    AccountId, Order, OrderCid, OrderSid, OrderStatus, OrderType, SyncOrders, TimeInForce, UpdateOrder,
};

use trading_model::core::{Time, TimeStampMs, TimeStampSec};
use trading_model::model::{Exchange, InstrumentManagerExt, Price, Quantity, Side, Symbol};
use trading_model::{InstrumentCode, InstrumentDetails, InstrumentManager};

#[derive(Serialize, Deserialize)]
pub struct SpotHttpOpenOrders {
    pub currency_pair: Symbol,
    pub total: i64,
    pub orders: Vec<SpotHttpOpenOrder>,
}
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Display, FromStr)]
pub enum KucoinTimeInForce {
    gtc,
    ioc,
    fok,
    gtt,
}
impl KucoinTimeInForce {
    pub fn from_tif_and_order_type(tif: TimeInForce, ty: OrderType) -> Self {
        //if ty == OrderType::PostOnly {
            //return KucoinTimeInForce::poc;
        //}
        match tif {
            TimeInForce::GoodTilCancel => GateioTimeInForce::gtc,
            TimeInForce::ImmediateOrCancel => GateioTimeInForce::ioc,
            TimeInForce::FillOrKill => GateioTimeInForce::fok,
            TimeInForce::GoodTilTime => GateioTimeInForce::gtt,
            _ => unreachable!("GoodTilCrossing is not supported"),
        }
    }
}
impl From<KucoinTimeInForce> for TimeInForce {
    fn from(tif: KucoinTimeInForce) -> Self {
        match tif {
            KucoinTimeInForce::gtc => TimeInForce::GoodTilCancel,
            KucoinTimeInForce::ioc => TimeInForce::ImmediateOrCancel,
            // FIXME: properly support PendingOrCancel
            KucoinTimeInForce::fok => TimeInForce::FillOrKill,
            KucoinTimeInForce::gtt => TimeInForce::GoodTilTime,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KucoinOrderStatus {
    open,
    closed,
    cancelled,
}
impl From<KucoinOrderStatus> for OrderStatus {
    fn from(status: KucoinOrderStatus) -> Self {
        match status {
            KucoinOrderStatus::open => OrderStatus::Open,
            KucoinOrderStatus::closed => OrderStatus::Filled,
            KucoinOrderStatus::cancelled => OrderStatus::Cancelled,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KucoinOrderType {
    limit,
    market,
}
impl From<KucoinOrderType> for OrderType {
    fn from(ty: KucoinOrderType) -> Self {
        match ty {
            KucoinOrderType::limit => OrderType::Limit,
            KucoinOrderType::market => OrderType::Market,
        }
    }
}

//       {
//         "id": "12332324",
//         "text": "t-123456",
//         "create_time": "1548000000",
//         "update_time": "1548000100",
//         "currency_pair": "ETH_BTC",
//         "status": "open",
//         "type": "limit",
//         "account": "spot",
//         "side": "buy",
//         "amount": "1",
//         "price": "5.00032",
//         "time_in_force": "gtc",
//         "left": "0.5",
//         "filled_total": "2.50016",
//         "fee": "0.005",
//         "fee_currency": "ETH",
//         "point_fee": "0",
//         "gt_fee": "0",
//         "gt_discount": false,
//         "rebated_fee": "0",
//         "rebated_fee_currency": "BTC"
//       }
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpotHttpOpenOrder {
    pub id: OrderSid,
    pub text: OrderCid,
    #[serde_as(as = "DisplayFromStr")]
    pub create_time: TimeStampSec,
    #[serde_as(as = "DisplayFromStr")]
    pub update_time: TimeStampSec,
    pub currency_pair: Symbol,
    pub status: KucoinOrderStatus,
    #[serde(rename = "type")]
    pub ty: KucoinOrderType,
    pub account: String,
    pub side: Side,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    pub price: Price,
    pub time_in_force: KucoinTimeInForce,
    pub left: String,
    #[serde_as(as = "DisplayFromStr")]
    pub filled_total: Quantity,
    pub fee: String,
    pub fee_currency: String,
    pub point_fee: String,
    pub gt_fee: String,
    pub gt_discount: bool,
    pub rebated_fee: String,
    pub rebated_fee_currency: String,
}
impl SpotHttpOpenOrder {
    pub fn into_order(self, account: AccountId, instrument: InstrumentCode) -> Order {
        Order {
            instrument,
            account,
            server_id: self.id,
            client_id: self.text,
            ty: self.ty.into(),
            status: self.status.into(),
            side: self.side,
            open_lt: Time::from_secs(self.create_time),
            price: self.price,
            size: self.amount,
            update_tst: Time::from_secs(self.update_time),
            update_est: Time::from_secs(self.update_time),
            update_lt: Time::now(),
            filled_size: self.filled_total,
            ..Order::empty()
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct SpotHttpOpenOrderPerSymbol {
    pub currency_pair: Symbol,
    pub total: i64,
    pub orders: Vec<SpotHttpOpenOrder>,
}

// {
//     "id": 15675394,
//     "user": 100000,
//     "contract": "BTC_USDT",
//     "create_time": 1546569968,
//     "size": 6024,
//     "iceberg": 0,
//     "left": 6024,
//     "price": "3765",
//     "fill_price": "0",
//     "mkfr": "-0.00025",
//     "tkfr": "0.00075",
//     "tif": "gtc",
//     "refu": 0,
//     "is_reduce_only": false,
//     "is_close": false,
//     "is_liq": false,
//     "text": "t-my-custom-id",
//     "status": "finished",
//     "finish_time": 1514764900,
//     "finish_as": "cancelled",
//     "stp_id": 0,
//     "stp_act": "-",
//     "amend_text": "-"
//   }
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct FuturesHttpOpenOrder {
    pub id: u64,
    // pub user: i64,
    pub contract: Symbol,
    pub create_time: f64,
    pub size: i64,
    pub iceberg: i64,
    pub left: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub price: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub fill_price: f64,
    // pub mkfr: String,
    // pub tkfr: String,
    pub tif: KucoinTimeInForce,
    // pub refu: i64,
    // pub is_reduce_only: bool,
    // pub is_close: bool,
    // pub is_liq: bool,
    pub text: OrderCid,
    pub status: String,
    // pub finish_time: i64,
    pub finish_as: Option<String>,
    // pub stp_id: i64,
    // pub stp_act: String,
    // pub amend_text: String,
}
impl FuturesHttpOpenOrder {
    pub fn status(&self) -> OrderStatus {
        match self.status.as_str() {
            "open" if self.left == self.size => OrderStatus::Open,
            "open" if self.left > 0 => OrderStatus::PartiallyFilled,
            "finished" => match self.finish_as.as_deref().expect("missing finish_as") {
                "cancelled" => OrderStatus::Cancelled,
                "filled" => OrderStatus::Filled,
                x => {
                    warn!("Unknown perpetual order finish_as: {}", x);
                    OrderStatus::Filled
                }
            },
            _ => {
                warn!("Unknown perpetual order status: {}", self.status);
                OrderStatus::Rejected
            }
        }
    }
    pub fn into_order(self, account: AccountId, instrument: &InstrumentDetails) -> Order {
        let mut size = instrument.size.multiply(self.size as f64);
        let side = Side::from_sign(size);
        size = size.abs();
        let left = instrument.size.multiply(self.left as f64);
        let status = self.status();
        Order {
            instrument: instrument.code_simple.clone(),
            account,
            server_id: self.id.into(),
            client_id: self.text,
            ty: OrderType::Limit,
            status,
            side,
            open_lt: Time::from_secs_f(self.create_time),
            price: self.price,
            size,
            open_tst: Time::from_secs_f(self.create_time),
            update_tst: Time::now(),
            update_est: Time::now(),
            update_lt: Time::now(),
            filled_size: size - left,
            average_filled_price: self.fill_price,
            ..Order::empty()
        }
    }
}
pub fn Kucoin_decode_http_open_orders(
    account: AccountId,
    data: &[u8],
    exchange: Exchange,
    manager: &InstrumentManager,
) -> eyre::Result<SyncOrders> {
    let mut sync_orders = SyncOrders::new(exchange, None).with_account(account);
    match exchange {
        Exchange::KucoinSpot | Exchange::KucoinMargin => {
            let orders: Vec<SpotHttpOpenOrderPerSymbol> = serde_json::from_slice(data)?;
            for order in orders.into_iter().flat_map(|o| o.orders.into_iter()) {
                let instrument = manager.maybe_lookup_instrument(exchange, order.currency_pair.clone());
                sync_orders.orders.push(order.into_order(account, instrument));
            }
        }
        Exchange::KucoinFutures => {
            let orders: Vec<FuturesHttpOpenOrder> = serde_json::from_slice(data)?;
            for order in orders.into_iter() {
                let instrument = manager.get_result(&(exchange, order.contract.clone()))?;
                sync_orders.orders.push(order.into_order(account, instrument));
            }
        }
        _ => {
            unreachable!("Unsupported exchange: {:?}", exchange);
        }
    }
    Ok(sync_orders)
}

// {
//   "id": "1852454420",
//   "text": "t-abc123",
//   "amend_text": "-",
//   "create_time": "1710488334",
//   "update_time": "1710488334",
//   "create_time_ms": 1710488334073,
//   "update_time_ms": 1710488334074,
//   "status": "closed",
//   "currency_pair": "BTC_USDT",
//   "type": "limit",
//   "account": "unified",
//   "side": "buy",
//   "amount": "0.001",
//   "price": "65000",
//   "time_in_force": "gtc",
//   "iceberg": "0",
//   "left": "0",
//   "filled_amount": "0.001",
//   "fill_price": "63.4693",
//   "filled_total": "63.4693",
//   "avg_deal_price": "63469.3",
//   "fee": "0.00000022",
//   "fee_currency": "BTC",
//   "point_fee": "0",
//   "gt_fee": "0",
//   "gt_maker_fee": "0",
//   "gt_taker_fee": "0",
//   "gt_discount": false,
//   "rebated_fee": "0",
//   "rebated_fee_currency": "USDT",
//   "finish_as": "filled"
// }

//#[serde_as]
//#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
//pub struct KucoinSpotNewOrderResponse {
//    pub id: OrderSid,
//    pub create_time_ms: TimeStampMs,
//    pub update_time_ms: TimeStampMs,
//    #[serde_as(as = "DisplayFromStr")]
//    pub price: Price,
//    #[serde_as(as = "DisplayFromStr")]
//    pub amount: Quantity,
//    #[serde_as(as = "DisplayFromStr")]
//    pub filled_amount: Quantity,
//}
/* 
impl GateioSpotNewOrderResponse {
   pub fn into_update_order(self, update: &mut UpdateOrder) {
        update.filled_size = self.filled_amount;
        update.server_id = self.id.into();
        update.price = self.price;
        update.size = self.amount;
        update.update_tst = Time::from_millis(self.update_time_ms);
        update.update_est = Time::from_millis(self.update_time_ms);

        update.open_est = Time::from_millis(self.create_time_ms);

        if self.filled_amount > 0.0 {
            if self.filled_amount < self.amount {
                update.status = OrderStatus::PartiallyFilled;
            } else if self.filled_amount >= self.amount {
                update.status = OrderStatus::Filled;
            }
        } else {
            update.status = OrderStatus::Open;
        }
    }
}
    */
// {
//   "id": 15675394,
//   "user": 100000,
//   "contract": "BTC_USDT",
//   "create_time": 1546569968,
//   "size": 6024,
//   "iceberg": 0,
//   "left": 6024,
//   "price": "3765",
//   "fill_price": "0",
//   "mkfr": "-0.00025",
//   "tkfr": "0.00075",
//   "tif": "gtc",
//   "refu": 0,
//   "is_reduce_only": false,
//   "is_close": false,
//   "is_liq": false,
//   "text": "t-my-custom-id",
//   "status": "finished",
//   "finish_time": 1514764900,
//   "finish_as": "cancelled",
//   "stp_id": 0,
//   "stp_act": "-",
//   "amend_text": "-"
// }
/* 
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct GateioPerpetualNewOrderResponse {
    pub id: u64,
    // pub user: i64,
    pub contract: Symbol,
    pub create_time: f64,
    pub size: i64,
    // pub iceberg: i64,
    // TODO: calculate left size later
    pub left: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub price: f64,
    // pub fill_price: String,
    // pub mkfr: String,
    // pub tkfr: String,
    // pub tif: String,
    // pub refu: i64,
    pub is_reduce_only: bool,
    pub is_close: bool,
    pub is_liq: bool,
    pub text: OrderCid,
    pub status: String,
    // pub finish_time: Option<i64>,
    // pub finish_as: String,
    // pub stp_id: i64,
    // pub stp_act: String,
    // pub amend_text: String,
}
impl GateioPerpetualNewOrderResponse {
    pub fn into_update_order(self, update: &mut UpdateOrder) {
        update.server_id = self.id.into();
        update.price = self.price;
        update.update_lt = Time::now();
        update.update_tst = Time::now();
        update.update_est = Time::now();
        update.open_est = Time::from_secs_f(self.create_time);
        update.status = match self.status.as_str() {
            "open" if self.left == self.size => OrderStatus::Open,
            "open" if self.left > 0 => OrderStatus::PartiallyFilled,
            "finished" => OrderStatus::Filled,
            _ => {
                warn!("Unknown perpetual order status: {}", self.status);
                OrderStatus::Rejected
            }
        }
    }
}
*/