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

async fn seed_in_pool(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_1', 'Test Product', 'test', 'published')")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_1', 'prod_1', 'Small', 'TEST-S', 1000)")
        .execute(pool).await.unwrap();
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

#[tokio::test]
async fn test_store_create_cart_with_defaults() {
    let (app, _) = common::setup_test_app().await;

    let payload = json!({});
    let request = Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert!(body["cart"]["id"].as_str().unwrap().starts_with("cart_"));
    assert_eq!(body["cart"]["currency_code"], "idr");
    assert_eq!(body["cart"]["items"].as_array().unwrap().len(), 0);
    assert_eq!(body["cart"]["item_total"], 0);
    assert_eq!(body["cart"]["total"], 0);
}

#[tokio::test]
async fn test_store_create_cart_with_email() {
    let (app, _) = common::setup_test_app().await;

    let payload = json!({
        "currency_code": "eur",
        "email": "buyer@example.com"
    });
    let request = Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["cart"]["currency_code"], "eur");
    assert_eq!(body["cart"]["email"], "buyer@example.com");
}

#[tokio::test]
async fn test_store_create_cart_validation_failure() {
    let (app, _) = common::setup_test_app().await;

    let payload = json!({
        "email": "not-an-email"
    });
    let request = Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.oneshot(request).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cart_full_flow() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    seed_in_pool(&pool).await;

    // 1. Create cart
    let payload = json!({"currency_code": "idr"});
    let res = app
        .clone()
        .oneshot(request(Method::POST, "/store/carts", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    let cart_id = cart_resp["cart"]["id"].as_str().unwrap();

    // 2. Add line item with snapshot
    let payload = json!({"variant_id": "var_1", "quantity": 2});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    assert_eq!(cart_resp["cart"]["items"].as_array().unwrap().len(), 1);
    let item = &cart_resp["cart"]["items"][0];
    assert_eq!(item["unit_price"], 1000);
    assert_eq!(item["quantity"], 2);
    let snapshot = &item["snapshot"];
    assert_eq!(snapshot["product_title"], "Test Product");
    assert_eq!(snapshot["variant_title"], "Small");
    assert_eq!(snapshot["variant_sku"], "TEST-S");
    assert_eq!(cart_resp["cart"]["item_total"], 2000);
    assert_eq!(cart_resp["cart"]["total"], 2000);
    let line_id = item["id"].as_str().unwrap();

    // 3. Update line item quantity to 5
    let payload = json!({"quantity": 5});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    assert_eq!(cart_resp["cart"]["items"][0]["quantity"], 5);
    assert_eq!(cart_resp["cart"]["item_total"], 5000);

    // 4. Update cart email
    let payload = json!({"email": "test@test.com"});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    assert_eq!(cart_resp["cart"]["email"], "test@test.com");

    // 5. Delete line item
    let res = app
        .clone()
        .oneshot(request(
            Method::DELETE,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let del_resp = body_json(res).await;
    assert_eq!(del_resp["id"], line_id);
    assert_eq!(del_resp["object"], "line-item");
    assert_eq!(del_resp["deleted"], true);
    assert_eq!(del_resp["parent"]["items"].as_array().unwrap().len(), 0);
    assert_eq!(del_resp["parent"]["item_total"], 0);

    // 6. GET cart still works
    let res = app
        .clone()
        .oneshot(request(
            Method::GET,
            &format!("/store/carts/{}", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Add item then update quantity to 0 triggers soft-delete
    let payload = json!({"variant_id": "var_1", "quantity": 1});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    let cart_resp = body_json(res).await;
    let line_id2 = cart_resp["cart"]["items"][0]["id"].as_str().unwrap();

    let payload = json!({"quantity": 0});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id2),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    assert_eq!(cart_resp["cart"]["items"].as_array().unwrap().len(), 0);

    // 8. Add item to non-existent cart → 404
    let payload = json!({"variant_id": "var_1", "quantity": 1});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts/invalid_cart/line-items",
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 9. GET non-existent cart → 404
    let res = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/store/carts/cart_nonexistent",
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 10. Update non-existent cart → 404
    let payload = json!({"email": "nope@test.com"});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts/cart_nonexistent",
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 11. Add item with non-existent variant → 404
    let payload = json!({"variant_id": "var_nonexistent", "quantity": 1});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 12. Add item with quantity 0 → 400
    let payload = json!({"variant_id": "var_1", "quantity": 0});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 13. Complete cart → real order (cart must have an item)
    let payload = json!({"variant_id": "var_1", "quantity": 1});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["cart"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

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
    let complete_resp = body_json(res).await;
    assert_eq!(complete_resp["type"], "order");
    assert_eq!(complete_resp["order"]["display_id"], 1);
}

#[tokio::test]
async fn test_cart_add_same_variant_merges_quantity() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    seed_in_pool(&pool).await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let payload = json!({"variant_id": "var_1", "quantity": 2});
    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();

    let payload = json!({"variant_id": "var_1", "quantity": 3});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    assert_eq!(cart_resp["cart"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(cart_resp["cart"]["items"][0]["quantity"], 5);
    assert_eq!(cart_resp["cart"]["item_total"], 5000);
}

#[tokio::test]
async fn test_cart_item_total_computed() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    seed_in_pool(&pool).await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd"}),
        ))
        .await
        .unwrap();
    let body = body_json(res).await;
    assert_eq!(body["cart"]["item_total"], 0);
    assert_eq!(body["cart"]["total"], 0);
    let cart_id = body["cart"]["id"].as_str().unwrap();

    let payload = json!({"variant_id": "var_1", "quantity": 3});
    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    let body = body_json(res).await;
    assert_eq!(body["cart"]["item_total"], 3000);
    assert_eq!(body["cart"]["total"], 3000);
}

#[tokio::test]
async fn test_cart_update_completed_cart_rejected() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&cart_id)
        .execute(&pool)
        .await
        .unwrap();

    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}", cart_id),
            &json!({"email": "new@test.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = body_json(res).await;
    assert_eq!(body["type"], "conflict");
}

#[tokio::test]
async fn test_cart_add_item_to_completed_cart_rejected() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    seed_in_pool(&pool).await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&cart_id)
        .execute(&pool)
        .await
        .unwrap();

    let payload = json!({"variant_id": "var_1", "quantity": 1});
    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &payload,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_cart_get_response_format() {
    let (app, db) = common::setup_test_app().await;
    let toko_rs::db::AppDb::Sqlite(pool) = db;
    seed_in_pool(&pool).await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .oneshot(request(
            Method::GET,
            &format!("/store/carts/{}", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let cart = &body["cart"];
    assert!(cart["id"].is_string());
    assert!(cart["currency_code"].is_string());
    assert!(cart["items"].is_array());
    assert!(cart["item_total"].is_number());
    assert!(cart["total"].is_number());
    assert!(cart["created_at"].is_string());
    assert!(cart["updated_at"].is_string());
    assert!(cart["completed_at"].is_null());
    assert!(cart["deleted_at"].is_null());
}
