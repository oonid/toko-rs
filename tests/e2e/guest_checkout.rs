use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

#[tokio::test]
async fn test_e2e_guest_checkout_flow() {
    let ctx = setup().await;

    // Step 1: Health check
    let resp = ctx.get("/health").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "connected");

    // Step 2: Browse published products
    let resp = ctx.get("/store/products").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let products = body["products"].as_array().unwrap();
    assert!(products.len() >= 3, "should have at least 3 seed products");

    // Step 3: View single product detail
    let resp = ctx.get("/store/products/prod_seed_kaos_polos").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let product = &body["product"];
    assert_eq!(product["title"], "Kaos Polos");
    assert_eq!(product["status"], "published");
    let variants = product["variants"].as_array().unwrap();
    assert!(variants.len() >= 4, "kaos polos has 4 size variants");
    let options = product["options"].as_array().unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0]["title"], "Ukuran");

    // Step 4: Create cart
    let resp = ctx
        .post_json(
            "/store/carts",
            &json!({"email": "buyer@example.com", "currency_code": "idr"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let cart_id = body["cart"]["id"].as_str().unwrap().to_string();
    assert!(cart_id.starts_with("cart_"));
    assert_eq!(body["cart"]["email"], "buyer@example.com");

    // Step 5: Add item (Kaos Polos M, qty 2)
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
    assert_eq!(items[0]["variant_id"], "var_seed_kaos_m");
    assert_eq!(items[0]["quantity"], 2);
    assert_eq!(items[0]["unit_price"], 75000);

    // Step 6: Add second item (Sneakers 41, qty 1)
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_snkr_41", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let items = body["cart"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(body["cart"]["item_total"], 600000);

    // Step 7: Update quantity (kaos M from 2 → 3)
    let line_id = items[0]["id"].as_str().unwrap();
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!({"quantity": 3}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["cart"]["item_total"], 675000);

    // Step 8: Verify cart totals
    let resp = ctx.get(&format!("/store/carts/{}", cart_id)).await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let cart = &body["cart"];
    assert_eq!(cart["item_total"], 675000);
    assert_eq!(cart["subtotal"], 675000);
    assert_eq!(cart["total"], 675000);
    assert_eq!(cart["tax_total"], 0);
    assert_eq!(cart["discount_total"], 0);
    assert_eq!(cart["shipping_total"], 0);

    // Step 9: Complete cart → order
    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["type"], "order");
    let order = &body["order"];
    assert_eq!(order["display_id"], 1);
    assert_eq!(order["status"], "pending");
    assert_eq!(order["item_total"], 675000);
    assert_eq!(order["payment_status"], "not_paid");
    assert_eq!(order["fulfillment_status"], "not_fulfilled");
    let order_id = order["id"].as_str().unwrap().to_string();
    assert!(order_id.starts_with("order_"));
}
