use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use trading_exchange_core::model::{
    AccountId, ExecutionResponse, OrderStatus, UpdateOrder, UpdatePosition, UpdatePositionAddValues, UpdatePositions,
};
use trading_model::core::{Time, NANOSECONDS_PER_MILLISECOND};
use trading_model::model::{Asset, Exchange, InstrumentCode, InstrumentManagerExt, SharedInstrumentManager, Symbol};

use crate::model::order::{parse_binance_order_type, BinanceOrderStatus};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    #[serde(alias = "a")]
    pub asset: Asset, // Asset

    #[serde(alias = "f")]
    #[serde_as(as = "DisplayFromStr")]
    pub free: f64, // Free

    #[serde(alias = "l")]
    #[serde_as(as = "DisplayFromStr")]
    pub locked: f64, // Locked
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountPosition {
    #[serde(rename = "E")]
    pub event_time: u64, // Event Time

    #[serde(rename = "u")]
    pub time_of_last_update: u64, // Time of last account update

    #[serde(rename = "B")]
    pub balances: Vec<Balance>, // Balances Array
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceUpdate {
    #[serde(rename = "E")]
    pub event_time: i64, // Event Time

    #[serde(rename = "a")]
    pub asset: Asset,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "d")]
    pub balance_delta: f64, // Balance Delta

    #[serde(rename = "T")]
    pub clear_time: i64, // Clear Time
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionReport {
    #[serde(rename = "E")]
    pub event_time: i64, // Event time

    #[serde(rename = "s")]
    pub symbol: Symbol, // Symbol

    #[serde(rename = "c")]
    pub client_order_id: String, // Client order ID

    #[serde(rename = "S")]
    pub side: String, // Side

    #[serde(rename = "o")]
    pub order_type: String, // Order type

    #[serde(rename = "f")]
    pub time_in_force: String, // Time in force

    #[serde(rename = "q")]
    #[serde_as(as = "DisplayFromStr")]
    pub order_quantity: f64, // Order quantity

    #[serde(rename = "p")]
    #[serde_as(as = "DisplayFromStr")]
    pub order_price: f64, // Order price

    #[serde(rename = "P")]
    #[serde_as(as = "DisplayFromStr")]
    pub stop_price: f64, // Stop price

    #[serde(rename = "F")]
    #[serde_as(as = "DisplayFromStr")]
    pub iceberg_quantity: f64, // Iceberg quantity

    #[serde(rename = "g")]
    pub order_list_id: i64, // OrderListId

    #[serde(rename = "C")]
    pub original_client_order_id: String, // Original client order ID; This is the ID of the order being canceled

    #[serde(rename = "x")]
    pub current_execution_type: String, // Current execution type

    #[serde(rename = "X")]
    pub current_order_status: BinanceOrderStatus, // Current order status

    #[serde(rename = "r")]
    pub order_reject_reason: String, // Order reject reason; will be an error code.

    #[serde(rename = "i")]
    pub order_id: u64, // Order ID

    #[serde(rename = "l")]
    #[serde_as(as = "DisplayFromStr")]
    pub last_executed_quantity: f64, // Last executed quantity

    #[serde(rename = "z")]
    #[serde_as(as = "DisplayFromStr")]
    pub cumulative_filled_quantity: f64, // Cumulative filled quantity

    #[serde(rename = "L")]
    #[serde_as(as = "DisplayFromStr")]
    pub last_executed_price: f64, // Last executed price

    #[serde(rename = "n")]
    #[serde_as(as = "DisplayFromStr")]
    pub commission_amount: f64, // Commission amount

    #[serde(rename = "N")]
    pub commission_asset: Option<String>, // Commission asset

    #[serde(rename = "T")]
    pub transaction_time: u64, // Transaction time

    #[serde(rename = "t")]
    pub trade_id: i64, // Trade ID

    #[serde(rename = "I")]
    pub ignore: i64, // Ignore

    #[serde(rename = "w")]
    pub is_order_on_book: bool, // Is the order on the book?

    #[serde(rename = "m")]
    pub is_trade_maker_side: bool, // Is this trade the maker side?

    #[serde(rename = "M")]
    pub ignore_2: bool, // Ignore

    #[serde(rename = "O")]
    pub order_creation_time: u64, // Order creation time

    #[serde(rename = "Z")]
    #[serde_as(as = "DisplayFromStr")]
    pub cumulative_quote_asset_transacted_quantity: f64, // Cumulative quote asset transacted quantity

    #[serde(rename = "Y")]
    #[serde_as(as = "DisplayFromStr")]
    pub last_quote_asset_transacted_quantity: f64, // Last quote asset transacted quantity (i.e. lastPrice * lastQty)

    #[serde(rename = "Q")]
    #[serde_as(as = "DisplayFromStr")]
    pub quote_order_quantity: f64, // Quote Order Quantity

    #[serde(rename = "W")]
    pub working_time: u64, // Working Time; This is only visible if the order has been placed on the book.

    #[serde(rename = "V")]
    pub self_trade_prevention_mode: String, // selfTradePreventionMode
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "e")]
pub enum BinanceSpotWebsocketData {
    #[serde(rename = "outboundAccountPosition")]
    AccountPosition(AccountPosition),

