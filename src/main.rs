use std::net::SocketAddr;

use browserless_html_to_pdf::{app, config};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config = config::Config::from_env();
    let port = config.port;

    let state = app::build_state(config).expect("failed to build app state");
    let router = app::build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");
    println!("listening on http://{addr}  (Scalar docs at /, spec at /openapi.json)");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("server error");
}
