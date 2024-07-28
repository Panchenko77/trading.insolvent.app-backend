use crate::db::gluesql::schema::user::{get_salt, hash_password, DbRowUser, DbRowUserExt, UnsafeBuiltinUser};
use build::model::*;
use eyre::{bail, ensure, ContextCompat, Result};
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use gluesql_shared_sled_storage::SharedSledStorage;
use lib::gluesql::{Table, TableOverwriteItem, TableSelectItem};
use lib::toolbox::*;
use lib::ws::*;
use num_traits::FromPrimitive;
use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use uuid::Uuid;

pub struct MethodAuthSignup {
    pub db: Table<SharedSledStorage, DbRowUser>,
    pub allow_cors_sites: Arc<Option<Vec<String>>>,
}

impl SubAuthController for MethodAuthSignup {
    fn auth(
        self: Arc<Self>,
        _toolbox: &ArcToolbox,
        param: Value,
        _ctx: RequestContext,
        _conn: Arc<WsConnection>,
    ) -> LocalBoxFuture<'static, Result<Value>> {
        // info!("Signup request: {:?}", param);

        let mut db = self.db.clone();
        async move {
            let req: SignupRequest = serde_json::from_value(param)
                .map_err(|x| CustomError::new(EnumErrorCode::BadRequest, format!("Invalid request: {}", x)))?;
            let username = req.username;
            db.get_by_username(&username)
                .await?
                .with_context(|| CustomError::new(EnumErrorCode::UsernameAlreadyRegistered, Value::Null))?;

            let password = req.password;

            let agreed_tos = req.agreed_tos;
            let agreed_privacy = req.agreed_privacy;

            if !agreed_tos {
                bail!(CustomError::new(EnumErrorCode::UserMustAgreeTos, Value::Null));
            }
            if !agreed_privacy {
                bail!(CustomError::new(EnumErrorCode::UserMustAgreePrivacyPolicy, Value::Null));
            }
            let public_id = chrono::Utc::now().timestamp_millis() as u64;
            let salt = get_salt(&username);
            let password_hashed = hash_password(&password, &salt);
            let id = db.next_index();
            db.insert(DbRowUser {
                id,
                public_id,
                username: username.clone(),
                salt,
                password_hashed,
                email: "".to_string(),
                role: EnumRole::User.to_string(),
                agreed_tos,
                agreed_privacy,
                ..DbRowUser::empty()
            })
            .await?;

            Ok(serde_json::to_value(SignupResponse {
                username,
                user_id: public_id as _,
            })?)
        }
        .boxed_local()
    }
}
pub struct MethodAuthLogin {
    pub db: Option<Table<SharedSledStorage, DbRowUser>>,
    pub unsafe_builtin_user: Arc<Vec<UnsafeBuiltinUser>>,
    pub allow_cors_sites: Arc<Option<Vec<String>>>,
}

impl SubAuthController for MethodAuthLogin {
    fn auth(
        self: Arc<Self>,
        _toolbox: &ArcToolbox,
        param: Value,
        _ctx: RequestContext,
        conn: Arc<WsConnection>,
    ) -> LocalBoxFuture<'static, Result<Value>> {
        // info!("Login request: {:?}", param);
        let db = self.db.clone();

        async move {
            let req: LoginRequest = serde_json::from_value(param)
                .map_err(|x| CustomError::new(EnumErrorCode::BadRequest, format!("Invalid request: {}", x)))?;
            let username = req.username;
            if let Some(user) = self.unsafe_builtin_user.iter().find(|x| x.username == username) {
                ensure!(
                    user.password == req.password,
                    CustomError::new(EnumErrorCode::InvalidPassword, Value::Null)
                );
                let user_token = Uuid::new_v4();
                *user.token.write().unwrap() = user_token;
                conn.user_id.store(user.user_id as _, Ordering::SeqCst);
                conn.role.store(user.role as _, Ordering::SeqCst);
                return Ok(serde_json::to_value(LoginResponse {
                    username: username.clone(),
                    display_name: username,
                    avatar: None,
                    role: user.role,
                    user_id: user.user_id as _,
                    user_token,
                    admin_token: Uuid::nil(),
                })?);
            }

            let password = req.password;
            tracing::info!("finding user in auth DB (login)");
            // let service_code = req.service;
            let Some(mut db) = db else {
                bail!(CustomError::new(
                    EnumErrorCode::InternalError,
                    "Database not initialized".to_string()
                ));
            };
            let mut row = db
                .get_by_username(&username)
                .await?
                .with_context(|| CustomError::new(EnumErrorCode::UserNotFound, Value::Null))?;

            ensure!(
                row.password_hashed == password,
                CustomError::new(EnumErrorCode::InvalidPassword, Value::Null)
            );
            let user_token = Uuid::new_v4();

            row.user_token = user_token;

            db.overwrite(row.id, &row).await?;

            Ok(serde_json::to_value(LoginResponse {
                username: username.clone(),
                display_name: username,
                avatar: None,
                role: row.role.parse()?,
                user_id: row.public_id as _,
                user_token,
                admin_token: Uuid::nil(),
            })?)
        }
        .boxed_local()
    }
}