    #[serde(rename = "balanceUpdate")]
    BalanceUpdate(BalanceUpdate),

    #[serde(rename = "executionReport")]
    ExecutionReport(ExecutionReport),
}

pub fn decode_binance_spot_websocket_message(
    account: AccountId,
    exchange: Exchange,
    data: &str,
    manager: Option<SharedInstrumentManager>,
) -> Result<Option<ExecutionResponse>> {
    let data: BinanceSpotWebsocketData = serde_json::from_str(data)?;
    match data {
        BinanceSpotWebsocketData::AccountPosition(_) => Ok(None),
        BinanceSpotWebsocketData::BalanceUpdate(update) => {
            let mut update_wallet = UpdatePositions::update(account, exchange);
            update_wallet.positions.push(UpdatePosition {
                account,
                instrument: InstrumentCode::from_asset(exchange, update.asset),
                times: (
                    update.event_time * NANOSECONDS_PER_MILLISECOND,
                    update.clear_time * NANOSECONDS_PER_MILLISECOND,
                )
                    .into(),
                add_values: Some(UpdatePositionAddValues {
                    delta_total: 0.0,
                    delta_available: update.balance_delta,
                    delta_locked: 0.0,
                }),
                ..UpdatePosition::empty()
            });

            Ok(Some(ExecutionResponse::UpdatePositions(update_wallet)))
        }
        BinanceSpotWebsocketData::ExecutionReport(execution_report) => {
            let instrument = manager.maybe_lookup_instrument(exchange, execution_report.symbol);
            let status = execution_report.current_order_status.into();
            let update_order = UpdateOrder {
                account,
                instrument,
                server_id: execution_report.order_id.into(),
                client_id: if execution_report.original_client_order_id.is_empty() {
                    execution_report.client_order_id.into()
                } else {
                    execution_report.original_client_order_id.into()
                },
                status: if status == OrderStatus::PartiallyFilled
                    && execution_report.cumulative_filled_quantity == execution_report.order_quantity
                {
                    OrderStatus::Filled
                } else {
                    status
                },
                side: execution_report.side.parse()?,
                ty: parse_binance_order_type(exchange, &execution_report.order_type)?,
                price: execution_report.order_price,
                size: execution_report.order_quantity,
                filled_size: execution_report.cumulative_filled_quantity,
                update_lt: Time::now(),
                update_est: Time::from_millis(execution_report.event_time),
                update_tst: Time::from_millis(execution_report.event_time),
                average_filled_price: execution_report.last_executed_price,
                last_filled_size: execution_report.last_executed_quantity,
                last_filled_price: execution_report.last_executed_price,
                ..UpdateOrder::empty()
            };
            Ok(Some(ExecutionResponse::UpdateOrder(update_order)))
        }
    }
}
