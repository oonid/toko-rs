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

async fn create_cart_with_item(app: &axum::Router, pool: &sqlx::SqlitePool) -> String {
    sqlx::query("INSERT OR IGNORE INTO products (id, title, handle, status) VALUES ('prod_1', 'Test Product', 'test', 'published')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT OR IGNORE INTO product_variants (id, product_id, title, sku, price) VALUES ('var_1', 'prod_1', 'Small', 'TEST-S', 1000)")
        .execute(pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "idr"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 2}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    cart_id
}

#[tokio::test]
async fn test_complete_cart_creates_order() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    let cart_id = create_cart_with_item(&app, &pool).await;

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
    assert_eq!(body["type"], "order");
    assert!(body["order"]["id"].as_str().unwrap().starts_with("order_"));
    assert_eq!(body["order"]["display_id"], 1);
    assert_eq!(body["order"]["status"], "pending");
    assert_eq!(body["order"]["currency_code"], "idr");
    assert_eq!(body["order"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["order"]["items"][0]["quantity"], 2);
    assert_eq!(body["order"]["items"][0]["unit_price"], 1000);
    assert_eq!(body["order"]["item_total"], 2000);
    assert_eq!(body["order"]["total"], 2000);
    assert_eq!(body["payment"]["status"], "pending");
    assert_eq!(body["payment"]["amount"], 2000);
    assert_eq!(body["payment"]["currency_code"], "idr");
    assert!(body["payment"]["id"].as_str().unwrap().starts_with("pay_"));
}

#[tokio::test]
async fn test_complete_empty_cart_rejected() {
    let (app, _) = common::setup_test_app().await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "idr"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = body_json(res).await;
    assert_eq!(body["type"], "unexpected_state");
}

#[tokio::test]
async fn test_complete_already_completed_cart_rejected() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    let cart_id = create_cart_with_item(&app, &pool).await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_display_id_increments() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;

    for i in 1..=3u64 {
        let cart_id = create_cart_with_item(&app, &pool).await;

        let res = app
            .clone()
            .oneshot(request(
                Method::POST,
                &format!("/store/carts/{}/complete", cart_id),
                &json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body["order"]["display_id"], i);
    }
}

#[tokio::test]
async fn test_get_order_by_id() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    let cart_id = create_cart_with_item(&app, &pool).await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    let body = body_json(res).await;
    let order_id = body["order"]["id"].as_str().unwrap().to_string();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/store/orders/{}", order_id))
                .header("X-Customer-Id", "cus_test1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["order"]["id"], order_id);
    assert_eq!(body["order"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["order"]["item_total"], 2000);
    assert_eq!(body["payment"]["status"], "pending");
}

#[tokio::test]
async fn test_get_order_not_found() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/store/orders/order_nonexistent")
                .header("X-Customer-Id", "cus_test1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_orders_by_customer() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;

    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_test1', 'Test', 'test@test.com', 1)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_1', 'Test', 'test', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_1', 'prod_1', 'Default', 500)")
        .execute(&pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "idr", "customer_id": "cus_test1"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 1}),
        ))
        .await
        .unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/store/orders")
                .header("X-Customer-Id", "cus_test1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["limit"], 20);
    assert_eq!(body["offset"], 0);
}

#[tokio::test]
async fn test_list_orders_without_auth_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(Method::GET, "/store/orders", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_complete_nonexistent_cart() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/store/carts/cart_nonexistent/complete",
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
