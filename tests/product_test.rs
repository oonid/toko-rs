mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn create_sample_product(app: &axum::Router) -> Value {
    let payload = json!({
        "title": "Classic T-Shirt",
        "description": "A comfortable tee",
        "options": [{"title": "Size", "values": ["S", "M", "L"]}],
        "variants": [
            {"title": "Small", "sku": "TS-S", "price": 2500, "options": {"Size": "S"}},
            {"title": "Medium", "sku": "TS-M", "price": 2500, "options": {"Size": "M"}}
        ]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_admin_create_product_success() {
    let (app, _) = common::setup_test_app().await;
    let resp = create_sample_product(&app).await;
    let product = &resp["product"];
    assert!(product["id"].as_str().unwrap().starts_with("prod_"));
    assert_eq!(product["title"], "Classic T-Shirt");
    assert_eq!(product["status"], "draft");
    assert!(product["handle"].as_str().unwrap().contains("classic"));
    assert_eq!(product["options"].as_array().unwrap().len(), 1);
    assert_eq!(product["options"][0]["values"].as_array().unwrap().len(), 3);
    assert_eq!(product["variants"].as_array().unwrap().len(), 2);
    let v0_opts = product["variants"][0]["options"].as_array().unwrap();
    assert_eq!(v0_opts.len(), 1);
    assert_eq!(v0_opts[0]["value"], "S");
    assert!(
        v0_opts[0]["option"].is_object(),
        "variant option must have nested 'option' object"
    );
    assert!(
        v0_opts[0]["option"]["id"].is_string(),
        "variant option.option must have 'id'"
    );
    assert!(
        v0_opts[0]["option"]["title"].is_string(),
        "variant option.option must have 'title'"
    );
    assert_eq!(v0_opts[0]["option"]["title"], "Size");
}

#[tokio::test]
async fn test_admin_get_product() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product"]["id"], id);
    assert_eq!(body["product"]["options"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_soft_delete_cascades_to_children() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", product_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let list_with_deleted_req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products?with_deleted=true"))
        .body(Body::empty())
        .unwrap();
    let list_resp = app.clone().oneshot(list_with_deleted_req).await.unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let body = body_json(list_resp).await;
    let deleted_product = body["products"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == product_id)
        .unwrap();
    assert!(
        deleted_product["deleted_at"].is_string(),
        "admin listing should expose deleted_at"
    );
    assert_eq!(
        deleted_product["variants"].as_array().unwrap().len(),
        0,
        "variants should be cascade-deleted and filtered by load_relations"
    );
    assert_eq!(
        deleted_product["options"].as_array().unwrap().len(),
        0,
        "options should be cascade-deleted and filtered by load_relations"
    );
}

#[tokio::test]
async fn test_soft_delete_does_not_affect_other_products() {
    let (app, _) = common::setup_test_app().await;

    let payload1 = json!({
        "title": "Product A",
        "options": [{"title": "Color", "values": ["Red"]}],
        "variants": [{"title": "Red", "price": 1000, "options": {"Color": "Red"}}]
    });
    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload1.to_string()))
        .unwrap();
    let resp1 = app.clone().oneshot(req1).await.unwrap();
    let product_a = body_json(resp1).await;
    let id_a = product_a["product"]["id"].as_str().unwrap();

    let payload2 = json!({
        "title": "Product B",
        "options": [{"title": "Size", "values": ["M"]}],
        "variants": [{"title": "Medium", "price": 2000, "options": {"Size": "M"}}]
    });
    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload2.to_string()))
        .unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    let product_b = body_json(resp2).await;
    let id_b = product_b["product"]["id"].as_str().unwrap();

    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id_a))
        .body(Body::empty())
        .unwrap();
    let del_resp = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    let get_req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}", id_b))
        .body(Body::empty())
        .unwrap();
    let get_resp = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let body = body_json(get_resp).await;
    assert_eq!(body["product"]["variants"].as_array().unwrap().len(), 1);
    assert_eq!(body["product"]["options"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_admin_list_products_with_deleted() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(del_req).await.unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/products?with_deleted=true")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["count"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn test_admin_list_products_pagination() {
    let (app, _) = common::setup_test_app().await;
    for i in 0..5 {
        let payload = json!({"title": format!("Product {}", i)});
        let req = Request::builder()
            .method(Method::POST)
            .uri("/admin/products")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();
    }
    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/products?offset=0&limit=2")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["products"].as_array().unwrap().len(), 2);
    assert!(body["count"].as_i64().unwrap() >= 5);
    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/admin/products?offset=2&limit=2")
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    let body2 = body_json(resp2).await;
    assert_eq!(body2["products"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_admin_update_product() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let payload = json!({"status": "published", "title": "Updated T-Shirt"});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product"]["status"], "published");
    assert_eq!(body["product"]["title"], "Updated T-Shirt");
}

#[tokio::test]
async fn test_admin_delete_product() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["id"], id);
    assert_eq!(body["object"], "product");
    assert_eq!(body["deleted"], true);
    let req2 = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_add_variant() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let payload = json!({"title": "Large", "sku": "TS-L", "price": 2900, "options": {"Size": "L"}});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let variants = body["product"]["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 3);
    let large = variants.iter().find(|v| v["title"] == "Large").unwrap();
    assert_eq!(large["sku"], "TS-L");
    assert_eq!(large["options"].as_array().unwrap().len(), 1);
    assert_eq!(large["options"][0]["value"], "L");
}

#[tokio::test]
async fn test_admin_add_variant_product_not_found() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({"title": "V", "price": 100});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products/prod_nonexistent/variants")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "not_found");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_admin_add_variant_validation_failure() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let payload = json!({"title": "", "price": -5});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "invalid_data");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_admin_create_product_no_options_no_variants() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({"title": "Simple Product", "description": "No variants"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product"]["title"], "Simple Product");
    assert_eq!(body["product"]["options"].as_array().unwrap().len(), 0);
    assert_eq!(body["product"]["variants"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_store_list_published_only() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .uri("/store/products")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["products"].as_array().unwrap().len(), 0);
    let pub_req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(json!({"status": "published"}).to_string()))
        .unwrap();
    app.clone().oneshot(pub_req).await.unwrap();
    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/store/products")
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    let body2 = body_json(resp2).await;
    assert!(body2["products"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn test_store_get_published_product() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/store/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let pub_req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(json!({"status": "published"}).to_string()))
        .unwrap();
    app.clone().oneshot(pub_req).await.unwrap();
    let req2 = Request::builder()
        .method(Method::GET)
        .uri(&format!("/store/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_store_deleted_product_returns_404() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let pub_req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(json!({"status": "published"}).to_string()))
        .unwrap();
    app.clone().oneshot(pub_req).await.unwrap();
    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(del_req).await.unwrap();
    let store_req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/store/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(store_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_update_product_partial() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let payload = json!({"description": "Updated desc only"});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product"]["description"], "Updated desc only");
    assert_eq!(body["product"]["title"], "Classic T-Shirt");
}

#[tokio::test]
async fn test_admin_create_product_reuse_handle_after_soft_delete() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();

    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let del_resp = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    let payload = json!({"title": "Classic T-Shirt"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["product"]["id"].as_str().unwrap().starts_with("prod_"));
}

#[tokio::test]
async fn test_admin_add_variant_duplicate_sku() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();
    let payload =
        json!({"title": "Dupe SKU", "sku": "TS-S", "price": 2500, "options": {"Size": "L"}});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "duplicate_error");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].as_str().unwrap().contains("TS-S"));
}

