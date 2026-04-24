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

async fn seed_in_pool(pool: &toko_rs::db::DbPool) {
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
async fn test_cart_add_item_empty_variant_id_rejected() {
    let (app, db) = common::setup_test_app().await;
    seed_in_pool(&db.pool).await;

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
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "", "quantity": 1}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cart_update_line_item_quantity_zero_rejected() {
    let (app, db) = common::setup_test_app().await;
    seed_in_pool(&db.pool).await;

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
    let body = body_json(res).await;
    let items = body["cart"]["items"].as_array().unwrap();
    let line_id = items[0]["id"].as_str().unwrap();

    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!({"quantity": 0}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cart_full_flow() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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
    assert!(
        item.get("snapshot").is_none(),
        "snapshot must not appear in API responses"
    );
    assert_eq!(item["product_title"], "Test Product");
    assert_eq!(item["variant_title"], "Small");
    assert_eq!(item["variant_sku"], "TEST-S");
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

    // 7. Update quantity to 0 is rejected — use DELETE instead
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
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Delete via DELETE endpoint
    let res = app
        .clone()
        .oneshot(request(
            Method::DELETE,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id2),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cart_resp = body_json(res).await;
    assert_eq!(cart_resp["parent"]["items"].as_array().unwrap().len(), 0);

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
    if res.status() != StatusCode::OK {
        let b = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
        panic!(
            "cart complete returned {}: {:?}",
            v.get("message").unwrap(),
            v
        );
    }
    let complete_resp = body_json(res).await;
    assert_eq!(complete_resp["type"], "order");
    assert_eq!(complete_resp["order"]["display_id"], 1);
}

#[tokio::test]
async fn test_cart_add_same_variant_merges_quantity() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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
    let pool = db.pool.clone();
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
    let pool = db.pool.clone();

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

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = $1")
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
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert_eq!(body["type"], "invalid_data");
}

#[tokio::test]
async fn test_cart_add_item_to_completed_cart_rejected() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = $1")
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
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cart_update_line_item_on_completed_cart_rejected() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_in_pool(&pool).await;

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "usd"}),
            ))
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
                &json!({"variant_id": "var_1", "quantity": 2}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let line_id = add_res["cart"]["items"][0]["id"].as_str().unwrap();

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(cart_id)
        .execute(&pool)
        .await
        .unwrap();

    let res = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!({"quantity": 5}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(res).await["type"], "invalid_data");
}

#[tokio::test]
async fn test_cart_delete_line_item_on_completed_cart_rejected() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_in_pool(&pool).await;

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "usd"}),
            ))
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
                &json!({"variant_id": "var_1", "quantity": 1}),
            ))
            .await
            .unwrap(),
    )
    .await;
    let line_id = add_res["cart"]["items"][0]["id"].as_str().unwrap();

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(cart_id)
        .execute(&pool)
        .await
        .unwrap();

    let res = app
        .oneshot(request(
            Method::DELETE,
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(res).await["type"], "invalid_data");
}

#[tokio::test]
async fn test_cart_get_response_format() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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
    assert!(cart.get("deleted_at").is_none());
}

#[tokio::test]
async fn test_same_variant_different_metadata_creates_separate_items() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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

    let add_a = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 2, "metadata": {"source": "a"}}),
        ))
        .await
        .unwrap();
    assert_eq!(add_a.status(), StatusCode::OK);
    let body_a = body_json(add_a).await;
    assert_eq!(body_a["cart"]["items"].as_array().unwrap().len(), 1);

    let add_b = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 3, "metadata": {"source": "b"}}),
        ))
        .await
        .unwrap();
    assert_eq!(add_b.status(), StatusCode::OK);
    let items = body_json(add_b).await["cart"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        items.len(),
        2,
        "different metadata should create separate line items"
    );
    assert_eq!(items[0]["quantity"], 2);
    assert_eq!(items[1]["quantity"], 3);
}

#[tokio::test]
async fn test_same_variant_same_metadata_merges_quantity() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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

    let _ = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 2, "metadata": {"source": "x"}}),
        ))
        .await
        .unwrap();

    let add2 = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 3, "metadata": {"source": "x"}}),
        ))
        .await
        .unwrap();
    assert_eq!(add2.status(), StatusCode::OK);
    let items = body_json(add2).await["cart"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        items.len(),
        1,
        "same metadata should merge into existing item"
    );
    assert_eq!(items[0]["quantity"], 5);
}

#[tokio::test]
async fn test_same_variant_different_price_creates_separate_items() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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

    let add1 = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 2}),
        ))
        .await
        .unwrap();
    assert_eq!(add1.status(), StatusCode::OK);
    let body1 = body_json(add1).await;
    assert_eq!(body1["cart"]["items"][0]["unit_price"], 1000);

    sqlx::query("UPDATE product_variants SET price = 1500 WHERE id = $1")
        .bind("var_1")
        .execute(&pool)
        .await
        .unwrap();

    let add2 = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 1}),
        ))
        .await
        .unwrap();
    assert_eq!(add2.status(), StatusCode::OK);
    let items = body_json(add2).await["cart"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        items.len(),
        2,
        "different unit_price should create separate line items"
    );
    assert_eq!(items[0]["unit_price"], 1000);
    assert_eq!(items[0]["quantity"], 2);
    assert_eq!(items[1]["unit_price"], 1500);
    assert_eq!(items[1]["quantity"], 1);
}

