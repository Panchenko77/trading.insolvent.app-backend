use crate::core::Slot;
use crate::{
    Exchange, InstrumentCategory, InstrumentCode, InstrumentDetails, InstrumentSelector, InstrumentSimple,
    InstrumentSymbol, NetworkSelector, SharedInstrumentDetails, Symbol,
};
use eyre::ContextCompat;
use eyre::Result;
use hashbrown::Equivalent;
use hashbrown::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

pub type SharedInstrumentManager = Arc<InstrumentManager>;

#[derive(Clone, Debug)]
pub struct InstrumentManager {
    instruments: Vec<Arc<InstrumentDetails>>,
    mapping: HashMap<InstrumentSelector, Slot<Arc<InstrumentDetails>>>,
}

impl InstrumentManager {
    pub fn new() -> Self {
        Self {
            instruments: Vec::new(),
            mapping: HashMap::new(),
        }
    }
    pub fn extend_from(&mut self, other: &Self) {
        self.instruments.extend(other.instruments.iter().cloned());
        for (selector, slot) in &other.mapping {
            self.mapping
                .entry(selector.clone())
                .or_default()
                .extend(slot.iter().cloned());
        }
    }
    pub fn from_instruments<T: Into<InstrumentDetails>>(instruments: impl IntoIterator<Item = T>) -> Self {
        let mut this = Self::new();
        this.extend(instruments);
        this
    }
    pub fn retain_network(&mut self, network: NetworkSelector) {
        self.instruments.retain(|i| network.match_network(i.network));
        self.mapping.retain(|_selector, slot| {
            slot.retain(|i| network.match_network(i.network));
            !slot.is_none()
        });
    }
    pub fn add(&mut self, instrument: impl Into<InstrumentDetails>) {
        let instrument = instrument.into();

        let instrument = instrument.into_shared();

        for selector in instrument.get_selectors() {
            self.mapping.entry(selector).or_default().push(instrument.clone());
        }
        self.instruments.push(instrument);
    }
    pub fn extend<T: Into<InstrumentDetails>>(&mut self, instruments: impl IntoIterator<Item = T>) {
        for instrument in instruments {
            self.add(instrument);
        }
    }
    pub fn get(&self, selector: &(impl Hash + Equivalent<InstrumentSelector>)) -> Option<&Arc<InstrumentDetails>> {
        self.mapping.get(selector).and_then(|slot| slot.get_first())
    }
    pub fn get_result(
        &self,
        selector: &(impl Hash + Equivalent<InstrumentSelector> + Debug),
    ) -> Result<&Arc<InstrumentDetails>> {
        self.get(selector)
            .with_context(|| format!("could not found instrument for {:?}", selector))
    }
    pub fn get_result_ctx(
        &self,
        selector: &(impl Hash + Equivalent<InstrumentSelector> + Debug),
        ctx: &str,
    ) -> Result<&Arc<InstrumentDetails>> {
        self.get(selector)
            .with_context(|| format!("could not found instrument for {} => {:?}", ctx, selector))
    }
    pub fn get_by_symbol(&self, exchange: Exchange, symbol: Symbol) -> Option<&Arc<InstrumentDetails>> {
        self.get_by_symbol_with_category_opt(exchange, symbol, None)
    }
    pub fn get_by_symbol_with_category_opt(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        cat: Option<InstrumentCategory>,
    ) -> Option<&Arc<InstrumentDetails>> {
        let symbol = InstrumentSymbol::new_with_category_opt(exchange, symbol, cat);
        self.get_by_instrument_symbol(&symbol)
    }

