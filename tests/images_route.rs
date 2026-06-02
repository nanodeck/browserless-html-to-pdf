mod common;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use axum::http::{Request, header};
use base64::Engine;
use browserless_html_to_pdf::{services::html_to_pdf::html_to_pdf, test_app};
use common::{body_json, post_json};
use std::net::SocketAddr;
use tower::ServiceExt;

fn pdf_b64(html: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(html_to_pdf(html).unwrap())
}

fn multipage_b64(n: usize) -> String {
    let mut body = String::new();
    for i in 1..=n {
        body.push_str(&format!(
            r#"<div style="page-break-after: always;">Page {i}</div>"#
        ));
    }
    pdf_b64(&format!("<html><body>{body}</body></html>"))
}

#[tokio::test]
async fn returns_png_images() {
    let body = format!(
        r#"{{"pdf_base64":"{}","format":"png"}}"#,
        pdf_b64("<h1>x</h1>")
    );
    let res = post_json(test_app(&[]), "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let first = &v["images"][0];
    assert_eq!(first["page"], 1);
    assert!(first["width"].as_u64().unwrap() > 0);
    let img = base64::engine::general_purpose::STANDARD
        .decode(first["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(&img[0..4], &[0x89, 0x50, 0x4e, 0x47]);
}

#[tokio::test]
async fn returns_jpeg_when_requested() {
    let body = format!(
        r#"{{"pdf_base64":"{}","format":"jpeg"}}"#,
        pdf_b64("<h1>x</h1>")
    );
    let res = post_json(test_app(&[]), "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let img = base64::engine::general_purpose::STANDARD
        .decode(v["images"][0]["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(&img[0..2], &[0xFF, 0xD8]);
}

#[tokio::test]
async fn renders_specific_pages() {
    let body = format!(r#"{{"pdf_base64":"{}","pages":"1,3"}}"#, multipage_b64(3));
    let res = post_json(test_app(&[]), "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let imgs = v["images"].as_array().unwrap();
    assert_eq!(imgs.len(), 2);
    assert_eq!(imgs[0]["page"], 1);
    assert_eq!(imgs[1]["page"], 3);
}

#[tokio::test]
async fn dedups_pages_preserving_order() {
    let body = format!(r#"{{"pdf_base64":"{}","pages":"2,2,1"}}"#, multipage_b64(3));
    let res = post_json(test_app(&[]), "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let pages: Vec<u64> = v["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["page"].as_u64().unwrap())
        .collect();
    assert_eq!(pages, vec![2, 1]);
}

#[tokio::test]
async fn rejects_too_many_pages() {
    let app = test_app(&[("MAX_IMAGE_PAGES", "3")]);
    let body = format!(r#"{{"pdf_base64":"{}","pages":"1-4"}}"#, multipage_b64(4));
    let res = post_json(app, "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn caps_automatic_pages_to_max() {
    let app = test_app(&[("MAX_IMAGE_PAGES", "3")]);
    let body = format!(r#"{{"pdf_base64":"{}"}}"#, multipage_b64(5));
    let res = post_json(app, "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let imgs = v["images"].as_array().unwrap();
    assert_eq!(imgs.len(), 3);
    assert_eq!(imgs[0]["page"], 1);
    assert_eq!(imgs[2]["page"], 3);
}

#[tokio::test]
async fn rejects_unsafe_json_scale() {
    let body = format!(
        r#"{{"pdf_base64":"{}","scale":1.1}}"#,
        pdf_b64("<h1>x</h1>")
    );
    let res = post_json(test_app(&[]), "/v1/images", &body).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v = body_json(res).await;
    assert!(v["error"].as_str().unwrap().contains("scale"));
}

#[tokio::test]
async fn rejects_unsafe_multipart_scale() {
    let res = post_multipart_image_with_scale("0").await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_invalid_multipart_scale() {
    let res = post_multipart_image_with_scale("abc").await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

async fn post_multipart_image_with_scale(scale: &str) -> axum::http::Response<Body> {
    let pdf = html_to_pdf("<h1>x</h1>").unwrap();
    let boundary = "test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"scale\"\r\n\r\n{scale}\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"x.pdf\"\r\nContent-Type: application/pdf\r\n\r\n").as_bytes());
    body.extend_from_slice(&pdf);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let mut req = Request::post("/v1/images")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    test_app(&[]).oneshot(req).await.unwrap()
}
