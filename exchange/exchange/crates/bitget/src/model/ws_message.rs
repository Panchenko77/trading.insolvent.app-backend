use crate::model::{
    parse_bitget_ws_futures_order, parse_bitget_ws_spot_order, parse_bitget_ws_position,
   BitgetWsPositionInfo,  BitgetWsSpotOrder,BitgetWsFuturesOrder, WsMessage,
};
use eyre::Result;
use serde::{Deserialize, Serialize};
use trading_exchange_core::model::{AccountId, ExecutionResponse};
use trading_model::model::SharedInstrumentManager;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsMessageEnum {
    Position(WsMessage<BitgetWsPositionInfo>),
    SpotOrder(WsMessage<BitgetWsSpotOrder>),
    FuturesOrder(WsMessage<BitgetWsFuturesOrder>),
    #[serde(other)]
    Other,
}

pub fn parse_bitget_ws_message(
    account: AccountId,

    message: &str,
    manager: Option<SharedInstrumentManager>,
) -> Result<ExecutionResponse> {
    let ws_message: WsMessageEnum = serde_json::from_str(message)?;
    match ws_message {
        WsMessageEnum::Position(msg) => {
            parse_bitget_ws_position(account, msg, manager).map(ExecutionResponse::UpdatePositions)
        }
        WsMessageEnum::SpotOrder(msg) => {
            parse_bitget_ws_spot_order(account, msg, manager).map(ExecutionResponse::SyncOrders)
        }
        WsMessageEnum::FuturesOrder(msg) => {
            parse_bitget_ws_futures_order(account, msg, manager).map(ExecutionResponse::SyncOrders)

    }
     _ => Ok(ExecutionResponse::Group(vec![])),


    }
 }

