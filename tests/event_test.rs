mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

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

#[tokio::test]
#[serial_test::serial]
async fn test_order_placed_creates_event_row() {
    let (app, db) = common::setup_test_app().await;

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

    // Complete cart (creates order and event)
    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let complete_res = app.clone().oneshot(complete_req).await.unwrap();
    let complete_data = body_json(complete_res).await;
    let order_id = complete_data["order"]["id"].as_str().unwrap();

    // Check event_outbox for order.placed event
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_name, resource_id FROM event_outbox WHERE event_name = 'order.placed' AND resource_id = $1",
    )
    .bind(order_id)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "order.placed");
    assert_eq!(rows[0].1, order_id);
}

#[tokio::test]
#[serial_test::serial]
async fn test_order_canceled_creates_event_row() {
    let (app, db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let complete_res = app.clone().oneshot(complete_req).await.unwrap();
    let complete_data = body_json(complete_res).await;
    let order_id = complete_data["order"]["id"].as_str().unwrap().to_string();

    // Cancel order
    let cancel_req = post_req(&format!("/admin/orders/{}/cancel", order_id), &json!({}));
    let _ = app.clone().oneshot(cancel_req).await.unwrap();

    // Check event_outbox for order.canceled event
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_name, resource_id FROM event_outbox WHERE event_name = 'order.canceled' AND resource_id = $1",
    )
    .bind(&order_id)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "order.canceled");
}

#[tokio::test]
#[serial_test::serial]
async fn test_order_fulfilled_creates_event_row() {
    let (app, db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let complete_res = app.clone().oneshot(complete_req).await.unwrap();
    let complete_data = body_json(complete_res).await;
    let order_id = complete_data["order"]["id"].as_str().unwrap().to_string();

    // Fulfill order
    let fulfill_req = post_req(&format!("/admin/orders/{}/fulfill", order_id), &json!({}));
    let _ = app.clone().oneshot(fulfill_req).await.unwrap();

    // Check event_outbox for order.fulfillment_created event
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_name, resource_id FROM event_outbox WHERE event_name = 'order.fulfillment_created' AND resource_id = $1",
    )
    .bind(&order_id)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "order.fulfillment_created");
}

#[tokio::test]
#[serial_test::serial]
async fn test_order_shipped_creates_event_row() {
    let (app, db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let complete_res = app.clone().oneshot(complete_req).await.unwrap();
    let complete_data = body_json(complete_res).await;
    let order_id = complete_data["order"]["id"].as_str().unwrap().to_string();

    // Fulfill then ship order
    let fulfill_req = post_req(&format!("/admin/orders/{}/fulfill", order_id), &json!({}));
    let _ = app.clone().oneshot(fulfill_req).await.unwrap();

    let ship_req = post_req(&format!("/admin/orders/{}/ship", order_id), &json!({}));
    let _ = app.clone().oneshot(ship_req).await.unwrap();

    // Check event_outbox for order.shipment_created event
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_name, resource_id FROM event_outbox WHERE event_name = 'order.shipment_created' AND resource_id = $1",
    )
    .bind(&order_id)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "order.shipment_created");
}

#[tokio::test]
#[serial_test::serial]
async fn test_payment_captured_creates_event_row() {
    let (app, db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let complete_res = app.clone().oneshot(complete_req).await.unwrap();
    let complete_data = body_json(complete_res).await;
    let order_id = complete_data["order"]["id"].as_str().unwrap().to_string();

    // Capture payment
    let capture_req = post_req(
        &format!("/admin/orders/{}/capture-payment", order_id),
        &json!({}),
    );
    let _ = app.clone().oneshot(capture_req).await.unwrap();

    // Check event_outbox for payment.captured event
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT event_name, resource_id FROM event_outbox WHERE event_name = 'payment.captured' AND resource_id = $1",
    )
    .bind(&order_id)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "payment.captured");
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_list_events_returns_events() {
    let (app, _db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let _ = app.clone().oneshot(complete_req).await.unwrap();

    // List events
    let list_req = get_req("/admin/events");
    let list_res = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);
    let list_data = body_json(list_res).await;
    assert!(list_data["events"].is_array());
    assert!(!list_data["events"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_list_events_filter_by_resource_type() {
    let (app, _db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let complete_res = app.clone().oneshot(complete_req).await.unwrap();
    let complete_data = body_json(complete_res).await;
    let order_id = complete_data["order"]["id"].as_str().unwrap().to_string();

    // Capture payment
    let capture_req = post_req(
        &format!("/admin/orders/{}/capture-payment", order_id),
        &json!({}),
    );
    let _ = app.clone().oneshot(capture_req).await.unwrap();

    // List events filtered by resource_type=order
    let list_req = get_req("/admin/events?resource_type=order");
    let list_res = app.clone().oneshot(list_req).await.unwrap();
    let list_data = body_json(list_res).await;
    let events = list_data["events"].as_array().unwrap();

    // All returned events should be order resource_type
    for event in events {
        assert_eq!(event["resource_type"].as_str().unwrap(), "order");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_mark_event_processed() {
    let (app, db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let _ = app.clone().oneshot(complete_req).await.unwrap();

    // Get event ID from outbox
    let event_id: String = sqlx::query_scalar("SELECT id FROM event_outbox LIMIT 1")
        .fetch_one(&db.pool)
        .await
        .unwrap();

    // Mark event as processed
    let mark_req = post_req(
        &format!("/admin/events/{}/mark-processed", event_id),
        &json!({}),
    );
    let mark_res = app.clone().oneshot(mark_req).await.unwrap();
    assert_eq!(mark_res.status(), StatusCode::OK);
    let mark_data = body_json(mark_res).await;

    // Check that processed_at is not null
    assert!(mark_data["event"]["processed_at"].is_string());
}

#[tokio::test]
#[serial_test::serial]
async fn test_admin_list_events_unprocessed_only() {
    let (app, db) = common::setup_test_app().await;

    // Create product and order
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

    // Create cart and order
    let cart_req = post_req("/store/carts", &json!({"currency_code": "idr"}));
    let cart_res = app.clone().oneshot(cart_req).await.unwrap();
    let cart_data = body_json(cart_res).await;
    let cart_id = cart_data["cart"]["id"].as_str().unwrap();

    let add_item_req = post_req(
        &format!("/store/carts/{}/line-items", cart_id),
        &json!({"variant_id": variant_id, "quantity": 1}),
    );
    let _ = app.clone().oneshot(add_item_req).await.unwrap();

    let complete_req = post_req(&format!("/store/carts/{}/complete", cart_id), &json!({}));
    let _ = app.clone().oneshot(complete_req).await.unwrap();

    // Get event ID from outbox
    let event_id: String = sqlx::query_scalar("SELECT id FROM event_outbox LIMIT 1")
        .fetch_one(&db.pool)
        .await
        .unwrap();

    // Mark event as processed
    let mark_req = post_req(
        &format!("/admin/events/{}/mark-processed", event_id),
        &json!({}),
    );
    let _ = app.clone().oneshot(mark_req).await.unwrap();

    // List unprocessed events only
    let list_req = get_req("/admin/events?unprocessed_only=true");
    let list_res = app.clone().oneshot(list_req).await.unwrap();
    let list_data = body_json(list_res).await;
    let events = list_data["events"].as_array().unwrap();

    // Should not include the marked event
    for event in events {
        assert_ne!(event["id"].as_str().unwrap(), &event_id);
    }
}
