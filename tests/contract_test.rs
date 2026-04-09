mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

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

async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn assert_oas_error(body: &serde_json::Value, expected_type: &str, expected_code: &str) {
    assert!(body["code"].is_string(), "error must have 'code' field");
    assert!(body["type"].is_string(), "error must have 'type' field");
    assert!(
        body["message"].is_string(),
        "error must have 'message' field"
    );
    assert_eq!(
        body.as_object().unwrap().keys().count(),
        3,
        "error must have exactly 3 fields: code, type, message"
    );
    assert_eq!(body["code"], expected_code);
    assert_eq!(body["type"], expected_type);
}

fn assert_has_fields(body: &serde_json::Value, fields: &[&str]) {
    let obj = body
        .as_object()
        .unwrap_or_else(|| panic!("expected JSON object, got: {}", body));
    for field in fields {
        assert!(
            obj.contains_key(*field),
            "missing field '{}'. Available: {:?}",
            field,
            obj.keys().collect::<Vec<_>>()
        );
    }
}

fn request_with_header(method: Method, uri: &str, header: (&str, &str)) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header.0, header.1)
        .body(Body::empty())
        .unwrap()
}

fn post_with_header(uri: &str, payload: &serde_json::Value, header: (&str, &str)) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json")
        .header(header.0, header.1)
        .body(Body::from(payload.to_string()))
        .unwrap()
}

// ============================================================
// 10.6 — Response contract tests (JSON shape validation)
// ============================================================

#[tokio::test]
async fn test_contract_product_response_shape() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": "Contract Test"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["product"]);
    let p = &body["product"];
    assert_has_fields(
        p,
        &[
            "id",
            "title",
            "handle",
            "status",
            "created_at",
            "updated_at",
            "options",
            "variants",
        ],
    );
}

#[tokio::test]
async fn test_contract_product_list_response_shape() {
    let (app, _) = common::setup_test_app().await;
    app.clone()
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": "P1"}),
        ))
        .await
        .unwrap();
    let res = app
        .oneshot(request(Method::GET, "/admin/products", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["products", "count", "offset", "limit"]);
}

#[tokio::test]
async fn test_contract_delete_response_shape() {
    let (app, _) = common::setup_test_app().await;
    let created = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/admin/products",
                &json!({"title": "ToDelete"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let id = created["product"]["id"].as_str().unwrap();
    let res = app
        .oneshot(request(
            Method::DELETE,
            &format!("/admin/products/{}", id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["id", "object", "deleted"]);
    assert_eq!(body["object"], "product");
    assert_eq!(body["deleted"], true);
}

#[tokio::test]
async fn test_contract_line_item_delete_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    sqlx::query(
        "INSERT INTO products (id, title, handle, status) VALUES ('p1', 'T', 't', 'published')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, price) VALUES ('v1', 'p1', 'V', 500)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cart = body_json(
        app.clone()
            .oneshot(request(Method::POST, "/store/carts", &json!({})))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();

    let add_res = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                &format!("/store/carts/{}/line-items", cart_id),
                &json!({"variant_id": "v1", "quantity": 1}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let line_id = add_res["cart"]["items"][0]["id"].as_str().unwrap();

    let res = app
        .oneshot(request(
            Method::DELETE,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["id", "object", "deleted", "parent"]);
    assert_eq!(body["object"], "line-item");
    assert_eq!(body["deleted"], true);
    assert_eq!(body["id"], line_id);
    assert_has_fields(&body["parent"], &["id", "items", "item_total", "total"]);
}

#[tokio::test]
async fn test_contract_cart_response_shape() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(Method::POST, "/store/carts", &json!({})))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["cart"]);
    let cart = &body["cart"];
    assert_has_fields(
        cart,
        &[
            "id",
            "currency_code",
            "items",
            "item_total",
            "total",
            "created_at",
            "updated_at",
        ],
    );
}

#[tokio::test]
async fn test_contract_customer_response_shape() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/store/customers",
            &json!({"email": "shape@test.com", "first_name": "Shape"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["customer"]);
    let c = &body["customer"];
    assert_has_fields(
        c,
        &["id", "email", "has_account", "created_at", "updated_at"],
    );
}

#[tokio::test]
async fn test_contract_order_complete_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    sqlx::query(
        "INSERT INTO products (id, title, handle, status) VALUES ('p1', 'T', 't', 'published')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, price) VALUES ('v1', 'p1', 'V', 500)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "idr"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();

    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "v1", "quantity": 1}),
        ))
        .await
        .unwrap();

    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["type", "order"]);
    let obj = body.as_object().unwrap();
    assert_eq!(
        obj.keys().count(),
        2,
        "cart complete response must have exactly 2 top-level fields: type, order"
    );
    assert!(
        !obj.contains_key("payment"),
        "cart complete response must NOT contain 'payment' (Medusa returns {{ type, order }} only)"
    );
    assert_has_fields(
        &body["order"],
        &[
            "id",
            "display_id",
            "status",
            "items",
            "item_total",
            "total",
            "currency_code",
            "created_at",
        ],
    );
}

