use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

#[tokio::test]
async fn test_e2e_error_responses() {
    let ctx = setup().await;

    // 404 nonexistent cart
    let resp = ctx.get("/store/carts/cart_nonexistent").await;
    assert_eq!(resp.status(), 404);
    let body = ctx.body(resp).await;
    assert_eq!(body["code"], "invalid_request_error");
    assert_eq!(body["type"], "not_found");
    assert!(body["message"].is_string());

    // 404 nonexistent product
    let resp = ctx.get("/store/products/prod_nonexistent").await;
    assert_eq!(resp.status(), 404);

    // 404 nonexistent order
    let resp = ctx
        .get_with_header("/store/orders/order_nonexistent", "X-Customer-Id", "cus_x")
        .await;
    assert_eq!(resp.status(), 404);

    // 422 duplicate email
    let resp = ctx
        .post_json(
            "/store/customers",
            &json!({"email": "budi@example.com", "first_name": "Dup"}),
        )
        .await;
    assert_eq!(resp.status(), 422);
    let body = ctx.body(resp).await;
    assert_eq!(body["type"], "duplicate_error");

    // 400 invalid quantity (0)
    let resp = ctx.post_json("/store/carts", &json!({})).await;
    let cart_id = ctx.body(resp).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 0}),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // 401 missing X-Customer-Id on protected endpoint
    let resp = ctx.get("/store/orders").await;
    assert_eq!(resp.status(), 401);

    // 400 unknown fields
    let resp = ctx
        .post_json("/store/carts", &json!({"totally_wrong_field": true}))
        .await;
    assert_eq!(resp.status(), 400);

    // 400 invalid product status
    let resp = ctx
        .post_json(
            "/admin/products",
            &json!({"title": "Bad Status", "status": "banana"}),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // 400 string metadata (must be object)
    let resp = ctx
        .post_json("/store/carts", &json!({"metadata": "not_an_object"}))
        .await;
    assert_eq!(resp.status(), 400);
}
