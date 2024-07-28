use std::path::PathBuf;

use async_trait::async_trait;

use lib::handler::{RequestHandler, Response};
use lib::log_reader::get_log_entries;
use lib::toolbox::CustomError;
use lib::toolbox::RequestContext;

use crate::endpoint_method::auth::ensure_user_role;

use super::convert_log_entry_to_user_debug_log_row;

#[derive(Clone)]
pub struct MethodUserGetDebugLog {
    pub log_file: Option<PathBuf>,
}
#[async_trait(?Send)]
impl RequestHandler for MethodUserGetDebugLog {
    type Request = build::model::UserGetDebugLogRequest;

    async fn handle(&self, ctx: RequestContext, req: Self::Request) -> Response<Self::Request> {
        ensure_user_role(ctx, build::model::EnumRole::User)?;

        let file = match &self.log_file {
            Some(file) => {
                if !file.exists() {
                    return Err(
                        CustomError::new(build::model::EnumErrorCode::InvalidService, "log file not exist").into(),
                    );
                }
                file.to_owned()
            }
            None => {
                return Err(CustomError::new(
                    build::model::EnumErrorCode::InvalidService,
                    "log file config is not provided",
                )
                .into());
            }
        };

        let mut limit: isize = req.limit.unwrap_or(1000) as _;
        // let mut page: usize = req.page.unwrap_or(1);
        let mut all_entries = vec![];

        for file_path in [file] {
            let entries = get_log_entries(&file_path, limit as _).await?;
            limit -= entries.len() as isize;
            all_entries.extend(entries);
            if limit <= 0 {
                break;
            }
        }
        let data: Vec<_> = all_entries
            .into_iter()
            .map(convert_log_entry_to_user_debug_log_row)
            .collect();
        Ok(build::model::UserGetDebugLogResponse { data })
    }
}
