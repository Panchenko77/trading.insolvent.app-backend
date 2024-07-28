use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use async_trait::async_trait;
use eyre::bail;
use gluesql::core::ast::Statement;
use gluesql::core::ast_builder::*;
use gluesql::core::error::Error;
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Glue;
use gluesql::prelude::Payload;
use gluesql::prelude::Value;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};

////////////////////////////// TABLE NAME

/// trait to represent a row in table, do not misuse it for other purpose
pub trait DbRow: ReflectGlueSqlRow + FromGlueSqlRow + ToGlueSqlRow + Debug {}
impl<T: ReflectGlueSqlRow + FromGlueSqlRow + ToGlueSqlRow + Debug> DbRow for T {}

/// single table with type, name and storage
pub struct Table<G: GStore + GStoreMut, D: DbRow> {
    glue: Glue<G>,
    table_name: String,
    row_type: PhantomData<D>,
    index: Arc<AtomicU64>,
}
impl<G: GStore + GStoreMut + Clone, D: DbRow> Clone for Table<G, D> {
    fn clone(&self) -> Self {
        Self {
            glue: Glue::new(self.glue.storage.clone()),
            table_name: self.table_name.clone(),
            row_type: self.row_type,
            index: self.index.clone(),
        }
    }
}
impl<G: GStore + GStoreMut, D: DbRow> Table<G, D> {
    /// we have to explicitly specify the row type of the table
    pub fn new(table_name: impl AsRef<str>, storage: G) -> Self {
        Table {
            glue: Glue::new(storage),
            table_name: table_name.as_ref().to_string(),
            row_type: PhantomData,
            index: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn next_index(&self) -> u64 {
        let old = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        old + 1
    }
    pub fn set_index(&mut self, index: u64) {
        self.index.store(index, std::sync::atomic::Ordering::SeqCst);
    }

    pub async fn insert(&mut self, row: D) -> eyre::Result<()>
    where
        Self: TableInfo<G>,
    {
        let sql: Statement = table(self.table_name())
            .insert()
            .columns(D::columns())
            .values(vec![row.to_gluesql_row()])
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Insert(1)) => Ok(()),
            Ok(Payload::Insert(size)) => bail!("unmatched insertion, expected 1, found {size}"),
            Ok(p) => bail!("unexpected payload {p:?}"),
            Err(Error::StorageMsg(e)) => bail!(e),
            Err(err) => bail!("error executing insert {:?}: {:?}", sql, err),
        }
    }
    pub async fn insert_to<R>(&mut self, table_name: &str, row: R) -> eyre::Result<()>
    where
        R: DbRow,
        Self: TableInfo<G>,
    {
        // info!("Inserting into table {}: {:?}", self.table_name(), row);
        let sql: Statement = table(table_name)
            .insert()
            .columns(R::columns())
            .values(vec![row.to_gluesql_row()])
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Insert(1)) => Ok(()),
            Ok(Payload::Insert(size)) => bail!("unmatched insertion, expected 1, found {size}"),
            Ok(p) => bail!("unexpected payload {p:?}"),
            Err(Error::StorageMsg(e)) => bail!(e),
            Err(err) => bail!("error executing insert {:?}: {:?}", sql, err),
        }
    }
    pub async fn upsert(&mut self, row: D, filter: Option<ExprNode<'static>>) -> eyre::Result<usize>
    where
        Self: TableSelectItem<D, G> + TableUpdateItem<D, G>,
    {
        let row_selected = self.select_unordered(filter.clone()).await?;
        if row_selected.is_empty() {
            self.insert(row).await.map(|_| 0)
        } else {
            self.update(row, filter).await
        }
    }
}

impl<G: GStore + GStoreMut, D: DbRow> Deref for Table<G, D> {
    type Target = Glue<G>;
    fn deref(&self) -> &Self::Target {
        &self.glue
    }
}
impl<G: GStore + GStoreMut, D: DbRow> DerefMut for Table<G, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.glue
    }
}

