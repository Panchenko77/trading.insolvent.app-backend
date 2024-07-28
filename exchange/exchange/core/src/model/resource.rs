use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ExecutionResource {
    /// place orders, order updates
    Execution,
    /// position & balance reporting
    Accounting,
}
