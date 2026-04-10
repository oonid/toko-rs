use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

#[tokio::test]
async fn test_e2e_admin_product_crud() {
    let ctx = setup().await;

    // Create draft product
    let resp = ctx
        .post_json(
            "/admin/products",
            &json!({
                "title": "Test Jacket",
                "handle": "test-jacket",
                "description": "A warm jacket"
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let product_id = body["product"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["product"]["status"], "draft");
    assert_eq!(body["product"]["title"], "Test Jacket");

    // List all products (includes drafts)
    let resp = ctx.get("/admin/products").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let products = body["products"].as_array().unwrap();
    assert!(products.len() >= 4, "3 seed + 1 new");

    // Get single product
    let resp = ctx.get(&format!("/admin/products/{}", product_id)).await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["product"]["title"], "Test Jacket");

    // Publish product
    let resp = ctx
        .post_json(
            &format!("/admin/products/{}", product_id),
            &json!({"status": "published"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["product"]["status"], "published");

    // Partial update
    let resp = ctx
        .post_json(
            &format!("/admin/products/{}", product_id),
            &json!({"thumbnail": "https://example.com/jacket.jpg"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(
        body["product"]["thumbnail"],
        "https://example.com/jacket.jpg"
    );

    // Add variant
    let resp = ctx
        .post_json(
            &format!("/admin/products/{}/variants", product_id),
            &json!({"title": "Large", "sku": "JACKET-L", "price": 350000}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let variants = body["product"]["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 1);
    assert_eq!(variants[0]["sku"], "JACKET-L");

    // Soft-delete
    let resp = ctx.delete(&format!("/admin/products/{}", product_id)).await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["id"], product_id);
    assert_eq!(body["deleted"], true);

    // Verify 404 on store GET
    let resp = ctx.get(&format!("/store/products/{}", product_id)).await;
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_e2e_admin_product_with_variants() {
    let ctx = setup().await;

    let resp = ctx
        .post_json(
            "/admin/products",
            &json!({
                "title": "T-Shirt",
                "options": [{"title": "Size", "values": ["S", "M"]}],
                "variants": [
                    {"title": "T-Shirt S", "sku": "TS-S", "price": 50000, "options": {"Size": "S"}},
                    {"title": "T-Shirt M", "sku": "TS-M", "price": 50000, "options": {"Size": "M"}}
                ]
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let product = &body["product"];

    let variants = product["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);

    let v0 = &variants[0];
    assert!(v0["calculated_price"]["calculated_amount"].is_number());
    assert!(v0["calculated_price"]["original_amount"].is_number());

    let options = product["options"].as_array().unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0]["values"].as_array().unwrap().len(), 2);

    // Verify unique option combo constraint
    let resp = ctx
        .post_json(
            "/admin/products",
            &json!({
                "title": "Dup Shirt",
                "options": [{"title": "Color", "values": ["Red"]}],
                "variants": [
                    {"title": "Red 1", "sku": "DUP-R1", "price": 100, "options": {"Color": "Red"}},
                    {"title": "Red 2", "sku": "DUP-R2", "price": 100, "options": {"Color": "Red"}}
                ]
            }),
        )
        .await;
    assert_eq!(resp.status(), 400);
}
