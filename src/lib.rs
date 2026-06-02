pub mod app;
pub mod config;
pub mod error;
pub mod models;
pub mod openapi;
pub mod routes;
pub mod services;

use std::collections::HashMap;

pub fn config_with(overrides: &[(&str, &str)]) -> config::Config {
    let map: HashMap<String, String> = overrides
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    config::Config::from_map(&map)
}

pub fn test_app(overrides: &[(&str, &str)]) -> axum::Router {
    let state = app::build_state(config_with(overrides)).expect("state");
    app::build_router(state)
}
