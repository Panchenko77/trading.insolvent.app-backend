use trading_exchange_core::model::{
    AccountId, ExecutionResponse, UpdatePosition, UpdatePositionSetValues, UpdatePositions,
};
use trading_model::core::NANOSECONDS_PER_MILLISECOND;
use trading_model::model::{
    Exchange, InstrumentCode, InstrumentManagerExt, SharedInstrumentManager,
};

use crate::model::usdm_futures::UsdmFuturesUserData;

pub fn parse_query_user_assets_usdm_futures(
    account: AccountId,
    response: eyre::Result<String>,
    manager: Option<SharedInstrumentManager>,
) -> ExecutionResponse {
    let exchange = Exchange::BinanceFutures;
    match response {
        Ok(resp) => {
            let user_data: UsdmFuturesUserData =
                serde_json::from_str(&resp).expect("failed to decode query user assets");
            let time = user_data.update_time * NANOSECONDS_PER_MILLISECOND;
            let mut update = UpdatePositions::sync_balance_and_position(account, exchange);

            update.extend_updates(
                user_data
                    .assets
                    .into_iter()
                    .filter(|b| b.available_balance > 0.0 || b.initial_margin > 0.0)
                    .map(|b| UpdatePosition {
                        instrument: InstrumentCode::from_asset(exchange, b.asset),
                        times: (time, time).into(),
                        set_values: Some(UpdatePositionSetValues {
                            total: b.available_balance + b.initial_margin,
                            available: b.available_balance,
                            locked: b.initial_margin,
                        }),
                        ..UpdatePosition::empty()
                    }),
            );
            update.extend_updates(
                user_data
                    .positions
                    .into_iter()
                    .filter(|x| x.position_amt != 0.0)
                    .map(|p| {
                        assert_eq!(p.position_side.as_str(), "BOTH");
                        let instrument = manager.maybe_lookup_instrument(exchange, p.symbol);
                        UpdatePosition {
                            instrument,

                            // side: match p.position_side.as_str() {
                            //     "BOTH" => PositionSide::Both,
                            //     "SHORT" => PositionSide::Short,
                            //     "LONG" => PositionSide::Long,
                            //     _ => unreachable!(),
                            // },
                            times: (time, time).into(),
                            set_values: Some(UpdatePositionSetValues {
                                total: p.position_amt,
                                available: p.position_amt,
                                locked: 0.0, // find this out from open orders
                            }),

                            entry_price: Some(p.entry_price),
                            ..UpdatePosition::empty()
                        }
                    }),
            );
            ExecutionResponse::UpdatePositions(update)
        }

        Err(err) => ExecutionResponse::Error(err.to_string()),
    }
}