#[tokio::test]
async fn test_soft_deleted_variant_excluded_from_product() {
    let (app, db) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][0]["id"].as_str().unwrap();

    sqlx::query("UPDATE product_variants SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(variant_id)
        .execute(&db.pool)
        .await
        .unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}", product_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let variants = body["product"]["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 1, "soft-deleted variant should be excluded");
    assert_ne!(variants[0]["id"], variant_id);
}

#[tokio::test]
async fn test_soft_deleted_option_excluded_from_product() {
    let (app, db) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let option_id = created["product"]["options"][0]["id"].as_str().unwrap();

    sqlx::query("UPDATE product_options SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(option_id)
        .execute(&db.pool)
        .await
        .unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}", product_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let options = body["product"]["options"].as_array().unwrap();
    assert_eq!(options.len(), 0, "soft-deleted option should be excluded");
}

#[tokio::test]
async fn test_soft_delete_variant_cleans_pivot_rows() {
    let (app, db) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][0]["id"].as_str().unwrap();

    let pivot_count_before: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM product_variant_option WHERE variant_id = $1")
            .bind(variant_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(
        pivot_count_before.0 > 0,
        "variant should have pivot rows before deletion"
    );

    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let pivot_count_after: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM product_variant_option WHERE variant_id = $1")
            .bind(variant_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        pivot_count_after.0, 0,
        "pivot rows should be cleaned up after variant soft-delete"
    );
}

