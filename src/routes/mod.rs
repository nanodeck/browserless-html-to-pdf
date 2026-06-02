pub mod docs;
pub mod downloads;
pub mod health;
pub mod images;
pub mod pdf;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use tower::ServiceBuilder;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::app::AppState;

pub fn routes(state: AppState) -> Router {
    let max = state.config.max_body_bytes;

    let governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60)
            .burst_size(state.config.rate_limit_per_min)
            .finish()
            .expect("governor config"),
    );

    Router::new()
        .route("/", get(docs::scalar))
        .route("/openapi.json", get(docs::openapi_spec))
        .route("/health", get(health::health))
        .route("/v1/pdf", post(pdf::create_pdf))
        .route("/v1/images", post(images::create_images))
        .route("/downloads/{*key}", get(downloads::serve))
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(max))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(30),
                ))
                .layer(GovernorLayer { config: governor }),
        )
        .with_state(state)
}
