use serde::Deserialize;

use trading_exchange::model::ExecutionConfig;
use trading_exchange::utils::crypto::PrivateKey;
use trading_model::Exchange;

mod batch;
mod registry;
mod router;

pub use batch::*;
pub use registry::*;
pub use router::*;

#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionPrivateKey {
    pub exchange: Exchange,
    pub account_id: String,
    pub private_key: PrivateKey,
}
#[derive(Debug, Clone)]
pub struct ExecutionKeys {
    pub keys: Vec<ExecutionPrivateKey>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionConfigMap {
    pub configs: Vec<ExecutionConfig>,
}
