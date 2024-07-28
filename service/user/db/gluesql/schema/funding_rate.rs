use async_trait::async_trait;
use eyre::bail;
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::{col, Build, ExprNode};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Payload;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use interning::{InternedString, InternedStringHash};

use build::model::UserFundingRates;
use lib::gluesql::{Table, TableCreate, TableInfo, TableUpdateItem};
use trading_model::Exchange;
use trading_model::FundingRateEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, FromGlueSqlRow, ToGlueSqlRow, ReflectGlueSqlRow)]
pub struct DbRowFundingRate {
    pub id: u64,
    pub exchange_id: u8,
    pub symbol: u64,
    pub funding_rate: f64,
    pub timestamp: i64,
}

impl DbRowFundingRate {
    pub fn exchange(&self) -> Exchange {
        self.exchange_id.try_into().unwrap()
    }
    pub fn symbol(&self) -> InternedString {
        unsafe { InternedString::from_hash(InternedStringHash::new(self.symbol)) }
    }
}

impl From<FundingRateEvent> for DbRowFundingRate {
    fn from(event: FundingRateEvent) -> Self {
        let symbol = event.instrument.get_symbol().unwrap();
        let exchange = event.instrument.get_exchange().unwrap();
        Self {
            id: 0,
            exchange_id: exchange as _,
            symbol: symbol._hash(),
            funding_rate: event.funding_rate,
            timestamp: event.exchange_time.millis(),
        }
    }
}
impl From<DbRowFundingRate> for UserFundingRates {
    fn from(value: DbRowFundingRate) -> Self {
        UserFundingRates {
            exchange: value.exchange().to_string(),
            symbol: value.symbol().to_string(),
            rate: value.funding_rate,
            datetime: value.timestamp,
        }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableCreate<DbRowFundingRate> for Table<T, DbRowFundingRate> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowFundingRate::get_ddl(self.table_name());
        self.execute(&sql).await?;
        Ok(())
    }
}
// update
#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableUpdateItem<DbRowFundingRate, T> for Table<T, DbRowFundingRate> {
    async fn update(&mut self, row: DbRowFundingRate, _filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let stmt = ast_builder::table(self.table_name())
            .update()
            .filter(
                col("exchange_id")
                    .eq(row.exchange_id.to_gluesql())
                    .and(col("symbol").eq(row.symbol.to_gluesql())),
            )
            .set("funding_rate", row.funding_rate.to_gluesql())
            .set("timestamp", row.timestamp.to_gluesql())
            .build()?;
        let payload = self.execute_stmt(&stmt).await?;
        match payload {
            Payload::Update(n) => Ok(n),
            _ => bail!("unexpected payload: {:?}", payload),
        }
    }
}
