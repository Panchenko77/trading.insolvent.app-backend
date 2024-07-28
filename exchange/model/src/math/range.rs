use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}
impl Range {
    pub const UNLIMITED: Self = Self {
        min: f64::MIN,
        max: f64::MAX,
    };
    pub fn min(min: f64) -> Self {
        Self { min, max: f64::MAX }
    }
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }
    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }
    pub fn clamp(&self, value: f64) -> f64 {
        value.max(self.min).min(self.max)
    }
}
