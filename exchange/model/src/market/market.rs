use crate::{InstrumentCode, L2OrderBook, TradeHistory};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Market {
    pub instrument: InstrumentCode,
    pub orderbook: L2OrderBook<100>,
    pub trades: TradeHistory,
}
impl Market {
    pub fn empty() -> Self {
        Self {
            instrument: InstrumentCode::None,
            orderbook: L2OrderBook::new(),
            trades: TradeHistory::new(),
        }
    }
    pub fn new(instrument: InstrumentCode) -> Self {
        Self {
            instrument,
            orderbook: L2OrderBook::new(),
            trades: TradeHistory::new(),
        }
    }
}
#[derive(Default, Clone, Debug)]
pub struct MarketUniversal {
    pub markets: HashMap<InstrumentCode, Market>,
}
impl MarketUniversal {
    pub fn new() -> Self {
        Self {
            markets: HashMap::new(),
        }
    }

    pub fn ensure_market(&mut self, instrument: InstrumentCode) -> &mut Market {
        self.markets
            .entry(instrument.clone())
            .or_insert_with(|| Market::new(instrument))
    }
    pub fn get_market(&self, instrument: &InstrumentCode) -> Option<&Market> {
        self.markets.get(instrument)
    }
}
