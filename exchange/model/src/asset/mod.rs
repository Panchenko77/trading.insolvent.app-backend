use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;

use interning::{InternedString, InternedStringHash};
use schemars::JsonSchema;
use serde_with_macros::{DeserializeFromStr, SerializeDisplay};

pub use details::*;
pub use info::*;
pub use manager::*;
pub use selector::*;

use crate::Location;

mod details;
mod info;
mod manager;
mod selector;

pub type AssetId = u32;

#[derive(Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub struct Asset(InternedString);

impl Asset {
    pub fn empty() -> Self {
        Self::from("")
    }
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
    pub fn from_str_checked(s: &str) -> Self {
        Self(s.into())
    }
    pub fn _hash(&self) -> u64 {
        self.0.hash().hash()
    }
    pub unsafe fn from_hash(bytes: u64) -> Self {
        let hash = InternedStringHash::from_bytes(bytes.to_be_bytes());
        Self(InternedString::from_hash(hash))
    }
}

impl FromStr for Asset {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into()))
    }
}

impl From<&str> for Asset {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for Asset {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}
impl From<&String> for Asset {
    fn from(s: &String) -> Self {
        Self(s.as_str().into())
    }
}
impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.0.as_ref(), f)
    }
}

impl Debug for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Deref for Asset {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl JsonSchema for Asset {
    fn schema_name() -> String {
        "Asset".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
}
impl PartialOrd for Asset {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Ord for Asset {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    SerializeDisplay,
    DeserializeFromStr,
    parse_display::Display,
    parse_display::FromStr,
)]
#[display("{location}:{asset}")]
pub struct AssetUniversal {
    pub location: Location,
    pub asset: Asset,
}

impl AssetUniversal {
    pub fn new(location: Location, asset: Asset) -> Self {
        Self { location, asset }
    }
}
