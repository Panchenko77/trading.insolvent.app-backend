use gluesql_derive::{FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow};

#[derive(Debug, Clone, FromGlueSqlRow, ReflectGlueSqlRow, ToGlueSqlRow, PartialEq, Eq, PartialOrd, Ord)]
pub struct DbRowJoinSymbol {
    pub id: u64, // symbol intern id
    pub symbol: String,
    pub status: String,
    pub flag: bool,
}
impl From<DbRowJoinSymbol> for build::model::UserSymbolList {
    fn from(x: DbRowJoinSymbol) -> Self {
        build::model::UserSymbolList {
            symbol: x.symbol,
            status: x.status,
            flag: x.flag,
        }
    }
}
