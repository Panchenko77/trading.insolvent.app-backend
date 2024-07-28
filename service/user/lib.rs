extern crate core;

pub const APP_VERSION: u64 = 1;
use std::sync::Arc;

/// config
pub mod config;
/// database
pub mod db;
/// endpoint method
pub mod endpoint_method;
pub mod events;
/// execution
pub mod execution;

/// core runner
pub mod main_core;
/// shared across services
pub mod signals;

pub mod balance_manager;
pub mod leger_manager;
/// strategy trait and implementation
pub mod strategy;
pub mod task;

pub type ServiceStarter = Arc<tokio::sync::Semaphore>;
