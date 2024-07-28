use eyre::Result;
use itertools::Itertools;
use std::fmt::Debug;
use std::ops::Deref;
use std::str::FromStr;
use tracing::warn;
use trading_exchange::model::{OrderStatus, OrderType, PositionEffect, RequestPlaceOrder, UpdateOrder};
use trading_model::{now, Exchange, InstrumentSymbol, Side, Symbol, Time, TimeStampNs};
use worktable::field;
use worktable::{RowView, RowViewMut, WorkTable};

pub struct OrdersWorkTable {
    worktable: WorkTable,
}
impl Debug for OrdersWorkTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OrderManagerWorkTable")
    }
}
impl std::fmt::Display for OrdersWorkTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.get_all_rows() {
            writeln!(f, "{row}")?;
        }
        Ok(())
    }
}
field!(0, LocalIdCol: String, "local_id", primary = true);
field!(1, ExchangeCol: String, "exchange");
field!(2, SymbolCol: String, "symbol");
field!(3, ClientIdCol: String, "client_id");
field!(4, ServerIdCol: String, "server_id");
field!(5, PriceCol: f64, "price");
field!(6, SizeCol: f64, "size");
field!(7, UpdateLtCol: TimeStampNs, "update_lt");
field!(8, OrderTypeCol: String, "order_type");
field!(9, SideCol: String, "side");
field!(10, PositionEffectCol: String, "position_effect");
field!(11, StatusCol: String, "status");
field!(12, CreateLtCol: TimeStampNs, "create_lt");
field!(13, StrategyIdCol: i64, "strategy_id");
field!(14, OpenOrderClientId: String, "open_order_client_id");
field!(15, EventId: i64, "event_id");
field!(16, FilledSizeCol: f64, "filled_size");
field!(17, UpdateTstCol: TimeStampNs, "update_tst");

impl OrdersWorkTable {
    pub fn new() -> Self {
        let mut worktable = WorkTable::new();
        worktable.add_field(LocalIdCol);
        worktable.add_field(ExchangeCol);
        worktable.add_field(SymbolCol);
        worktable.add_field(ClientIdCol);
        worktable.add_field(ServerIdCol);
        worktable.add_field(PriceCol);
        worktable.add_field(SizeCol);
        worktable.add_field(UpdateLtCol);
        worktable.add_field(OrderTypeCol);
        worktable.add_field(SideCol);
        worktable.add_field(PositionEffectCol);
        worktable.add_field(StatusCol);
        worktable.add_field(CreateLtCol);
        worktable.add_field(StrategyIdCol);
        worktable.add_field(OpenOrderClientId);
        worktable.add_field(EventId);
        worktable.add_field(FilledSizeCol);
        worktable.add_field(UpdateTstCol);
        Self { worktable }
    }
    pub fn remove_by_cloid(&mut self, cloid: &str) {
        self.worktable.retain(|row| row.index(ClientIdCol) != cloid);
    }

