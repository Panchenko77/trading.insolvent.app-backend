use async_trait::async_trait;
use gluesql::shared_memory_storage::SharedMemoryStorage;

use build::model::EnumRole;
use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

use crate::db::gluesql::schema::DbRowStrategyAccuracy;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetLiveTestAccuracyLog {
    pub table: Table<SharedMemoryStorage, DbRowStrategyAccuracy>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetLiveTestAccuracyLog {
    type Request = build::model::UserGetLiveTestAccuracyLogRequest;

    async fn handle(&self, ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, EnumRole::User)?;
        let mut this = self.clone();
        // get accuracy log by its time
        let rows = this.table.select(None, "datetime DESC").await?;
        Ok(build::model::UserGetLiveTestAccuracyLogResponse {
            data: rows.into_iter().map(response_from_row).collect(),
        })
    }
}
fn response_from_row(row: DbRowStrategyAccuracy) -> build::model::UserAccuracyLog {
    let sum = row.count_correct + row.count_wrong;
    let sum = std::cmp::max(sum, 1);
    build::model::UserAccuracyLog {
        datetime: row.datetime,
        count_pass: row.count_correct as i64,
        count_fail: row.count_wrong as i64,
        accuracy: row.count_correct as f64 * 100.0 / (sum) as f64,
    }
}
