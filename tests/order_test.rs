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

async fn create_cart_with_item(app: &axum::Router, pool: &toko_rs::db::DbPool) -> String {
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_1', 'Test Product', 'test', 'published') ON CONFLICT (id) DO NOTHING")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_1', 'prod_1', 'Small', 'TEST-S', 1000) ON CONFLICT (id) DO NOTHING")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_test1', 'Test', 'test@test.com', TRUE) ON CONFLICT (id) DO NOTHING")
        .execute(pool).await.unwrap();

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
    let pool = db.pool.clone();
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

    let payment_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_records WHERE order_id = $1")
            .bind(body["order"]["id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        payment_count.0, 1,
        "payment record must exist in database after order creation"
    );
}

#[tokio::test]
async fn test_complete_already_completed_cart_is_idempotent() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let cart_id = create_cart_with_item(&app, &pool).await;

    let res1 = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let order_id1 = body_json(res1).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res2 = app
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let order_id2 = body_json(res2).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        order_id1, order_id2,
        "idempotent retry should return same order"
    );
}

#[tokio::test]
async fn test_display_id_increments() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();

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
    let pool = db.pool.clone();
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
}

#[tokio::test]
async fn test_get_order_rejects_wrong_customer() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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

    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_other', 'Other', 'other@test.com', TRUE) ON CONFLICT (id) DO NOTHING")
        .execute(&pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/store/orders/{}", order_id))
                .header("X-Customer-Id", "cus_other")
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
    let pool = db.pool.clone();

    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_test1', 'Test', 'test@test.com', TRUE)")
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
    assert_eq!(body["limit"], 50);
    assert_eq!(body["offset"], 0);
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

#[tokio::test]
async fn test_order_and_payment_are_atomic() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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
    let order_id = body["order"]["id"].as_str().unwrap();

    let payment_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_records WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        payment_count.0, 1,
        "payment record must exist after order creation"
    );

    let order_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(order_count.0, 1);
}

#[tokio::test]
async fn test_payment_repo_create_and_find() {
    let (_, db) = common::setup_test_app().await;
    let pool = db.pool.clone();

    let repo = toko_rs::payment::repository::PaymentRepository::new(pool.clone());

    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_pay', 'P', 'p', 'published') ON CONFLICT (id) DO NOTHING")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_pay', 'prod_pay', 'V', 500) ON CONFLICT (id) DO NOTHING")
        .execute(&pool).await.unwrap();

    let cart_repo = toko_rs::cart::repository::CartRepository::new(pool.clone(), "idr".to_string());
    let input = toko_rs::cart::types::CreateCartInput {
        customer_id: None,
        email: None,
        currency_code: Some("idr".to_string()),
        shipping_address: None,
        billing_address: None,
        metadata: None,
    };
    let cart = cart_repo.create_cart(input).await.unwrap();

    sqlx::query("INSERT INTO cart_line_items (id, cart_id, title, quantity, unit_price, variant_id, product_id) VALUES ('cli_pay', $1, 'V', 1, 500, 'var_pay', 'prod_pay')")
        .bind(&cart.cart.id)
        .execute(&pool).await.unwrap();

    let order_repo = toko_rs::order::repository::OrderRepository::new(pool.clone());
    let order = order_repo.create_from_cart(&cart.cart.id).await.unwrap();

    let found = repo.find_by_order_id(&order.order.id).await.unwrap();
    assert!(found.is_some(), "payment should exist from order creation");
    let tx_payment = found.unwrap();
    assert!(tx_payment.id.starts_with("pay_"));
    assert_eq!(tx_payment.order_id, order.order.id);
    assert_eq!(tx_payment.amount, 500);
    assert_eq!(tx_payment.status, "pending");
    assert_eq!(tx_payment.provider, "manual");

    let payment = repo.create(&order.order.id, 1000, "usd").await.unwrap();
    assert!(payment.id.starts_with("pay_"));
    assert_eq!(payment.amount, 1000);
    assert_eq!(payment.currency_code, "usd");

    let not_found = repo.find_by_order_id("order_nonexistent").await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_cart_complete_error_response_type() {
    let success_val = serde_json::to_value(toko_rs::order::types::CartCompleteResponse::success(
        toko_rs::order::models::OrderWithItems::from_items(
            toko_rs::order::models::Order {
                id: "order_test".into(),
                display_id: 1,
                cart_id: None,
                customer_id: None,
                email: None,
                currency_code: "idr".into(),
                status: "pending".into(),
                metadata: None,
                canceled_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                deleted_at: None,
                shipping_address: None,
                billing_address: None,
            },
            vec![],
            "not_paid",
            "not_fulfilled",
        ),
    ))
    .unwrap();
    assert_eq!(success_val["type"], "order");
    assert!(success_val["order"].is_object());
    assert!(success_val.get("cart").is_none());
    assert!(success_val.get("error").is_none());

    let cart = toko_rs::cart::models::CartWithItems::from_items(
        toko_rs::cart::models::Cart {
            id: "cart_test".into(),
            customer_id: None,
            email: None,
            currency_code: "idr".into(),
            metadata: None,
            completed_at: None,
            shipping_address: None,
            billing_address: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
        },
        vec![],
    );
    let error_val = serde_json::to_value(toko_rs::order::types::CartCompleteResponse::error(
        cart,
        "Payment authorization failed",
    ))
    .unwrap();
    assert_eq!(error_val["type"], "cart");
    assert!(error_val["cart"].is_object());
    assert!(error_val["error"].is_object());
    assert_eq!(
        error_val["error"]["message"],
        "Payment authorization failed"
    );
    assert_eq!(error_val["error"]["name"], "unknown_error");
    assert_eq!(error_val["error"]["type"], "invalid_data");
    assert!(error_val.get("order").is_none());
}

