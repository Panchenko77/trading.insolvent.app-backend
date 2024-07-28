use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use gluesql::core::ast_builder::col;
use gluesql::prelude::SharedMemoryStorage;
use gluesql_derive::ToGlueSql;
use tokio::sync::RwLock;

use build::model::{UserSubFundingRatesRequest, UserSubFundingRatesResponse};
use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::{ArcToolbox, RequestContext, TOOLBOX};
use lib::ws::SubscriptionManager;
use trading_exchange::utils::future::interval;
use trading_model::Exchange;

use crate::db::gluesql::schema::funding_rate::DbRowFundingRate;
use crate::endpoint_method::auth::ensure_user_role;
use crate::endpoint_method::SubsManagerKey;

#[derive(Clone)]
pub struct MethodUserSubFundingRates {
    subs: Arc<RwLock<SubscriptionManager<UserSubFundingRatesRequest>>>,
    table: Table<SharedMemoryStorage, DbRowFundingRate>,
    toolbox: Arc<tokio::sync::OnceCell<ArcToolbox>>,
}

impl MethodUserSubFundingRates {
    pub fn new(index_table: Table<SharedMemoryStorage, DbRowFundingRate>) -> Self {
        let this = Self {
            table: index_table,
            subs: Arc::new(RwLock::new(SubscriptionManager::new(
                SubsManagerKey::UserSubFundingRate as _,
            ))),
            toolbox: Arc::new(Default::default()),
        };
        this.spawn();
        this
    }

    // publishes websocket data
    fn spawn(&self) {
        let mut this = self.clone();
        tokio::task::spawn_local(async move {
            let interval_ms = 3000;
            let mut interval = interval(interval_ms);

            loop {
                interval.tick().await;
                let Some(toolbox) = this.toolbox.get() else { continue };
                let rows = this.table.select(None, "id").await.expect("failed to select");
                this.subs.write().await.publish_with_filter(toolbox, |req| {
                    Some(UserSubFundingRatesResponse {
                        data: rows
                            .iter()
                            .filter(|row| {
                                req.settings
                                    .exchange
                                    .as_ref()
                                    .map_or(true, |exchange| row.exchange().to_string() == *exchange)
                            })
                            .filter(|row| {
                                req.settings
                                    .symbol
                                    .as_ref()
                                    .map_or(true, |symbol| row.symbol().as_str() == symbol)
                            })
                            .map(|row| row.clone().into())
                            .collect(),
                    })
                });
            }
        });
    }
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserSubFundingRates {
    type Request = UserSubFundingRatesRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let _ = this.toolbox.set(TOOLBOX.get());

        this.subs
            .write()
            .await
            .subscribe(ctx, req.clone(), |req0| req0.settings.clone_from(&req));
        let mut filter = true.to_gluesql();
        if let Some(exchange) = req.exchange {
            let exchange = Exchange::from_str(&exchange)? as u8;
            filter = filter.and(col("exchange_id").eq(exchange.to_gluesql()));
        }
        if let Some(symbol) = req.symbol {
            filter = filter.and(col("symbol").eq(symbol.to_gluesql()));
        }

        let rows = this.table.select(Some(filter), "id").await?;
        Ok(UserSubFundingRatesResponse {
            data: rows.into_iter().map(|i| i.into()).collect(),
        })
    }
}
