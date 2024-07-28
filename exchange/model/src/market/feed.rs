use eyre::bail;
use std::cmp::Ordering;
use std::hash::Hash;
use std::str::FromStr;

use crate::InstrumentCode;
use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use serde_with_macros::{DeserializeFromStr, SerializeDisplay};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Deserialize, Serialize, Display, FromStr)]
pub enum MarketFeedDepthUpdateKind {
    Update,
    Snapshot,
}
#[derive(Copy, Clone, Eq, Ord, Debug, Deserialize, Serialize, Display, FromStr)]
pub enum MarketFeedDepthLevels {
    #[display("1")]
    LEVEL1,
    #[display("5")]
    LEVEL5,
    #[display("10")]
    LEVEL10,
    #[display("20")]
    LEVEL20,
    #[display("50")]
    LEVEL50,
    #[display("100")]
    LEVEL100,
    #[display("500")]
    LEVEL500,
    Full,
    #[display("{0}")]
    Custom(u16),
}
impl MarketFeedDepthLevels {
    pub fn to_levels(&self) -> u16 {
        match self {
            Self::LEVEL1 => 1,
            Self::LEVEL5 => 5,
            Self::LEVEL10 => 10,
            Self::LEVEL20 => 20,
            Self::LEVEL50 => 50,
            Self::LEVEL100 => 100,
            Self::LEVEL500 => 500,
            Self::Full => 1000,
            Self::Custom(l) => *l,
        }
    }
}
impl PartialOrd for MarketFeedDepthLevels {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.to_levels().cmp(&other.to_levels()))
    }
}
impl PartialEq for MarketFeedDepthLevels {
    fn eq(&self, other: &Self) -> bool {
        self.to_levels() == other.to_levels()
    }
}
impl Hash for MarketFeedDepthLevels {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_levels().hash(state);
    }
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, DeserializeFromStr, SerializeDisplay)]
pub struct MarketFeedDepthSelector {
    pub kind: Option<MarketFeedDepthUpdateKind>,
    pub levels: Option<MarketFeedDepthLevels>,
}
impl MarketFeedDepthSelector {
    pub fn depth() -> Self {
        Self {
            kind: None,
            levels: None,
        }
    }
    pub fn depth_update() -> Self {
        Self {
            kind: Some(MarketFeedDepthUpdateKind::Update),
            levels: None,
        }
    }
    pub fn depth_snapshot() -> Self {
        Self {
            kind: Some(MarketFeedDepthUpdateKind::Snapshot),
            levels: None,
        }
    }
    pub fn depth_snapshot_l5() -> Self {
        Self {
            kind: Some(MarketFeedDepthUpdateKind::Snapshot),
            levels: Some(MarketFeedDepthLevels::LEVEL5),
        }
    }
    pub fn match_depth(&self, other: MarketFeedDepthKind) -> bool {
        self.kind.map_or(true, |k| k == other.kind) && self.levels.map_or(true, |l| l <= other.levels)
    }
}
impl FromStr for MarketFeedDepthSelector {
    type Err = eyre::Error;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("Depth") {
            bail!("Invalid MarketFeedDepthSelector: {}", s);
        }
        s = &s[5..];
        let kind_p = s.find('@').unwrap_or(s.len());
        let kind_s = &s[..kind_p];
        let kind = if kind_s.is_empty() {
            None
        } else {
            Some(MarketFeedDepthUpdateKind::from_str(&s[..kind_p])?)
        };
        let levels = if kind_p < s.len() {
            Some(MarketFeedDepthLevels::from_str(&s[kind_p + 1..])?)
        } else {
            None
        };
        Ok(Self { kind, levels })
    }
}
impl std::fmt::Display for MarketFeedDepthSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Depth")?;
        if let Some(kind) = self.kind {
            write!(f, "{}", kind)?;
        }
        if let Some(levels) = self.levels {
            write!(f, "@{}", levels)?;
        }
        Ok(())
    }
}
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, DeserializeFromStr, SerializeDisplay, Display, FromStr,
)]
pub enum MarketFeedSelector {
    Trade,
    BookTicker,
    OHLCVT,
    Liquidation,
    Price,
    FundingRate,
    #[display("{0}")]
    Depth(MarketFeedDepthSelector),
}
impl MarketFeedSelector {
    pub fn match_kind(&self, other: MarketFeedKind) -> bool {
        match (self, other) {
            (Self::Trade, MarketFeedKind::Trade) => true,
            (Self::BookTicker, MarketFeedKind::BookTicker) => true,
            (Self::OHLCVT, MarketFeedKind::OHLCVT) => true,
            (Self::Liquidation, MarketFeedKind::Liquidation) => true,
            (Self::Price, MarketFeedKind::Price) => true,
            (Self::FundingRate, MarketFeedKind::FundingRate) => true,
            (Self::Depth(s), MarketFeedKind::Depth(o)) => s.match_depth(o),
            _ => false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct MarketFeedDepthKind {
    pub kind: MarketFeedDepthUpdateKind,
    pub levels: MarketFeedDepthLevels,
}
impl MarketFeedDepthKind {
    pub const SNAPSHOT_LEVEL5: Self = Self {
        kind: MarketFeedDepthUpdateKind::Snapshot,
        levels: MarketFeedDepthLevels::LEVEL5,
    };
    pub const UPDATE_FULL: Self = Self {
        kind: MarketFeedDepthUpdateKind::Update,
        levels: MarketFeedDepthLevels::Full,
    };
    pub const SNAPSHOT_FULL: Self = Self {
        kind: MarketFeedDepthUpdateKind::Snapshot,
        levels: MarketFeedDepthLevels::Full,
    };
}
impl std::fmt::Display for MarketFeedDepthKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Depth{}@{}", self.kind, self.levels)
    }
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, SerializeDisplay, Display)]
pub enum MarketFeedKind {
    Trade,
    BookTicker,
    OHLCVT,
    Liquidation,
    Price,
    FundingRate,
    #[display("{0}")]
    Depth(MarketFeedDepthKind),
}

#[derive(Debug, Clone)]
pub struct MarketFeedMeta {
    pub instrument: InstrumentCode,
    pub kind: MarketFeedKind,
}
