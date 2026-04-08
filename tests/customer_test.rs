mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use common::setup_test_app;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_register_customer_success() {
    let (app, _) = setup_test_app().await;
    let payload = json!({
        "first_name": "Budi",
        "last_name": "Santoso",
        "email": "budi@example.com",
        "phone": "+6281234567890"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let c = &body["customer"];
    assert!(c["id"].as_str().unwrap().starts_with("cus_"));
    assert_eq!(c["first_name"], "Budi");
    assert_eq!(c["last_name"], "Santoso");
    assert_eq!(c["email"], "budi@example.com");
    assert_eq!(c["phone"], "+6281234567890");
    assert_eq!(c["has_account"], true);
}

#[tokio::test]
async fn test_register_customer_duplicate_email() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "budi@example.com", "first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    app.clone().oneshot(req).await.unwrap();
    let payload2 = json!({"email": "budi@example.com", "first_name": "Other"});
    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload2.to_string()))
        .unwrap();
    let resp = app.oneshot(req2).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "duplicate_error");
}

#[tokio::test]
async fn test_register_customer_missing_email() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert!(
        resp.status() == StatusCode::BAD_REQUEST
            || resp.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_register_customer_invalid_email() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "not-an-email", "first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_profile_with_valid_header() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "budi@example.com", "first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    let cus_id = body["customer"]["id"].as_str().unwrap();

    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/store/customers/me")
        .header("X-Customer-Id", cus_id)
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    assert_eq!(body2["customer"]["email"], "budi@example.com");
}

#[tokio::test]
async fn test_get_profile_without_header() {
    let (app, _) = setup_test_app().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/store/customers/me")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "unauthorized");
}

#[tokio::test]
async fn test_get_profile_not_found() {
    let (app, _) = setup_test_app().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/store/customers/me")
        .header("X-Customer-Id", "cus_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_customer_profile() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "budi@example.com", "first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    let cus_id = body["customer"]["id"].as_str().unwrap();

    let update = json!({"phone": "+6289876543210"});
    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/store/customers/me")
        .header("content-type", "application/json")
        .header("X-Customer-Id", cus_id)
        .body(Body::from(update.to_string()))
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    assert_eq!(body2["customer"]["phone"], "+6289876543210");
    assert_eq!(body2["customer"]["first_name"], "Budi");
}

#[tokio::test]
async fn test_update_customer_without_header() {
    let (app, _) = setup_test_app().await;
    let update = json!({"phone": "+6289876543210"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers/me")
        .header("content-type", "application/json")
        .body(Body::from(update.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_customer_response_format() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "fmt@example.com", "first_name": "Fmt"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["customer"].is_object());
    assert!(body["customer"]["id"].is_string());
    assert!(body["customer"]["email"].is_string());
    assert!(body["customer"]["has_account"].is_boolean());
    assert!(body["customer"]["created_at"].is_string());
    assert!(body["customer"]["updated_at"].is_string());
}
