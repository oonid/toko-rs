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

async fn seed_product_and_variant(pool: &toko_rs::db::DbPool) {
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_inv', 'Invoice Product', 'inv-prod', 'published') ON CONFLICT (id) DO NOTHING")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_inv', 'prod_inv', 'Std', 'INV-S', 50000) ON CONFLICT (id) DO NOTHING")
        .execute(pool).await.unwrap();
}

async fn create_order_for_invoice(app: &axum::Router, pool: &toko_rs::db::DbPool) -> String {
    seed_product_and_variant(pool).await;

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
            &json!({"variant_id": "var_inv", "quantity": 3}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

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
    body_json(res).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn seed_invoice_config(pool: &toko_rs::db::DbPool) {
    sqlx::query(
        "INSERT INTO invoice_config (id, company_name, company_address, company_phone, company_email) \
         VALUES ('invcfg_seed', 'Toko Test', 'Jl. Test No. 1', '+628000000000', 'test@tokotest.com') \
         ON CONFLICT (id) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_config_returns_404_when_empty() {
    let (app, _db) = common::setup_test_app().await;

    let req = Request::builder()
        .uri("/admin/invoice-config")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "not_found");
}

#[tokio::test]
async fn test_create_config_via_upsert() {
    let (app, _db) = common::setup_test_app().await;

    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/invoice-config",
            &json!({
                "company_name": "Toko Sejahtera",
                "company_address": "Jl. Merdeka No. 10, Jakarta",
                "company_phone": "+6281234567890",
                "company_email": "admin@tokosejahtera.com"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["invoice_config"]["company_name"], "Toko Sejahtera");
    assert_eq!(
        body["invoice_config"]["company_address"],
        "Jl. Merdeka No. 10, Jakarta"
    );
    assert_eq!(
        body["invoice_config"]["company_phone"],
        "+6281234567890"
    );
    assert_eq!(
        body["invoice_config"]["company_email"],
        "admin@tokosejahtera.com"
    );
    assert!(body["invoice_config"]["id"].is_string());
    assert!(body["invoice_config"]["created_at"].is_string());
}

#[tokio::test]
async fn test_get_config_after_create() {
    let (app, _db) = common::setup_test_app().await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/admin/invoice-config",
            &json!({"company_name": "Test Co", "company_address": "Addr", "company_phone": "123", "company_email": "a@b.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .uri("/admin/invoice-config")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["invoice_config"]["company_name"], "Test Co");
}

#[tokio::test]
async fn test_update_config_partial() {
    let (app, _db) = common::setup_test_app().await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/admin/invoice-config",
            &json!({"company_name": "Old Name", "company_address": "Old Addr", "company_phone": "111", "company_email": "old@test.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/invoice-config",
            &json!({"company_name": "New Name", "notes": "Thank you for shopping"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["invoice_config"]["company_name"], "New Name");
    assert_eq!(body["invoice_config"]["company_address"], "Old Addr");
    assert_eq!(body["invoice_config"]["notes"], "Thank you for shopping");
}

#[tokio::test]
async fn test_get_invoice_generates_on_the_fly() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_invoice_config(&pool).await;
    let order_id = create_order_for_invoice(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/admin/orders/{}/invoice", order_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    let inv = &body["invoice"];
    assert!(inv["invoice_number"].is_string());
    assert!(inv["invoice_number"]
        .as_str()
        .unwrap()
        .starts_with("INV-"));
    assert_eq!(inv["status"], "latest");
    assert!(inv["date"].is_string());
    assert!(inv["order"]["id"].is_string());
    assert_eq!(inv["order"]["id"], order_id);
    assert_eq!(inv["issuer"]["company_name"], "Toko Test");
    assert_eq!(
        inv["issuer"]["company_email"],
        "test@tokotest.com"
    );
    assert!(inv["order"]["items"].is_array());
    assert_eq!(inv["order"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(inv["order"]["items"][0]["quantity"], 3);
    assert_eq!(inv["order"]["items"][0]["unit_price"], 50000);
}

#[tokio::test]
async fn test_get_invoice_number_matches_display_id() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_invoice_config(&pool).await;
    let order_id = create_order_for_invoice(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/admin/orders/{}/invoice", order_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    let display_id = body["invoice"]["order"]["display_id"].as_i64().unwrap();
    let invoice_number = body["invoice"]["invoice_number"].as_str().unwrap();
    assert_eq!(invoice_number, format!("INV-{:04}", display_id));
}

#[tokio::test]
async fn test_get_invoice_returns_404_no_config() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_order_for_invoice(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/admin/orders/{}/invoice", order_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "not_found");
}

#[tokio::test]
async fn test_get_invoice_returns_404_nonexistent_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_invoice_config(&pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/admin/orders/order_nonexistent/invoice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_invoice_includes_order_totals() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_invoice_config(&pool).await;
    let order_id = create_order_for_invoice(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/admin/orders/{}/invoice", order_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;

    assert_eq!(body["invoice"]["order"]["item_total"], 150000);
    assert_eq!(body["invoice"]["order"]["total"], 150000);
    assert_eq!(body["invoice"]["order"]["currency_code"], "idr");
}

#[tokio::test]
async fn test_get_invoice_includes_issuer_logo_and_notes() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/admin/invoice-config",
            &json!({
                "company_name": "Logo Co",
                "company_address": "Addr",
                "company_phone": "123",
                "company_email": "logo@co.com",
                "company_logo": "https://example.com/logo.png",
                "notes": "Payment due in 30 days"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let order_id = create_order_for_invoice(&app, &pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/admin/orders/{}/invoice", order_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;

    assert_eq!(
        body["invoice"]["issuer"]["company_logo"],
        "https://example.com/logo.png"
    );
    assert_eq!(body["invoice"]["notes"], "Payment due in 30 days");
}
