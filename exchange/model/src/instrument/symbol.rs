use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;

use eyre::{eyre, Context, Result};
use interning::{InternedString, InternedStringHash};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with_macros::{DeserializeFromStr, SerializeDisplay};

use crate::{Exchange, InstrumentCategory};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Symbol(InternedString);

impl Symbol {
    pub fn empty() -> Self {
        Self::from("")
    }

    pub fn from_str_checked(s: &str) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn _hash(&self) -> u64 {
        self.0.hash().hash()
    }
    pub unsafe fn from_hash(hash: u64) -> Self {
        let hash = hash.to_be_bytes();
        let hash = InternedStringHash::from_bytes(hash);
        Self(InternedString::from_hash(hash))
    }
}

impl FromStr for Symbol {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into()))
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|s| Self::from(s))
    }
}

impl Deref for Symbol {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}
impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
pub struct InstrumentSymbol {
    pub exchange: Exchange,
    pub symbol: Symbol,
    pub category: Option<InstrumentCategory>,
}

impl InstrumentSymbol {
    pub fn new(exchange: Exchange, symbol: Symbol) -> Self {
        Self {
            exchange,
            symbol,
            category: None,
        }
    }
    pub fn new_with_category_opt(exchange: Exchange, symbol: Symbol, category: Option<InstrumentCategory>) -> Self {
        Self {
            exchange,
            symbol,
            category: category,
        }
    }
    pub fn new_with_category(exchange: Exchange, symbol: Symbol, cat: InstrumentCategory) -> Self {
        Self {
            exchange,
            symbol,
            category: Some(cat),
        }
    }

    pub fn without_category(self) -> Self {
        Self { category: None, ..self }
    }
}
impl Hash for InstrumentSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.exchange.hash(state);
        self.symbol.hash(state);
        if let Some(cat) = self.category {
            cat.hash(state);
        }
    }
}

impl Display for InstrumentSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.exchange, self.symbol)?;
        if let Some(cat) = self.category {
            write!(f, ":{}", cat)?;
        }
        Ok(())
    }
}

impl FromStr for InstrumentSymbol {
    type Err = eyre::Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split(":");
        let exchange = parts
            .next()
            .ok_or_else(|| eyre!("missing exchange: {}", s))?
            .parse()
            .with_context(|| format!("SymbolUniversal: {}", s))?;
        let symbol = parts
            .next()
            .ok_or_else(|| eyre!("missing symbol: {}", s))?
            .parse()
            .with_context(|| format!("SymbolUniversal: {}", s))?;
        let cat = parts
            .next()
            .map(|x| x.parse())
            .transpose()
            .with_context(|| format!("SymbolUniversal: {}", s))?;

        Ok(Self {
            exchange,
            symbol,
            category: cat,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol_universal() {
        let s = "BinanceMargin:BTCUSDT";
        let sym: InstrumentSymbol = s.parse().unwrap();
        assert_eq!(sym.exchange, Exchange::BinanceMargin);
        assert_eq!(sym.symbol, Symbol::from("BTCUSDT"));
        assert_eq!(sym.category, None);
    }
}
