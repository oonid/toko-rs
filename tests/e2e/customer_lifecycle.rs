use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

#[tokio::test]
async fn test_e2e_customer_lifecycle() {
    let ctx = setup().await;

    // Step 10: Register customer
    let resp = ctx
        .post_json(
            "/store/customers",
            &json!({
                "first_name": "Budi",
                "last_name": "Santoso",
                "email": "budi2@example.com",
                "phone": "+6281234567890"
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let customer = &body["customer"];
    let cus_id = customer["id"].as_str().unwrap().to_string();
    assert!(cus_id.starts_with("cus_"));
    assert_eq!(customer["email"], "budi2@example.com");
    assert_eq!(customer["first_name"], "Budi");

    // Step 11: Get profile with auth header
    let resp = ctx
        .get_with_header("/store/customers/me", "X-Customer-Id", &cus_id)
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["customer"]["email"], "budi2@example.com");
    assert!(body["customer"]["addresses"].is_array());

    // Step 12: Update profile
    let resp = ctx
        .post_json_with_header(
            "/store/customers/me",
            &json!({"first_name": "Budiman"}),
            "X-Customer-Id",
            &cus_id,
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["customer"]["first_name"], "Budiman");

    // Step 13: Create cart with customer_id
    let resp = ctx
        .post_json(
            "/store/carts",
            &json!({"customer_id": cus_id, "currency_code": "idr"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let cart_id = body["cart"]["id"].as_str().unwrap().to_string();

    // Step 14: Add item
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 200);

    // Step 15: Complete as authenticated customer
    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let order_id = body["order"]["id"].as_str().unwrap().to_string();

    // Step 16: List orders with auth
    let resp = ctx
        .get_with_header("/store/orders", "X-Customer-Id", &cus_id)
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let orders = body["orders"].as_array().unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0]["id"], order_id);

    // Step 17: View order detail with auth
    let resp = ctx
        .get_with_header(
            &format!("/store/orders/{}", order_id),
            "X-Customer-Id",
            &cus_id,
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["order"]["id"], order_id);
    assert_eq!(body["order"]["display_id"], 1);
    assert_eq!(body["order"]["customer_id"], cus_id);
}
