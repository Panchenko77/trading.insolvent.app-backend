use async_trait::async_trait;
use eyre::bail;
use gluesql::core::ast_builder;
use gluesql::core::ast_builder::{num, Build, ExprNode};
use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Payload;
use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSql, ToGlueSqlRow};
use lib::utils::get_time_milliseconds;
use trading_model::{Asset, InstrumentCode, SharedInstrumentManager};

use crate::db::gluesql::AssetIndexTable;
use crate::strategy::broadcast::AsyncBroadcaster;
use lib::gluesql::{Table, TableCreate, TableInfo, TableUpdateItem};
use lib::warn::WarnManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use trading_model::{L2OrderBook, MarketEvent};

#[derive(Debug, Clone, Copy, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, PartialEq, Serialize, Deserialize)]
pub struct DbRowPriceVolume {
    pub exchange_id: u8,
    pub asset_id: u64,

    // sell side
    pub best_ask_price: f64,
    pub best_ask_size: f64,
    pub best_ask_volume: f64,

    // buy side
    pub best_bid_price: f64,
    pub best_bid_size: f64,
    pub best_bid_volume: f64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl DbRowPriceVolume {
    pub fn asset(&self) -> Asset {
        let symbol_id = self.asset_id;
        unsafe { Asset::from_hash(symbol_id) }
    }
}

#[async_trait(?Send)]
impl<T: GStore + GStoreMut + Clone> TableCreate<DbRowPriceVolume> for Table<T, DbRowPriceVolume> {
    async fn create_table(&mut self) -> eyre::Result<()> {
        let sql = DbRowPriceVolume::get_ddl(self.table_name());
        match self.glue().execute(sql.as_str()).await {
            Err(e) => Err(e.into()),
            _ => Ok(()),
        }
    }
}
#[async_trait(?Send)]
impl<T: GStore + GStoreMut> TableUpdateItem<DbRowPriceVolume, T> for Table<T, DbRowPriceVolume> {
    async fn update(&mut self, row: DbRowPriceVolume, filter: Option<ExprNode<'static>>) -> eyre::Result<usize> {
        let Some(filter) = filter else {
            bail!("filter is needed for this update function");
        };
        let sql = ast_builder::table(self.table_name())
            .update()
            .set("best_ask_price", row.best_ask_price.to_gluesql())
            .set("best_ask_size", row.best_ask_size.to_gluesql())
            .set("best_ask_volume", row.best_ask_volume.to_gluesql())
            .set("best_bid_price", row.best_bid_price.to_gluesql())
            .set("best_bid_size", row.best_bid_size.to_gluesql())
            .set("best_bid_volume", row.best_bid_volume.to_gluesql())
            .set("updated_at", row.updated_at.to_gluesql())
            .filter(filter)
            .build()?;
        match self.glue().execute_stmt(&sql).await {
            Ok(Payload::Update(d)) => Ok(d),
            e => bail!("{e:?}"),
        }
    }
}

pub struct PriceVolumeManager<T: GStore + GStoreMut + Clone> {
    rx: kanal::AsyncReceiver<MarketEvent>,
    tx_external: AsyncBroadcaster<DbRowPriceVolume>,
    tx_worker: kanal::AsyncSender<DbRowPriceVolume>,
    worker: PriceVolumeMinion<T>,
    orderbooks: HashMap<InstrumentCode, L2OrderBook<100>>,
    manager: SharedInstrumentManager,
}

/// helper that stores best bid offer into the database concurrently
pub struct PriceVolumeMinion<T: GStore + GStoreMut + Clone> {
    rx: kanal::AsyncReceiver<DbRowPriceVolume>,
    table: Table<T, DbRowPriceVolume>,
    index_table: AssetIndexTable<T, DbRowPriceVolume>,
}
impl<T: GStore + GStoreMut + Clone> PriceVolumeMinion<T> {
    async fn load_store(&mut self) -> eyre::Result<()> {
        let row = self.rx.recv().await?;
        // this upsert should be handled to minion
        let filter = ast_builder::expr("asset_id").eq(num(row.asset_id));
        if let Err(err) = self.table.upsert(row.clone(), Some(filter)).await {
            tracing::error!("{err}");
        }
        let table = self.index_table.tables.get_mut(&row.asset()).unwrap();
        table.insert(row.clone()).await?;
        Ok(())
    }
}

impl<T: GStore + GStoreMut + Clone> PriceVolumeManager<T> {
    pub fn new(
        rx: kanal::AsyncReceiver<MarketEvent>,
        tx_external: AsyncBroadcaster<DbRowPriceVolume>,
        table: Table<T, DbRowPriceVolume>,
        index_table: AssetIndexTable<T, DbRowPriceVolume>,
        manager: SharedInstrumentManager,
    ) -> Self {
        let channel_size = 1;
        let (tx_worker, rx_worker) = kanal::bounded_async::<DbRowPriceVolume>(channel_size);
        PriceVolumeManager {
            rx,
            tx_external,
            tx_worker,
            worker: PriceVolumeMinion {
                rx: rx_worker,
                table,
                index_table,
            },
            orderbooks: HashMap::new(),
            manager,
        }
    }
    // TODO this rx receiver buffer gets full immediately after order is received,arguably because of the upsert
    /// maintains price volume table
    pub async fn run(&mut self) -> eyre::Result<()> {
        let mut warn_manager = WarnManager::new();
        loop {
            tokio::select! {
                res = self.rx.recv() => {
                    match res {
                        Ok(MarketEvent::Quotes(quotes)) => {
                            // insert internal orderbook buffer
                            let orderbook = self
                                .orderbooks
                                .entry(quotes.instrument.clone())
                                .or_insert_with(L2OrderBook::new);
                            orderbook.update_quotes(quotes.get_quotes());
                            // obtain L1 from L2
                            let Some((best_bid_price, best_bid_size, best_bid_volume)) = orderbook
                                .bids
                                .levels
                                .first()
                                .map(|level| (level.price, level.size, level.size * level.price))
                            else {
                                continue
                            };
                            let Some((best_ask_price, best_ask_size, best_ask_volume)) = orderbook
                                .asks
                                .levels
                                .first()
                                .map(|level| (level.price, level.size, level.size * level.price))
                            else {
                                continue
                            };
                            let Some(instrument) = self.manager.get(&quotes.instrument) else {
                                continue
                            };
                            // upsert into table
                            let row = DbRowPriceVolume {
                                asset_id: instrument.base.asset._hash(),
                                exchange_id: instrument.exchange as _,
                                best_ask_price,
                                best_ask_size,
                                best_ask_volume,
                                best_bid_price,
                                best_bid_size,
                                best_bid_volume,
                                created_at: get_time_milliseconds(),
                                updated_at: get_time_milliseconds(),
                            };
                            if let Err(err) = self.tx_worker.try_send(row) {
                                warn_manager.warn(&format!("send to worker error: {err}"));
                            }
                            if let Err(err) = self.tx_external.broadcast(row) {
                                warn_manager.warn(&format!("send to external error: {err}"));

                            }
                        }

                        // unrelated market event
                        Ok(_) => continue,
                        // channel error
                        Err(e) => {
                            if lib::signal::get_terminate_flag() {
                                return Ok(());
                            }
                            bail!("received error from channel: {e}")},
                    }
                }
                // concurrently let the worker do its job
                _ = self.worker.load_store() => {}
            }
        }
    }
}
