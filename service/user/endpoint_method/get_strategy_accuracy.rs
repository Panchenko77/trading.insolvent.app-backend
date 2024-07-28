use async_trait::async_trait;
use gluesql::prelude::SharedMemoryStorage;

use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

use crate::db::gluesql::schema::DbRowStrategyAccuracy;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetStrategyOneAccuracy {
    pub table_accuracy: Table<SharedMemoryStorage, DbRowStrategyAccuracy>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetStrategyOneAccuracy {
    type Request = build::model::UserGetStrategyOneAccuracyRequest;

    async fn handle(&self, ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        // there is nothing in this request
        let mut this = self.clone();
        // get the latest accuracy
        let row = this
            .table_accuracy
            .select_one(None, "datetime DESC")
            .await?
            .unwrap_or_default();
        let sum = row.count_correct + row.count_wrong;
        // edge case where no count was detected yet
        let sum = std::cmp::max(sum, 1);
        Ok(build::model::UserGetStrategyOneAccuracyResponse {
            count_correct: row.count_correct as i64,
            count_wrong: row.count_wrong as i64,
            accuracy: row.count_correct as f64 * 100.0 / (sum) as f64,
        })
    }
}
