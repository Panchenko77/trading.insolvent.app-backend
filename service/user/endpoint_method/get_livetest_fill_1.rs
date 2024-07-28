use async_trait::async_trait;
use gluesql::shared_memory_storage::SharedMemoryStorage;

use lib::gluesql::{Table, TableSelectItem};
use lib::handler::{RequestHandler, Response};
use lib::toolbox::RequestContext;

use crate::db::gluesql::schema::accuracy::DbRowLiveTestFillPrice;
use crate::endpoint_method::auth::ensure_user_role;

#[derive(Clone)]
pub struct MethodUserGetLiveTestCloseOrder1 {
    pub table: Table<SharedMemoryStorage, DbRowLiveTestFillPrice>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetLiveTestCloseOrder1 {
    type Request = build::model::UserGetLiveTestCloseOrder1Request;

    async fn handle(&self, ctx: RequestContext, _req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;
        let mut this = self.clone();
        let rows = this.table.select(None, "datetime DESC").await?;
        Ok(build::model::UserGetLiveTestCloseOrder1Response {
            data: rows.into_iter().map(|x| x.into()).collect(),
        })
    }
}
