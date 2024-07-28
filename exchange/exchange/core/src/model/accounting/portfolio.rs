use crate::model::{AccountId, Order, OrderCache, OrderLid, Position, UpdateOrder};
use indenter::indented;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use trading_model::{Asset, Exchange, InstrumentCode, QuantityUnit, Symbol, TickSeries};

#[derive(Clone, Debug)]
pub struct Portfolio {
    pub account: AccountId,
    pub orders: OrderCache,
    pub order_updates: TickSeries<UpdateOrder>,
    pub positions: HashMap<InstrumentCode, Position>,
}

impl Portfolio {
    pub fn empty() -> Self {
        Self::new(AccountId::default())
    }
    pub fn new(account: AccountId) -> Self {
        Self {
            orders: Default::default(),
            order_updates: TickSeries::new_tick(1000),
            account,
            positions: Default::default(),
        }
    }
    pub fn get_balance(&self, exchange: Exchange, asset: &Asset) -> Option<&Position> {
        self.positions.get(&InstrumentCode::from_asset(exchange, asset.clone()))
    }
    pub fn ensure_balance(&mut self, exchange: Exchange, asset: Asset) -> &mut Position {
        let instrument = InstrumentCode::from_asset(exchange, asset);
        self.positions
            .entry(instrument.clone())
            .or_insert_with(|| Position::with_account_id(instrument, self.account, QuantityUnit::Base))
    }
    pub fn get_position(&self, symbol: &InstrumentCode) -> Option<&Position> {
        self.positions.get(symbol)
    }
    pub fn ensure_position(&mut self, exchange: Exchange, symbol: Symbol) -> &mut Position {
        let instrument = InstrumentCode::from_symbol(exchange, symbol);
        self.positions
            .entry(instrument.clone())
            .or_insert_with(|| Position::with_account_id(instrument, self.account, exchange.to_position_unit()))
    }
    pub fn ensure_by_instrument(&mut self, instrument: InstrumentCode) -> &mut Position {
        let unit = instrument.to_unit().unwrap();
        self.positions
            .entry(instrument.clone())
            .or_insert_with(|| Position::with_account_id(instrument, self.account, unit))
    }

    pub fn iter_orders(&self) -> impl Iterator<Item = &Order> {
        self.orders.iter()
    }
    pub fn iter_orders_mut(&mut self) -> impl Iterator<Item = &mut Order> {
        self.orders.iter_mut()
    }
}
impl Display for Portfolio {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Portfolio#{}:", self.account)?;
        writeln!(f, "Positions:")?;
        for (_instrument, position) in &self.positions {
            writeln!(indented(f), "{}", position)?;
        }
        writeln!(f, "Orders:")?;
        for order in self.orders.iter() {
            writeln!(indented(f), "{}", order)?;
        }

        Ok(())
    }
}
#[derive(Default, Clone, Debug)]
pub struct PortfolioMulti {
    portfolios: HashMap<AccountId, RefCell<Portfolio>>,
}
impl PortfolioMulti {
    pub fn new() -> Self {
        Self {
            portfolios: Default::default(),
        }
    }

    pub fn get_portfolio(&self, account: AccountId) -> Option<&RefCell<Portfolio>> {
        self.portfolios.get(&account)
    }

    pub fn ensure_portfolio(&mut self, account: AccountId) -> &mut Portfolio {
        self.portfolios
            .entry(account)
            .or_insert_with(|| RefCell::new(Portfolio::new(account)))
            .get_mut()
    }
    pub fn for_order_by_lid<R>(&self, account: AccountId, lid: &OrderLid, f: impl FnOnce(&Order) -> R) -> Option<R> {
        let portfolio = self.get_portfolio(account)?;
        let p = portfolio.borrow();
        let order = p.orders.get(lid)?;
        Some(f(order))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RefCell<Portfolio>> {
        self.portfolios.values_mut()
    }
    pub fn iter(&self) -> impl Iterator<Item = &RefCell<Portfolio>> {
        self.portfolios.values()
    }
}
impl Display for PortfolioMulti {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "UniversalPortfolio:")?;
        for (_exchange, portfolio) in &self.portfolios {
            writeln!(indented(f), "{}", portfolio.borrow())?;
        }
        Ok(())
    }
}