#[tokio::test]
async fn test_double_delete_returns_ok() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();

    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp1 = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(
        resp1.status(),
        StatusCode::OK,
        "first delete should succeed"
    );

    let del_req2 = Request::builder()
        .method(Method::DELETE)
        .uri(&format!("/admin/products/{}", id))
        .body(Body::empty())
        .unwrap();
    let resp2 = app.clone().oneshot(del_req2).await.unwrap();
    let status2 = resp2.status();
    let body2 = body_json(resp2).await;
    assert_eq!(
        status2,
        StatusCode::OK,
        "second delete should return 200, got {}: {:?}",
        status2,
        body2
    );
    assert_eq!(body2["id"], id);
    assert_eq!(body2["object"], "product");
    assert_eq!(body2["deleted"], true);
}

#[tokio::test]
async fn test_delete_nonexistent_returns_404() {
    let (app, _) = common::setup_test_app().await;

    let del_req = Request::builder()
        .method(Method::DELETE)
        .uri("/admin/products/prod_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(del_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_list_variants() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}/variants", product_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let variants = body["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);
    assert_eq!(body["count"], 2);
    assert_eq!(body["offset"], 0);
    assert!(body["limit"].is_number());
    assert!(variants[0]["id"].as_str().unwrap().starts_with("variant_"));
    assert!(variants[0]["options"].is_array());
    assert!(variants[0]["calculated_price"].is_object());
}

#[tokio::test]
async fn test_admin_list_variants_pagination() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!(
            "/admin/products/{}/variants?limit=1&offset=0",
            product_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["variants"].as_array().unwrap().len(), 1);
    assert_eq!(body["count"], 2);
    assert_eq!(body["limit"], 1);
}

#[tokio::test]
async fn test_admin_get_variant() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][0]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["variant"]["id"], variant_id);
    assert_eq!(body["variant"]["title"], "Small");
    assert!(body["variant"]["options"].is_array());
    assert!(body["variant"]["calculated_price"].is_object());
}

#[tokio::test]
async fn test_admin_get_variant_not_found() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!(
            "/admin/products/{}/variants/variant_nonexistent",
            product_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "not_found");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_admin_update_variant() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][1]["id"].as_str().unwrap();

    let payload = json!({
        "title": "Medium Updated",
        "price": 3000
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product"]["id"], product_id);
    let updated_variant = body["product"]["variants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == variant_id)
        .unwrap();
    assert_eq!(updated_variant["title"], "Medium Updated");
    assert_eq!(updated_variant["price"], 3000);
}

#[tokio::test]
async fn test_admin_update_variant_negative_price_rejected() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][0]["id"].as_str().unwrap();

    let payload = json!({"price": -500});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_update_variant_sku_uniqueness() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][1]["id"].as_str().unwrap();

    let payload = json!({"sku": "TS-S"});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "duplicate_error");
}

