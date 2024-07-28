////////////////////////////// ONE CHANCE SIGNAL
use async_trait::async_trait;
use gluesql::core::ast_builder::{table, Build, ExprNode};
use gluesql::core::executor::Payload;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};
use lib::gluesql::{Table, TableCreate, TableInfo};
use num_enum::{TryFromPrimitive, TryFromPrimitiveError};
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use trading_model::Asset;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TryFromPrimitive, Display)]
#[repr(u8)]
pub enum EventStatus {
    /// default when it just got generated but not analysed by the order placement
    Unused,
    /// when the opportunity size at event detection is too small
    TooSmallOpportunitySize,
    /// when fund is insufficient to generate orde
    InsufficientFund,
    /// when event level is insufficient to generate order
    BelowTriggerThreshold,
    /// when event is captured to generate order
    Captured,
    /// order is unfilled
    MissedOpportunity,
    /// position is partially filled
    PartialHit,
    /// position is fully filled
    FullyHit,
    /// Closing
    Closing,
    /// position is partially closed
    PartialClosed,
    /// position is fully closed
    FullyClosed,
    /// throttled
    Throttled,
    /// NotReady
    NotReady,
    /// ZeroPriceOrSize
    ZeroPriceOrSize,
    /// Errored
    Errored,
}
#[derive(Default, Debug, Clone, ReflectGlueSqlRow, FromGlueSqlRow, ToGlueSqlRow)]
pub struct DbRowEventPriceChangeAndDiff {
    pub id: u64,
    pub datetime: i64,
    pub asset_id: u64,
    pub signal_level: u8,
    pub signal_difference_id: u64,
    pub signal_change_id: u64,
    pub is_rising: bool,
    pub price: f64,
    pub last_price: f64,
    pub binance_price: f64,
    pub hyper_price: f64,
    pub difference_in_basis_points: f64,
    // below are appended later by livetest, thus optional
    // this is the same as the hyper_price above
    // pub hyper_price_at_event_generation: Option<f64>,
    pub hyper_price_at_order_close: Option<f64>,
    pub hyper_price_order_fill: Option<f64>,
    // for strategy 2 we also trade on binance to hedge risks
    pub bin_price_at_order_close: Option<f64>,
    pub bin_price_order_fill: Option<f64>,
    pub event_status_id: u8,
}
impl DbRowEventPriceChangeAndDiff {
    pub fn asset(&self) -> Asset {
        let symbol_id = self.asset_id;
        unsafe { Asset::from_hash(symbol_id) }
    }
    pub fn event_status(&self) -> Result<EventStatus, TryFromPrimitiveError<EventStatus>> {
        EventStatus::try_from(self.event_status_id)
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowEventPriceChangeAndDiff>
    for Table<T, DbRowEventPriceChangeAndDiff>
{
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowEventPriceChangeAndDiff::get_ddl(self.table_name());
        let res = self.glue().execute(sql.as_str()).await;
        match res {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}

#[async_trait(?Send)]
pub trait DbRowEventPriceChangeAndDiffExt {
    async fn update_hyper_price_current_actual(
        &mut self,
        symbol_id: u64,
        current: f64,
        actual: f64,
    ) -> eyre::Result<()>;
    async fn update_event_status(&mut self, event_id: u64, event_status: EventStatus) -> eyre::Result<()>;
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> DbRowEventPriceChangeAndDiffExt for Table<T, DbRowEventPriceChangeAndDiff> {
    async fn update_hyper_price_current_actual(
        &mut self,
        event_id: u64,
        current: f64,
        actual: f64,
    ) -> eyre::Result<()> {
        let filter = lib::gluesql::QueryFilter::id(event_id);
        let sql = table(self.table_name())
            .update()
            .set("hyper_price_at_order_close", ExprNode::Numeric(current.into()))
            .set("hyper_price_order_fill", ExprNode::Numeric(actual.into()))
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(1)) => Ok(()),
            Ok(Payload::Update(d)) => eyre::bail!("unmatched count [{d}] for event_id: {}", event_id),
            e => eyre::bail!("{e:?}"),
        }
    }
    async fn update_event_status(&mut self, event_id: u64, event_status: EventStatus) -> eyre::Result<()> {
        let filter = lib::gluesql::QueryFilter::id(event_id);
        let sql = table(self.table_name())
            .update()
            .set("event_status_id", ExprNode::Numeric((event_status as u8).into()))
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(1)) => Ok(()),
            Ok(Payload::Update(d)) => eyre::bail!("unmatched count [{d}] for event_id: {}", event_id),
            e => eyre::bail!("{e:?}"),
        }
    }
}
