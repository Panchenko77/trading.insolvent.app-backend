use trading_model::{LevelOperation, Quote, Quotes};
use worktable::worktable::{Column, WorkTable};

#[derive(Default)]
pub struct L2OrderBookWorkTable {
    table: WorkTable,
}
impl L2OrderBookWorkTable {
    pub fn new() -> Self {
        let mut table = WorkTable::new();
        table.add_column("side", Column::String(vec![]));
        table.add_column("level", Column::Int(vec![]));
        table.add_column("price", Column::Float(vec![]));
        table.add_column("size", Column::Float(vec![]));
        Self { table }
    }
    pub fn update_by_level(&mut self, level: i64, price: f64, size: f64, side: String) {
        match self.table.find_row_index(|row| {
            row.get::<String>("side").unwrap() == side && row.get::<i64>("level").unwrap() == level
        }) {
            Some(index) => {
                self.table
                    .update_row(index, [side.into(), level.into(), price.into(), size.into()]);
            }
            None => {
                self.table.push([side.into(), level.into(), price.into(), size.into()]);
            }
        }
    }
    pub fn update_by_price(&mut self, price: f64, size: f64, side: String) {
        match self.table.find_row_index(|row| {
            row.get::<String>("side").unwrap() == side && row.get::<f64>("price").unwrap() == price
        }) {
            Some(index) => {
                self.table
                    .update_row(index, [side.into(), 0.into(), price.into(), size.into()]);
            }
            None => {
                self.table.push([side.into(), 0.into(), price.into(), size.into()]);
            }
        }
    }

    pub fn get_best_bid(&self) -> Option<(f64, f64)> {
        self.table
            .get_rows()
            .filter(|row| row.get::<String>("side").unwrap() == "bid")
            .map(|row| (row.get::<f64>("price").unwrap(), row.get::<f64>("size").unwrap()))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
    }
    pub fn get_best_ask(&self) -> Option<(f64, f64)> {
        self.table
            .get_rows()
            .filter(|row| row.get::<String>("side").unwrap() == "ask")
            .map(|row| (row.get::<f64>("price").unwrap(), row.get::<f64>("size").unwrap()))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
    }
    // pub fn delete_side(&mut self, side: String) {
    //     self.table.delete_rows(|row| row.get::<String>("side").unwrap() == side);
    // }
    pub fn update_quote(&mut self, quote: &Quote) {
        match quote.operation {
            LevelOperation::UpdateByPrice => {
                self.update_by_price(quote.price, quote.size, quote.side.to_string());
            }
            LevelOperation::UpdateByLevel => {
                self.update_by_level(quote.level as _, quote.price, quote.size, quote.side.to_string());
            }
            LevelOperation::DeleteFirstN => todo!(),
            LevelOperation::DeleteLastN => todo!(),
            LevelOperation::DeleteSide => todo!(),
        }
    }
    pub fn update_quotes(&mut self, quotes: &Quotes) {
        for quote in quotes.get_quotes() {
            self.update_quote(quote);
        }
    }
}
