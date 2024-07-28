use crate::utils::serde::CowStrVisitor;
use crate::{
    Asset, AssetUniversal, Blockchain, BlockchainToken, DefiSwap, Exchange, InstrumentSimple, InstrumentSymbol,
    Location, QuantityUnit, Symbol,
};
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalPairMeta {
    pub base: Asset,
    pub quote: Asset,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)]
#[display("{exchange}:{base}-{quote}")]
pub struct ExchangePairMeta {
    pub exchange: Exchange,
    pub base: Asset,
    pub quote: Asset,
}

#[derive(Clone, PartialEq, Eq, Hash, Display, FromStr)]
pub enum InstrumentCode {
    // Global.
    None,
    #[display("X:{0}")]
    Exposure(Asset),

    // Localized.
    #[display("A:{0}")]
    Asset(AssetUniversal),
    #[display("T:{0}")]
    Token(BlockchainToken),
    #[display("s:{0}")]
    Symbol(InstrumentSymbol),

    #[display("S:{0}")]
    Simple(InstrumentSimple),
    #[display("C:{0}")]
    CFD(ExchangePairMeta),
    #[display("D:{0}")]
    DefiSwap(DefiSwap),
}

impl InstrumentCode {
    pub fn from_symbol(exchange: Exchange, symbol: Symbol) -> Self {
        Self::Symbol(InstrumentSymbol::new(exchange, symbol))
    }
    pub fn from_simple(instrument: InstrumentSimple) -> Self {
        Self::Simple(instrument)
    }
    pub fn from_asset(exchange: Exchange, asset: Asset) -> Self {
        Self::Asset(AssetUniversal::new(Location::Exchange(exchange), asset))
    }
    pub fn is_asset(&self) -> bool {
        matches!(self, Self::Asset(_))
    }
    pub fn is_position(&self) -> bool {
        match self {
            Self::Simple(s) => s.is_futures(),
            // only partial information, but when referring to a position, it is a position
            Self::Symbol(_) => true,
            _ => false,
        }
    }

    pub fn to_unit(&self) -> Option<QuantityUnit> {
        match self {
            Self::Asset(_) => Some(QuantityUnit::Base),
            Self::Symbol(symbol) => Some(symbol.exchange.to_position_unit()),
            Self::Simple(instrument) => Some(instrument.exchange.to_position_unit()),
            _ => None,
        }
    }
    pub fn get_exchange(&self) -> Option<Exchange> {
        match self {
            Self::None => None,
            Self::Exposure(_) => None,
            Self::Asset(asset) => asset.location.get_exchange(),
            Self::Token(_) => None,
            Self::CFD(pair) => Some(pair.exchange),
            Self::DefiSwap(_) => None,
            Self::Symbol(symbol) => Some(symbol.exchange),
            Self::Simple(instrument) => Some(instrument.exchange),
        }
    }
    pub fn get_symbol(&self) -> Option<Symbol> {
        match self {
            Self::None => None,
            Self::Exposure(_) => None,
            Self::Asset(_) => None,
            Self::Token(_) => None,
            Self::CFD(_) => None,
            Self::DefiSwap(_) => None,
            Self::Symbol(symbol) => Some(symbol.symbol.clone()),
            Self::Simple(_) => None,
        }
    }

    #[inline]
    pub fn get_asset(&self) -> Option<Asset> {
        match self {
            Self::None => None,
            Self::Exposure(underlying) => Some(underlying.clone()),
            Self::Asset(native) => Some(native.asset.clone()),
            Self::Token(token) => Some(token.underlying.clone()),
            Self::DefiSwap(_) => None,
            Self::CFD(cfd) => Some(cfd.base.clone()),
            Self::Symbol(_) => None,
            Self::Simple(instrument) => Some(instrument.base.clone()),
        }
    }

    pub fn get_asset_or_symbol(&self) -> Option<Asset> {
        match self {
            Self::None => None,
            Self::Exposure(underlying) => Some(underlying.clone()),
            Self::Asset(native) => Some(native.asset.clone()),
            Self::Token(token) => Some(token.underlying.clone()),
            Self::DefiSwap(_) => None,
            Self::CFD(cfd) => Some(cfd.base.clone()),
            Self::Symbol(symbol) => Some(symbol.symbol.as_str().into()),
            Self::Simple(instrument) => Some(instrument.base.clone()),
        }
    }
    #[inline]
    pub fn base(&self) -> Option<Asset> {
        match self {
            Self::CFD(cfd) => Some(cfd.base.clone()),
            Self::Simple(s) => Some(s.base.clone()),
            Self::None | Self::Exposure(_) | Self::Symbol(_) | Self::Asset(_) | Self::Token(_) | Self::DefiSwap(_) => {
                None
            }
        }
    }

    #[inline]
    pub fn quote(&self) -> Option<Asset> {
        match self {
            Self::CFD(cfd) => Some(cfd.quote.clone()),
            Self::Simple(s) => Some(s.quote.clone()),

            Self::None | Self::Symbol(_) | Self::Exposure(_) | Self::Asset(_) | Self::Token(_) | Self::DefiSwap(_) => {
                None
            }
        }
    }

    #[inline]
    pub fn from_chain_native(chain: Blockchain, native: Asset) -> Self {
        Self::Asset(AssetUniversal {
            location: Location::Blockchain(chain),
            asset: native,
        })
    }

    /// The location of the particular instrument.
    #[inline]
    pub fn location(&self) -> Location {
        match self {
            Self::None => Location::Global,
            Self::Exposure(_) => Location::Global,
            Self::Asset(native) => native.location,
            Self::Token(token) => Location::Blockchain(token.chain),
            Self::DefiSwap(swap) => Location::Blockchain(swap.chain),
            Self::CFD(cfd) => Location::Exchange(cfd.exchange),
            Self::Symbol(s) => Location::Exchange(s.exchange),
            Self::Simple(s) => Location::Exchange(s.exchange),
        }
    }

    /// The short string code for the location of the particular instrument.
    #[inline]
    pub fn location_ticker(&self) -> &'static str {
        match self {
            Self::None => Location::Global.ticker(),
            Self::Exposure(_) => Location::Global.ticker(),
            Self::Asset(native) => native.location.ticker(),
            Self::Token(token) => token.chain.ticker(),
            Self::DefiSwap(swap) => swap.chain.ticker(),
            Self::CFD(cfd) => cfd.exchange.ticker(),
            Self::Symbol(s) => s.exchange.ticker(),
            Self::Simple(s) => s.exchange.ticker(),
        }
    }
    pub fn includes(&self, other: &InstrumentCode) -> bool {
        match (self, other) {
            (Self::Simple(s), Self::Simple(o)) => s.includes(o),
            _ => self == other,
        }
    }
}

impl Debug for InstrumentCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InstrumentCode({self})")
    }
}

impl Serialize for InstrumentCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for InstrumentCode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = deserializer.deserialize_str(CowStrVisitor)?;
        InstrumentCode::from_str(&s).map_err(serde::de::Error::custom)
    }
}