#[tokio::test]
async fn test_admin_delete_variant() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][0]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["id"], variant_id);
    assert_eq!(body["object"], "variant");
    assert_eq!(body["deleted"], true);
    assert!(body["parent"]["id"].is_string());
    assert_eq!(body["parent"]["variants"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_admin_delete_variant_idempotent() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let variant_id = created["product"]["variants"][0]["id"].as_str().unwrap();

    let del_req1 = Request::builder()
        .method(Method::DELETE)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp1 = app.clone().oneshot(del_req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let del_req2 = Request::builder()
        .method(Method::DELETE)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp2 = app.clone().oneshot(del_req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    assert_eq!(body2["id"], variant_id);
    assert_eq!(body2["deleted"], true);
}

#[tokio::test]
async fn test_admin_delete_variant_not_found() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!(
            "/admin/products/{}/variants/variant_nonexistent",
            product_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "not_found");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_add_variant_duplicate_option_combo_against_db() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let payload = json!({
        "title": "Small Duplicate",
        "price": 2500,
        "options": {"Size": "S"}
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", product_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "duplicate_error");
}

#[tokio::test]
async fn test_add_variant_different_option_combo_allowed() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let payload = json!({
        "title": "Large",
        "price": 3000,
        "options": {"Size": "L"}
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", product_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product"]["variants"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_add_variant_missing_option_rejected() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let payload = json!({
        "title": "Large",
        "price": 3000,
        "options": {"Color": "Red"}
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", product_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(
        body["message"].as_str().unwrap().contains("missing option"),
        "expected missing option error, got: {:?}",
        body
    );
}

#[tokio::test]
async fn test_add_variant_no_options_when_product_has_options_rejected() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let payload = json!({
        "title": "Large",
        "price": 3000
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", product_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("must specify options"),
        "expected 'must specify options' error, got: {:?}",
        body
    );
}

#[tokio::test]
async fn test_create_product_variant_missing_option_rejected() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({
        "title": "Shirt",
        "options": [{"title": "Size", "values": ["S", "M"]}],
        "variants": [
            {"title": "Small", "price": 1000, "options": {"Size": "S"}},
            {"title": "Medium", "price": 1000}
        ]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("must specify options"),
        "expected 'must specify options' error, got: {:?}",
        body
    );
}

#[tokio::test]
async fn test_admin_list_variants_product_not_found() {
    let (app, _) = common::setup_test_app().await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/products/prod_nonexistent/variants")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["type"], "not_found");
    assert_eq!(body["code"], "invalid_request_error");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_product_option_and_value_metadata_in_response() {
    let (app, _) = common::setup_test_app().await;
    let body = create_sample_product(&app).await;
    let options = body["product"]["options"].as_array().unwrap();
    assert!(!options.is_empty(), "product should have options");
    let opt = &options[0];
    assert!(
        opt.get("metadata").is_some(),
        "option should have metadata field"
    );
    let values = opt["values"].as_array().unwrap();
    assert!(!values.is_empty(), "option should have values");
    assert!(
        values[0].get("metadata").is_some(),
        "option value should have metadata field"
    );
}

#[tokio::test]
async fn test_create_product_accepts_is_giftcard_and_discountable() {
    let (app, _) = common::setup_test_app().await;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "title": "Gift Card",
                "is_giftcard": true,
                "discountable": false
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert!(body["product"]["is_giftcard"].is_boolean());
    assert!(body["product"]["discountable"].is_boolean());
}

#[tokio::test]
async fn test_update_product_accepts_is_giftcard_and_discountable() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"discountable": false, "subtitle": "Updated"}).to_string(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_add_variant_with_explicit_rank() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/variants", id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "title": "Extra",
                "price": 9999,
                "variant_rank": 42,
                "options": {"Size": "L"}
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let variants = body["product"]["variants"].as_array().unwrap();
    let extra = variants.iter().find(|v| v["title"] == "Extra").unwrap();
    assert_eq!(extra["variant_rank"], 42);
}

#[tokio::test]
async fn test_invalid_order_param_rejected() {
    let (app, db) = common::setup_test_app().await;
    let pool = db.pool.clone();
    sqlx::query("INSERT INTO products (id, title, handle, status) VALUES ('p1', 'Test', 'test', 'published')")
        .execute(&pool).await.unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/products?order=(SELECT+1)")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/admin/products?order=created_at+DESC")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_product_subtitle_persists() {
    let (app, _db) = common::setup_test_app().await;

    let payload = json!({
        "title": "Classic T-Shirt",
        "subtitle": "Premium Cotton Blend",
        "status": "published",
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let product_id = body["product"]["id"].as_str().unwrap();
    assert_eq!(body["product"]["subtitle"], "Premium Cotton Blend");

    let payload = json!({"subtitle": "Updated Subtitle"});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", product_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["product"]["subtitle"], "Updated Subtitle");
}

#[tokio::test]
async fn test_product_is_giftcard_and_discountable_persist() {
    let (app, _) = common::setup_test_app().await;

    let payload = json!({
        "title": "Gift Card Product",
        "is_giftcard": true,
        "discountable": false,
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let product_id = body["product"]["id"].as_str().unwrap();
    assert_eq!(body["product"]["is_giftcard"], true);
    assert_eq!(body["product"]["discountable"], false);

    let payload = json!({"discountable": true});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", product_id))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["product"]["is_giftcard"], true);
    assert_eq!(body["product"]["discountable"], true);
}

#[tokio::test]
async fn test_product_all_status_values() {
    let (app, _) = common::setup_test_app().await;

    for status in &["draft", "proposed", "published", "rejected"] {
        let payload = json!({"title": format!("{} Product", status), "status": *status});
        let req = Request::builder()
            .method(Method::POST)
            .uri("/admin/products")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body["product"]["status"], *status);
    }
}

#[tokio::test]
async fn test_product_bool_string_fields() {
    let (app, _) = common::setup_test_app().await;

    let payload = json!({
        "title": "String Bool Product",
        "is_giftcard": "true",
        "discountable": "false"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["product"]["is_giftcard"], true);
    assert_eq!(body["product"]["discountable"], false);
}

#[tokio::test]
async fn test_variant_thumbnail_crud() {
    let (app, _) = common::setup_test_app().await;

    let payload = json!({
        "title": "Thumbnail Product",
        "options": [{"title": "Size", "values": ["M"]}],
        "variants": [
            {
                "title": "M",
                "price": 1000,
                "options": {"Size": "M"},
                "thumbnail": "https://example.com/m.jpg"
            }
        ]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let product_id = body["product"]["id"].as_str().unwrap();
    let variant_id = body["product"]["variants"][0]["id"].as_str().unwrap();
    assert_eq!(
        body["product"]["variants"][0]["thumbnail"],
        "https://example.com/m.jpg"
    );

    let update_payload = json!({
        "thumbnail": "https://example.com/m-updated.jpg"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!(
            "/admin/products/{}/variants/{}",
            product_id, variant_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(update_payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let updated = body["product"]["variants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == variant_id)
        .unwrap();
    assert_eq!(updated["thumbnail"], "https://example.com/m-updated.jpg");
}

#[tokio::test]
async fn test_admin_list_options() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}/options", product_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let options = body["product_options"].as_array().unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(body["count"], 1);
    assert_eq!(body["offset"], 0);
    assert!(body["limit"].is_number());
    assert!(options[0]["id"].as_str().unwrap().starts_with("opt_"));
    assert_eq!(options[0]["title"], "Size");
    assert!(options[0]["values"].is_array());
    assert_eq!(options[0]["values"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_admin_create_option() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({"title": "Simple Product"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let created = body_json(resp).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    assert_eq!(created["product"]["options"].as_array().unwrap().len(), 0);

    let opt_payload = json!({"title": "Color", "values": ["Red", "Blue"]});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}/options", product_id))
        .header("content-type", "application/json")
        .body(Body::from(opt_payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let options = body["product"]["options"].as_array().unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0]["title"], "Color");
    assert_eq!(options[0]["values"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_admin_get_option() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let option_id = created["product"]["options"][0]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!(
            "/admin/products/{}/options/{}",
            product_id, option_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["product_option"]["id"], option_id);
    assert_eq!(body["product_option"]["title"], "Size");
    assert!(body["product_option"]["values"].is_array());
    assert_eq!(
        body["product_option"]["values"].as_array().unwrap().len(),
        3
    );
}

#[tokio::test]
async fn test_admin_update_option() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let option_id = created["product"]["options"][0]["id"].as_str().unwrap();

    let payload = json!({"title": "Shirt Size"});
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!(
            "/admin/products/{}/options/{}",
            product_id, option_id
        ))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let opt = body["product"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == option_id)
        .unwrap();
    assert_eq!(opt["title"], "Shirt Size");
}

#[tokio::test]
async fn test_admin_delete_option() {
    let (app, _) = common::setup_test_app().await;
    let created = create_sample_product(&app).await;
    let product_id = created["product"]["id"].as_str().unwrap();
    let option_id = created["product"]["options"][0]["id"].as_str().unwrap();

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(&format!(
            "/admin/products/{}/options/{}",
            product_id, option_id
        ))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["id"], option_id);
    assert_eq!(body["object"], "product_option");
    assert_eq!(body["deleted"], true);
    assert!(body["parent"]["id"].is_string());

    let get_req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/products/{}", product_id))
        .body(Body::empty())
        .unwrap();
    let get_resp = app.oneshot(get_req).await.unwrap();
    let get_body = body_json(get_resp).await;
    assert_eq!(get_body["product"]["options"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_product_images_on_create() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({
        "title": "Image Product",
        "images": [
            "https://example.com/img1.jpg",
            "https://example.com/img2.jpg"
        ]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let images = body["product"]["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    assert!(images[0]["id"].as_str().unwrap().starts_with("img_"));
    assert_eq!(images[0]["url"], "https://example.com/img1.jpg");
    assert_eq!(images[0]["rank"], 0);
    assert!(images[1]["id"].as_str().unwrap().starts_with("img_"));
    assert_eq!(images[1]["url"], "https://example.com/img2.jpg");
    assert_eq!(images[1]["rank"], 1);
}

#[tokio::test]
async fn test_product_images_on_update() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({"title": "No Images Yet"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let created = body_json(resp).await;
    let id = created["product"]["id"].as_str().unwrap();
    assert_eq!(created["product"]["images"].as_array().unwrap().len(), 0);

    let update_payload = json!({
        "images": ["https://example.com/updated1.jpg", "https://example.com/updated2.jpg"]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(update_payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let images = body["product"]["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0]["url"], "https://example.com/updated1.jpg");
    assert_eq!(images[1]["url"], "https://example.com/updated2.jpg");
}

#[tokio::test]
async fn test_product_images_replace_on_update() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({
        "title": "Replace Images",
        "images": ["https://example.com/old1.jpg", "https://example.com/old2.jpg"]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let created = body_json(resp).await;
    let id = created["product"]["id"].as_str().unwrap();
    let original_images = created["product"]["images"].as_array().unwrap();
    assert_eq!(original_images.len(), 2);
    let old_id = original_images[0]["id"].as_str().unwrap();

    let update_payload = json!({
        "images": ["https://example.com/new1.jpg"]
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri(&format!("/admin/products/{}", id))
        .header("content-type", "application/json")
        .body(Body::from(update_payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let images = body["product"]["images"].as_array().unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0]["url"], "https://example.com/new1.jpg");
    assert_ne!(images[0]["id"].as_str().unwrap(), old_id);
}