#[tokio::test]
async fn test_contract_order_detail_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    sqlx::query(
        "INSERT INTO products (id, title, handle, status) VALUES ('p1', 'T', 't', 'published')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, price) VALUES ('v1', 'p1', 'V', 500)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "idr"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();
    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "v1", "quantity": 1}),
        ))
        .await
        .unwrap();
    let complete = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                &format!("/store/carts/{}/complete", cart_id),
                &json!(null),
            ))
            .await
            .unwrap(),
    )
    .await;
    let order_id = complete["order"]["id"].as_str().unwrap();

    let res = app
        .oneshot(request_with_header(
            Method::GET,
            &format!("/store/orders/{}", order_id),
            ("X-Customer-Id", "any"),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["order"]);
    let obj = body.as_object().unwrap();
    assert_eq!(
        obj.keys().count(),
        1,
        "order detail response must have exactly 1 top-level field: order"
    );
    assert!(
        !obj.contains_key("payment"),
        "order detail response must NOT contain 'payment' (Medusa StoreOrderResponse is {{ order }} only)"
    );
    assert_has_fields(
        &body["order"],
        &["id", "display_id", "status", "items", "item_total", "total"],
    );
}

#[tokio::test]
async fn test_contract_order_list_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('c1', 'T', 't@t.com', 1)")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO products (id, title, handle, status) VALUES ('p1', 'T', 't', 'published')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO product_variants (id, product_id, title, price) VALUES ('v1', 'p1', 'V', 500)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "idr", "customer_id": "c1"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();
    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "v1", "quantity": 1}),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();

    let res = app
        .oneshot(request_with_header(
            Method::GET,
            "/store/orders",
            ("X-Customer-Id", "c1"),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["orders", "count", "offset", "limit"]);
}

#[tokio::test]
async fn test_contract_health_response_shape() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(Method::GET, "/health", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_has_fields(&body, &["status", "database", "version"]);
}

// ============================================================
// 10.7 — Error contract tests (3-field OAS Error schema)
// ============================================================

#[tokio::test]
async fn test_error_404_product_not_found() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::GET,
            "/admin/products/prod_nope",
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_oas_error(&body_json(res).await, "not_found", "invalid_request_error");
}

#[tokio::test]
async fn test_error_404_cart_not_found() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(Method::GET, "/store/carts/cart_nope", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_oas_error(&body_json(res).await, "not_found", "invalid_request_error");
}

#[tokio::test]
async fn test_error_404_order_not_found() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request_with_header(
            Method::GET,
            "/store/orders/order_nope",
            ("X-Customer-Id", "any"),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_oas_error(&body_json(res).await, "not_found", "invalid_request_error");
}

#[tokio::test]
async fn test_error_401_missing_customer_header() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(Method::GET, "/store/customers/me", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_oas_error(&body_json(res).await, "unauthorized", "unknown_error");
}

