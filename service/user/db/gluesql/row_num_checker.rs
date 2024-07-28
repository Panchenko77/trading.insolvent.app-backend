use gluesql::core::store::{GStore, GStoreMut};
use gluesql::prelude::Payload;
use lib::gluesql::{DbRow, Table, TableInfo};
use std::collections::BTreeMap;
use tracing::{error, info};

pub struct RowNumChecker {
    mapping: BTreeMap<String, i64>,
}
impl RowNumChecker {
    pub fn new() -> Self {
        Self {
            mapping: BTreeMap::new(),
        }
    }
    pub async fn count_table<G, T>(&mut self, table: &mut Table<G, T>)
    where
        G: GStore + GStoreMut,
        T: DbRow,
    {
        let table_name = table.table_name().clone();
        let count = match table.execute(format!("SELECT COUNT(*) FROM {}", table_name)).await {
            Ok(pld) => match &pld[0] {
                Payload::Select { rows, .. } => rows.len(),
                p => {
                    error!(
                        "Failed to count rows in table {}: mismatched payload {:?}",
                        table_name, p
                    );
                    return;
                }
            },
            Err(_) => {
                error!("Failed to count rows in table {}", table_name);
                return;
            }
        };
        let entry = self.mapping.entry(table_name.to_string()).or_default();
        *entry += count as i64;
    }
    pub fn print_sorted(&self) {
        let mut sorted: Vec<_> = self.mapping.iter().collect();
        sorted.sort_by_key(|(_, &v)| -v);
        info!("Row counts:");
        for (table_name, count) in sorted {
            println!("{}: {}", table_name, count);
        }
    }
}
