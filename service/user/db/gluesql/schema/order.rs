use crate::db::worktable::orders::OrderRowView;
use async_trait::async_trait;
use gluesql::core::ast_builder::ExprNode;
use gluesql::core::{
    ast_builder::Build,
    error::Error,
    executor::Payload,
    store::{GStore, GStoreMut},
};
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use lib::gluesql::{QueryFilter, Table, TableCreate, TableInfo, TableSelectItem, TableUpdateItem};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use trading_exchange::model::{Order, OrderStatus, PositionEffect, RequestPlaceOrder};
use trading_model::{Side, TimeStampMs, NANOSECONDS_PER_MILLISECOND};

#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, Default, PartialEq, Serialize, Deserialize)]
pub struct DbRowOrder {
    /// primary key, the order id
    pub id: u64,
    // event ID of the event captured by the signal
    pub event_id: u64,
    /// exchange enum ID
    pub exchange_id: u8,
    /// symbol intern hash
    pub symbol_id: u64,
    /// client ID is the event ID
    pub client_id: String,
    /// price
    pub price: f64,
    /// volume
    pub volume: f64,
    /// utc time
    pub datetime: TimeStampMs,
    // OrderType enum ID
    pub order_type_id: u8,
    // Side enum ID
    pub side_id: u8,
    // position effect enum id
    pub position_effect_id: u8,
    // open order ID
    // TODO: the type is wrong
    pub open_order_id: Option<u8>,
    // status ID
    pub status_id: u8,
}
impl DbRowOrder {
    pub fn effect(&self) -> PositionEffect {
        PositionEffect::from_repr(self.position_effect_id).expect("failed parsing position type")
    }
    pub fn filter_by_cloid(&self) -> ExprNode<'static> {
        QueryFilter::eq_string("client_id", self.client_id.clone())
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableCreate<DbRowOrder> for Table<T, DbRowOrder> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowOrder::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
// update
#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableUpdateItem<DbRowOrder, T> for Table<T, DbRowOrder> {
    async fn update(&mut self, row: DbRowOrder, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let filter = filter.unwrap_or_else(|| row.filter_by_cloid());
        let sql = gluesql::core::ast_builder::table(self.table_name())
            .update()
            .set("exchange_id", row.exchange_id.to_gluesql())
            .set("symbol_id", row.symbol_id.to_gluesql())
            .set("client_id", row.client_id.to_gluesql())
            .set("price", row.price.to_gluesql())
            .set("volume", row.volume.to_gluesql())
            .set("datetime", row.datetime.to_gluesql())
            .set("order_type_id", row.order_type_id.to_gluesql())
            .set("side_id", row.side_id.to_gluesql())
            .set("position_effect_id", row.position_effect_id.to_gluesql())
            .set("event_id", row.event_id.to_gluesql())
            // .set("open_order_id", row.open_order_id.to_gluesql())
            .set("status_id", row.status_id.to_gluesql())
            .filter(filter)
            .build()?;

        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(d)) => Ok(d),
            Err(Error::StorageMsg(e)) => eyre::bail!("{e}"),
            e => eyre::bail!("{e:?}"),
        }
    }
}
#[async_trait(?Send)]
pub trait QueryByCloid {
    async fn get_row_by_cloid(&mut self, cloid: String) -> eyre::Result<Option<DbRowOrder>>;
    async fn update_status_by_cloid(&mut self, cloid: String, status: OrderStatus) -> eyre::Result<()>;
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> QueryByCloid for Table<T, DbRowOrder> {
    async fn get_row_by_cloid(&mut self, cloid: String) -> eyre::Result<Option<DbRowOrder>> {
        let filter = QueryFilter::eq_string("client_id", cloid);
        self.select_one(Some(filter), "id").await
    }
    async fn update_status_by_cloid(&mut self, cloid: String, status: OrderStatus) -> eyre::Result<()> {
        let filter = QueryFilter::eq_string("client_id", cloid.clone());
        let sql = gluesql::core::ast_builder::table(self.table_name())
            .update()
            .set("status_id", (status as u8).to_gluesql())
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(1)) => Ok(()),
            Ok(Payload::Update(d)) => eyre::bail!("unmatched count [{d}] for cloid: {}", cloid),
            Err(Error::StorageMsg(e)) => eyre::bail!("{e}"),
            e => eyre::bail!("{e:?}"),
        }
    }
}

impl From<RequestPlaceOrder> for DbRowOrder {
    fn from(order: RequestPlaceOrder) -> Self {
        DbRowOrder {
            // well local is fully under our control and is u64
            id: order.order_lid.parse().unwrap_or_else(|e| {
                warn!("Failed to parse local_id {:?}: {}", order.order_lid, e);
                0
            }),
            exchange_id: order.instrument.get_exchange().unwrap() as u8,
            symbol_id: order.instrument.get_symbol().unwrap()._hash(),
            // NOTE: client_id can be various
            client_id: order.order_cid.to_string(),
            price: order.price,
            volume: order.size,
            datetime: order.create_lt.millis(),
            order_type_id: order.ty as u8,
            side_id: order.side as u8,
            position_effect_id: order.effect as u8,
            event_id: order.event_id,
            open_order_id: None,
            status_id: OrderStatus::Pending as u8,
        }
    }
}

impl From<Order> for DbRowOrder {
    fn from(order: Order) -> Self {
        DbRowOrder {
            id: order.local_id.parse().unwrap_or_else(|e| {
                warn!("Failed to parse local_id {:?}: {}", order.local_id, e);
                0
            }),
            exchange_id: order.instrument.get_exchange().unwrap() as u8,
            symbol_id: order.instrument.get_symbol().unwrap()._hash(),
            client_id: order.client_id.to_string(),
            price: order.price,
            volume: order.size,
            datetime: order.create_lt.millis(),
            order_type_id: order.ty as u8,
            side_id: order.side as u8,
            position_effect_id: order.effect as u8,
            event_id: order.event_id,
            open_order_id: None,
            status_id: order.status as u8,
        }
    }
}
impl<'a> From<OrderRowView<'a>> for DbRowOrder {
    fn from(order: OrderRowView) -> Self {
        DbRowOrder {
            id: order.local_id().parse().unwrap_or_else(|e| {
                warn!("Failed to parse local_id {:?}: {}", order.local_id(), e);
                0
            }),
            exchange_id: order.exchange() as _,
            symbol_id: order.symbol()._hash(),
            client_id: order.client_id().to_string(),
            price: order.price(),
            volume: order.size(),
            datetime: order.create_lt() / NANOSECONDS_PER_MILLISECOND,
            order_type_id: order.ty() as _,
            side_id: order.side().unwrap_or(Side::Unknown) as _,
            position_effect_id: order.position_effect() as _,
            event_id: order.event_id() as _,
            open_order_id: None,
            status_id: order.status() as _,
        }
    }
}
