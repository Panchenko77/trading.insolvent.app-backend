use eyre::Result;
use trading_exchange_core::model::{
    AccountId, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_model::{now, Exchange, InstrumentCode};

use crate::model::spot;

pub fn gateio_spot_parse_query_user_assets(
    account: AccountId,
    resp: &str,
) -> Result<UpdatePositions> {
    let exchange = Exchange::GateioSpot;

    let user_assets: Vec<spot::Balance> =
        serde_json::from_str(&resp).expect("failed to decode query user assets");
    let time = now();
    let mut update = UpdatePositions::sync_balance(account, exchange);

    update.extend_updates(
        user_assets
            .into_iter()
            .filter(|x| x.available + x.locked > 0.0)
            .map(|b| UpdatePosition {
                account,
                instrument: InstrumentCode::from_asset(exchange, b.currency),
                times: (time, time).into(),
                set_values: Some(UpdatePositionSetValues {
                    total: b.available + b.locked,
                    available: b.available,
                    locked: b.locked,
                }),
                ..UpdatePosition::empty()
            }),
    );
    Ok(update)
}
