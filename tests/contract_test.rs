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
//
// Response shapes verified against Medusa vendor models:
//   Product:  vendor/medusa/packages/modules/product/src/models/product.ts
//   Variant:  vendor/medusa/packages/modules/product/src/models/product-variant.ts
//   Cart:     vendor/medusa/packages/modules/cart/src/models/cart.ts
//   LineItem: vendor/medusa/packages/modules/cart/src/models/line-item.ts
//   Customer: vendor/medusa/packages/modules/customer/src/models/customer.ts
//   Address:  vendor/medusa/packages/modules/customer/src/models/address.ts
//   Order:    vendor/medusa/packages/modules/order/src/models/order.ts
//   OAS spec: specs/store.oas.yaml — Error schema
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
            "images",
            "is_giftcard",
            "discountable",
            "collection_id",
            "type_id",
        ],
    );
    assert_eq!(p["images"].as_array().unwrap().len(), 0);
    assert_eq!(p["is_giftcard"], false);
    assert_eq!(p["discountable"], true);
    assert!(p["collection_id"].is_null());
    assert!(p["type_id"].is_null());
    if let Some(variants) = p["variants"].as_array() {
        if !variants.is_empty() {
            let v = &variants[0];
            assert_has_fields(v, &["calculated_price"]);
            assert_has_fields(
                &v["calculated_price"],
                &[
                    "calculated_amount",
                    "original_amount",
                    "is_calculated_price_tax_inclusive",
                    "currency_code",
                ],
            );
            assert!(
                v["calculated_price"]["currency_code"]
                    .as_str()
                    .unwrap()
                    .len()
                    >= 3,
                "currency_code must be a non-empty string"
            );
        }
    }
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
    let pool = db.pool.clone();
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
            "item_subtotal",
            "item_tax_total",
            "total",
            "subtotal",
            "tax_total",
            "discount_total",
            "discount_tax_total",
            "shipping_total",
            "shipping_subtotal",
            "shipping_tax_total",
            "original_total",
            "original_subtotal",
            "original_tax_total",
            "original_item_total",
            "original_item_subtotal",
            "original_item_tax_total",
            "original_shipping_total",
            "original_shipping_subtotal",
            "original_shipping_tax_total",
            "gift_card_total",
            "gift_card_tax_total",
            "credit_line_total",
            "credit_line_subtotal",
            "credit_line_tax_total",
            "discount_subtotal",
            "created_at",
            "updated_at",
        ],
    );
    assert!(
        cart["completed_at"].is_null(),
        "new cart completed_at must be null"
    );
    assert!(
        cart.get("deleted_at").is_none(),
        "cart must not expose deleted_at"
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
        &[
            "id",
            "email",
            "has_account",
            "created_at",
            "updated_at",
            "addresses",
            "default_billing_address_id",
            "default_shipping_address_id",
        ],
    );
    assert!(
        c["addresses"].is_array(),
        "customer.addresses must be an array"
    );
    assert_eq!(
        c["addresses"].as_array().unwrap().len(),
        0,
        "new customer has no addresses"
    );
    assert!(
        c["default_billing_address_id"].is_null(),
        "new customer has no default billing address"
    );
    assert!(
        c["default_shipping_address_id"].is_null(),
        "new customer has no default shipping address"
    );
}

#[tokio::test]
async fn test_contract_order_complete_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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
            "item_subtotal",
            "item_tax_total",
            "total",
            "subtotal",
            "tax_total",
            "discount_total",
            "discount_tax_total",
            "shipping_total",
            "shipping_subtotal",
            "shipping_tax_total",
            "original_total",
            "original_subtotal",
            "original_tax_total",
            "gift_card_total",
            "gift_card_tax_total",
            "credit_line_total",
            "credit_line_subtotal",
            "credit_line_tax_total",
            "discount_subtotal",
            "currency_code",
            "payment_status",
            "fulfillment_status",
            "fulfillments",
            "shipping_methods",
            "created_at",
        ],
    );
    assert_eq!(body["order"]["payment_status"], "not_paid");
    assert_eq!(body["order"]["fulfillment_status"], "not_fulfilled");
    assert!(body["order"]["fulfillments"].is_array());
    assert!(body["order"]["shipping_methods"].is_array());
    assert!(
        body["order"]["cart_id"].is_null() || body["order"]["cart_id"].is_string(),
        "order must have cart_id field"
    );
    let items = body["order"]["items"].as_array().unwrap();
    assert!(!items.is_empty(), "order must have line items");
    let first_item = &items[0];
    assert_has_fields(
        first_item,
        &[
            "thumbnail",
            "is_giftcard",
            "is_discountable",
            "is_tax_inclusive",
            "requires_shipping",
            "compare_at_unit_price",
        ],
    );
    assert!(
        first_item["thumbnail"].is_null() || first_item["thumbnail"].is_string(),
        "line item thumbnail must be null or string"
    );
    assert!(
        first_item["is_giftcard"].is_boolean(),
        "line item is_giftcard must be boolean"
    );
}

