use crate::model::{AccountId, Order, OrderCache, Portfolio, PortfolioMulti, UpdateOrder};
use eyre::Result;
use serde::{Deserialize, Serialize};
use trading_model::{Exchange, InstrumentCode, InstrumentSelector};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncOrders {
    pub account: AccountId,
    pub range: InstrumentSelector,
    pub full: bool,
    pub orders: Vec<Order>,
}

impl SyncOrders {
    pub fn with_account(mut self, account: AccountId) -> Self {
        self.account = account;
        self
    }
    pub fn new(exchange: Exchange, symbol: Option<InstrumentCode>) -> Self {
        let range = if let Some(symbol) = symbol {
            InstrumentSelector::Instrument(exchange, symbol)
        } else {
            InstrumentSelector::Exchange(exchange)
        };

        Self {
            account: 0,
            range,
            full: true,
            orders: Vec::new(),
        }
    }
    pub fn empty() -> Self {
        Self {
            account: 0,
            range: InstrumentSelector::All,
            full: true,
            orders: Vec::new(),
        }
    }
    pub fn multi(exchanges: Vec<Exchange>) -> Self {
        Self {
            account: 0,
            range: InstrumentSelector::Exchanges(exchanges),
            full: true,
            orders: Vec::new(),
        }
    }
    pub fn get_exchange(&self) -> Option<Exchange> {
        self.range.get_exchange()
    }
    pub fn sync_order_cache(&self, cache: &mut OrderCache) {
        cache.iter_mut().for_each(|x| x.updated = false);
        for order in self.orders.iter() {
            let update = UpdateOrder::from_order(order);
            update.update_order_cache(cache);
        }

        self.try_retain_orders(cache);
    }

    fn try_retain_orders(&self, orders: &mut OrderCache) {
        fn should_retain(order: &Order, sync_orders: &SyncOrders) -> bool {
            if order.updated {
                return true;
            }
            if order.status.is_new() {
                return true;
            }

            // if this sync orders is for a specific instrument, only remove untouched orders for that symbol
            if sync_orders.range.match_instrument(&order.instrument) {
                return false;
            }
            true
        }
        if self.full {
            orders.retain(|x| should_retain(x, self));
        }
    }
    pub fn update_portfolio(&self, portfolio: &mut Portfolio) -> Result<()> {
        let orders = &mut portfolio.orders;
        self.sync_order_cache(orders);
        Ok(())
    }
    pub fn update_universal_portfolio(&self, portfolio: &mut PortfolioMulti) -> Result<()> {
        let exchange = self.account;
        let portfolio = portfolio.ensure_portfolio(exchange);
        self.update_portfolio(portfolio)
    }
}
