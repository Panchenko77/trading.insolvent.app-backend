use async_trait::async_trait;

use build::model::{EnumRole, UserListTradingSymbolsRequest, UserListTradingSymbolsResponse, UserTradingSymbol};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;
use trading_model::SharedInstrumentManager;

use crate::endpoint_method::auth::ensure_user_role;

#[derive(Debug, Clone)]
pub struct MethodUserListTradingSymbols {
    pub symbols: SharedInstrumentManager,
}
impl MethodUserListTradingSymbols {
    pub fn new(symbols: SharedInstrumentManager) -> Self {
        Self { symbols }
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserListTradingSymbols {
    type Request = UserListTradingSymbolsRequest;

    async fn handle(&self, ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;
        let symbols = self
            .symbols
            .iter()
            .map(|x| UserTradingSymbol {
                exchange: x.exchange.to_string(),
                symbol: x.symbol.to_string(),
                base: x.base.asset.to_string(),
                lot_size: x.lot.size.precision,
                base_decimals: x.base.wire.decimals,
                quote: x.quote.asset.to_string(),
                tick_size: x.tick.size.precision,
                quote_decimals: x.quote.wire.decimals,
            })
            .collect::<Vec<_>>();
        Ok(UserListTradingSymbolsResponse { data: symbols })
    }
}