#[tokio::test]
async fn test_order_line_item_snapshot_fields_surface_top_level() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, description, status) VALUES ('prod_osnap', 'Order Snap Product', 'order-snap', 'Desc here', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_osnap', 'prod_osnap', 'XL', 'OSNAP-XL', 7500)")
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

    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_osnap", "quantity": 3}),
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
    let item = &body["order"]["items"].as_array().unwrap()[0];
    assert_eq!(item["product_title"], "Order Snap Product");
    assert_eq!(item["variant_title"], "XL");
    assert_eq!(item["variant_sku"], "OSNAP-XL");
    assert_eq!(item["product_handle"], "order-snap");
    assert_eq!(item["product_description"], "Desc here");
}

#[tokio::test]
async fn test_concurrent_cart_completion_is_idempotent() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let cart_id = create_cart_with_item(&app, &pool).await;

    let app1 = app.clone();
    let app2 = app.clone();
    let cart_id1 = cart_id.clone();
    let cart_id2 = cart_id.clone();

    let h1 = tokio::spawn(async move {
        app1.oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id1),
            &json!(null),
        ))
        .await
        .unwrap()
    });
    let h2 = tokio::spawn(async move {
        app2.oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id2),
            &json!(null),
        ))
        .await
        .unwrap()
    });

    let r1 = h1.await.unwrap();
    let r2 = h2.await.unwrap();

    let s1 = r1.status();
    let s2 = r2.status();

    assert!(
        s1 == StatusCode::OK && s2 == StatusCode::OK,
        "both completions should succeed (idempotent), got {} and {}",
        s1,
        s2
    );

    let b1 = body_json(r1).await;
    let b2 = body_json(r2).await;
    assert_eq!(
        b1["order"]["id"], b2["order"]["id"],
        "both should return the same order"
    );

    let order_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM orders WHERE display_id IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        order_count.0, 1,
        "only one order should be created for the cart"
    );
}

#[tokio::test]
async fn test_line_item_per_item_totals() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_tot', 'Totals Product', 'totals', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_tot', 'prod_tot', 'One', 2000)")
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
            &json!({"variant_id": "var_tot", "quantity": 5}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let item = &body["cart"]["items"].as_array().unwrap()[0];
    assert_eq!(item["item_total"], 10000);
    assert_eq!(item["subtotal"], 10000);
    assert_eq!(item["total"], 10000);
    assert_eq!(item["original_total"], 10000);
    assert_eq!(item["tax_total"], 0);
    assert_eq!(item["discount_total"], 0);
}

