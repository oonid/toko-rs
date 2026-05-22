mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

async fn body_text(resp: axum::http::Response<Body>) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn get_req(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn post_req(uri: &str, payload: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

/// Create a completed order via the HTTP API.
/// Inserts product/variant if not present, creates a cart with optional email,
/// adds a line item, completes the cart, and returns the order ID.
async fn create_order(
    app: &axum::Router,
    pool: &toko_rs::db::DbPool,
    email: Option<&str>,
) -> String {
    sqlx::query(
        "INSERT INTO products (id, title, handle, status) \
         VALUES ('prod_exp', 'Export Test', 'export-test', 'published') \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, sku, price) \
         VALUES ('var_exp', 'prod_exp', 'Default', 'EXP-DEFAULT', 10000) \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();

    let mut cart_body = json!({"currency_code": "idr"});
    if let Some(e) = email {
        cart_body["email"] = serde_json::Value::String(e.to_string());
    }
    let res = app
        .clone()
        .oneshot(post_req("/store/carts", &cart_body))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    app.clone()
        .oneshot(post_req(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_exp", "quantity": 1}),
        ))
        .await
        .unwrap();

    let res = app
        .clone()
        .oneshot(post_req(
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    body_json(res).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_export_orders_returns_200_with_csv_content_type() {
    let (app, db) = common::setup_test_app().await;
    create_order(&app, &db.pool, None).await;

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/csv"), "Expected text/csv, got: {}", ct);
}

#[tokio::test]
async fn test_admin_export_orders_csv_has_correct_headers() {
    let (app, _db) = common::setup_test_app().await;

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let first_line = body.lines().next().unwrap_or("");
    assert_eq!(
        first_line,
        "Order ID,Display ID,Email,Currency,Status,Fulfillment Status,Payment Status,Item Count,Total (cents),Created At,Shipped At,Canceled At"
    );
}

#[tokio::test]
async fn test_admin_export_orders_one_row_per_order() {
    let (app, db) = common::setup_test_app().await;
    create_order(&app, &db.pool, None).await;
    create_order(&app, &db.pool, None).await;
    create_order(&app, &db.pool, None).await;

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        4,
        "Expected 1 header + 3 data rows, got {}. Body:\n{}",
        lines.len(),
        body
    );
}

#[tokio::test]
async fn test_admin_export_orders_empty_db_returns_headers_only() {
    let (app, _db) = common::setup_test_app().await;

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "Expected only the header row, got {}. Body:\n{}",
        lines.len(),
        body
    );
}

#[tokio::test]
async fn test_admin_export_orders_filter_by_status() {
    let (app, db) = common::setup_test_app().await;

    let id1 = create_order(&app, &db.pool, None).await;
    let id2 = create_order(&app, &db.pool, None).await;
    let _pending = create_order(&app, &db.pool, None).await;

    // Complete two orders via admin API
    app.clone()
        .oneshot(post_req(
            &format!("/admin/orders/{}/complete", id1),
            &json!(null),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_req(
            &format!("/admin/orders/{}/complete", id2),
            &json!(null),
        ))
        .await
        .unwrap();

    let res = app
        .oneshot(get_req("/admin/orders/export?status=completed"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        3,
        "Expected 1 header + 2 completed rows, got {}. Body:\n{}",
        lines.len(),
        body
    );
    for data_line in &lines[1..] {
        let cols: Vec<&str> = data_line.splitn(13, ',').collect();
        assert_eq!(
            cols[4], "completed",
            "Expected status=completed, got '{}' in row: {}",
            cols[4], data_line
        );
    }
}

#[tokio::test]
async fn test_admin_export_orders_invalid_status_returns_400() {
    let (app, _db) = common::setup_test_app().await;

    let res = app
        .oneshot(get_req("/admin/orders/export?status=bogus"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_export_orders_filter_by_date_from() {
    let (app, db) = common::setup_test_app().await;
    let pool = &db.pool;

    // Insert two orders directly with controlled timestamps
    sqlx::query(
        "INSERT INTO orders \
         (id, display_id, email, currency_code, status, fulfillment_status, created_at, updated_at) \
         VALUES \
         ('order_old1', 901, 'old@example.com', 'idr', 'pending', 'not_fulfilled', \
          '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO orders \
         (id, display_id, email, currency_code, status, fulfillment_status, created_at, updated_at) \
         VALUES \
         ('order_new1', 902, 'new@example.com', 'idr', 'pending', 'not_fulfilled', \
          '2025-06-01T00:00:00Z', '2025-06-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();

    let res = app
        .oneshot(get_req(
            "/admin/orders/export?created_at_from=2024-12-31T23%3A59%3A59Z",
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected 1 header + 1 row (only 2025 order), got {}. Body:\n{}",
        lines.len(),
        body
    );
    assert!(
        lines[1].contains("new@example.com"),
        "Expected new@example.com in data row: {}",
        lines[1]
    );
}

#[tokio::test]
async fn test_admin_export_orders_filter_by_email() {
    let (app, db) = common::setup_test_app().await;
    create_order(&app, &db.pool, Some("budi@example.com")).await;
    create_order(&app, &db.pool, Some("other@example.com")).await;

    let res = app
        .oneshot(get_req("/admin/orders/export?q=budi"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected 1 header + 1 matching row, got {}. Body:\n{}",
        lines.len(),
        body
    );
    assert!(
        lines[1].contains("budi@example.com"),
        "Expected budi@example.com in row: {}",
        lines[1]
    );
}

#[tokio::test]
async fn test_admin_export_orders_payment_status_captured() {
    let (app, db) = common::setup_test_app().await;
    let order_id = create_order(&app, &db.pool, None).await;

    // Capture payment via admin API
    let res = app
        .clone()
        .oneshot(post_req(
            &format!("/admin/orders/{}/capture-payment", order_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "Expected 1 header + 1 data row");
    let cols: Vec<&str> = lines[1].splitn(13, ',').collect();
    assert_eq!(
        cols[6], "captured",
        "Expected payment_status=captured, got '{}'. Row: {}",
        cols[6], lines[1]
    );
}

#[tokio::test]
async fn test_admin_export_orders_item_count_and_total() {
    let (app, db) = common::setup_test_app().await;
    let pool = &db.pool;

    // Insert product and two variants with known prices
    sqlx::query(
        "INSERT INTO products (id, title, handle, status) \
         VALUES ('prod_exp', 'Export Test', 'export-test', 'published') \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, sku, price) \
         VALUES ('var_exp', 'prod_exp', 'Var A', 'EXP-A', 50000) \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, sku, price) \
         VALUES ('var_exp2', 'prod_exp', 'Var B', 'EXP-B', 100000) \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();

    // Create cart
    let res = app
        .clone()
        .oneshot(post_req("/store/carts", &json!({"currency_code": "idr"})))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add two line items: qty=2 @ 50000 and qty=1 @ 100000
    app.clone()
        .oneshot(post_req(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_exp", "quantity": 2}),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_req(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_exp2", "quantity": 1}),
        ))
        .await
        .unwrap();

    // Complete cart
    app.clone()
        .oneshot(post_req(
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "Expected 1 header + 1 data row");
    let cols: Vec<&str> = lines[1].splitn(13, ',').collect();
    assert_eq!(
        cols[7], "3",
        "Expected item_count=3 (qty 2 + qty 1), got '{}'. Row: {}",
        cols[7], lines[1]
    );
    assert_eq!(
        cols[8], "200000",
        "Expected total_cents=200000 (2×50000 + 1×100000), got '{}'. Row: {}",
        cols[8], lines[1]
    );
}

#[tokio::test]
async fn test_admin_export_orders_chronological_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = &db.pool;

    // Insert three orders in REVERSE time order to confirm the query sorts them
    sqlx::query(
        "INSERT INTO orders \
         (id, display_id, email, currency_code, status, fulfillment_status, created_at, updated_at) \
         VALUES \
         ('order_t3', 903, 't3@example.com', 'idr', 'pending', 'not_fulfilled', \
          '2025-03-01T00:00:00Z', '2025-03-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO orders \
         (id, display_id, email, currency_code, status, fulfillment_status, created_at, updated_at) \
         VALUES \
         ('order_t1', 901, 't1@example.com', 'idr', 'pending', 'not_fulfilled', \
          '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO orders \
         (id, display_id, email, currency_code, status, fulfillment_status, created_at, updated_at) \
         VALUES \
         ('order_t2', 902, 't2@example.com', 'idr', 'pending', 'not_fulfilled', \
          '2025-02-01T00:00:00Z', '2025-02-01T00:00:00Z')",
    )
    .execute(pool)
    .await
    .unwrap();

    let res = app.oneshot(get_req("/admin/orders/export")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        4,
        "Expected 1 header + 3 data rows, got {}. Body:\n{}",
        lines.len(),
        body
    );
    assert!(
        lines[1].contains("t1@example.com"),
        "Row 1 should be oldest (t1), got: {}",
        lines[1]
    );
    assert!(
        lines[2].contains("t2@example.com"),
        "Row 2 should be middle (t2), got: {}",
        lines[2]
    );
    assert!(
        lines[3].contains("t3@example.com"),
        "Row 3 should be newest (t3), got: {}",
        lines[3]
    );
}
