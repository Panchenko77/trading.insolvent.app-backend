use serde::{Deserialize, Serialize};

use crate::math::size::{Size, Value};
use crate::model::Asset;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetInfo {
    pub asset: Asset,
    /// convert between wire and human values
    ///
    /// human value = wire.multiply(wire value)
    ///
    /// wire value = wire.multiple_of(human value)
    pub wire: Size,
}

impl AssetInfo {
    pub fn empty() -> Self {
        Self {
            asset: Asset::empty(),
            wire: Size::ONE,
        }
    }
    pub fn new(asset: Asset, wire: Size) -> Self {
        Self { asset, wire }
    }
    pub fn new_one(asset: Asset) -> Self {
        Self {
            asset,
            wire: Size::ONE,
        }
    }
    pub fn to_wire(&self, v: Value) -> Value {
        self.wire.multiple_of(v)
    }
    pub fn from_wire(&self, v: Value) -> Value {
        self.wire.multiply(v)
    }
}