pub struct MethodAuthAuthorize {
    pub db: Option<Table<SharedSledStorage, DbRowUser>>,
    pub unsafe_builtin_user: Arc<Vec<UnsafeBuiltinUser>>,
    // pub accept_service: EnumService,
}
impl SubAuthController for MethodAuthAuthorize {
    fn auth(
        self: Arc<Self>,
        _toolbox: &ArcToolbox,
        param: Value,
        _ctx: RequestContext,
        conn: Arc<WsConnection>,
    ) -> LocalBoxFuture<'static, Result<Value>> {
        // info!("Authorize request: {:?}", param);
        let db_auth = self.db.clone();
        async move {
            let req: AuthorizeRequest = serde_json::from_value(param)
                .map_err(|x| CustomError::new(EnumErrorCode::BadRequest, format!("Invalid request: {}", x)))?;
            let username = req.username;
            // let service = req.service;

            // if service != self.accept_service {
            //     bail!(CustomError::new(
            //         EnumErrorCode::InvalidService,
            //         format!(
            //             "Invalid service, only {:?} {} permitted",
            //             self.accept_service, self.accept_service as u32
            //         ),
            //     ));
            // }
            if let Some(user) = self.unsafe_builtin_user.iter().find(|x| x.username == username) {
                // check for token
                ensure!(
                    *user.token.read().unwrap() == req.token,
                    CustomError::new(EnumErrorCode::UserInvalidAuthToken, Value::Null)
                );
                conn.user_id.store(user.user_id as _, Ordering::Relaxed);
                conn.role.store(user.role as _, Ordering::Relaxed);
                return Ok(serde_json::to_value(AuthorizeResponse {
                    success: true,
                    user_id: user.user_id as _,
                    role: user.role,
                })?);
            }
            tracing::info!("finding user in auth DB (authorize)");
            let Some(mut db_auth) = db_auth else {
                bail!(CustomError::new(
                    EnumErrorCode::InternalError,
                    "Database not initialized".to_string()
                ));
            };
            let user = db_auth
                .get_by_username(&username)
                .await?
                .with_context(|| CustomError::new(EnumErrorCode::UserNotFound, Value::Null))?;

            ensure!(
                user.user_token == req.token,
                CustomError::new(EnumErrorCode::UserInvalidAuthToken, Value::Null)
            );

            conn.user_id.store(user.id as _, Ordering::Relaxed);
            let role: EnumRole = user.role.parse()?;
            conn.role.store(role as _, Ordering::Relaxed);
            Ok(serde_json::to_value(AuthorizeResponse {
                success: true,
                user_id: user.id as _,
                role,
            })?)
        }
        .boxed_local()
    }
}

pub struct MethodAuthLogout {
    pub db: Table<SharedSledStorage, DbRowUser>,
}
impl SubAuthController for MethodAuthLogout {
    fn auth(
        self: Arc<Self>,
        _toolbox: &ArcToolbox,
        _param: Value,
        ctx: RequestContext,
        conn: Arc<WsConnection>,
    ) -> LocalBoxFuture<'static, Result<Value>> {
        let db_auth = self.db.clone();

        async move {
            if ctx.user_id > 0 {
                let mut db = db_auth.clone();
                let mut row = db
                    .get_by_id(ctx.user_id as _)
                    .await?
                    .with_context(|| CustomError::new(EnumErrorCode::UserNotFound, Value::Null))?;
                row.user_token = Uuid::nil();
                db.overwrite(row.id, &row).await?;
            }
            conn.user_id.store(0, Ordering::Relaxed);
            conn.role.store(EnumRole::Guest as _, Ordering::Relaxed);
            Ok(serde_json::to_value(&LogoutResponse {})?)
        }
        .boxed_local()
    }
}

pub fn ensure_user_role(ctx: RequestContext, role: EnumRole) -> Result<()> {
    let ctx_role = EnumRole::from_u32(ctx.role).context("Invalid role")?;
    ensure!(
        ctx_role >= role,
        CustomError::new(
            EnumErrorCode::InvalidRole,
            format!("Requires {} Actual {}", role, ctx_role)
        )
    );
    Ok(())
}
