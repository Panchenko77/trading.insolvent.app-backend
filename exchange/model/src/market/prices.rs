use crate::{BucketSeries, PriceEvent, NANOSECONDS_PER_SECOND};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistory {
    pub(crate) prices: BucketSeries<PriceEvent>,
}
impl PriceHistory {
    pub fn new() -> Self {
        Self {
            prices: BucketSeries::new_bucket(60, Duration::from_secs(1)),
        }
    }
    pub fn push(&mut self, event: PriceEvent) {
        self.prices.push(event);
    }
    pub fn last(&self) -> Option<&PriceEvent> {
        self.prices.get(0)
    }
    pub fn len(&self) -> usize {
        self.prices.len()
    }
    pub fn total_len(&self) -> usize {
        self.prices.total_len()
    }
    /// returns the sample volatility of the price series
    /// vol(sample) = sd(log(r))
    pub fn volatility(&self) -> f64 {
        let mut log_returns: Vec<f64> = Vec::with_capacity(self.prices.len() - 1);
        let mut prev_price: Option<f64> = None;
        for price in self.prices.iter() {
            if let Some(prev_price) = prev_price {
                log_returns.push((price.price / prev_price).ln());
            }
            prev_price = Some(price.price);
        }
        let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
        let variance = log_returns.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / log_returns.len() as f64;
        variance.sqrt()
    }
    /// returns the daily volatility of the price series
    /// vol(day) = vol(sample) * sqrt(n of intervals)
    pub fn day_volatility(&self) -> f64 {
        let Some(duration) = self.prices.duration() else {
            return 0.0;
        };
        let n_intervals = (86400 * NANOSECONDS_PER_SECOND) / duration;
        self.volatility() * (n_intervals as f64).sqrt()
    }
}
