use crate::model::{AccountingUpdateOrder, FundingPayment, OrderTrade};

#[derive(Debug, Clone, PartialEq)]
pub enum AccountingUpdate {
    Order(AccountingUpdateOrder),
    Trade(OrderTrade),
    Funding(FundingPayment),
}

impl From<AccountingUpdateOrder> for AccountingUpdate {
    fn from(val: AccountingUpdateOrder) -> Self {
        AccountingUpdate::Order(val)
    }
}

impl From<OrderTrade> for AccountingUpdate {
    fn from(val: OrderTrade) -> Self {
        AccountingUpdate::Trade(val)
    }
}

impl From<FundingPayment> for AccountingUpdate {
    fn from(val: FundingPayment) -> Self {
        AccountingUpdate::Funding(val)
    }
}
