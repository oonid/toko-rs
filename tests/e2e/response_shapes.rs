use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

#[tokio::test]
async fn test_e2e_response_shapes() {
    let ctx = setup().await;

    // --- Product shape ---
    let resp = ctx.get("/store/products/prod_seed_kaos_polos").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let p = &body["product"];

    assert!(p["images"].is_array(), "images must be array");
    assert_eq!(p["images"].as_array().unwrap().len(), 0);
    assert!(p["is_giftcard"].is_boolean(), "is_giftcard must be bool");
    assert_eq!(p["is_giftcard"], false);
    assert!(p["discountable"].is_boolean());
    assert_eq!(p["discountable"], true);

    let variants = p["variants"].as_array().unwrap();
    let v = &variants[0];
    assert!(v["calculated_price"]["calculated_amount"].is_number());
    assert!(v["calculated_price"]["original_amount"].is_number());
    assert!(v["calculated_price"]["is_calculated_price_tax_inclusive"].is_boolean());

    // --- Cart shape ---
    let resp = ctx
        .post_json("/store/carts", &json!({"currency_code": "idr"}))
        .await;
    let body = ctx.body(resp).await;
    let cart = &body["cart"];
    let cart_id = cart["id"].as_str().unwrap().to_string();

    let cart_fields = [
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
    ];
    for field in &cart_fields {
        assert!(cart[*field].is_number(), "cart missing field: {}", field);
    }

    // Add item to check line item shape
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_kaos_m", "quantity": 1}),
        )
        .await;
    let body = ctx.body(resp).await;
    let items = body["cart"]["items"].as_array().unwrap();
    let item = &items[0];
    assert_eq!(item["requires_shipping"], true);
    assert_eq!(item["is_discountable"], true);
    assert_eq!(item["is_tax_inclusive"], false);

    // --- Order shape ---
    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    let body = ctx.body(resp).await;
    let order = &body["order"];

    for field in &cart_fields {
        assert!(order[*field].is_number(), "order missing field: {}", field);
    }
    assert_eq!(order["payment_status"], "not_paid");
    assert_eq!(order["fulfillment_status"], "not_fulfilled");
    assert!(order["fulfillments"].is_array());
    assert_eq!(order["fulfillments"].as_array().unwrap().len(), 0);
    assert!(order["shipping_methods"].is_array());

    let order_items = order["items"].as_array().unwrap();
    assert_eq!(order_items[0]["requires_shipping"], true);
    assert_eq!(order_items[0]["is_discountable"], true);
    assert_eq!(order_items[0]["is_tax_inclusive"], false);

    // --- Customer shape ---
    let resp = ctx
        .post_json(
            "/store/customers",
            &json!({"email": "shape@test.com", "first_name": "Shape"}),
        )
        .await;
    let body = ctx.body(resp).await;
    let customer = &body["customer"];
    let _cus_id = customer["id"].as_str().unwrap().to_string();

    assert!(customer["addresses"].is_array(), "addresses must be array");
    assert_eq!(customer["addresses"].as_array().unwrap().len(), 0);
    assert!(customer["default_billing_address_id"].is_null());
    assert!(customer["default_shipping_address_id"].is_null());

    // --- Error shape ---
    let resp = ctx.get("/store/products/prod_nonexistent").await;
    let body = ctx.body(resp).await;
    assert!(body["code"].is_string(), "error must have code");
    assert!(body["type"].is_string(), "error must have type");
    assert!(body["message"].is_string(), "error must have message");
}
