use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use trading_model::model::{AccountId, Asset, Exchange, InstrumentCode, Position, UpdatePositions};
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

pub fn gateio_perpetual_parse_query_book_account(
    account: AccountId,
    text: &str,
) -> Result<UpdatePositions> {
    let resp: GateioQueryBookAccountResp = serde_json::from_str(&text)?;
    let mut positions = UpdatePositions::update(Exchange::GateioPerpetual).with_account(account);

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
