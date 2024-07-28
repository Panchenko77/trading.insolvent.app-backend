use std::sync::Arc;

use async_trait::async_trait;
use eyre::Result;
use eyre::{bail, ContextCompat};
use tokio::sync::RwLock;
use tracing::info;

use build::model::{EnumErrorCode, EnumRole, UserCancelOrClosePositionRequest, UserCancelOrClosePositionResponse};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{CustomError, RequestContext};
use trading_exchange::exchange::gen_order_cid;
use trading_exchange::model::{
    gen_local_id, OrderCid, OrderStatus, OrderType, PositionEffect, RequestCancelOrder, RequestPlaceOrder, TimeInForce,
    UpdateOrder,
};
use trading_model::{Asset, Exchange, InstrumentCode, PriceType, SharedInstrumentManager, Side, Symbol, Time};

use crate::db::worktable::position_manager::PositionManager;
use crate::endpoint_method::auth::ensure_user_role;
use crate::execution::OrderRegistry;
use crate::strategy::data_factory::{LastPriceMap, PriceSourceAsset};
use crate::strategy::instrument::convert_asset_to_instrument;

pub struct MethodUserCancelOrClosePosition {
    manual_trade_service: Arc<OrderRegistry>,
    portfolio: Arc<RwLock<PositionManager>>,
    prices: Arc<LastPriceMap>,
    manager: SharedInstrumentManager,
}
impl MethodUserCancelOrClosePosition {
    pub fn new(
        manual_trade_service: Arc<OrderRegistry>,
        portfolio: Arc<RwLock<PositionManager>>,
        prices: Arc<LastPriceMap>,
        manager: SharedInstrumentManager,
    ) -> Self {
        Self {
            manual_trade_service,
            portfolio,
            prices,
            manager,
        }
    }
    pub async fn close_position(&self, exchange: Exchange, asset: Asset, size: f64) -> Result<UpdateOrder> {
        let side = if size > 0.0 { Side::Sell } else { Side::Buy };
        let asset: Asset = asset.trim_end_matches("USDT").into();
        let mut price = self
            .prices
            .get(&PriceSourceAsset {
                asset: asset.clone(),
                exchange,
                price_type: PriceType::Bid,
            })
            .with_context(|| CustomError::new(EnumErrorCode::NotFound, "Price not found"))?;
        let Some(symbol) = convert_asset_to_instrument(&self.manager, exchange, &asset) else {
            bail!(CustomError::new(
                EnumErrorCode::NotFound,
                format!("Could not find the instrument for asset {} {}", exchange, asset)
            ));
        };

        match side {
            Side::Buy => price.price *= 0.95,
            Side::Sell => price.price *= 1.05,
            _ => unreachable!(),
        }

        let new_order = RequestPlaceOrder {
            instrument: symbol.code_symbol.clone(),
            order_lid: gen_local_id(),
            order_cid: gen_order_cid(exchange),
            size: size.abs(),
            price: price.price,
            ty: OrderType::Market,
            side,
            effect: PositionEffect::Close,
            tif: TimeInForce::ImmediateOrCancel,
            account: 0,
            create_lt: Time::now(),
            ..RequestPlaceOrder::empty()
        };
        let order = self.manual_trade_service.send_order(new_order).await?;
        let mut response = UpdateOrder::empty();
        while let Some(update) = order.recv().await {
            info!("update: {:?}", update);
            response = update;
        }

        Ok(response)
    }
    pub async fn cancel_order(&self, exchange: Exchange, symbol: Symbol, cloid: OrderCid) -> Result<()> {
        let new_order = RequestCancelOrder {
            instrument: InstrumentCode::from_symbol(exchange, symbol),
            order_lid: gen_local_id(),
            order_cid: cloid,
            order_sid: "".into(),
            account: 0,
            strategy_id: 0,
            cancel_lt: Time::now(),
        };
        self.manual_trade_service.cancel_order(new_order).await;

        Ok(())
    }
}

#[async_trait(?Send)]
impl RequestHandler for MethodUserCancelOrClosePosition {
    type Request = UserCancelOrClosePositionRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::Trader)?;

        let portfolio = self.portfolio.read().await;
        let position = portfolio.positions.iter().find(|p| p.id() == req.id);

        if let Some(position) = position {
            if position.cloid().is_none() {
                let size = position.size();
                let exchange = position.exchange();
                let symbol = position.symbol().into();
                drop(portfolio);
                let update = self.close_position(exchange, symbol, size).await?;
                if update.status == OrderStatus::Rejected {
                    bail!(CustomError::new(
                        EnumErrorCode::InternalServerError,
                        format!("Order rejected: {}", update.reason)
                    ))
                }
            } else {
                let cloid: OrderCid = position.cloid().unwrap().into();
                let exchange = position.exchange();
                let symbol: Symbol = position.symbol().into();
                drop(portfolio);
                self.cancel_order(exchange, symbol, cloid).await?;
            }
        } else {
            bail!(CustomError::new(EnumErrorCode::NotFound, "Position not found"))
        }

        Ok(UserCancelOrClosePositionResponse {})
    }
}
