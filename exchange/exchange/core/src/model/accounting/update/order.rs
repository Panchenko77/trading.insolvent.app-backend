use crate::model::OrderLid;
use trading_model::{InstrumentCode, Quantity, Side, Time};

#[derive(Debug, Clone, PartialEq)]
pub struct AccountingUpdateOrder {
    pub order_lid: OrderLid,
    pub instrument: InstrumentCode,
    pub side: Side,
    pub source_creation_timestamp: Time,
    pub accounting_close_timestamp: Option<Time>,
    pub total_quantity: Quantity,
    pub filled_quantity: Quantity,
    pub filled_cost_min: Quantity,
}

impl AccountingUpdateOrder {
    pub fn closed(&self) -> bool {
        self.accounting_close_timestamp.is_some()
    }
}
