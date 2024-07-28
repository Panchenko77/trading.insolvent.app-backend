use std::ops::Deref;

use eyre::{ContextCompat, Result};
use worktable::{field, RowView, RowViewMut, Value, WorkTable};

use trading_model::Exchange;

/// used by order placement to check if we have enough fund to generate an order
/// if the fund is insufficient, we NSF the event

pub struct WorktableBalance {
    table: WorkTable,
}
field!(0, ExchangeCol: i64, "exchange_id");
field!(1, AvailableFundCol: f64, "available_fund");
// field!(2, ExpectedFund: f64, "expected_fund");
// field!(3, UserIdCol: i64, "user_id");

impl WorktableBalance {
    pub fn new() -> Self {
        let mut table = WorkTable::new();
        table.add_field(ExchangeCol);
        table.add_field(AvailableFundCol);
        Self { table }
    }
    pub fn insert(&mut self, exchange: Exchange, fund: f64) {
        self.table.push([Value::Int(exchange as _), Value::Float(fund as _)]);
    }
    pub fn ensure(&mut self, exchange: Exchange) -> BalanceRowViewMut {
        if let Some(row) = self.get_row_mut(exchange) {
            // SAFETY: rustc fool
            unsafe {
                return std::mem::transmute(row);
            }
        } else {
            self.table
                .insert()
                .set(ExchangeCol, exchange as _)
                .set(AvailableFundCol, 0.0)
                .finish();
            self.get_row_mut(exchange).unwrap()
        }
    }
    pub fn get_row_mut(&mut self, exchange: Exchange) -> Option<BalanceRowViewMut> {
        self.table
            .iter_mut()
            .map(BalanceRowViewMut)
            .find(|x| x.exchange() == exchange)
    }
    pub fn get_row(&mut self, exchange: Exchange) -> Option<BalanceRowView> {
        self.table.iter().map(BalanceRowView).find(|x| x.exchange() == exchange)
    }

    pub fn get_fund(&mut self, exchange: Exchange) -> Option<f64> {
        let fund = self.get_row(exchange)?.available_fund();
        Some(fund)
    }

    pub fn add_fund(&mut self, exchange: Exchange, fund: f64) -> Result<f64> {
        let current = self.get_fund(exchange).unwrap_or_default();
        let mut row = self
            .find_by_exchange(exchange)
            .with_context(|| format!("balance not initialized for exchange {}", exchange))?;
        row.set(AvailableFundCol, current + fund);
        Ok(current + fund)
    }

    pub fn deduct_fund(&mut self, exchange: Exchange, fund: f64) -> eyre::Result<f64> {
        let current = self.get_fund(exchange).unwrap_or_default();
        if current < fund {
            eyre::bail!("insufficient fund: {}, expected {}, got {}", exchange, fund, current)
        }
        let mut row = self.ensure(exchange);
        row.0.set(AvailableFundCol, current - fund);
        Ok(current - fund)
    }

    /// get row with matching order IDs (either one match will return)
    pub fn find_by_exchange(&mut self, exchange: Exchange) -> Option<RowViewMut> {
        self.table
            .iter_mut()
            .find(|row| *row.index(ExchangeCol) == exchange as i64)
    }
}
pub struct BalanceRowView<'a>(RowView<'a>);
impl<'a> BalanceRowView<'a> {
    pub fn exchange(&self) -> Exchange {
        Exchange::try_from(*self.0.index(ExchangeCol) as u8).expect("invalid exchange")
    }
    pub fn available_fund(&self) -> f64 {
        *self.0.index(AvailableFundCol)
    }
}

pub struct BalanceRowViewMut<'a>(RowViewMut<'a>);
impl<'a> Deref for BalanceRowViewMut<'a> {
    type Target = BalanceRowView<'a>;
    fn deref(&self) -> &Self::Target {
        unsafe { std::mem::transmute(self) }
    }
}
