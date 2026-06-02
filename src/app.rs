use std::sync::Arc;

use axum::Router;
use opendal::Operator;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub operator: Option<Operator>,
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
        config: Arc::new(config),
        operator,
    })
}

pub fn build_router(state: AppState) -> Router {
    crate::routes::routes(state)
}
