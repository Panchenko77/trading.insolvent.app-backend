use std::mem::transmute;
use std::ops::Deref;
use tracing::warn;

use worktable::{field, RowView, RowViewMut, WorkTable};

use trading_exchange::model::{gen_local_id, RequestPlaceOrder, UpdateOrder, UpdatePosition};
use trading_model::{now, Exchange, Time};

/// A position can be one of the followings:
/// - confirmed position/balance by exchange
/// - predicted position/balance backed by an order
pub struct PositionsTable {
    pub worktable: WorkTable,
}
field!(0, IdCol: i64, "id");
field!(1, ExchangeCol: String, "exchange");
// if the position is confirmed by exchange, cloid is empty
field!(2, CloidCol: String, "cloid");
field!(3, SymbolCol: String, "symbol");
field!(4, SizeCol: f64, "size");
field!(5, FilledSizeCol: f64, "filled_size");
field!(6, UpdateTst: i64, "update_tst");

impl PositionsTable {
    pub fn new() -> Self {
        let mut worktable = WorkTable::new();
        worktable.add_field(IdCol);
        worktable.add_field(ExchangeCol);
        worktable.add_field(CloidCol);
        worktable.add_field(SymbolCol);
        worktable.add_field(SizeCol);
        worktable.add_field(FilledSizeCol);
        worktable.add_field(UpdateTst);

        Self { worktable }
    }
    pub fn push_new_order(&mut self, update: &RequestPlaceOrder) {
        let exchange = update.instrument.get_exchange().unwrap().to_string();
        let symbol = update.instrument.get_symbol().unwrap();
        self.worktable
            .insert()
            .set(
                IdCol,
                update.order_lid.parse().unwrap_or_else(|e| {
                    warn!("Failed to parse local_id {:?}: {}", update.order_lid, e);
                    0
                }),
            )
            .set(ExchangeCol, exchange)
            .set(CloidCol, update.order_cid.to_string())
            .set(SymbolCol, symbol.to_string())
            .set(SizeCol, update.size)
            .set(FilledSizeCol, 0.0)
            .set(UpdateTst, now())
            .finish();
    }
    pub fn push_order_update(&mut self, update: &UpdateOrder) {
        let symbol = update.instrument.get_symbol().unwrap();
        let exchange = update.instrument.get_exchange().unwrap().to_string();
        let Ok(local_id) = update.local_id.parse() else {
            // order manager should have inserted local ID already
            tracing::error!("skipping position insertion, missing local ID");
            return;
        };
        self.worktable
            .insert()
            .set(IdCol, local_id)
            .set(ExchangeCol, exchange)
            .set(CloidCol, update.client_id.to_string())
            .set(SymbolCol, symbol.to_string())
            .set(SizeCol, update.size)
            .set(FilledSizeCol, update.filled_size)
            .set(UpdateTst, update.update_lt.nanos())
            .finish();
    }

    pub fn push_position_update(&mut self, update: &UpdatePosition, time: Time) {
        let symbol = update.instrument.get_asset_or_symbol().unwrap();
        let exchange = update.instrument.get_exchange().unwrap().to_string();
        let available = update
            .set_values
            .as_ref()
            .or(update.set_values.as_ref())
            .unwrap()
            .available;
        self.worktable
            .insert()
            .set(IdCol, gen_local_id().parse().unwrap())
            .set(ExchangeCol, exchange)
            .set(CloidCol, "".to_string())
            .set(SymbolCol, symbol.to_string())
            .set(SizeCol, available)
            .set(FilledSizeCol, 0.0)
            .set(UpdateTst, time.nanos())
            .finish();
    }
    /// returns if the cloid exists and is removed
    pub fn remove(&mut self, cloid: &str) -> bool {
        self.worktable
            .iter_mut()
            .map(PositionRowViewMut)
            .find(|x| x.cloid() == Some(cloid))
            .map(|x| x.remove())
            .is_some()
    }
    pub fn contains_order(&self, cloid: &str) -> bool {
        self.worktable
            .iter()
            .map(PositionRowView)
            .any(|x| x.cloid() == Some(cloid))
    }
    pub fn get_by_cloid_mut(&mut self, cloid: &str) -> Option<PositionRowViewMut> {
        self.worktable
            .iter_mut()
            .map(PositionRowViewMut)
            .find(|x| x.cloid() == Some(cloid))
    }
    pub fn get_position_by_symbol_mut(&mut self, exchange: Exchange, symbol: &str) -> Option<PositionRowViewMut> {
        self.iter_mut()
            .find(|x| x.cloid().is_none() && x.exchange() == exchange && x.symbol() == symbol)
    }
    pub fn get_position_by_symbol(&self, exchange: Exchange, symbol: &str) -> Option<PositionRowView> {
        self.iter()
            .find(|x| x.cloid().is_none() && x.exchange() == exchange && x.symbol() == symbol)
    }
    pub fn iter(&self) -> impl Iterator<Item = PositionRowView> {
        self.worktable.iter().map(PositionRowView)
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = PositionRowViewMut> {
        self.worktable.iter_mut().map(PositionRowViewMut)
    }
}
pub struct PositionRowView<'a>(RowView<'a>);
impl<'a> PositionRowView<'a> {
    pub fn id(&self) -> i64 {
        *self.0.index(IdCol)
    }
    pub fn exchange(&self) -> Exchange {
        self.0.index(ExchangeCol).parse().unwrap()
    }
    pub fn cloid(&self) -> Option<&str> {
        match self.0.index(CloidCol).as_str() {
            "" => None,
            x => Some(x),
        }
    }
    pub fn symbol(&self) -> &str {
        self.0.index(SymbolCol)
    }
    pub fn size(&self) -> f64 {
        *self.0.index(SizeCol)
    }
    pub fn filled_size(&self) -> f64 {
        *self.0.index(FilledSizeCol)
    }
    pub fn update_tst(&self) -> i64 {
        *self.0.index(UpdateTst)
    }
}

pub struct PositionRowViewMut<'a>(RowViewMut<'a>);
impl<'a> Deref for PositionRowViewMut<'a> {
    type Target = PositionRowView<'a>;
    fn deref(&self) -> &Self::Target {
        // SAFETY: same size
        unsafe { transmute(self) }
    }
}
impl<'a> PositionRowViewMut<'a> {
    pub fn set_size(&mut self, size: f64) {
        self.0.set(SizeCol, size);
    }
    pub fn set_filled_size(&mut self, filled_size: f64) {
        self.0.set(FilledSizeCol, filled_size);
    }
    pub fn set_update_tst(&mut self, update_tst: i64) {
        self.0.set(UpdateTst, update_tst);
    }
    pub fn remove(self) {
        self.0.remove();
    }
}
