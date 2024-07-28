use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use trading_model::{DeFiTrade, Exchange, InstrumentCode, Quantity};

use crate::model::{AccountId, FundingPayment, OrderLid, OrderTrade, Portfolio};

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourceStatus {
    pub alive: bool,
    pub initial_positions: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateBook {
    pub account: AccountId,

    /// Map of [`SourceId`] to [`SourceStatus`].
    ///
    /// After the initial snapshot only the statuses that changed will be sent.
    pub source_status: HashMap<Exchange, SourceStatus>,

    /// Absolute positions.
    ///
    /// After the initial snapshot only the positions that changed will be sent.
    pub positions: HashMap<InstrumentCode, Quantity>,

    /// Orders that are considered "settled" by accounting.
    ///
    /// # Settled Definition
    ///
    /// A settled order:
    /// - Is closed.
    /// - Has had all of its trades observed by accounting.
    /// - Has had its filled quantity & cost reflected in positions.
    pub settled_orders: Vec<(Exchange, OrderLid)>,

    /// CeFi trades that have occurred after accounting startup and thus affect
    /// position & PNL.
    pub trades: Vec<OrderTrade>,
    /// Historical CeFi trades that have occurred before accounting startup.
    pub historical_trades: Vec<OrderTrade>,
    /// DeFi trades that have occurred after accounting startup and thus affect
    /// position & PNL.
    pub defi_trades: Vec<DeFiTrade>,
    /// Historical DeFi trades that have occurred before accounting startup.
    pub historical_defi_trades: Vec<DeFiTrade>,
    /// Funding payments that have occurred after accounting startup and thus affect
    /// position & PNL.
    pub funding: Vec<FundingPayment>,
    /// Historical funding payments that have occurred before accounting startup.
    pub historical_funding: Vec<FundingPayment>,
}
impl UpdateBook {
    pub fn update_portfolio(&self, portfolio: &mut Portfolio) -> eyre::Result<()> {
        for (instrument, &position) in self.positions.iter() {
            let pos = portfolio.ensure_by_instrument(instrument.clone());
            pos.total = position;
            pos.available = position;
        }
        Ok(())
    }
}