    #[deprecated]
    pub fn get_row_by_ids(&self, local_id: &str, client_id: &str, server_id: &str) -> Option<OrderRowView> {
        self.worktable
            .iter()
            .find(|row| {
                if !local_id.is_empty() && row.index(LocalIdCol) == local_id {
                    return true;
                }
                if !client_id.is_empty() && row.index(ClientIdCol) == client_id {
                    return true;
                }
                if !server_id.is_empty() && row.index(ServerIdCol) == server_id {
                    return true;
                }
                false
            })
            .map(OrderRowView)
    }
    /// get row with matching order IDs (either one match will return)
    pub fn get_row_mut_by_ids(&mut self, local_id: &str, client_id: &str, server_id: &str) -> Option<OrderRowViewMut> {
        self.worktable.iter_mut().map(OrderRowViewMut).find(|row| {
            if !local_id.is_empty() && row.index(LocalIdCol) == local_id {
                return true;
            }
            if !client_id.is_empty() && row.index(ClientIdCol) == client_id {
                return true;
            }
            if !server_id.is_empty() && row.index(ServerIdCol) == server_id {
                return true;
            }
            false
        })
    }
    pub fn get_row_by_cloid(&self, cloid: &str) -> Option<OrderRowView> {
        let is_match = |row: &RowView| row.index(ClientIdCol) == cloid;
        self.worktable.iter().find(is_match).map(OrderRowView)
    }
    pub fn get_row_by_local_id(&self, local_id: &str) -> Option<OrderRowView> {
        let is_match = |row: &RowView| row.index(LocalIdCol) == local_id;
        self.worktable.iter().find(is_match).map(OrderRowView)
    }
    pub fn get_row_mut_by_cloid(&mut self, cloid: &str) -> Option<OrderRowViewMut> {
        let is_match = |row: &RowViewMut| row.index(ClientIdCol) == cloid;
        self.worktable.iter_mut().find(is_match).map(OrderRowViewMut)
    }
    /// update order row that has same same IDs as input UpdateOrder struct
    pub fn update(&mut self, update: &mut UpdateOrder) -> Result<()> {
        if !update.reason.is_empty() {
            warn!(
                "UpdateOrder: lid={} {:?} {}",
                update.local_id, update.status, update.reason
            );
        }
        let mut row = match self.get_row_mut_by_cloid(&update.client_id) {
            Some(row) => row,
            None => {
                if !update.status.is_dead() {
                    self.insert_update(update);
                    return Ok(());
                }
                return Ok(());
            }
        };
        row.apply_update(update);
        // write back the missing IDs
        update.local_id = row.index(LocalIdCol).as_str().into();
        update.client_id = row.index(ClientIdCol).as_str().into();
        update.server_id = row.index(ServerIdCol).as_str().into();
        Ok(())
    }