#[tokio::test]
async fn test_contract_order_detail_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
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
    sqlx::query(
        "INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_contract', 'C', 'c@test.com', TRUE) ON CONFLICT (id) DO NOTHING",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "idr", "customer_id": "cus_contract"}),
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
            ("X-Customer-Id", "cus_contract"),
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
        &["id", "display_id", "status", "items", "item_total", "total", "summary"],
    );
    assert_has_fields(
        &body["order"]["summary"],
        &["pending_difference", "current_order_total", "original_order_total", "transaction_total", "paid_total", "refunded_total", "accounting_total"],
    );
}

#[tokio::test]
async fn test_contract_order_list_response_shape() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO customers (id, first_name, email, has_account) VALUES ('c1', 'T', 't@t.com', TRUE)")
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
    sqlx::query(
        "INSERT INTO customers (id, first_name, email, has_account) VALUES ('cus_contract', 'C', 'c@test.com', TRUE) ON CONFLICT (id) DO NOTHING",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cart = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/store/carts",
                &json!({"currency_code": "idr", "customer_id": "cus_contract"}),
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
// Ref: vendor/medusa/packages/core/framework/src/http/middlewares/error-handler.ts
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
async fn test_error_400_completed_cart_update() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    let cart = body_json(
        app.clone()
            .oneshot(request(Method::POST, "/store/carts", &json!({})))
            .await
            .unwrap(),
    )
    .await;
    let cart_id = cart["cart"]["id"].as_str().unwrap();
    sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP WHERE id = $1")
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
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_oas_error(
        &body_json(res).await,
        "invalid_data",
        "invalid_request_error",
    );
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

// ============================================================
// 14a — Post-audit business logic correctness tests
// ============================================================

#[tokio::test]
async fn test_product_invalid_status_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": "Bad Status", "status": "banana"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_product_update_validates() {
    let (app, _) = common::setup_test_app().await;
    let created = body_json(
        app.clone()
            .oneshot(request(
                Method::POST,
                "/admin/products",
                &json!({"title": "ValidateUpdate"}),
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
            &json!({"status": "invalid_status_value"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_variant_option_value_not_found_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({
                "title": "OptTest",
                "options": [{"title": "Size", "values": ["S", "M"]}],
                "variants": [{"title": "V1", "price": 100, "options": {"Size": "L"}}]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = body_json(res).await;
    assert!(body["message"].as_str().unwrap().contains("Option value"));
}

#[tokio::test]
async fn test_variant_missing_option_coverage_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({
                "title": "OptCoverage",
                "options": [{"title": "Size", "values": ["S", "M"]}, {"title": "Color", "values": ["Red"]}],
                "variants": [{"title": "V1", "price": 100, "options": {"Size": "S"}}]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert!(body["message"].as_str().unwrap().contains("missing option"));
}

#[tokio::test]
async fn test_variant_duplicate_option_combination_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({
                "title": "OptDup",
                "options": [{"title": "Size", "values": ["S", "M"]}],
                "variants": [
                    {"title": "V1", "price": 100, "options": {"Size": "S"}},
                    {"title": "V2", "price": 200, "options": {"Size": "S"}}
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Duplicate option combination"));
}

// ============================================================
// 14b — Input validation tests
// ============================================================

#[tokio::test]
async fn test_unknown_fields_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd", "unknown_field": "value"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_product_unknown_fields_rejected() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/admin/products",
            &json!({"title": "Test", "nonexistent": true}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_metadata_must_be_object() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"currency_code": "usd", "metadata": "not_an_object"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_limit_capped() {
    let (app, _) = common::setup_test_app().await;
    let res = app
        .oneshot(request(
            Method::GET,
            "/store/products?limit=999999",
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert!(body["limit"].as_i64().unwrap() <= 100);
}

// ============================================================
// 18d — JSON rejection handler tests
// ============================================================

#[tokio::test]
async fn test_malformed_json_returns_consistent_error() {
    let (app, _) = common::setup_test_app().await;
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("{invalid json".to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert_eq!(body["type"], "invalid_data");
    assert!(body["code"].is_string());
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_wrong_json_type_returns_consistent_error() {
    let (app, _) = common::setup_test_app().await;
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({"currency_code": 123}).to_string(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert_eq!(body["type"], "invalid_data");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_missing_content_type_returns_400() {
    let (app, _) = common::setup_test_app().await;
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/store/carts")
        .body(axum::body::Body::from(
            json!({"currency_code": "usd"}).to_string(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_json(res).await;
    assert_eq!(body["type"], "invalid_data");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].as_str().unwrap().contains("Content-Type"));
}
