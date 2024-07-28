use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use trading_exchange_core::model::{
    AccountId, ExecutionResponse, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_model::core::NANOSECONDS_PER_MILLISECOND;
use trading_model::model::{Asset, Exchange};
use trading_model::{InstrumentCode, Quantity};

use crate::model::{ResponseDataListed, WsMessage};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct BybitWalletBalanceCoin {
    #[serde(rename = "availableToBorrow")]
    pub available_to_borrow: String,
    pub bonus: String,
    #[serde(rename = "accruedInterest")]
    pub accrued_interest: String,
    #[serde(rename = "availableToWithdraw")]
    pub available_to_withdraw: String,
    #[serde(rename = "totalOrderIM")]
    pub total_order_im: String,
    pub equity: String,
    #[serde(rename = "totalPositionMM")]
    pub total_position_mm: String,
    #[serde(rename = "usdValue")]
    pub usd_value: String,
    #[serde(rename = "spotHedgingQty")]
    pub spot_hedging_qty: String,
    #[serde(rename = "unrealisedPnl")]
    pub unrealised_pnl: String,
    #[serde(rename = "collateralSwitch")]
    pub collateral_switch: bool,
    #[serde(rename = "borrowAmount")]
    pub borrow_amount: String,
    #[serde(rename = "totalPositionIM")]
    pub total_position_im: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "walletBalance")]
    pub wallet_balance: Quantity,
    #[serde(rename = "cumRealisedPnl")]
    pub cum_realised_pnl: String,
    #[serde_as(as = "DisplayFromStr")]
    pub locked: Quantity,
    #[serde(rename = "marginCollateral")]
    pub margin_collateral: bool,
    pub coin: Asset,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BybitWalletBalanceRoot {
    #[serde(rename = "totalEquity")]
    pub total_equity: String,
    #[serde(rename = "accountIMRate")]
    pub account_imrate: String,
    #[serde(rename = "totalMarginBalance")]
    pub total_margin_balance: String,
    #[serde(rename = "totalInitialMargin")]
    pub total_initial_margin: String,
    #[serde(rename = "accountType")]
    pub account_type: String,
    #[serde(rename = "totalAvailableBalance")]
    pub total_available_balance: String,
    #[serde(rename = "accountMMRate")]
    pub account_mmrate: String,
    #[serde(rename = "totalPerpUPL")]
    pub total_perp_upl: String,
    #[serde(rename = "totalWalletBalance")]
    pub total_wallet_balance: String,
    #[serde(rename = "accountLTV")]
    pub account_ltv: String,
    #[serde(rename = "totalMaintenanceMargin")]
    pub total_maintenance_margin: String,
    pub coin: Vec<BybitWalletBalanceCoin>,
}

pub fn parse_wallet_balance(
    account: AccountId,
    response: eyre::Result<String>,
) -> Result<ExecutionResponse, String> {
    let exchange = Exchange::Bybit;
    let resp = response.map_err(|e| e.to_string())?;
    let wallet: ResponseDataListed<BybitWalletBalanceRoot> =
        serde_json::from_str(&resp).expect("failed to decode query user assets");
    let Some(result) = wallet.result.into_option() else {
        return Err(format!(
            "failed to decode wallet_balance: {}: {}",
            wallet.retCode, wallet.retMsg
        ));
    };
    let est = wallet.time * NANOSECONDS_PER_MILLISECOND;
    let mut update = UpdatePositions::sync_balance(account, exchange);
    update.extend_updates(
        result
            .list
            .into_iter()
            .map(|r| {
                r.coin.into_iter().map(|b| {
                    let instrument = InstrumentCode::from_asset(exchange, b.coin);

                    UpdatePosition {
                        account,
                        instrument,
                        times: (est, est).into(),
                        set_values: Some(UpdatePositionSetValues {
                            total: b.wallet_balance,
                            available: b.wallet_balance - b.locked,
                            locked: b.locked,
                        }),
                        ..UpdatePosition::empty()
                    }
                })
            })
            .flatten(),
    );

    Ok(ExecutionResponse::UpdatePositions(update))
}

pub fn parse_bybit_ws_wallet_balance(
    account: AccountId,

    msg: WsMessage<BybitWalletBalanceRoot>,
) -> eyre::Result<UpdatePositions> {
    let exchange = Exchange::Bybit;
    let mut update = UpdatePositions::update(account, exchange);
    let est = msg.creation_time * NANOSECONDS_PER_MILLISECOND;

    update.extend_updates(
        msg.data
            .into_iter()
            .map(|r| {
                r.coin.into_iter().map(|b| {
                    let instrument = InstrumentCode::from_asset(exchange, b.coin);

                    UpdatePosition {
                        account,
                        instrument,
                        times: (est, est).into(),
                        set_values: Some(UpdatePositionSetValues {
                            total: b.wallet_balance,
                            available: b.wallet_balance - b.locked,
                            locked: b.locked,
                        }),
                        ..UpdatePosition::empty()
                    }
                })
            })
            .flatten(),
    );

    Ok(update)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wallet_balance() {
        let data = r#"{
  "retCode": 0,
  "retMsg": "OK",
  "result": {
    "list": [
      {
        "totalEquity": "21.621109",
        "accountIMRate": "0",
        "totalMarginBalance": "21.621109",
        "totalInitialMargin": "0",
        "accountType": "UNIFIED",
        "totalAvailableBalance": "3.64689521",
        "accountMMRate": "0",
        "totalPerpUPL": "0",
        "totalWalletBalance": "21.621109",
        "accountLTV": "0",
        "totalMaintenanceMargin": "0",
        "coin": [
          {
            "availableToBorrow": "",
            "bonus": "0",
            "accruedInterest": "0",
            "availableToWithdraw": "3.648475",
            "totalOrderIM": "0",
            "equity": "21.630475",
            "totalPositionMM": "0",
            "usdValue": "21.621109",
            "unrealisedPnl": "0",
            "collateralSwitch": true,
            "spotHedgingQty": "0",
            "borrowAmount": "0.000000000000000000",
            "totalPositionIM": "0",
            "walletBalance": "21.630475",
            "cumRealisedPnl": "0",
            "locked": "17.982",
            "marginCollateral": true,
            "coin": "USDT"
          }
        ]
      }
    ]
  },
  "retExtInfo": {},
  "time": 1707272128301
}
        "#;
        let result = parse_wallet_balance(0, Ok(data.to_string())).unwrap();
        println!("{:?}", result);
    }
}