#[tokio::test]
async fn test_error_400_invalid_product_title() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": ""}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_oas_error(
        &body_json(res).await,
        "invalid_data",
        "invalid_request_error",
    );
}

#[tokio::test]
async fn test_error_400_invalid_customer_email() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/store/customers",
            &json!({"email": "nope", "first_name": "T"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_oas_error(
        &body_json(res).await,
        "invalid_data",
        "invalid_request_error",
    );
}

#[tokio::test]
async fn test_error_422_duplicate_product_handle() {
    let (app, _) = common::setup_test_app().await;
    app.clone()
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": "Unique"}),
        ))
        .await
        .unwrap();
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": "Other", "handle": "unique"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_oas_error(
        &body_json(res).await,
        "duplicate_error",
        "invalid_request_error",
    );
}

#[tokio::test]
async fn test_error_422_duplicate_customer_email() {
    let (app, _) = common::setup_test_app().await;
    app.clone()
        .oneshot(request(
            Method::POST,
            "/store/customers",
            &json!({"email": "dup@test.com", "first_name": "A"}),
        ))
        .await
        .unwrap();
    let res = app
        .oneshot(request(
            Method::POST,
            "/store/customers",
            &json!({"email": "dup@test.com", "first_name": "B"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_oas_error(
        &body_json(res).await,
        "duplicate_error",
        "invalid_request_error",
    );
}

#[tokio::test]
async fn test_error_400_empty_cart_completion() {
    let (app, _) = common::setup_test_app().await;
    let cart = body_json(
        app.clone()
            .oneshot(request(Method::POST, "/store/carts", &json!({})))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();
    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_oas_error(
        &body_json(res).await,
        "invalid_data",
        "invalid_request_error",
    );
}

#[tokio::test]
async fn test_error_409_completed_cart_update() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    let cart = body_json(
        app.clone()
            .oneshot(request(Method::POST, "/store/carts", &json!({})))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();
    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(cart_id)
        .execute(&pool)
        .await
        .unwrap();
    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}", cart_id),
            &json!({"email": "x@x.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert_oas_error(&body_json(res).await, "conflict", "invalid_state_error");
}

#[tokio::test]
async fn test_error_401_orders_without_auth() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(Method::GET, "/store/orders", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_oas_error(&body_json(res).await, "unauthorized", "unknown_error");
}

// ============================================================
// 10.8 / 12.7 — HTTP method convention audit
// ============================================================

#[tokio::test]
async fn test_http_method_post_for_product_update() {
    let (app, _) = common::setup_test_app().await;
    let created = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/admin/products",
                &json!({"title": "Method"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let id = created["product"]["id"].as_str().unwrap();
    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/admin/products/{}", id),
            &json!({"title": "Updated"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["product"]["title"], "Updated");
}

#[tokio::test]
async fn test_http_method_post_for_customer_update() {
    let (app, _) = common::setup_test_app().await;
    let created = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/customers",
                &json!({"email": "method@test.com", "first_name": "M"}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let cus_id = created["customer"]["id"].as_str().unwrap();
    let res = app
        .oneshot(post_with_header(
            "/store/customers/me",
            &json!({"first_name": "Updated"}),
            ("X-Customer-Id", cus_id),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["customer"]["first_name"], "Updated");
}

#[tokio::test]
async fn test_http_method_post_for_cart_update() {
    let (app, _) = common::setup_test_app().await;
    let cart = body_json(
        app.clone()
            .oneshot(request(Method::POST, "/store/carts", &json!({})))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();
    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}", cart_id),
            &json!({"email": "post@test.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["cart"]["email"], "post@test.com");
}

// ============================================================
// CORS preflight (foundation spec)
// ============================================================

#[tokio::test]
async fn test_cors_preflight_headers() {
    let (app, _) = common::setup_test_app().await;
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/store/products")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let headers = res.headers();
    assert!(
        headers.get("access-control-allow-origin").is_some(),
        "CORS must include access-control-allow-origin header"
    );
    assert!(
        headers.get("access-control-allow-methods").is_some(),
        "CORS must include access-control-allow-methods header"
    );
}