// implement traits for Table<G, D>
impl<G: GStore + GStoreMut, D: DbRow> TableSelectItem<D, G> for Table<G, D> {}
impl<R: DbRow, T: GStore + GStoreMut> TableOverwriteItem<R, T> for Table<T, R> {}
impl<G: GStore + GStoreMut, D: DbRow> TableDeleteItem<D, G> for Table<G, D> {}
impl<G: GStore + GStoreMut, D: DbRow> TableGetIndex<D, G> for Table<G, D> {}
impl<G: GStore + GStoreMut, D: DbRow> TableInfo<G> for Table<G, D> {
    type DbRowType = D;
    fn table_name(&mut self) -> &mut String {
        &mut self.table_name
    }
    fn glue(&mut self) -> &mut Glue<G> {
        &mut self.glue
    }
}
pub trait TableInfo<T: GStore + GStoreMut> {
    /// associated type to represent a table
    type DbRowType;
    /// get reference of table name
    fn table_name(&mut self) -> &mut String;
    /// get mutable reference of glue
    fn glue(&mut self) -> &mut Glue<T>;
}

////////////////////////////// STATEMENT

pub struct QueryStatement<R: DbRow> {
    row_type: PhantomData<R>,
}
impl<R: DbRow> QueryStatement<R> {
    /// get last index (AST)
    pub fn get_last_index(table_name: &str) -> gluesql::prelude::Result<Statement> {
        table(table_name)
            .select()
            .project("id")
            .order_by("id DESC")
            .limit(1)
            .build()
    }
    /// delete from FOO until BAR (AST)
    pub fn delete_from_until(
        table_name: &str,
        from_ms: Option<i64>,
        until_ms: Option<i64>,
    ) -> gluesql::prelude::Result<Statement> {
        table(table_name)
            .delete()
            .filter(QueryFilter::range(from_ms, until_ms))
            .build()
    }
    /// delete from FOO until BAR (AST)
    pub fn delete(table_name: &str, filter: Option<ExprNode<'static>>) -> gluesql::prelude::Result<Statement> {
        match filter {
            Some(filter) => table(table_name).delete().filter(filter).build(),
            None => table(table_name).delete().build(),
        }
    }
    // select all
    pub fn select_all(
        table_name: &str,
        filter: impl Into<ExprNode<'static>>,
        order: impl Into<OrderByExprList<'static>>,
    ) -> gluesql::prelude::Result<Statement> {
        table(table_name)
            .select()
            .filter(filter)
            .project(R::columns())
            .order_by(order)
            .build()
    }
}

////////////////////////////// FILTERS