    /// get all rows as OrderRowView
    pub fn get_all_rows(&self) -> Vec<OrderRowView> {
        self.worktable.iter().map(OrderRowView).collect_vec()
    }
    pub fn get_all_rows_mut(&mut self) -> Vec<OrderRowViewMut> {
        self.worktable.iter_mut().map(OrderRowViewMut).collect_vec()
    }
    pub fn iter(&self) -> impl Iterator<Item = OrderRowView> {
        self.worktable.iter().map(OrderRowView)
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = OrderRowViewMut> {
        self.worktable.iter_mut().map(OrderRowViewMut)
    }

    pub fn insert_update(&mut self, update: &UpdateOrder) {
        let exchange = update.instrument.get_exchange().unwrap().to_string();
        let symbol = update.instrument.get_symbol().unwrap().to_string();

        self.worktable
            .insert()
            .set(LocalIdCol, update.local_id.to_string())
            .set(ExchangeCol, exchange.to_string())
            .set(SymbolCol, symbol.to_string())
            .set(ClientIdCol, update.client_id.to_string())
            .set(ServerIdCol, update.server_id.to_string())
            .set(PriceCol, update.price)
            .set(SizeCol, update.size)
            .set(UpdateLtCol, update.update_lt.nanos())
            .set(OrderTypeCol, update.ty.to_string())
            .set(SideCol, update.side.to_string())
            .set(PositionEffectCol, update.effect.to_string())
            .set(StatusCol, update.status.to_string())
            .set(CreateLtCol, update.create_lt.nanos())
            .set(StrategyIdCol, update.strategy_id as _)
            .set(OpenOrderClientId, update.opening_cloid.clone())
            .set(EventId, update.event_id as _)
            .set(FilledSizeCol, update.filled_size)
            .set(UpdateTstCol, update.update_tst.nanos())
            .finish();
    }
    pub fn insert_new_order_request(&mut self, request: &RequestPlaceOrder) {
        let exchange = request.instrument.get_exchange().unwrap().to_string();
        let symbol = request.instrument.get_symbol().unwrap().to_string();
        self.worktable
            .insert()
            .set(LocalIdCol, request.order_lid.to_string())
            .set(ExchangeCol, exchange)
            .set(SymbolCol, symbol)
            .set(ClientIdCol, request.order_cid.to_string())
            .set(ServerIdCol, String::default())
            .set(PriceCol, request.price)
            .set(SizeCol, request.size)
            .set(UpdateLtCol, request.create_lt.nanos())
            .set(OrderTypeCol, request.ty.to_string())
            .set(SideCol, request.side.to_string())
            .set(PositionEffectCol, request.effect.to_string())
            .set(StatusCol, OrderStatus::Pending.to_string())
            .set(CreateLtCol, request.create_lt.nanos())
            .set(StrategyIdCol, request.strategy_id as _)
            .set(OpenOrderClientId, request.opening_cloid.clone())
            .set(EventId, request.event_id as i64)
            .set(FilledSizeCol, 0.0)
            .set(UpdateTstCol, 0) // set to 0 to make sure it's being correctly updated
            .finish();
    }
    pub fn insert_order_row_view(&mut self, row: &OrderRowView) {
        self.worktable
            .insert()
            .set(LocalIdCol, row.local_id().to_string())
            .set(ExchangeCol, row.exchange().to_string())
            .set(SymbolCol, row.symbol().to_string())
            .set(ClientIdCol, row.client_id().to_string())
            .set(ServerIdCol, row.server_id().to_string())
            .set(PriceCol, row.price())
            .set(SizeCol, row.size())
            .set(UpdateLtCol, row.update_lt())
            .set(OrderTypeCol, row.ty().to_string())
            .set(SideCol, row.side().unwrap().to_string())
            .set(PositionEffectCol, row.position_effect().to_string())
            .set(StatusCol, row.status().to_string())
            .set(CreateLtCol, row.create_lt())
            .set(StrategyIdCol, row.strategy_id() as _)
            .set(OpenOrderClientId, row.open_order_client_id())
            .set(EventId, row.event_id())
            .set(FilledSizeCol, row.filled_size())
            .set(UpdateTstCol, row.update_tst())
            .finish();
    }
}

#[derive(Clone)]
pub struct OrderRowView<'a>(RowView<'a>);
impl<'a> OrderRowView<'a> {
    pub fn local_id(&self) -> &str {
        self.0.index(LocalIdCol)
    }
    pub fn exchange(&self) -> Exchange {
        self.0.index(ExchangeCol).parse().unwrap()
    }
    pub fn symbol(&self) -> Symbol {
        Symbol::from_str(self.0.index(SymbolCol)).unwrap()
    }
    pub fn instrument_symbol(&self) -> InstrumentSymbol {
        InstrumentSymbol::new(self.exchange(), self.symbol())
    }
    pub fn client_id(&self) -> &str {
        self.0.index(ClientIdCol)
    }
    pub fn server_id(&self) -> &str {
        self.0.index(ServerIdCol)
    }
    pub fn status(&self) -> OrderStatus {
        self.0.index(StatusCol).parse().unwrap()
    }
    pub fn position_effect(&self) -> PositionEffect {
        let value = self.0.index(PositionEffectCol);
        value
            .parse()
            .unwrap_or_else(|_| panic!("position effect parsing failed, {:?}", value))
    }
    pub fn side(&self) -> Option<Side> {
        self.0.index(SideCol).parse().ok()
    }
    /// in nano
    pub fn update_lt(&self) -> i64 {
        *self.0.index(UpdateLtCol)
    }
    pub fn price(&self) -> f64 {
        *self.0.index(PriceCol)
    }
    pub fn size(&self) -> f64 {
        *self.0.index(SizeCol)
    }
    pub fn create_lt(&self) -> TimeStampNs {
        *self.0.index(CreateLtCol)
    }
    pub fn ty(&self) -> OrderType {
        let str_name: String = self.0.index(OrderTypeCol).to_string();
        OrderType::from_str(&str_name).unwrap()
    }
    pub fn strategy_id(&self) -> u64 {
        *self.0.index(StrategyIdCol) as _
    }
    pub fn open_order_client_id(&self) -> String {
        self.0.index(OpenOrderClientId).to_string()
    }
    pub fn event_id(&self) -> i64 {
        *self.0.index(EventId)
    }
    pub fn filled_size(&self) -> f64 {
        *self.0.index(FilledSizeCol)
    }
    pub fn update_tst(&self) -> i64 {
        *self.0.index(UpdateTstCol)
    }
}

