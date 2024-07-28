use async_trait::async_trait;
use eyre::bail;
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::{num, Build, ExprNode};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Payload;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use serde::{Deserialize, Serialize};

use lib::gluesql::{Table, TableCreate, TableInfo, TableUpdateItem};
use trading_model::TimeStampUs;

/// status per trade found by the event
#[derive(Default, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum TradeStatus {
    #[default]
    Unknown,
    NotStarted,
    OpeningLong,
    OpeningShort,
    ClosingLong,
    ClosingShort,
    Complete,
    Aborted,
}
impl TradeStatus {
    pub fn to_number(&self) -> u8 {
        match self {
            TradeStatus::Unknown => 0,
            TradeStatus::NotStarted => 1,
            TradeStatus::OpeningLong => 2,
            TradeStatus::OpeningShort => 3,
            TradeStatus::ClosingLong => 4,
            TradeStatus::ClosingShort => 5,
            TradeStatus::Complete => 6,
            TradeStatus::Aborted => 7,
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            TradeStatus::Unknown => "Unknown",
            TradeStatus::NotStarted => "NotStarted",
            TradeStatus::OpeningLong => "OpeningLong",
            TradeStatus::OpeningShort => "OpeningShort",
            TradeStatus::ClosingLong => "ClosingLong",
            TradeStatus::ClosingShort => "ClosingShort",
            TradeStatus::Complete => "Complete",
            TradeStatus::Aborted => "Aborted",
        }
        .to_string()
    }
    pub fn from_number(n: u8) -> TradeStatus {
        match n {
            0 => TradeStatus::Unknown,
            1 => TradeStatus::NotStarted,
            2 => TradeStatus::OpeningLong,
            3 => TradeStatus::OpeningShort,
            4 => TradeStatus::ClosingLong,
            5 => TradeStatus::ClosingShort,
            6 => TradeStatus::Complete,
            7 => TradeStatus::Aborted,
            _ => panic!("Invalid trade status number"),
        }
    }
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowTradeStatus {
    pub id: u64,
    // /// event id from event that triggers the order
    // pub event_id: String,
    pub datetime: TimeStampUs,
    pub status: u8,
}
impl DbRowTradeStatus {
    pub fn status(&self) -> TradeStatus {
        TradeStatus::from_number(self.status)
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowTradeStatus> for Table<T, DbRowTradeStatus> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowTradeStatus::get_ddl(self.table_name());
        match self.execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableUpdateItem<DbRowTradeStatus, T> for Table<T, DbRowTradeStatus> {
    async fn update(&mut self, row: DbRowTradeStatus, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let Some(filter) = filter else {
            eyre::bail!("filter is needed for this update function");
        };
        let sql = ast_builder::table(self.table_name())
            .update()
            .set("status", num(row.status))
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(d)) => Ok(d),
            e => bail!("{e:?}"),
        }
    }
}