pub struct QueryFilter;
impl QueryFilter {
    /// set up range filter
    pub fn range(from_ms: Option<i64>, until_ms: Option<i64>) -> ExprNode<'static> {
        match (from_ms, until_ms) {
            (Some(from), Some(until)) => expr("datetime").gte(num(from)).and(expr("datetime").lte(num(until))),
            (Some(from), None) => expr("datetime").gte(num(from)),
            (None, Some(until)) => expr("datetime").lte(num(until)),
            (None, None) => expr("true"),
        }
    }
    /// filter.symbol.eq
    pub fn symbol_id(symbol_id: u64) -> ExprNode<'static> {
        QueryFilter::u64("symbol_id", symbol_id)
    }
    pub fn asset_id(asset_id: u64) -> ExprNode<'static> {
        QueryFilter::u64("asset_id", asset_id)
    }
    /// filter.symbol.eq
    pub fn id(id: u64) -> ExprNode<'static> {
        QueryFilter::u64("id", id)
    }
    pub fn eq(field: impl AsRef<str>, v: impl Into<ExprNode<'static>>) -> ExprNode<'static> {
        expr(field.as_ref().to_string()).eq(v.into())
    }

    pub fn gte(field: impl AsRef<str>, v: impl Into<ExprNode<'static>>) -> ExprNode<'static> {
        expr(field.as_ref().to_string()).gte(v.into())
    }

    pub fn gt(field: impl AsRef<str>, v: impl Into<ExprNode<'static>>) -> ExprNode<'static> {
        expr(field.as_ref().to_string()).gt(v.into())
    }

    pub fn lte(field: impl AsRef<str>, v: impl Into<ExprNode<'static>>) -> ExprNode<'static> {
        expr(field.as_ref().to_string()).lte(v.into())
    }

    pub fn lt(field: impl AsRef<str>, v: impl Into<ExprNode<'static>>) -> ExprNode<'static> {
        expr(field.as_ref().to_string()).lt(v.into())
    }

    /// filter any u64 value
    pub fn u64(key: impl AsRef<str>, value: u64) -> ExprNode<'static> {
        col(key.as_ref().to_string()).eq(num(value))
    }
    /// filter any string balue
    pub fn eq_string(key: impl AsRef<str>, value: impl AsRef<str>) -> ExprNode<'static> {
        col(key.as_ref().to_string()).eq(text(value.as_ref().to_string()))
    }
}

////////////////////////////// traits

/// create table
/// T to work around orphan rule
#[async_trait(?Send)]
pub trait TableCreate<T: ReflectGlueSqlRow> {
    async fn create_table(&mut self) -> eyre::Result<()>;
}

/// select item
#[async_trait(?Send)]
pub trait TableSelectItem<R: DbRow, T: GStore + GStoreMut>: TableInfo<T> {
    async fn select_unordered(&mut self, filter: Option<ExprNode<'static>>) -> eyre::Result<Vec<R>> {
        let sql = if let Some(filter) = filter {
            table(self.table_name())
                .select()
                .filter(filter)
                .project(R::columns())
                .build()?
        } else {
            table(self.table_name()).select().project(R::columns()).build()?
        };
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Select { labels, rows }) => Ok(R::from_gluesql_rows(&labels, rows)?),
            Err(e) => bail!("{e:?}"),
            e => bail!("{e:?}"),
        }
    }
    async fn select(
        &mut self,
        filter: Option<ExprNode<'static>>,
        order: impl Into<OrderByExprList<'static>>,
    ) -> eyre::Result<Vec<R>> {
        let sql = if let Some(filter) = filter {
            table(self.table_name())
                .select()
                .filter(filter)
                .project(R::columns())
                .order_by(order)
                .build()?
        } else {
            table(self.table_name())
                .select()
                .project(R::columns())
                .order_by(order)
                .build()?
        };
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Select { labels, rows }) => Ok(R::from_gluesql_rows(&labels, rows)?),
            Err(Error::StorageMsg(e)) => bail!("{e:?}"),
            e => bail!("{e:?}"),
        }
    }
    async fn select_limit(
        &mut self,
        filter: Option<ExprNode<'static>>,
        order: impl Into<OrderByExprList<'static>>,
        limit: Option<u64>,
    ) -> eyre::Result<Vec<R>> {
        let sql = if let Some(filter) = filter {
            table(self.table_name())
                .select()
                .filter(filter)
                .project(R::columns())
                .order_by(order)
                .limit(limit.to_gluesql())
                .build()?
        } else {
            table(self.table_name()).select().project(R::columns()).build()?
        };
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Select { labels, rows }) => Ok(R::from_gluesql_rows(&labels, rows)?),
            Err(Error::StorageMsg(e)) => bail!("{e:?}"),
            e => bail!("{e:?}"),
        }
    }
    async fn select_one_unordered(&mut self, filter: Option<ExprNode<'static>>) -> eyre::Result<R> {
        let mut rows = self.select_unordered(filter).await?;
        let len = rows.len();
        match len {
            0 => bail!("expected 1, but no item found in database"),
            _ => Ok(rows.swap_remove(0)),
        }
    }
    async fn select_one(
        &mut self,
        filter: Option<ExprNode<'static>>,
        order: impl Into<OrderByExprList<'static>>,
    ) -> eyre::Result<Option<R>> {
        let mut rows = self.select(filter, order).await?;
        let len = rows.len();
        match len {
            0 => Ok(None),
            _ => Ok(Some(rows.swap_remove(0))),
        }
    }
    async fn get_by_id(&mut self, id: u64) -> eyre::Result<Option<R>> {
        let filter = QueryFilter::id(id);
        let mut rows = self.select_unordered(Some(filter)).await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows.swap_remove(0))),
            _ => bail!("expected 1, but {} item found in database", rows.len()),
        }
    }
}
/// update item
#[async_trait(?Send)]
pub trait TableUpdateItem<R: DbRow, T: GStore + GStoreMut>: TableInfo<T> {
    /// option::None is for the edge case where there is only one item in the table
    async fn update(&mut self, row: R, filter: Option<ExprNode<'static>>) -> eyre::Result<usize>;
}
// overwrite row
#[async_trait(?Send)]
pub trait TableOverwriteItem<R: DbRow, T: GStore + GStoreMut>: TableInfo<T> {
    async fn overwrite(&mut self, id: u64, row: &R) -> eyre::Result<()> {
        let mut sql = table(self.table_name()).update().filter(expr("id").eq(num(id)));
        let columns = R::columns();
        let values = row.to_gluesql_row();
        for (column, value) in columns.into_iter().zip(values.into_iter()) {
            sql = sql.set(column, value);
        }
        let sql = sql.build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(0)) => bail!("no item found to update"),
            Ok(Payload::Update(1)) => Ok(()),
            Ok(Payload::Update(n)) => bail!("unexpected update count {n}"),
            Ok(res) => bail!("wrong AST payload, check AST [{res:?}]"),
            Err(e) => Err(e.into()),
        }
    }
}
/// TODO implement update field
#[async_trait(?Send)]
pub trait TableUpdateField<T: GStore + GStoreMut>: TableInfo<T> {
    async fn update_field(
        &mut self,
        _fields: Vec<(&str, ExprNode<'async_trait>)>,
        _filter: ExprNode<'static>,
    ) -> eyre::Result<usize> {
        // let tablename = self.table_name();
        // let mut this = table(tablename).update().clone();
        // for (id, value) in fields.clone() {
        //     this = this.set(id, value.clone());
        // }
        todo!("implement this")
    }
}

/// delete item
#[async_trait(?Send)]
pub trait TableDeleteItem<R: DbRow, T: GStore + GStoreMut>: TableInfo<T> {
    async fn delete_from_until(
        &mut self,
        datetime_from_ms: Option<i64>,
        datetime_until_ms: Option<i64>,
    ) -> eyre::Result<usize> {
        let sql = QueryStatement::<R>::delete_from_until(self.table_name(), datetime_from_ms, datetime_until_ms)?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Delete(count)) => Ok(count),
            Ok(res) => bail!("wrong AST payload, check AST [{res:?}]"),
            Err(e) => Err(e.into()),
        }
    }
    async fn delete(&mut self, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let sql = QueryStatement::<R>::delete(self.table_name(), filter)?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Delete(count)) => Ok(count),
            Ok(res) => bail!("wrong AST payload, check AST [{res:?}]"),
            Err(e) => Err(e.into()),
        }
    }
}
/// get last index
#[async_trait(?Send)]
pub trait TableGetIndex<R: DbRow, T: GStore + GStoreMut>: TableInfo<T> {
    /// get last index
    async fn get_last_index(&mut self) -> eyre::Result<Option<u64>> {
        let sql = QueryStatement::<R>::get_last_index(self.table_name())?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Select { labels: _, rows }) => {
                let res = rows.first();

                let row = match res {
                    Some(row) => row,
                    None => return Ok(None),
                };
                if let Value::U64(s) = row[0] {
                    Ok(Some(s))
                } else {
                    bail!("unexpected result")
                }
            }
            Err(e) => bail!("unexpected result, {e}"),
            Ok(e) => bail!("unexpected payload {e:?}"),
        }
    }
}
