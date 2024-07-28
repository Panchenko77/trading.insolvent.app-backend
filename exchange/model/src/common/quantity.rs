use serde::{Deserialize, Serialize};
pub type Quantity = f64;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialOrd,
    PartialEq,
    Serialize,
    Deserialize,
    parse_display::Display,
    parse_display::FromStr,
)]
pub enum QuantityUnit {
    Raw,
    Base,
    Quote,
    Notional,
}
