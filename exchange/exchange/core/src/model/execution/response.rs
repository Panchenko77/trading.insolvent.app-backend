use derive_from_one::FromOne;
use eyre::Result;
use serde::{Deserialize, Serialize};

use trading_model::Exchange;

use crate::model::{
    AccountId, FundingPayment, Order, OrderCache, OrderTrade, Portfolio, PortfolioMulti, SyncOrders, UpdateBook,
    UpdateOrder, UpdatePosition, UpdatePositions,
};

#[derive(Clone, Debug, Serialize, Deserialize, FromOne)]
pub enum ExecutionResponse {
    Noop,
    Text(String),
    Error(String),
    SyncOrders(SyncOrders),
    UpdateOrder(UpdateOrder),
    UpdatePositions(UpdatePositions),
    UpdatePosition(UpdatePosition),
    UpdateFunding(FundingPayment),
    UpdateBook(UpdateBook),
    TradeOrder(OrderTrade),
    CompleteOrder(Order),
    Group(Vec<ExecutionResponse>),
}

impl ExecutionResponse {
    pub fn get_account(&self) -> Option<AccountId> {
        match self {
            Self::SyncOrders(sync_orders) => Some(sync_orders.account),
            Self::UpdateOrder(update_order) => Some(update_order.account),
            Self::UpdatePositions(update) => Some(update.account),
            Self::UpdatePosition(update_position) => Some(update_position.account),
            Self::TradeOrder(trade) => Some(trade.account),
            Self::CompleteOrder(order) => Some(order.account),
            Self::UpdateBook(update_book) => Some(update_book.account),
            Self::Group(g) => g.get(0)?.get_account(),
            _ => None,
        }
    }
    pub fn get_exchange(&self) -> Option<Exchange> {
        match self {
            Self::SyncOrders(sync_orders) => sync_orders.get_exchange(),
            Self::UpdateOrder(update_order) => update_order.instrument.get_exchange(),
            Self::UpdatePositions(update) => update.range.get_exchange(),
            Self::UpdatePosition(update_position) => update_position.instrument.get_exchange(),
            Self::TradeOrder(trade) => trade.instrument.get_exchange(),
            Self::CompleteOrder(order) => order.instrument.get_exchange(),
            Self::Group(g) => g.get(0)?.get_exchange(),
            _ => None,
        }
    }
    pub fn is_portfolio_update(&self) -> bool {
        match self {
            Self::UpdatePositions(_) => true,
            Self::UpdatePosition(_) => true,
            Self::TradeOrder(_) => true,
            Self::UpdateBook(_) => true,
            Self::Group(g) => g.iter().any(|x| x.is_portfolio_update()),
            _ => false,
        }
    }
    pub fn is_execution_update(&self) -> bool {
        match self {
            Self::SyncOrders(_) => true,
            Self::UpdateOrder(_) => true,
            Self::CompleteOrder(_) => true,
            Self::Group(g) => g.iter().any(|x| x.is_execution_update()),
            _ => false,
        }
    }

    pub fn update_portfolio_multi(&self, portfolio: &mut PortfolioMulti) -> Result<()> {
        let account = self.get_account().unwrap();
        let portfolio = portfolio.ensure_portfolio(account);
        self.update_portfolio(portfolio)?;
        Ok(())
    }

    pub fn update_portfolio(&self, portfolio: &mut Portfolio) -> Result<()> {
        match self {
            Self::SyncOrders(command) => {
                command.update_portfolio(portfolio)?;
            }
            Self::UpdateOrder(command) => {
                command.update_order_cache(&mut portfolio.orders);
            }
            Self::UpdatePositions(command) => {
                command.update_portfolio(portfolio)?;
            }
            Self::UpdatePosition(command) => {
                command.update_portfolio(portfolio)?;
            }
            Self::UpdateBook(command) => {
                command.update_portfolio(portfolio)?;
            }
            Self::Group(group) => {
                for g in group {
                    g.update_portfolio(portfolio)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
    pub fn update_order_cache(&self, orders: &mut OrderCache) -> Result<()> {
        match self {
            Self::SyncOrders(command) => {
                command.sync_order_cache(orders);
            }
            Self::UpdateOrder(command) => {
                command.update_order_cache(orders);
            }
            Self::Group(group) => {
                for g in group {
                    g.update_order_cache(orders)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
impl From<String> for ExecutionResponse {
    fn from(value: String) -> Self {
        ExecutionResponse::Text(value)
    }
}
impl<Ok: Into<ExecutionResponse>, Err: ToString> From<Result<Ok, Err>> for ExecutionResponse {
    fn from(value: Result<Ok, Err>) -> Self {
        match value {
            Ok(response) => response.into(),
            Err(err) => ExecutionResponse::Error(err.to_string()),
        }
    }
}
