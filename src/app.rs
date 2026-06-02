use std::sync::Arc;

use axum::Router;
use opendal::Operator;
use tokio::sync::Semaphore;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub operator: Option<Operator>,
    pub render_limiter: Arc<Semaphore>,
}

pub fn build_state(config: Config) -> Result<AppState, String> {
    let operator = if config.storage_enabled {
        Some(crate::services::storage::build_operator(
            &config.scheme,
            &config.opendal_opts,
        )?)
    } else {
        None
    };
    Ok(AppState {
        render_limiter: Arc::new(Semaphore::new(config.max_concurrent_renders)),
        config: Arc::new(config),
        operator,
    })
}

pub fn build_router(state: AppState) -> Router {
    crate::routes::routes(state)
}
