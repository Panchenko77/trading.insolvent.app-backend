use crate::model::{
    parse_bybit_ws_order, parse_bybit_ws_position, parse_bybit_ws_wallet_balance,
    BybitOrderExecution, BybitPositionInfo, BybitWalletBalanceRoot, BybitWsOrder, WsMessage,
};
use eyre::Result;
use serde::{Deserialize, Serialize};
use trading_exchange_core::model::{AccountId, ExecutionResponse};
use trading_model::model::SharedInstrumentManager;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsMessageEnum {
    Position(WsMessage<BybitPositionInfo>),
    Order(WsMessage<BybitWsOrder>),
    Wallet(WsMessage<BybitWalletBalanceRoot>),
    Execution(WsMessage<BybitOrderExecution>),
    #[serde(other)]
    Other,
}

pub fn parse_bybit_ws_message(
    account: AccountId,

    message: &str,
    manager: Option<SharedInstrumentManager>,
) -> Result<ExecutionResponse> {
    let ws_message: WsMessageEnum = serde_json::from_str(message)?;
    match ws_message {
        WsMessageEnum::Position(msg) => {
            parse_bybit_ws_position(account, msg, manager).map(ExecutionResponse::UpdatePositions)
        }
        WsMessageEnum::Order(msg) => {
            parse_bybit_ws_order(account, msg, manager).map(ExecutionResponse::SyncOrders)
        }
        WsMessageEnum::Wallet(msg) => {
            parse_bybit_ws_wallet_balance(account, msg).map(ExecutionResponse::UpdatePositions)
        }
        // WsMessageEnum::Execution(msg) => Ok(ExecutionResponse::TradeOrder(msg.data)),
        _ => Ok(ExecutionResponse::Group(vec![])),
    }
}
