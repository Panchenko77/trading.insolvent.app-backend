use crate::get_bitget_order_lid;
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
    InstrumentCode,InstrumentDetails, InstrumentManagerExt, InstrumentSelector, Price, Quantity, SharedInstrumentManager, Side, Symbol,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum BitgetOrderType {
    Limit,
    Market,
}

impl Into<OrderType> for BitgetOrderType {
    fn into(self) -> OrderType {
        match self {
            Self::Limit => OrderType::Limit,
            Self::Market => OrderType::Market,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BitgetTimeInForce {
    GTC,
    IOC,
    FOK,
}

impl Into<TimeInForce> for BitgetTimeInForce {
    fn into(self) -> TimeInForce {
        match self {
            Self::GTC => TimeInForce::GoodTilCancel,
            Self::IOC => TimeInForce::ImmediateOrCancel,
            Self::FOK => TimeInForce::FillOrKill,
        }
    }
}

//decoder for spot
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct BitgetHttpLiveSpotOrder {
    userId: String,
    symbol: Symbol,
    orderId: OrderSid,
    clientOid: OrderCid,
    #[serde_as(as = "DisplayFromStr")]
    priceAvg: f64,
    #[serde_as(as = "DisplayFromStr")]
    size: f64,
    #[serde(rename = "orderType")]
    order_type: OrderType,
    side: Side,
    status: BitgetOrderStatus,
    #[serde_as(as = "DisplayFromStr")]
    basePrice: f64,
    #[serde_as(as = "DisplayFromStr")]
    baseVolume: f64,
    #[serde_as(as = "DisplayFromStr")]
    quoteVolume: f64,
    //enterPointSource: String,
    //orderSource: String,
    #[serde_as(as = "DisplayFromStr")]
    cTime: TimeStampMs, // Unix millisecond timestamp
    #[serde_as(as = "DisplayFromStr")]
    uTime: TimeStampMs, // Unix millisecond timestamp
    //#[serde_as(as = "DisplayFromStr")]
    triggerPrice: f64,
    //tpslType: String,
    reduceOnly: bool,
    time_in_force: BitgetTimeInForce,

}

impl BitgetHttpLiveSpotOrder {
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
          //  effect: self.get_pos_effect(),
            side: self.side,
            price: self.priceAvg,
            size: self.size,
            filled_size: self.baseVolume,
            average_filled_price: self.priceAvg / self.baseVolume,
            local_id: get_bitget_order_lid(&self.orderId),
            client_id: self.clientOid,
            server_id: self.orderId,
            status: self.status.into(),
            open_lt: Time::NULL,
            open_tst: Time::from_millis(self.cTime),
            update_lt: Time::NULL,
            update_est: Time::from_millis(self.uTime),
            update_tst: Time::NULL,
            ty: self.order_type,
            tif: self.time_in_force.into(),
            ..Order::empty()
        }
    }
}

pub fn decode_http_spot_orders(
    range: InstrumentSelector,
    data: &str,
    manager: Option<SharedInstrumentManager>,
) -> Result<ExecutionResponse, String> {
    let mut sync_orders = SyncOrders::empty();
    sync_orders.range = range;
    let orders: ResponseDataListed<BitgetHttpLiveSpotOrder> =
        serde_json::from_str(data).expect("failed to decode_http_open_spot_orders");

        let Some(result) = orders.data.into_option() else {
            return Err(format!(
                "failed to decode http open orders: {}: {}",
                orders.code, orders.message
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
pub struct BitgetWsSpotOrder {
    #[serde(rename = "instId")]
    symbol: Symbol,
    orderId: OrderSid,
    clientOid: OrderCid,
    #[serde_as(as = "DisplayFromStr")]
    size: Quantity,
    newSize: String,
    #[serde_as(as = "DisplayFromStr")]
    notional: f64,
    #[serde(rename = "orderType")]
    order_type: OrderType,
    #[serde(rename = "force")]
    time_in_force: BitgetTimeInForce,
    side: Side,
    #[serde_as(as = "DisplayFromStr")]
    fillPrice: Price,
    tradeId: String,
    #[serde_as(as = "DisplayFromStr")]
    baseVolume: f64,
    fillTime: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    fillFee: f64,
    fillFeeCoin: String,
    tradeScope: String,
    #[serde_as(as = "DisplayFromStr")]
    accBaseVolume: f64,
    #[serde_as(as = "DisplayFromStr")]
    priceAvg: Price,
    #[serde(rename = "status")]
    order_status: OrderStatus,
    #[serde(rename = "cTime")]
    created_time: TimeStampMs,
    #[serde(rename = "uTime")]
    updated_time: TimeStampMs,
    stpMode: String,
    feeDetail: Vec<String>,
    enterPointSource: String,
}
impl BitgetWsSpotOrder {
   // pub fn get_pos_effect(&self) -> PositionEffect {
     //   match self.side {
       //     Side::Buy => PositionEffect::Open,
         //   Side::Sell => PositionEffect::Close,
       // }
//    }

    pub fn into_order(self, instrument: InstrumentCode) -> Order {
        Order {
            instrument,
          //  effect: self.get_pos_effect(),
            side: self.side,
            price: self.priceAvg,
            size: self.size,
            filled_size: self.accBaseVolume,
            average_filled_price: self.priceAvg / self.accBaseVolume,
            local_id: get_bitget_order_lid(&self.orderId),
            client_id: self.clientOid,
            server_id: self.orderId,
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

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BitgetWsFuturesOrder {
    accBaseVolume: Quantity,
    #[serde(rename = "cTime")]
    created_time: TimeStampMs,
    clientOId: OrderCid,
    feeDetail: Vec<String>,
    fillFee: f64,
    fillFeeCoin: String,
    fillNotionalUsd: f64,
    #[serde_as(as = "DisplayFromStr")]
    fillPrice: Price,
    #[serde_as(as = "DisplayFromStr")]
    baseVolume: f64,
    fillTime: TimeStampMs,
    #[serde(rename = "force")]
    time_in_force: BitgetTimeInForce,
    #[serde(rename = "instId")]
    symbol: Symbol,
    leverage: f64,
    marginCoin: String,
    marginMode: String,
    notionalUsd: f64,
    orderId: OrderSid,
    #[serde(rename = "orderType")]
    order_type: OrderType,
    pnl: f64,
    posMode: String,
    posSide: Side,
    #[serde_as(as = "DisplayFromStr")]
    price: Price,
    #[serde_as(as = "DisplayFromStr")]
    priceAvg: Price,
    #[serde(rename = "reduceOnly")]
    reduce_only: String,
    stpMode: String,
    side: Side,
    size: Quantity,
    enterPointSource: String,
    #[serde(rename = "status")]
    order_status: OrderStatus,
    tradeScope: String,
    tradeId: String,
    tradeSide: String,
#[serde(rename = "uTime")]
    updated_time: TimeStampMs,
}




impl BitgetWsFuturesOrder {
    pub fn get_pos_effect(&self) -> PositionEffect {
        if self.reduce_only == "yes" {
            PositionEffect::Close
        } else {
            PositionEffect::NA
        }
    }
    pub fn into_order(self, instrument: InstrumentCode) -> Order {
        Order { instrument,
            effect: self.get_pos_effect(),
            side: self.side,
            price: self.price,
            size: self.size,
            filled_size: self.accBaseVolume,
            average_filled_price: self.priceAvg / self.accBaseVolume,
          //  stop_price: self.trigger_price.parse().unwrap(),
            local_id: get_bitget_order_lid(&self.orderId),
            client_id: self.clientOId,
            server_id: self.orderId,
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


#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BitgetOrderStatus {
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

impl Into<OrderStatus> for BitgetOrderStatus {
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

pub fn parse_bitget_ws_futures_order(
    account: AccountId,

    msg: WsMessage<BitgetWsFuturesOrder>,
    manager: Option<SharedInstrumentManager>,
) -> Result<SyncOrders> {
    let mut sync_orders = SyncOrders::new(Exchange::Bitget, None);
    sync_orders.full = false;
    for order_changes in msg.data {
        let instrument = manager.maybe_lookup_instrument(Exchange::Bitget, order_changes.symbol.clone());

        let mut order = order_changes.into_order(instrument);
        order.account = account;

        sync_orders.orders.push(order);
    }
    Ok(sync_orders)
}
pub fn parse_bitget_ws_spot_order(
    account: AccountId,

    msg: WsMessage<BitgetWsSpotOrder>,
    manager: Option<SharedInstrumentManager>,
) -> Result<SyncOrders> {
    let mut sync_orders = SyncOrders::new(Exchange::Bitget, None);
    sync_orders.full = false;
    for order_changes in msg.data {
        let instrument = manager.maybe_lookup_instrument(Exchange::Bitget, order_changes.symbol.clone());

        let mut order = order_changes.into_order(instrument);
        order.account = account;

        sync_orders.orders.push(order);
    }
    Ok(sync_orders)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitgetOrderExecution {
   }

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetCreateOrder {
    pub order_id: OrderSid,
    pub order_link_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetCancelOrder {
    pub order_id: OrderSid,
    pub order_link_id: String,
}


