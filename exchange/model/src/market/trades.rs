use crate::{MarketTrade, TickSeries};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHistory {
    pub(crate) trades: TickSeries<MarketTrade>,
}
impl TradeHistory {
    pub fn new() -> Self {
        Self {
            trades: TickSeries::new_tick(1000),
        }
    }
    pub fn add_trade(&mut self, trade: MarketTrade) {
        self.trades.push(trade);
    }
    pub fn get_trades(&self) -> &TickSeries<MarketTrade> {
        &self.trades
    }
    pub fn total_len(&self) -> usize {
        self.trades.total_len()
    }
    pub fn len(&self) -> usize {
        self.trades.len()
    }
    pub fn base_volume(&self) -> f64 {
        self.trades.iter().map(|x| x.size).sum()
    }
    pub fn quote_volume(&self) -> f64 {
        self.trades.iter().map(|x| x.size * x.price).sum()
    }
}
