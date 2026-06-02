#![allow(dead_code)]

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, Response};
use std::net::SocketAddr;
use tower::ServiceExt;

fn with_connect_info(mut req: Request<Body>) -> Request<Body> {
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    req
}

pub async fn get(app: Router, path: &str) -> Response<Body> {
    let req = with_connect_info(Request::get(path).body(Body::empty()).unwrap());
    app.oneshot(req).await.unwrap()
}

pub async fn post_json(app: Router, path: &str, json: &str) -> Response<Body> {
    let req = with_connect_info(
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(json.to_string()))
            .unwrap(),
    );
    app.oneshot(req).await.unwrap()
}

pub async fn body_json(res: Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

pub async fn body_text(res: Response<Body>) -> String {
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}
