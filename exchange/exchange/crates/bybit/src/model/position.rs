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
    InstrumentCategory, InstrumentManagerExt, InstrumentSelector, Price, Quantity,
    SharedInstrumentManager, Symbol,
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BybitPositionInfo {
    // #[serde(rename = "positionIdx")]
    // pub position_idx: i64,
    // #[serde(rename = "riskId")]
    // pub risk_id: i64,
    // #[serde(rename = "riskLimitValue")]
    // pub risk_limit_value: String,
    pub symbol: Symbol,
    // TODO: handle position side
    // pub side: PositionSide,
    #[serde_as(as = "DisplayFromStr")]
    pub size: Quantity,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "avgPrice")]
    pub avg_price: Price,
    // #[serde(rename = "positionValue")]
    // pub position_value: String,
    // #[serde(rename = "tradeMode")]
    // pub trade_mode: i64,
    // #[serde(rename = "positionStatus")]
    // pub position_status: String,
    // #[serde(rename = "autoAddMargin")]
    // pub auto_add_margin: i64,
    // #[serde(rename = "adlRankIndicator")]
    // pub adl_rank_indicator: i64,
    // pub leverage: String,
    // #[serde(rename = "positionBalance")]
    // pub position_balance: String,
    // #[serde(rename = "markPrice")]
    // pub mark_price: String,
    // #[serde(rename = "liqPrice")]
    // pub liq_price: String,
    // #[serde(rename = "bustPrice")]
    // pub bust_price: String,
    // #[serde(rename = "positionMM")]
    // pub position_mm: String,
    // #[serde(rename = "positionIM")]
    // pub position_im: String,
    // #[serde(rename = "tpslMode")]
    // pub tpsl_mode: String,
    // #[serde(rename = "takeProfit")]
    // pub take_profit: String,
    // #[serde(rename = "stopLoss")]
    // pub stop_loss: String,
    // #[serde(rename = "trailingStop")]
    // pub trailing_stop: String,
    // #[serde(rename = "unrealisedPnl")]
    // pub unrealised_pnl: String,
    // #[serde(rename = "cumRealisedPnl")]
    // pub cum_realised_pnl: String,
    // pub seq: i64,
    // #[serde(rename = "isReduceOnly")]
    // pub is_reduce_only: bool,
    // #[serde(rename = "mmrSysUpdateTime")]
    // pub mmr_sys_update_time: String,
    // #[serde(rename = "leverageSysUpdatedTime")]
    // pub leverage_sys_updated_time: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "createdTime")]
    pub created_time: TimeStampMs,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "updatedTime")]
    pub updated_time: TimeStampMs,
}

pub fn parse_user_positions(
    account: AccountId,

    settlement: Asset,
    response: eyre::Result<String>,
    manager: Option<SharedInstrumentManager>,
) -> ExecutionResponse {
    let exchange = Exchange::Bybit;
    match response {
        Ok(resp) => {
            let resp: ResponseDataListed<BybitPositionInfo> =
                serde_json::from_str(&resp).expect("failed to decode query user position");
            let Some(result) = resp.result.into_option() else {
                return ExecutionResponse::Error(format!(
                    "failed to decode query user assets: {}: {}",
                    resp.retCode, resp.retMsg
                ));
            };
            let est = resp.time * NANOSECONDS_PER_MILLISECOND;
            let mut update = UpdatePositions::sync_range(
                account,
                InstrumentSelector::CategoryQuote(
                    exchange,
                    InstrumentCategory::Futures,
                    settlement,
                ),
            );
            update.extend_updates(result.list.into_iter().map(|p| {
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

            ExecutionResponse::UpdatePositions(update)
        }

        Err(err) => ExecutionResponse::Error(err.to_string()),
    }
}

pub fn parse_bybit_ws_position(
    account: AccountId,
    msg: WsMessage<BybitPositionInfo>,
    manager: Option<SharedInstrumentManager>,
) -> eyre::Result<UpdatePositions> {
    let exchange = Exchange::Bybit;
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