#[tokio::test]
async fn test_cart_line_item_snapshot_fields_surface_top_level() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, description, status) VALUES ('prod_snap', 'Snap Product', 'snap-product', 'A nice product', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_snap', 'prod_snap', 'Large', 'SNAP-L', 5000)")
        .execute(&pool).await.unwrap();

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
            &json!({"variant_id": "var_snap", "quantity": 1}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let item = &body["cart"]["items"].as_array().unwrap()[0];
    assert_eq!(item["product_title"], "Snap Product");
    assert_eq!(item["variant_title"], "Large");
    assert_eq!(item["variant_sku"], "SNAP-L");
    assert_eq!(item["product_handle"], "snap-product");
    assert_eq!(item["product_description"], "A nice product");
}

#[tokio::test]
async fn test_cart_line_item_product_subtitle_surfaces() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, description, subtitle, status) VALUES ('prod_sub', 'Sub Product', 'sub-product', 'Desc', 'My Subtitle', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_sub', 'prod_sub', 'Small', 'SUB-S', 1000)")
        .execute(&pool).await.unwrap();

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
            &json!({"variant_id": "var_sub", "quantity": 1}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let item = &body["cart"]["items"].as_array().unwrap()[0];
    assert_eq!(item["product_subtitle"], "My Subtitle");
}

#[tokio::test]
async fn test_cart_line_item_discountable_and_shipping_from_product() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, status, is_giftcard, discountable) VALUES ('prod_gc', 'Gift Card', 'gift-card', 'published', TRUE, FALSE)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_gc', 'prod_gc', '$25 Card', 'GC-25', 2500)")
        .execute(&pool).await.unwrap();

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
            &json!({"variant_id": "var_gc", "quantity": 1}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let item = &body["cart"]["items"].as_array().unwrap()[0];
    assert_eq!(item["is_discountable"], false);
    assert_eq!(item["requires_shipping"], false);
}

#[tokio::test]
async fn test_variant_option_values_in_cart_line_item() {
    let (app, _) = common::setup_test_app().await;

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({
                "title": "Shirt",
                "options": [{"title": "Color", "values": ["Red", "Blue"]}, {"title": "Size", "values": ["M"]}],
                "variants": [
                    {"title": "Red M", "price": 3000, "options": {"Color": "Red", "Size": "M"}},
                    {"title": "Blue M", "price": 3000, "options": {"Color": "Blue", "Size": "M"}}
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let product = body_json(res).await["product"].clone();
    let red_m_id = product["variants"].as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap();

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
            &json!({"variant_id": red_m_id, "quantity": 1}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let item = &body["cart"]["items"].as_array().unwrap()[0];

    let opts = item["variant_option_values"].as_object().unwrap();
    assert_eq!(opts["Color"], "Red");
    assert_eq!(opts["Size"], "M");
}

#[tokio::test]
async fn test_concurrent_add_line_item_dedup() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_in_pool(&pool).await;

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

    let app1 = app.clone();
    let app2 = app.clone();
    let cart_id1 = cart_id.clone();
    let cart_id2 = cart_id.clone();

    let h1 = tokio::spawn(async move {
        app1.oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id1),
            &json!({"variant_id": "var_1", "quantity": 1}),
        ))
        .await
        .unwrap()
    });
    let h2 = tokio::spawn(async move {
        app2.oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id2),
            &json!({"variant_id": "var_1", "quantity": 1}),
        ))
        .await
        .unwrap()
    });

    let r1 = h1.await.unwrap();
    let r2 = h2.await.unwrap();

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);

    let item_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM cart_line_items WHERE cart_id = $1 AND deleted_at IS NULL",
    )
    .bind(&cart_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        item_count.0, 1,
        "concurrent add_line_item should produce exactly 1 merged line item, got {}",
        item_count.0
    );

    let body = body_json(
        app.oneshot(request(
            Method::GET,
            &format!("/store/carts/{}", cart_id),
            &json!(null),
        ))
        .await
        .unwrap(),
    )
    .await;
    let item = &body["cart"]["items"].as_array().unwrap()[0];
    assert_eq!(item["quantity"], 2, "merged item should have quantity 2");
}

#[tokio::test]
async fn test_update_nonexistent_line_item_returns_404() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_in_pool(&pool).await;

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
            &format!("/store/carts/{}/line-items/nonexistent_item", cart_id),
            &json!({"quantity": 5}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = body_json(res).await;
    assert_eq!(body["type"], "not_found");
}

#[tokio::test]
async fn test_delete_nonexistent_line_item_returns_404() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    seed_in_pool(&pool).await;

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
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!(
                    "/store/carts/{}/line-items/nonexistent_item",
                    cart_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = body_json(res).await;
    assert_eq!(body["type"], "not_found");
}
