mod common;

use axum::{body::Body, http::{Method, Request, StatusCode}};
use tower::ServiceExt; 
use serde_json::json;

#[tokio::test]
async fn test_store_create_cart_success() {
    let (app, _db) = common::setup_test_app().await;

    let payload = json!({
        "email": "customer@example.com",
        "currency_code": "usd"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    if response.status() != StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        panic!("API Error Response: {:?}", String::from_utf8_lossy(&body_bytes));
    }
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_store_create_cart_validation_failure() {
    let (app, _db) = common::setup_test_app().await;

    // missing required currency and invalid email
    let payload = json!({
        "email": "not-an-email"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST); // hits validation
}

#[tokio::test]
async fn test_cart_full_flow() {
    let (app, db) = common::setup_test_app().await;

    let pool = match db {
        toko_rs::db::AppDb::Sqlite(p) => p,
        _ => panic!("Expected Sqlite for tests"),
    };

    // Insert dummy product & variant
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_1', 'Test', 'test', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_1', 'prod_1', 'Default', 1000)")
        .execute(&pool).await.unwrap();

    // 1. Create cart
    let payload = json!({"currency_code": "usd"});
    let request = Request::builder().method(Method::POST).uri("/store/carts").header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cart_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let cart_id = cart_resp["cart"]["id"].as_str().unwrap();

    // 2. Add line item
    let payload = json!({"variant_id": "var_1", "quantity": 2});
    let request = Request::builder().method(Method::POST).uri(&format!("/store/carts/{}/line-items", cart_id)).header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cart_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(cart_resp["cart"]["items"].as_array().unwrap().len(), 1);
    let line_id = cart_resp["cart"]["items"][0]["id"].as_str().unwrap();

    // 3. Update line item (quantity = 3)
    let payload = json!({"quantity": 3});
    let request = Request::builder().method(Method::POST).uri(&format!("/store/carts/{}/line-items/{}", cart_id, line_id)).header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cart_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(cart_resp["cart"]["items"][0]["quantity"].as_i64().unwrap(), 3);

    // 4. Update cart email
    let payload = json!({"email": "test@test.com"});
    let request = Request::builder().method(Method::POST).uri(&format!("/store/carts/{}", cart_id)).header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cart_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(cart_resp["cart"]["email"].as_str().unwrap(), "test@test.com");

    // 5. Delete line item
    let request = Request::builder().method(Method::DELETE).uri(&format!("/store/carts/{}/line-items/{}", cart_id, line_id)).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cart_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(cart_resp["cart"]["items"].as_array().unwrap().len(), 0);

    // 6. Test GET cart
    let request = Request::builder().method(Method::GET).uri(&format!("/store/carts/{}", cart_id)).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Add line item and update to 0 to trigger delete branch
    let payload = json!({"variant_id": "var_1", "quantity": 1});
    let request = Request::builder().method(Method::POST).uri(&format!("/store/carts/{}/line-items", cart_id)).header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cart_resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let line_id = cart_resp["cart"]["items"][0]["id"].as_str().unwrap();

    let payload = json!({"quantity": 0});
    let request = Request::builder().method(Method::POST).uri(&format!("/store/carts/{}/line-items/{}", cart_id, line_id)).header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 8. Test invalid cart ID on add_line_item
    let payload = json!({"variant_id": "var_1", "quantity": 1});
    let request = Request::builder().method(Method::POST).uri("/store/carts/invalid_cart/line-items").header("content-type", "application/json").body(Body::from(payload.to_string())).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 9. Test complete cart stub
    let request = Request::builder().method(Method::POST).uri(&format!("/store/carts/{}/complete", cart_id)).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
}