#[tokio::test]
async fn test_cart_metadata_and_address_copied_to_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_1', 'Test', 'test', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price) VALUES ('var_1', 'prod_1', 'Small', 'T-S', 1000)")
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
            &format!("/store/carts/{}", cart_id),
            &json!({
                "metadata": {"order_note": "gift wrap please"},
                "shipping_address": {"city": "Jakarta", "country_code": "id"},
                "billing_address": {"city": "Bandung", "country_code": "id"},
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_1", "quantity": 2, "metadata": {"engraving": "ABC"}}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let order_id = body["order"]["id"].as_str().unwrap();

    assert_eq!(body["order"]["cart_id"], cart_id);
    assert!(body["order"]["email"].is_null());
    assert_eq!(body["order"]["shipping_address"]["city"], "Jakarta");
    assert_eq!(body["order"]["billing_address"]["city"], "Bandung");

    let order: (Option<sqlx::types::Json<serde_json::Value>>,) =
        sqlx::query_as("SELECT metadata FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let meta = order.0.unwrap().0;
    assert_eq!(meta["order_note"], "gift wrap please");

    let line_item: (Option<sqlx::types::Json<serde_json::Value>>,) =
        sqlx::query_as("SELECT metadata FROM order_line_items WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let li_meta = line_item.0.unwrap().0;
    assert_eq!(li_meta["engraving"], "ABC");

    let order_addr: (
        Option<sqlx::types::Json<serde_json::Value>>,
        Option<sqlx::types::Json<serde_json::Value>>,
    ) = sqlx::query_as("SELECT shipping_address, billing_address FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let ship = order_addr.0.unwrap().0;
    assert_eq!(ship["city"], "Jakarta");
    let bill = order_addr.1.unwrap().0;
    assert_eq!(bill["city"], "Bandung");
}

#[tokio::test]
async fn test_email_propagates_from_cart_to_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_eml', 'Email Test', 'email-test', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_eml', 'prod_eml', 'V', 500)")
        .execute(&pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "idr", "email": "order-email@test.com"}),
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
            &json!({"variant_id": "var_eml", "quantity": 1}),
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
    assert_eq!(body["order"]["email"], "order-email@test.com");
}

#[tokio::test]
async fn test_list_orders_filter_by_id() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();

    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_fid', 'Filter', 'fid@test.com', TRUE)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_fid', 'Filter', 'filter', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_fid', 'prod_fid', 'V', 500)")
        .execute(&pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "idr", "customer_id": "cus_fid"}),
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
            &json!({"variant_id": "var_fid", "quantity": 1}),
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
    let order_id = body_json(res).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/store/orders?id={}", order_id))
                .header("X-Customer-Id", "cus_fid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["orders"][0]["id"], order_id.as_str());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/store/orders?id=nonexistent_id")
                .header("X-Customer-Id", "cus_fid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 0);
    assert_eq!(body["orders"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_orders_filter_by_status() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();

    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_fst', 'Status', 'fst@test.com', TRUE)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('prod_fst', 'Status', 'status-test', 'published')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, price) VALUES ('var_fst', 'prod_fst', 'V', 500)")
        .execute(&pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "idr", "customer_id": "cus_fst"}),
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
            &json!({"variant_id": "var_fst", "quantity": 1}),
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
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/store/orders?status=pending")
                .header("X-Customer-Id", "cus_fst")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["orders"][0]["status"], "pending");

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/store/orders?status=canceled")
                .header("X-Customer-Id", "cus_fst")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 0);
    assert_eq!(body["orders"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_complete_cart_completed_without_order_returns_error() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let cart_id = create_cart_with_item(&app, &pool).await;

    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(&cart_id)
        .execute(&pool)
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
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert_eq!(body["type"], "invalid_data");
}

#[tokio::test]
async fn test_complete_cart_with_pre_existing_order_returns_existing() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let cart_id = create_cart_with_item(&app, &pool).await;

    sqlx::query(
        "INSERT INTO orders (id, display_id, cart_id, currency_code, status) VALUES ('order_preexist', 999, $1, 'idr', 'pending')",
    )
    .bind(&cart_id)
    .execute(&pool)
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
    assert_eq!(body["order"]["id"], "order_preexist");
    assert_eq!(body["order"]["display_id"], 999);
}

async fn create_pending_order(app: &axum::Router, pool: &toko_rs::db::DbPool) -> String {
    let cart_id = create_cart_with_item(app, pool).await;
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

#[tokio::test]
async fn test_admin_cancel_pending_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/cancel", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["order"]["status"], "canceled");
    assert!(body["order"]["canceled_at"].is_string());
}

#[tokio::test]
async fn test_admin_cancel_already_canceled_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/cancel", order_id))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/cancel", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_cancel_completed_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/complete", order_id))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/cancel", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_cancel_payment_status_updated() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/cancel", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let payment: (String,) =
        sqlx::query_as("SELECT status FROM payment_records WHERE order_id = $1")
            .bind(&order_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(payment.0, "canceled");
}

#[tokio::test]
async fn test_admin_complete_pending_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/complete", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["order"]["status"], "completed");
}

#[tokio::test]
async fn test_admin_complete_already_completed_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/complete", order_id))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/complete", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_complete_canceled_order() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let order_id = create_pending_order(&app, &pool).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/cancel", order_id))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/orders/{}/complete", order_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