    pub fn get_by_symbol_result(&self, exchange: Exchange, symbol: Symbol) -> Result<&Arc<InstrumentDetails>> {
        self.get_by_symbol(exchange, symbol.clone())
            .with_context(|| format!("could not found symbol for {}:{}", exchange, symbol))
    }
    pub fn get_by_symbol_with_category_opt_result(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        cat: Option<InstrumentCategory>,
    ) -> Result<&Arc<InstrumentDetails>> {
        self.get_by_symbol_with_category_opt(exchange, symbol.clone(), cat)
            .with_context(|| format!("could not found symbol for {}:{}:{:?}", exchange, symbol, cat))
    }
    pub fn get_by_instrument_symbol(&self, symbol: &InstrumentSymbol) -> Option<&Arc<InstrumentDetails>> {
        self.get(&InstrumentSelector::Symbol(symbol.clone()))
    }
    pub fn get_by_instrument_symbol_margin(
        &self,
        symbol: &InstrumentSymbol,
        ctx: &str,
    ) -> Result<(&SharedInstrumentDetails, bool)> {
        let instrument = self
            .get(&InstrumentSelector::Symbol(symbol.clone()))
            .with_context(|| format!("could not found instrument for {} => {:?}", ctx, symbol));
        match instrument {
            Ok(i) => Ok((i, i.margin)),
            Err(err) => {
                let symbol = InstrumentSymbol::new_with_category(
                    symbol.exchange,
                    symbol.symbol.clone(),
                    InstrumentCategory::Spot,
                );
                let Some(spot) = self.get(&InstrumentSelector::Symbol(symbol.clone())) else {
                    return Err(err);
                };
                if spot.margin {
                    return Ok((spot, true));
                }

                Err(err)
            }
        }
    }

    pub fn get_by_simple(&self, simple: &InstrumentSimple) -> Option<&Arc<InstrumentDetails>> {
        self.get(&InstrumentSelector::Simple(simple.clone()))
    }
    pub fn get_by_code(&self, code: &InstrumentCode) -> Option<&Arc<InstrumentDetails>> {
        self.get(&InstrumentSelector::Code(code.clone()))
    }
    pub fn get_by_code_result(&self, code: &InstrumentCode) -> Result<&Arc<InstrumentDetails>> {
        self.get_result(&InstrumentSelector::Code(code.clone()))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<InstrumentDetails>> {
        self.instruments.iter()
    }

    pub fn collect(&self) -> Vec<InstrumentDetails> {
        self.instruments.iter().map(|i| (**i).clone()).collect()
    }

    pub fn into_shared(self) -> SharedInstrumentManager {
        Arc::new(self)
    }
}

pub trait InstrumentManagerExt {
    fn maybe_lookup_instrument(&self, exchange: Exchange, symbol: Symbol) -> InstrumentCode {
        self.maybe_lookup_instrument_with_category_opt(exchange, symbol, None)
    }
    fn maybe_lookup_instrument_with_category(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        category: InstrumentCategory,
    ) -> InstrumentCode {
        self.maybe_lookup_instrument_with_category_opt(exchange, symbol, Some(category))
    }
    fn maybe_lookup_instrument_with_category_opt(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        category: Option<InstrumentCategory>,
    ) -> InstrumentCode;
}

impl InstrumentManagerExt for InstrumentManager {
    fn maybe_lookup_instrument_with_category_opt(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        category: Option<InstrumentCategory>,
    ) -> InstrumentCode {
        return InstrumentCode::Symbol(InstrumentSymbol::new_with_category_opt(exchange, symbol, category));

        // let symbol = InstrumentSymbol::new_with_category_opt(exchange, symbol, category);
        // self.get_by_instrument_symbol(&symbol)
        //     .map(|instrument| InstrumentCode::Simple(instrument.to_simple()))
        //     .unwrap_or_else(|| InstrumentCode::Symbol(symbol))
    }
}

impl InstrumentManagerExt for Option<SharedInstrumentManager> {
    fn maybe_lookup_instrument_with_category_opt(
        &self,
        exchange: Exchange,
        symbol: Symbol,
        category: Option<InstrumentCategory>,
    ) -> InstrumentCode {
        return InstrumentCode::Symbol(InstrumentSymbol::new_with_category_opt(exchange, symbol, category));
        // self.as_ref()
        //     .map(|manager| manager.maybe_lookup_instrument_with_category_opt(exchange, symbol.clone(), category))
        //     .unwrap_or_else(|| {
        //         InstrumentCode::Symbol(InstrumentSymbol::new_with_category_opt(exchange, symbol, category))
        //     })
    }
}
