use crate::model::exchange::request::{HyperliquidOrderType, HyperliquidTif};
use crate::model::exchange::response::Status;
use crate::HYPERLIQUID;
use eyre::bail;
use trading_exchange_core::model::{FundingLid, OrderLid, OrderStatus, OrderType, TimeInForce, TradeLid};
use trading_model::Side;
use uuid::Uuid;

pub(crate) fn trim_float_in_string_for_hashing(x: &mut String) -> &str {
    // x.trim_end_matches('0').trim_end_matches('.')
    while x.ends_with('0') {
        x.pop();
    }
    if x.ends_with('.') {
        x.pop();
    }
    x.as_str()
}

pub fn uuid_to_hex_string(uuid: Uuid) -> String {
    format!("0x{}", uuid.simple())
}

pub fn convert_tif(tif: TimeInForce) -> HyperliquidTif {
    match tif {
        TimeInForce::GoodTilCancel => HyperliquidTif::Gtc,
        TimeInForce::ImmediateOrCancel => HyperliquidTif::Ioc,
        _ => panic!("Unsupported TIF: {:?}", tif),
    }
}

pub fn convert_order_type(ty: OrderType, tif: TimeInForce) -> eyre::Result<HyperliquidOrderType> {
    Ok(match ty {
        OrderType::PostOnly => HyperliquidOrderType::Limit {
            tif: HyperliquidTif::Alo,
        },
        OrderType::Limit => HyperliquidOrderType::Limit { tif: convert_tif(tif) },
        OrderType::Market => HyperliquidOrderType::Limit {
            // if we don't do FrontendMarket, we can't close positions less than $10
            tif: HyperliquidTif::FrontendMarket,
        },
        _ => bail!("Unsupported order type: {:?}", ty),
    })
}

pub fn convert_status(status: Status) -> OrderStatus {
    match status {
        Status::Resting(_) => OrderStatus::Open,
        Status::Filled(_) => OrderStatus::Filled, // TODO: check if partially filled
        Status::Error(_) => OrderStatus::Rejected,
        Status::Success => OrderStatus::Open,
        Status::WaitingForFill => OrderStatus::Open,
        Status::WaitingForTrigger => OrderStatus::Open,
    }
}

pub fn create_trade_lid(coin: &str, hash: &str, start_position: &str) -> TradeLid {
    TradeLid(format!("{HYPERLIQUID}|{coin}|{hash}|{start_position}").into())
}

pub fn create_order_lid(order_id: u64) -> OrderLid {
    format!("{HYPERLIQUID}|{order_id}").into()
}

pub fn create_order_lid_str(order_id: &str) -> OrderLid {
    format!("{HYPERLIQUID}|{order_id}").into()
}

pub fn create_funding_lid(coin: &str, time: i64) -> FundingLid {
    FundingLid(format!("{HYPERLIQUID}|{}|{}", coin, time))
}

pub fn hyperliquid_parse_side(side: char) -> Side {
    match side {
        'B' => Side::Buy,
        'S' => Side::Sell,
        'A' => Side::Sell,
        _ => panic!("Invalid side: {}", side),
    }
}
