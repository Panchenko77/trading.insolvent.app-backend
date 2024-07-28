use crate::model::{
    Asset, Exchange, InstrumentCategory, InstrumentCode, InstrumentId, InstrumentSimple, InstrumentSymbol,
};
use crate::Symbol;
use hashbrown::Equivalent;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tracing::warn;

/// Instrument selector is meant to locate exactly one instrument.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentSelector {
    Symbol(InstrumentSymbol),
    Simple(InstrumentSimple),
    Code(InstrumentCode),
    Index(Exchange, InstrumentId),
    CategoryIndex(Exchange, InstrumentCategory, InstrumentId),

    // below are copied from InstrumentRange and cannot be used as keys
    All,
    None,
    Exchange(Exchange),
    Exchanges(Vec<Exchange>),
    Instrument(Exchange, InstrumentCode),
    Category(Exchange, InstrumentCategory),
    CategoryQuote(Exchange, InstrumentCategory, Asset),
}

impl InstrumentSelector {
    pub fn get_exchange(&self) -> Option<Exchange> {
        match self {
            Self::Symbol(s) => Some(s.exchange),
            Self::Simple(s) => Some(s.exchange),
            Self::Code(c) => c.get_exchange(),
            Self::Index(exchange, _) => Some(*exchange),
            Self::CategoryIndex(exchange, _, _) => Some(*exchange),

            Self::All => None,
            Self::None => None,
            Self::Exchange(exchange) => Some(*exchange),
            Self::Exchanges(exchanges) => {
                if exchanges.len() == 1 {
                    Some(exchanges[0])
                } else {
                    warn!(
                        "InstrumentRange with multiple exchanges, but getting one: {:?}",
                        exchanges
                    );
                    None
                }
            }
            Self::Instrument(exchange, _) => Some(*exchange),
            Self::Category(exchange, _) => Some(*exchange),
            Self::CategoryQuote(exchange, _, _) => Some(*exchange),
        }
    }
    pub fn match_instrument(&self, instrument: &InstrumentCode) -> bool {
        match self {
            Self::Symbol(_) => false,
            Self::Simple(_) => false,
            Self::Code(code) => code == instrument,
            Self::Index(_, _) => false,
            Self::CategoryIndex(_, _, _) => false,

            Self::All => true,
            Self::None => false,
            Self::Exchange(exchange) => instrument.get_exchange() == Some(*exchange),
            Self::Exchanges(exchanges) => exchanges.contains(&instrument.get_exchange().unwrap()),
            Self::Instrument(exchange, instrument1) => {
                instrument.get_exchange() == Some(*exchange) && *instrument == *instrument1
            }
            Self::Category(exchange, category) => {
                instrument.get_exchange() == Some(*exchange) && category.match_instrument(&instrument)
            }
            Self::CategoryQuote(exchange, category, quote) => {
                instrument.get_exchange() == Some(*exchange)
                    && category.match_instrument(&instrument)
                    && instrument.quote() == Some(quote.clone())
            }
        }
    }
}

impl Hash for InstrumentSelector {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // no need to hash the enum variant to support hashbrown::Equivalent
        match self {
            Self::Symbol(symbol) => symbol.hash(state),
            Self::Simple(simple) => simple.hash(state),
            Self::Code(code) => code.hash(state),
            Self::Index(exchange, index) => {
                exchange.hash(state);
                index.hash(state);
            }
            Self::CategoryIndex(exchange, category, index) => {
                exchange.hash(state);
                category.hash(state);
                index.hash(state);
            }
            _ => unreachable!(),
        }
    }
}

impl Equivalent<InstrumentSelector> for InstrumentSymbol {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::Symbol(symbol) => self == symbol,
            _ => false,
        }
    }
}

impl Equivalent<InstrumentSelector> for InstrumentSimple {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::Simple(simple) => self == simple,
            _ => false,
        }
    }
}

impl Equivalent<InstrumentSelector> for InstrumentCode {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::Code(code) => self == code,
            _ => false,
        }
    }
}

impl Equivalent<InstrumentSelector> for (Exchange, InstrumentId) {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::Index(exchange, index) => self == &(*exchange, *index),
            _ => false,
        }
    }
}
impl Equivalent<InstrumentSelector> for (Exchange, Symbol) {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::Symbol(symbol) => {
                symbol.exchange == self.0 && symbol.symbol == self.1 && symbol.category.is_none()
            }
            _ => false,
        }
    }
}
impl Equivalent<InstrumentSelector> for (Exchange, Symbol, InstrumentCategory) {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::Symbol(symbol) => {
                symbol.exchange == self.0 && symbol.symbol == self.1 && symbol.category == Some(self.2)
            }
            _ => false,
        }
    }
}
impl Equivalent<InstrumentSelector> for (Exchange, InstrumentCategory, InstrumentId) {
    fn equivalent(&self, key: &InstrumentSelector) -> bool {
        match key {
            InstrumentSelector::CategoryIndex(exchange, category, index) => self == &(*exchange, *category, *index),
            _ => false,
        }
    }
}
