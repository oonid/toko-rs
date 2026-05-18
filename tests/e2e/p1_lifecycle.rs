use super::common::E2eContext;
use serde_json::json;

async fn setup() -> E2eContext {
    super::common::setup_e2e().await
}

/// Full P1 MVP commerce lifecycle following docs/seed-data.md.
///
/// Covers steps 0-12 (guest checkout, auth customer, profile),
/// admin product CRUD (A1-A8), admin order lifecycle (AC4-AC9),
/// and all new endpoints from store-modification.md:
///   GET /admin/orders, GET /admin/orders/{id},
///   phone uniqueness 422, ?phone= admin filter.
#[tokio::test]
async fn test_e2e_p1_full_lifecycle() {
    let ctx = setup().await;

    // ── Step 0: Health check ───────────────────────────────────────────────
    let resp = ctx.get("/health").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "connected");
    assert!(body["version"].is_string());

    // ── Step 1: Browse published products ──────────────────────────────────
    let resp = ctx.get("/store/products").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let products = body["products"].as_array().unwrap();
    assert_eq!(products.len(), 3);
    assert_eq!(body["count"], 3);
    let titles: Vec<&str> = products
        .iter()
        .map(|p| p["title"].as_str().unwrap())
        .collect();
    assert!(titles.contains(&"Kaos Polos"));
    assert!(titles.contains(&"Jeans Slim Fit"));
    assert!(titles.contains(&"Sneakers Classic"));

    // ── Step 2: View single product ────────────────────────────────────────
    let resp = ctx.get("/store/products/prod_seed_kaos_polos").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let p = &body["product"];
    assert_eq!(p["title"], "Kaos Polos");
    assert_eq!(p["status"], "published");
    assert_eq!(p["variants"].as_array().unwrap().len(), 4);
    assert_eq!(p["options"].as_array().unwrap()[0]["title"], "Ukuran");
    let v = &p["variants"].as_array().unwrap()[0];
    assert!(v["calculated_price"]["calculated_amount"].is_number());

    // 404 for nonexistent
    let resp = ctx.get("/store/products/prod_nope").await;
    assert_eq!(resp.status(), 404);

    // ── Step 3: Create cart ────────────────────────────────────────────────
    let resp = ctx
        .post_json(
            "/store/carts",
            &json!({"email": "buyer@example.com", "currency_code": "idr"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let cart = &body["cart"];
    let cart_id = cart["id"].as_str().unwrap().to_string();
    assert!(cart_id.starts_with("cart_"));
    assert_eq!(cart["email"], "buyer@example.com");
    assert_eq!(cart["item_total"], 0);
    assert!(cart["completed_at"].is_null());

    // ── Step 4: Add item (Kaos Polos M, qty 2) ────────────────────────────
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
    assert_eq!(items[0]["unit_price"], 75000);
    let line_id = items[0]["id"].as_str().unwrap().to_string();

    // ── Step 5: Add second item (Sneakers 41, qty 1) ──────────────────────
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": "var_seed_snkr_41", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["cart"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["cart"]["item_total"], 600000); // 2×75k + 450k

    // ── Step 6: Update quantity (kaos M qty 2 → 3) ────────────────────────
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!({"quantity": 3}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["cart"]["item_total"], 675000); // 3×75k + 450k

    // ── C1: GET cart ───────────────────────────────────────────────────────
    let resp = ctx.get(&format!("/store/carts/{}", cart_id)).await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["cart"]["item_total"], 675000);
    assert!(body["cart"]["completed_at"].is_null());

    // ── C2: Update cart email ──────────────────────────────────────────────
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}", cart_id),
            &json!({"email": "updated@example.com"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["cart"]["email"], "updated@example.com");

    // ── Step 7: Complete cart → order ─────────────────────────────────────
    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["type"], "order");
    let order = &body["order"];
    let guest_order_id = order["id"].as_str().unwrap().to_string();
    assert!(guest_order_id.starts_with("order_"));
    assert_eq!(order["display_id"], 1);
    assert_eq!(order["status"], "pending");
    assert_eq!(order["item_total"], 675000);
    assert_eq!(order["payment_status"], "not_paid");
    assert_eq!(order["fulfillment_status"], "not_fulfilled");
    assert_eq!(order["items"].as_array().unwrap().len(), 2);
    assert!(order["summary"].is_object());

    // Completed cart is sealed
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items/{}", cart_id, line_id),
            &json!({"quantity": 5}),
        )
        .await;
    assert_eq!(resp.status(), 400);

    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id), &json!(null))
        .await;
    assert_eq!(resp.status(), 200); // idempotent
    assert_eq!(ctx.body(resp).await["order"]["id"], guest_order_id.as_str());

    // ── Step 8: Register new customer ─────────────────────────────────────
    let resp = ctx
        .post_json(
            "/store/customers",
            &json!({
                "first_name": "Andi",
                "last_name": "Pratama",
                "email": "andi@example.com",
                "phone": "081234509876"
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let new_cus_id = body["customer"]["id"].as_str().unwrap().to_string();
    assert!(new_cus_id.starts_with("cus_"));
    assert_eq!(body["customer"]["email"], "andi@example.com");
    assert_eq!(body["customer"]["has_account"], true);
    assert!(body["customer"]["addresses"].is_array());

    // ── Step 9: Seed customer creates cart + order ─────────────────────────
    let resp = ctx
        .post_json(
            "/store/carts",
            &json!({"customer_id": "cus_seed_budi", "currency_code": "idr"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let cart_id2 = ctx.body(resp).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    ctx.post_json(
        &format!("/store/carts/{}/line-items", cart_id2),
        &json!({"variant_id": "var_seed_jeans_30", "quantity": 1}),
    )
    .await;

    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id2), &json!(null))
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let budi_order_id = body["order"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["order"]["display_id"], 2);
    assert_eq!(body["order"]["item_total"], 250000);

    // ── Step 10: Order history (authenticated) ────────────────────────────
    let resp = ctx
        .get_with_header("/store/orders", "X-Customer-Id", "cus_seed_budi")
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let orders = body["orders"].as_array().unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0]["id"], budi_order_id.as_str());

    // Missing auth → 401
    let resp = ctx.get("/store/orders").await;
    assert_eq!(resp.status(), 401);

    // ── Step 11: Order detail ─────────────────────────────────────────────
    let resp = ctx
        .get_with_header(
            &format!("/store/orders/{}", budi_order_id),
            "X-Customer-Id",
            "cus_seed_budi",
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["order"]["id"], budi_order_id.as_str());
    assert_eq!(body["order"]["display_id"], 2);
    assert_eq!(body["order"]["customer_id"], "cus_seed_budi");
    assert!(body["order"]["summary"].is_object());

    // Wrong customer cannot view
    let resp = ctx
        .get_with_header(
            &format!("/store/orders/{}", budi_order_id),
            "X-Customer-Id",
            &new_cus_id,
        )
        .await;
    assert_eq!(resp.status(), 404);

    // ── Step 12: Profile update ────────────────────────────────────────────
    let resp = ctx
        .post_json_with_header(
            "/store/customers/me",
            &json!({"phone": "0855566677"}),
            "X-Customer-Id",
            "cus_seed_budi",
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["customer"]["phone"], "0855566677");

    // CU3: Update email
    let resp = ctx
        .post_json_with_header(
            "/store/customers/me",
            &json!({"email": "budi.new@example.com"}),
            "X-Customer-Id",
            "cus_seed_budi",
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        ctx.body(resp).await["customer"]["email"],
        "budi.new@example.com"
    );

    // ── A1: Admin create product with images, options, variants ───────────
    let resp = ctx
        .post_json(
            "/admin/products",
            &json!({
                "title": "Hoodie Oversize",
                "description": "Hoodie tebal bahan fleece.",
                "images": [{"url": "https://example.com/hoodie-front.jpg"}],
                "options": [{"title": "Ukuran", "values": ["M", "L", "XL"]}],
                "variants": [
                    {"title": "Hoodie - M", "sku": "HOD-M", "price": 185000, "options": {"Ukuran": "M"}},
                    {"title": "Hoodie - L", "sku": "HOD-L", "price": 185000, "options": {"Ukuran": "L"}}
                ]
            }),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    let hoodie_id = body["product"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["product"]["status"], "draft");
    assert_eq!(body["product"]["variants"].as_array().unwrap().len(), 2);
    assert_eq!(body["product"]["images"].as_array().unwrap().len(), 1);

    // A3: Admin lists products (includes draft)
    let resp = ctx.get("/admin/products").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["count"], 4);

    // Draft not visible on storefront
    let resp = ctx.get("/store/products").await;
    assert_eq!(ctx.body(resp).await["count"], 3);

    // A5: Publish
    let resp = ctx
        .post_json(
            &format!("/admin/products/{}", hoodie_id),
            &json!({"status": "published"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["product"]["status"], "published");

    // Now visible on storefront
    let resp = ctx.get("/store/products").await;
    assert_eq!(ctx.body(resp).await["count"], 4);

    // A7: Add variant
    let resp = ctx
        .post_json(
            &format!("/admin/products/{}/variants", hoodie_id),
            &json!({"title": "Hoodie - XL", "sku": "HOD-XL", "price": 195000, "options": {"Ukuran": "XL"}}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        ctx.body(resp).await["product"]["variants"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    // A8: Delete
    let resp = ctx.delete(&format!("/admin/products/{}", hoodie_id)).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["deleted"], true);

    let resp = ctx.get(&format!("/admin/products/{}", hoodie_id)).await;
    assert_eq!(resp.status(), 404);

    let resp = ctx.get("/store/products").await;
    assert_eq!(ctx.body(resp).await["count"], 3);

    // ── AC1: Admin list customers ─────────────────────────────────────────
    let resp = ctx.get("/admin/customers").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert!(body["count"].as_i64().unwrap() >= 2);

    // Phone filter (new)
    let resp = ctx.get("/admin/customers?phone=081234509876").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["customers"][0]["email"], "andi@example.com");

    // ── AC2: Admin get customer ────────────────────────────────────────────
    let resp = ctx.get("/admin/customers/cus_seed_budi").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["customer"]["id"], "cus_seed_budi");

    let resp = ctx.get("/admin/customers/cus_nope").await;
    assert_eq!(resp.status(), 404);

    // ── AC3: Admin list carts ─────────────────────────────────────────────
    let resp = ctx.get("/admin/carts").await;
    assert_eq!(resp.status(), 200);
    assert!(ctx.body(resp).await["count"].as_i64().unwrap() >= 2);

    // ── Create lifecycle order (for AC6–AC9 pipeline) ─────────────────────
    let resp = ctx
        .post_json("/store/carts", &json!({"currency_code": "idr"}))
        .await;
    let cart_id3 = ctx.body(resp).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    ctx.post_json(
        &format!("/store/carts/{}/line-items", cart_id3),
        &json!({"variant_id": "var_seed_kaos_m", "quantity": 1}),
    )
    .await;
    let resp = ctx
        .post_json(&format!("/store/carts/{}/complete", cart_id3), &json!(null))
        .await;
    let lifecycle_order_id = ctx.body(resp).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // ── AC4: Cancel guest order ────────────────────────────────────────────
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/cancel", guest_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["order"]["status"], "canceled");
    assert!(!body["order"]["canceled_at"].is_null());

    // Cannot cancel twice
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/cancel", guest_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // ── AC8: Capture payment ──────────────────────────────────────────────
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/capture-payment", lifecycle_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["order"]["payment_status"], "captured");
    assert_eq!(body["order"]["summary"]["paid_total"], 75000);
    assert_eq!(body["order"]["summary"]["pending_difference"], 0);

    // Cannot capture twice
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/capture-payment", lifecycle_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // ── AC6: Fulfill ───────────────────────────────────────────────────────
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/fulfill", lifecycle_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        ctx.body(resp).await["order"]["fulfillment_status"],
        "fulfilled"
    );

    // Cannot fulfill twice
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/fulfill", lifecycle_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // ── AC7: Ship ─────────────────────────────────────────────────────────
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/ship", lifecycle_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["order"]["fulfillment_status"], "shipped");
    assert!(!body["order"]["shipped_at"].is_null());

    // ── AC5: Complete budi's order ────────────────────────────────────────
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/complete", budi_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(ctx.body(resp).await["order"]["status"], "completed");

    // Cannot complete twice
    let resp = ctx
        .post_json(
            &format!("/admin/orders/{}/complete", budi_order_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // ── NEW: GET /admin/orders (list all) ─────────────────────────────────
    let resp = ctx.get("/admin/orders").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    // guest_order (canceled) + budi_order (completed) + lifecycle_order (shipped)
    assert_eq!(body["count"], 3);
    assert_eq!(body["orders"].as_array().unwrap().len(), 3);
    assert!(body["orders"][0]["items"].is_array());
    assert_eq!(body["offset"], 0);
    assert!(body["limit"].as_i64().unwrap() > 0);

    // Filter by customer_id
    let resp = ctx.get("/admin/orders?customer_id=cus_seed_budi").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["orders"][0]["id"], budi_order_id.as_str());

    // Filter by status
    let resp = ctx.get("/admin/orders?status=canceled").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["orders"][0]["id"], guest_order_id.as_str());

    // Pagination
    let resp = ctx.get("/admin/orders?limit=2&offset=0").await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["orders"].as_array().unwrap().len(), 2);
    assert_eq!(body["count"], 3);
    assert_eq!(body["limit"], 2);

    // ── NEW: GET /admin/orders/{id} ────────────────────────────────────────
    let resp = ctx
        .get(&format!("/admin/orders/{}", lifecycle_order_id))
        .await;
    assert_eq!(resp.status(), 200);
    let body = ctx.body(resp).await;
    assert_eq!(body["order"]["id"], lifecycle_order_id.as_str());
    assert_eq!(body["order"]["fulfillment_status"], "shipped");
    assert_eq!(body["order"]["payment_status"], "captured");
    assert!(!body["order"]["items"].as_array().unwrap().is_empty());

    // No X-Customer-Id needed (admin endpoint)
    let resp = ctx.get(&format!("/admin/orders/{}", budi_order_id)).await;
    assert_eq!(resp.status(), 200);

    // 404 for nonexistent
    let resp = ctx.get("/admin/orders/order_nope").await;
    assert_eq!(resp.status(), 404);

    // ── NEW: Phone uniqueness 422 ─────────────────────────────────────────
    // Budi's phone is now "0855566677" (updated above); registering with same phone → 422
    let resp = ctx
        .post_json(
            "/store/customers",
            &json!({"email": "dupphone@test.com", "phone": "0855566677"}),
        )
        .await;
    assert_eq!(resp.status(), 422);
    let body = ctx.body(resp).await;
    assert_eq!(body["type"], "duplicate_error");
    assert!(body["message"].as_str().unwrap().contains("phone"));

    // ── Error: duplicate handle ────────────────────────────────────────────
    let resp = ctx
        .post_json(
            "/admin/products",
            &json!({"title": "Another Kaos", "handle": "kaos-polos"}),
        )
        .await;
    assert_eq!(resp.status(), 422);
    assert_eq!(ctx.body(resp).await["type"], "duplicate_error");

    // ── Error: empty cart checkout ─────────────────────────────────────────
    let resp = ctx.post_json("/store/carts", &json!({})).await;
    let empty_cart_id = ctx.body(resp).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/complete", empty_cart_id),
            &json!(null),
        )
        .await;
    assert_eq!(resp.status(), 400);
    assert_eq!(ctx.body(resp).await["type"], "invalid_data");

    // ── Error: duplicate email ─────────────────────────────────────────────
    let resp = ctx
        .post_json(
            "/store/customers",
            &json!({"email": "budi.new@example.com"}),
        )
        .await;
    assert_eq!(resp.status(), 422);
    assert_eq!(ctx.body(resp).await["type"], "duplicate_error");

    // ── Error: nonexistent variant in cart ─────────────────────────────────
    let resp = ctx
        .post_json("/store/carts", &json!({"currency_code": "idr"}))
        .await;
    let tmp_cart = ctx.body(resp).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let resp = ctx
        .post_json(
            &format!("/store/carts/{}/line-items", tmp_cart),
            &json!({"variant_id": "var_nope", "quantity": 1}),
        )
        .await;
    assert_eq!(resp.status(), 404);
}
