use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use trading_exchange_core::model::{AccountId, Position, UpdatePositions};
use trading_model::{Asset, Exchange, InstrumentCode, InstrumentManager, Symbol};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct GateioQueryBookAccountResp {
    pub user: i64,

    pub currency: Asset,
    #[serde_as(as = "DisplayFromStr")]
    pub total: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub position_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub order_margin: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub available: f64,
    pub in_dual_mode: bool,
}

pub fn gateio_perpetual_parse_query_accounts(
    account: AccountId,
    text: &str,
) -> Result<UpdatePositions> {
    let resp: GateioQueryBookAccountResp = serde_json::from_str(&text)?;
    let mut positions = UpdatePositions::sync_balance(account, Exchange::GateioPerpetual);

    let instrument = InstrumentCode::from_asset(Exchange::GateioPerpetual, resp.currency);

    positions.add_position(&Position {
        instrument,
        account,
        total: resp.total,
        available: resp.available,
        locked: resp.total - resp.available,
        ..Position::empty()
    });
    Ok(positions)
}

//  {
//     "user": 10000,
//     "contract": "BTC_USDT",
//     "size": -9440,
//     "leverage": "0",
//     "risk_limit": "100",
//     "leverage_max": "100",
//     "maintenance_rate": "0.005",
//     "value": "2.497143098997",
//     "margin": "4.431548146258",
//     "entry_price": "3779.55",
//     "liq_price": "99999999",
//     "mark_price": "3780.32",
//     "unrealised_pnl": "-0.000507486844",
//     "realised_pnl": "0.045543982432",
//     "history_pnl": "0",
//     "last_close_pnl": "0",
//     "realised_point": "0",
//     "history_point": "0",
//     "adl_ranking": 5,
//     "pending_orders": 16,
//     "close_order": {
//       "id": 232323,
//       "price": "3779",
//       "is_liq": false
//     },
//     "mode": "single",
//     "update_time": 1684994406,
//     "cross_leverage_limit": "0"
//   }
#[derive(Serialize, Deserialize)]
struct GateioPerpetualPosition {
    // pub user: i64,
    pub contract: Symbol,
    pub size: i64,
    // pub leverage: String,
    // pub risk_limit: String,
    // pub leverage_max: String,
    // pub maintenance_rate: String,
    // pub value: String,
    // pub margin: String,
    pub entry_price: String,
    // pub liq_price: String,
    // pub mark_price: String,
    // pub unrealised_pnl: String,
    // pub realised_pnl: String,
    // pub history_pnl: String,
    // pub last_close_pnl: String,
    // pub realised_point: String,
    // pub history_point: String,
    // pub adl_ranking: i64,
    // pub pending_orders: i64,
    // pub close_order: CloseOrder,
    // pub mode: String,
    pub update_time: i64,
    // pub cross_leverage_limit: String,
}

pub fn gateio_perpetual_parse_query_positions(
    account: AccountId,
    text: &str,
    manager: &InstrumentManager,
) -> Result<UpdatePositions> {
    let resp: Vec<GateioPerpetualPosition> = serde_json::from_str(&text)?;
    let mut positions = UpdatePositions::sync_position(account, Exchange::GateioPerpetual);

    for pos in resp {
        let instrument = manager.get_result(&(Exchange::GateioPerpetual, pos.contract))?;
        let size = instrument.size.multiple_of(pos.size as f64);
        positions.add_position(&Position {
            instrument: instrument.code_simple.clone(),
            account,
            total: size,
            available: size,
            locked: 0.0,
            ..Position::empty()
        });
    }
    Ok(positions)
}
