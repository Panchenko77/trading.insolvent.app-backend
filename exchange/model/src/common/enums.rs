use eyre::{bail, ContextCompat, Result};
use parse_display::{Display, FromStr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::str::FromStr;
use strum_macros::FromRepr;

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    SerializeDisplay,
    DeserializeFromStr,
    Ord,
    PartialOrd,
    Eq,
    Display,
    JsonSchema,
    FromRepr,
)]
#[repr(u8)]
pub enum Side {
    Unknown = 0,
    Buy = 1,
    Sell = 2,
}

impl Side {
    pub fn to_opt(&self) -> Option<Self> {
        match self {
            Self::Unknown => None,
            _ => Some(*self),
        }
    }
    pub fn upper(&self) -> &str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
    pub fn lower(&self) -> &str {
        match self {
            Self::Unknown => "unknown",
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }
    pub fn camel(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::Buy => "Buy",
            Self::Sell => "Sell",
        }
    }
    pub fn opposite(&self) -> Self {
        match self {
            Self::Unknown => Self::Unknown,
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
    pub fn sign(&self) -> f64 {
        match self {
            Self::Unknown => 0.0,
            Self::Buy => 1.0,
            Self::Sell => -1.0,
        }
    }
    pub fn from_sign(num: f64) -> Self {
        if num > 0.0 {
            Self::Buy
        } else {
            Self::Sell
        }
    }
}

impl FromStr for Side {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next().context("invalid side")? {
            'B' => Ok(Side::Buy),
            'b' => Ok(Side::Buy),
            'S' => Ok(Side::Sell),
            's' => Ok(Side::Sell),
            _ => bail!("invalid side: {}", s),
        }
    }
}

#[derive(
    Debug, Copy, Clone, PartialEq, SerializeDisplay, DeserializeFromStr, Ord, PartialOrd, Eq, Display, JsonSchema,
)]
pub enum Intent {
    Bid,
    Ask,
}

impl Intent {
    pub fn upper(&self) -> &str {
        match self {
            Self::Bid => "BID",
            Self::Ask => "ASK",
        }
    }
    pub fn lower(&self) -> &str {
        match self {
            Self::Bid => "bid",
            Self::Ask => "ask",
        }
    }
    pub fn camel(&self) -> &str {
        match self {
            Self::Bid => "Bid",
            Self::Ask => "Ask",
        }
    }
    pub fn opposite(&self) -> Self {
        match self {
            Self::Bid => Self::Ask,
            Self::Ask => Self::Bid,
        }
    }
    pub fn sign(&self) -> f64 {
        match self {
            Self::Bid => 1.0,
            Self::Ask => -1.0,
        }
    }
}
impl From<Side> for Intent {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => Intent::Bid,
            Side::Sell => Intent::Ask,
            _ => unreachable!(),
        }
    }
}
impl From<Intent> for Side {
    fn from(value: Intent) -> Self {
        match value {
            Intent::Bid => Side::Buy,
            Intent::Ask => Side::Sell,
        }
    }
}
impl FromStr for Intent {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next().context("invalid intent")? {
            'B' => Ok(Intent::Bid),
            'b' => Ok(Intent::Bid),
            'A' => Ok(Intent::Ask),
            'a' => Ok(Intent::Ask),
            _ => bail!("invalid intent: {}", s),
        }
    }
}

/// Position direction indicates the direction of the position: long, short, or both
/// some exchanges support hedge mode, which means you can open long and short positions at the same time
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize, Display, FromStr, Hash)]
pub enum PositionDirection {
    /// the default position direction, which means the position can be either long or short
    /// In case of spot, it's always long
    #[default]
    #[display("E")]
    Either,
    /// long position
    #[display("L")]
    Long,
    /// short position, and sell a short position means you are closing it
    #[display("S")]
    Short,
}
