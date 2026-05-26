mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn request(method: Method, uri: &str, payload: &serde_json::Value) -> Request<Body> {
    let is_body = method == Method::POST || method == Method::PUT || method == Method::PATCH;
    let mut builder = Request::builder().method(method).uri(uri);
    if is_body {
        builder = builder.header("content-type", "application/json");
        builder.body(Body::from(payload.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    }
}

fn post_req(uri: &str, payload: &serde_json::Value) -> Request<Body> {
    request(Method::POST, uri, payload)
}

fn get_req(uri: &str) -> Request<Body> {
    request(Method::GET, uri, &json!({}))
}

fn delete_req(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_create_webhook_success() {
    let (app, _db) = common::setup_test_app().await;

    let payload = json!({
        "url": "https://example.com/hook",
        "events": ["order.placed"],
        "secret": "s3cr3t"
    });

    let req = post_req("/admin/webhooks", &payload);
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = body_json(res).await;
    assert!(!body["webhook"]["id"].as_str().unwrap().is_empty());
    assert_eq!(body["webhook"]["url"], "https://example.com/hook");
    assert_eq!(body["webhook"]["events"][0], "order.placed");
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_list_webhooks_empty() {
    let (app, _db) = common::setup_test_app().await;

    let req = get_req("/admin/webhooks");
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = body_json(res).await;
    assert_eq!(body["webhooks"].as_array().unwrap().len(), 0);
    assert_eq!(body["count"], 0);
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_list_webhooks_returns_created() {
    let (app, _db) = common::setup_test_app().await;

    // Create webhook
    let payload = json!({
        "url": "https://example.com/hook",
        "events": ["order.placed"],
        "secret": "s3cr3t"
    });

    let req = post_req("/admin/webhooks", &payload);
    let res = app.clone().oneshot(req).await.unwrap();
    let create_body = body_json(res).await;
    let webhook_url = create_body["webhook"]["url"].as_str().unwrap();

    // List webhooks
    let req = get_req("/admin/webhooks");
    let res = app.clone().oneshot(req).await.unwrap();
    let list_body = body_json(res).await;

    assert_eq!(list_body["count"], 1);
    assert_eq!(list_body["webhooks"][0]["url"], webhook_url);
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_delete_webhook() {
    let (app, _db) = common::setup_test_app().await;

    // Create webhook
    let payload = json!({
        "url": "https://example.com/hook",
        "events": ["order.placed"],
        "secret": "s3cr3t"
    });

    let req = post_req("/admin/webhooks", &payload);
    let res = app.clone().oneshot(req).await.unwrap();
    let create_body = body_json(res).await;
    let webhook_id = create_body["webhook"]["id"].as_str().unwrap().to_string();

    // Delete webhook
    let req = delete_req(&format!("/admin/webhooks/{}", webhook_id));
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Verify deletion
    let req = get_req("/admin/webhooks");
    let res = app.clone().oneshot(req).await.unwrap();
    let list_body = body_json(res).await;
    assert_eq!(list_body["count"], 0);
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_delete_webhook_not_found() {
    let (app, _db) = common::setup_test_app().await;

    let req = delete_req("/admin/webhooks/nonexistent-id");
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial_test::serial]
async fn test_webhook_delivered_on_order_placed() {
    let (app, _db) = common::setup_test_app().await;

    // Start mock server
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hook"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Create webhook subscription
    let payload = json!({
        "url": format!("{}/hook", mock_server.uri()),
        "events": ["order.placed"],
        "secret": "test-secret"
    });

    let req = post_req("/admin/webhooks", &payload);
    let _ = app.clone().oneshot(req).await.unwrap();

    // Create a product with variant
    let product_body = json!({
        "title": "Test Product",
        "description": "Test",
        "status": "published",
        "options": [{"title": "Size", "values": ["S"]}],
        "variants": [
            {
                "title": "S",
                "sku": "TEST-S",
                "price": 1000,
                "options": {"Size": "S"}
            }
        ]
    });

    let req = request(Method::POST, "/admin/products", &product_body);
    let res = app.clone().oneshot(req).await.unwrap();
    let product_data = body_json(res).await;
    let variant_id = product_data["product"]["variants"][0]["id"]
        .as_str()
        .unwrap();

    // Create a cart
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    // Add line item
    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    // Complete cart (creates order and event, which should trigger webhook)
    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let _ = app.clone().oneshot(complete_req).await.unwrap();

    // Give the spawned task time to complete
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Check that webhook was delivered
    let reqs = mock_server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);

    // Verify headers
    assert!(reqs[0]
        .headers
        .iter()
        .any(|(k, _)| k.as_str().eq_ignore_ascii_case("x-toko-signature")));
    assert!(reqs[0]
        .headers
        .iter()
        .any(|(k, _)| k.as_str().eq_ignore_ascii_case("content-type")));

    // Verify body is JSON with event_name
    let body_str = String::from_utf8(reqs[0].body.clone()).unwrap();
    let event_json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(event_json["event_name"], "order.placed");
}

#[tokio::test]
#[serial_test::serial]
async fn test_webhook_hmac_signature_correct() {
    let (app, _db) = common::setup_test_app().await;

    // Start mock server
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hook"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Create webhook subscription
    let payload = json!({
        "url": format!("{}/hook", mock_server.uri()),
        "events": ["order.placed"],
        "secret": "test-secret"
    });

    let req = post_req("/admin/webhooks", &payload);
    let _ = app.clone().oneshot(req).await.unwrap();

    // Create a product with variant
    let product_body = json!({
        "title": "Test Product",
        "description": "Test",
        "status": "published",
        "options": [{"title": "Size", "values": ["S"]}],
        "variants": [
            {
                "title": "S",
                "sku": "TEST-S",
                "price": 1000,
                "options": {"Size": "S"}
            }
        ]
    });

    let req = request(Method::POST, "/admin/products", &product_body);
    let res = app.clone().oneshot(req).await.unwrap();
    let product_data = body_json(res).await;
    let variant_id = product_data["product"]["variants"][0]["id"]
        .as_str()
        .unwrap();

    // Create a cart
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    // Add line item
    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    // Complete cart
    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let _ = app.clone().oneshot(complete_req).await.unwrap();

    // Give the spawned task time to complete
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Get received request
    let reqs = mock_server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);

    // Extract signature from header
    let sig_header = reqs[0]
        .headers
        .iter()
        .find(|(k, _)| k.as_str().eq_ignore_ascii_case("x-toko-signature"))
        .map(|(_, v)| v.clone())
        .unwrap();

    // Compute expected HMAC
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let body_str = String::from_utf8(reqs[0].body.clone()).unwrap();
    let mut mac = HmacSha256::new_from_slice(b"test-secret").unwrap();
    mac.update(body_str.as_bytes());
    let expected = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

    assert_eq!(sig_header, expected);
}
