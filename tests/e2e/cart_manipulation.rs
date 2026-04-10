use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

#[tokio::test]
async fn test_e2e_cart_update_and_delete() {
    let ctx = setup().await;

    // Create cart
    let resp = ctx
        .post_json("/store/carts", &json!({"currency_code": "usd"}))
        .await;
    let body = ctx.body(resp).await;
    let cart_id = body["cart"]["id"].as_str().unwrap().to_string();

    // Add item
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 200);

    // Update cart email
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}", cart_id),
            &json!({"email": "test@example.com"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["cart"]["email"], "test@example.com");

    // Delete line item
    let items = body["cart"]["items"].as_array().unwrap();
    let line_id = items[0]["id"].as_str().unwrap();
    let resp = ctx
        .delete(&format!("/store/carts/{}/line-items/{}", cart_id, line_id))
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert!(body["parent"]["items"].is_array());
    assert_eq!(body["parent"]["items"].as_array().unwrap().len(), 0);

    // Add same variant again (verify it works on empty cart)
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 2}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let items = body["cart"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["quantity"], 2);

    // Verify empty cart completion returns 400
    // First create a separate empty cart
    let resp = ctx.post_json("/store/carts", &json!({})).await;
    let body = ctx.body(resp).await;
    let empty_cart_id = body["cart"]["id"].as_str().unwrap().to_string();

    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/complete", empty_cart_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 400);
    let body = ctx.body(resp).await;
    assert_eq!(body["type"], "invalid_data");
}

#[tokio::test]
async fn test_e2e_cart_completed_guards() {
    let ctx = setup().await;

    // Create cart + add item + complete
    let resp = ctx
        .post_json("/store/carts", &json!({"currency_code": "idr"}))
        .await;
    let body = ctx.body(resp).await;
    let cart_id = body["cart"]["id"].as_str().unwrap().to_string();

    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 200);

    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    assert_eq!(resp.status(), 200);

    // Attempt update cart → 409
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}", cart_id),
            &json!({"email": "nope@test.com"}),
        )
        .await;
    assert_eq!(resp.status(), 409);

    // Attempt add item → 409
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 409);

    // Attempt complete again → 409
    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    assert_eq!(resp.status(), 409);
}
