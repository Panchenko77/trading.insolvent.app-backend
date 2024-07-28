use crate::model::{ResponseDataListed, WsMessage};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use trading_exchange_core::model::{
    AccountId, ExecutionResponse, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_model::core::{Time, TimeStampMs, NANOSECONDS_PER_MILLISECOND};
use trading_model::model::{Asset, Exchange};
use trading_model::{
    InstrumentCategory, InstrumentManagerExt, InstrumentSelector, Price, Quantity, Side,
    SharedInstrumentManager, Symbol,
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BitgetPositionInfo {
    margin_coin: Asset,
    symbol: Symbol,
    hold_side: Side,
    open_delegate_size: f64,
    margin_size: f64,
    available: f64,
    locked: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "total")]
    size: Quantity,
    leverage: i32,
    achieved_profits: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "openeEntryPrice")]
    avg_price: Price,
    margin_mode: String,
    pos_mode: String,
    unrealized_pl: f64,
    liquidation_price: f64,
    keep_margin_rate: f64,
    mark_price: f64,
    break_even_price: f64,
    total_fee: f64,
    deducted_fee: f64,
    margin_ratio: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "Ctime")]
    c_time: TimeStampMs,

}
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BitgetWsPositionInfo {
    pos_id: String,
    #[serde(rename = "instId")]
    symbol: Symbol,
    #[serde(rename = "marginCoin")]
    margin_coin: Asset,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "marginSize")]
    margin_size: f64,
    #[serde(rename = "marginMode")]
    margin_mode: String,
    #[serde(rename = "holdSide")]
    hold_side: Side,
    #[serde(rename = "posMode")]
    pos_mode: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "total")]
    size: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    available: f64,
    #[serde_as(as = "DisplayFromStr")]
    frozen: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "openPriceAvg")]
    avg_price: Price,
    leverage: i32,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "achievedProfits")]
    achieved_profits: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "unrealizedPL")]
    unrealized_pl: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "unrealizedPLR")]
    unrealized_plr: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "liquidationPrice")]
    liquidation_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "keepMarginRate")]
    keep_margin_rate: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "marginRate")]
    margin_ratio: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cTime")]
    c_time: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "breakEvenPrice")]
    break_even_price: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "totalFee")]
    total_fee: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "deductedFee")]
    deducted_fee: f64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "uTime")]
    updated_time: TimeStampMs,
    #[serde(rename = "autoMargin")]
    auto_margin: String,
}


pub fn parse_user_positions(
    account: AccountId,

    settlement: Asset,
    response: eyre::Result<String>,
    manager: Option<SharedInstrumentManager>,
) -> ExecutionResponse {
    let exchange = Exchange::Bitget;
    match response {
        Ok(resp) => {
            let resp: ResponseDataListed<BitgetPositionInfo> =
                serde_json::from_str(&resp).expect("failed to decode query user position");
            let Some(result) = resp.data.into_option() else {
                return ExecutionResponse::Error(format!(
                    "failed to decode query user assets: {}: {}",
                    resp.code, resp.message
                ));
            };
            let est = resp.requestTime * NANOSECONDS_PER_MILLISECOND;
            let mut update = UpdatePositions::sync_range(
                account,
                InstrumentSelector::CategoryQuote(
                    exchange,
                    InstrumentCategory::Futures,
                    settlement,
                ),
            );
            update.extend_updates(result.list.into_iter().map(|p| {
                let tst = p.c_time * NANOSECONDS_PER_MILLISECOND;
                let instrument = manager.maybe_lookup_instrument_with_category(
                    exchange,
                    p.symbol,
                    InstrumentCategory::Futures,
                );

                UpdatePosition {
                    instrument,
                    times: (est, tst).into(),
                    set_values: Some(UpdatePositionSetValues {
                        total: p.size,
                        available: p.size,
                        locked: 0.0,
                    }),
                    entry_price: Some(p.avg_price),
                    ..UpdatePosition::empty()
                }
            }));

            ExecutionResponse::UpdatePositions(update)
        }

        Err(err) => ExecutionResponse::Error(err.to_string()),
    }
}

pub fn parse_bitget_ws_position(
    account: AccountId,
    msg: WsMessage<BitgetWsPositionInfo>,
    manager: Option<SharedInstrumentManager>,
) -> eyre::Result<UpdatePositions> {
    let exchange = Exchange::Bitget;
    let mut update = UpdatePositions::sync_position(account, exchange);
    let est = msg.creation_time * NANOSECONDS_PER_MILLISECOND;
    update.exchange_time = Time::from_millis(msg.creation_time);

    update.extend_updates(msg.data.into_iter().map(|p| {
        let tst = p.updated_time * NANOSECONDS_PER_MILLISECOND;
        let instrument = manager.maybe_lookup_instrument_with_category(
            exchange,
            p.symbol,
            InstrumentCategory::Futures,
        );

        UpdatePosition {
            instrument,
            times: (est, tst).into(),
            set_values: Some(UpdatePositionSetValues {
                total: p.size,
                available: p.size,
                locked: 0.0,
            }),
            entry_price: Some(p.avg_price),
            ..UpdatePosition::empty()
        }
    }));

    Ok(update)
}