impl<'a> std::fmt::Display for OrderRowView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OrderRow[{}][{}][{}]",
            self.symbol(),
            self.client_id(),
            self.status()
        )
    }
}
impl<'a> Deref for OrderRowView<'a> {
    type Target = RowView<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[repr(transparent)]
pub struct OrderRowViewMut<'a>(RowViewMut<'a>);
impl<'a> Deref for OrderRowViewMut<'a> {
    type Target = OrderRowView<'a>;
    fn deref(&self) -> &Self::Target {
        // SAFETY: see the first 2 fields of RowViewMut and RowView are the same
        unsafe { std::mem::transmute(self) }
    }
}

impl OrderRowViewMut<'_> {
    pub fn set_status_lt(&mut self, status: OrderStatus, lt: TimeStampNs) {
        self.0.set(StatusCol, status.to_string());
        self.0.set(UpdateLtCol, lt);
    }
    pub fn set_status(&mut self, status: OrderStatus) {
        self.0.set(StatusCol, status.to_string());
    }
    pub fn set_client_id(&mut self, client_id: &str) {
        self.0.set(ClientIdCol, client_id.to_string());
    }
    pub fn set_local_id(&mut self, local_id: &str) {
        self.0.set(LocalIdCol, local_id.to_string());
    }
    pub fn set_size(&mut self, size: f64) {
        self.0.set(SizeCol, size);
    }
    pub fn set_server_id(&mut self, server_id: &str) {
        self.0.set(ServerIdCol, server_id.to_string());
    }
    pub fn set_update_lt(&mut self, update_lt: i64) {
        self.0.set(UpdateLtCol, update_lt);
    }
    pub fn set_update_tst(&mut self, update_tst: i64) {
        self.0.set(UpdateTstCol, update_tst);
    }
    pub fn set_filled_size(&mut self, filled_size: f64) {
        self.0.set(FilledSizeCol, filled_size);
    }
    pub fn apply_update(&mut self, update: &UpdateOrder) {
        self.0.set(StatusCol, update.status.to_string());
        if PositionEffect::Unknown != update.effect {
            // only convert when we have it (hyper WS message does not have position effect)
            self.0.set(PositionEffectCol, update.effect.to_string());
        }
        if !update.local_id.is_empty() {
            self.0.set(LocalIdCol, update.local_id.as_str().into());
        }

        if !update.client_id.is_empty() {
            self.0.set(ClientIdCol, update.client_id.as_str().into());
        }

        if !update.server_id.is_empty() {
            self.0.set(ServerIdCol, update.server_id.as_str().into());
        }
        if update.price != 0.0 {
            self.0.set(PriceCol, update.price);
        }
        if update.size != 0.0 {
            self.0.set(SizeCol, update.size);
        } else if update.filled_size != 0.0 {
            // sometimes we only got filled_size, but not size
            self.0.set(SizeCol, update.filled_size);
        }
        if update.filled_size != 0.0 {
            self.0.set(FilledSizeCol, update.filled_size);
        }
        if update.average_filled_price != 0.0 {
            self.0.set(FilledSizeCol, update.average_filled_price);
        }

        if update.update_lt != Time::NULL {
            self.0.set(UpdateLtCol, update.update_lt.nanos());
        } else {
            // as a fallback, in case update_lt is not provided
            self.0.set(UpdateLtCol, now());
        }
        if OrderType::Unknown != update.ty {
            self.0.set(OrderTypeCol, update.ty.to_string());
        }
        if update.strategy_id != 0 {
            self.0.set(StrategyIdCol, update.strategy_id as _);
        }
        if !update.opening_cloid.is_empty() {
            self.0.set(OpenOrderClientId, update.opening_cloid.clone());
        }
        if update.event_id != 0 {
            self.0.set(EventId, update.event_id as i64);
        }
    }
    pub fn remove(self) {
        self.0.remove()
    }
}
