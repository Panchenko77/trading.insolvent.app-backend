use std::str::FromStr;

use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

/// price
pub mod price;
/// sig price change
pub mod price_change;
/// bid difference signal event
pub mod price_difference;
pub mod price_manager;
/// price pair
pub mod price_spread;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TryFromPrimitive, IntoPrimitive, Deserialize,
)]
/// signal level
#[repr(u8)]
pub enum SignalLevel {
    Normal,
    High,
    Critical,
}

impl FromStr for SignalLevel {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "normal" => SignalLevel::Normal,
            "high" => SignalLevel::High,
            "critical" => SignalLevel::Critical,
            _ => eyre::bail!("no match"),
        })
    }
}
