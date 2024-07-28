use crate::math::size::Size;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentDetailsBuilder {
    pub exchange: Exchange,
    pub network: Network,
    pub name: String,
    pub symbol: Symbol,
    pub id: u32,
    pub base: AssetInfo,
    pub quote: AssetInfo,

    pub size: Size,
    pub price: Size,

    pub lot: SizedLimit,
    pub tick: SizedLimit,

    pub status: InstrumentStatus,
    pub ty: InstrumentType,
    pub margin: bool,
    pub max_leverage: f64,
    // pub delivery_date_type: DeliveryDateType,
}

impl InstrumentDetailsBuilder {
    pub fn empty() -> Self {
        Self {
            exchange: Exchange::Null,
            network: Network::Mainnet,
            name: "".into(),
            symbol: "".into(),
            id: 0,
            base: AssetInfo::empty(),
            quote: AssetInfo::empty(),
            size: Size::ONE,
            price: Size::ONE,
            lot: SizedLimit::UNLIMITED,
            tick: SizedLimit::UNLIMITED,
            status: InstrumentStatus::Close,
            ty: InstrumentType::Spot,
            margin: false,
            max_leverage: 1.0,
            // delivery_date_type: DeliveryDateType::Unknown,
        }
    }
    pub fn to_symbol(&self) -> InstrumentSymbol {
        InstrumentSymbol::new(self.exchange, self.symbol.clone())
    }
    pub fn to_simple(&self) -> InstrumentSimple {
        InstrumentSimple {
            exchange: self.exchange,
            base: self.base.asset.clone(),
            quote: self.quote.asset.clone(),
            ty: self.ty,
        }
    }
    pub fn to_simple_code(&self) -> InstrumentCode {
        InstrumentCode::Simple(self.to_simple())
    }
    pub fn to_symbol_code(&self) -> InstrumentCode {
        InstrumentCode::Symbol(self.to_symbol())
    }

    pub fn build(mut self) -> InstrumentDetails {
        if self.lot == SizedLimit::UNLIMITED {
            self.lot = SizedLimit::from_size(self.size);
        }
        if self.tick == SizedLimit::UNLIMITED {
            self.tick = SizedLimit::from_size(self.price);
        }
        InstrumentDetails {
            instrument_symbol: self.to_symbol(),
            simple: self.to_simple(),
            code_simple: self.to_simple_code(),
            code_symbol: self.to_symbol_code(),
            settlement_asset: if self.ty.is_linear() {
                self.quote.asset.clone()
            } else {
                self.base.asset.clone()
            },
            exchange: self.exchange,
            network: self.network,
            name: self.name,
            symbol: self.symbol,
            id: self.id,
            base: self.base.clone(),
            quote: self.quote.clone(),
            size: self.size,
            price: self.price,

            ty: self.ty,
            margin: self.margin || self.ty.is_margin(),
            contract_val_asset: self.base.asset.clone(),
            tick: self.tick,
            lot: self.lot,
            max_leverage: self.max_leverage,
            listing_date: Date::NULL,
            delivery_date: Date::NULL,
            is_fee_percentage: false,
            is_fee_tier_based: false,
            fee_side: None,
            amount_limits_min_notional: None,
            allowed_pending_orders: 0,
            contract_value: ContractValue::SPOT,
            status: self.status,
        }
    }
}

impl Into<InstrumentDetails> for InstrumentDetailsBuilder {
    fn into(self) -> InstrumentDetails {
        self.build()
    }
}
pub type SharedInstrumentDetails = Arc<InstrumentDetails>;

/// very detailed information about an instrument
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentDetails {
    pub exchange: Exchange,
    pub network: Network,
    pub name: String,
    pub symbol: Symbol,
    pub id: InstrumentId,
    pub base: AssetInfo,
    pub quote: AssetInfo,
    /// order size, to regulate the precision displaying the size
    pub size: Size,

    /// order price, to regulate the precision displaying the price
    pub price: Size,

    /// lot size, it regulates the minimum/maximum order size and rounding
    pub lot: SizedLimit,
    /// tick size, it regulates the minimum/maximum order price and rounding
    pub tick: SizedLimit,

    pub ty: InstrumentType,
    pub margin: bool,

    pub simple: InstrumentSimple,
    pub instrument_symbol: InstrumentSymbol,
    pub code_simple: InstrumentCode,
    pub code_symbol: InstrumentCode,
    pub settlement_asset: Asset,
    pub contract_val_asset: Asset,
    pub max_leverage: f64,
    pub listing_date: Date,
    pub delivery_date: Date,
    pub is_fee_percentage: bool,
    pub is_fee_tier_based: bool,
    pub fee_side: Option<FeeSideEnum>,
    pub amount_limits_min_notional: Option<f64>,
    pub allowed_pending_orders: i64,
    pub contract_value: ContractValue,
    pub status: InstrumentStatus,
}

impl InstrumentDetails {
    pub fn into_shared(self) -> SharedInstrumentDetails {
        Arc::new(self)
    }
    pub fn to_symbol_code(&self) -> InstrumentCode {
        self.code_symbol.clone()
    }
    pub fn to_simple(&self) -> InstrumentSimple {
        self.simple.clone()
    }
    pub fn to_simple_code(&self) -> InstrumentCode {
        self.code_simple.clone()
    }
    pub fn get_selectors(&self) -> Vec<InstrumentSelector> {
        let mut selectors = vec![
            InstrumentSelector::Simple(self.simple.clone()),
            InstrumentSelector::Code(self.code_simple.clone()),
            InstrumentSelector::Code(self.code_symbol.clone()),
            InstrumentSelector::Index(self.exchange, self.id),
        ];
        for category in InstrumentCategory::iter() {
            if category.match_instrument_type(self.ty) {
                selectors.push(InstrumentSelector::CategoryIndex(self.exchange, category, self.id));
            }
        }
        for symbol in self.to_symbols() {
            selectors.push(InstrumentSelector::Symbol(symbol.clone()));
            selectors.push(InstrumentSelector::Code(InstrumentCode::Symbol(symbol)));
        }

        selectors
    }
    pub fn to_symbols(&self) -> Vec<InstrumentSymbol> {
        let mut symbols = vec![InstrumentSymbol::new(self.exchange, self.symbol.clone())];
        for cat in InstrumentCategory::iter() {
            if cat.match_instrument_type(self.ty) {
                symbols.push(InstrumentSymbol::new_with_category(
                    self.exchange,
                    self.symbol.clone(),
                    cat,
                ));
            }
        }
        symbols
    }

    pub fn get_lot_size(&self) -> f64 {
        // there should be only Absolute mode for size
        self.lot.size.precision
    }
    pub fn get_tick_size(&self, price: f64) -> f64 {
        self.tick.size.precision_by(price)
    }
}
