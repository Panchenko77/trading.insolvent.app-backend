use crate::model::spot;
use trading_exchange_core::model::{
    AccountId, ExecutionResponse, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_model::core::now;
use trading_model::model::{Exchange, InstrumentCode};

pub fn parse_query_user_assets_spot(
    account: AccountId,
    response: eyre::Result<String>,
) -> ExecutionResponse {
    let exchange = Exchange::BinanceSpot;
    match response {
        Ok(resp) => {
            let user_assets: Vec<spot::Balance> =
                serde_json::from_str(&resp).expect("failed to decode query user assets");
            let time = now();
            let mut update = UpdatePositions::sync_balance(account, exchange);

            update.extend_updates(
                user_assets
                    .into_iter()
                    .filter(|x| x.free + x.locked > 0.0)
                    .map(|b| UpdatePosition {
                        instrument: InstrumentCode::from_asset(exchange, b.asset),
                        times: (time, time).into(),
                        set_values: Some(UpdatePositionSetValues {
                            total: b.free + b.locked,
                            available: b.free,
                            locked: b.locked,
                        }),
                        ..UpdatePosition::empty()
                    }),
            );
            ExecutionResponse::UpdatePositions(update)
        }
        Err(err) => ExecutionResponse::Error(err.to_string()),
    }
}
