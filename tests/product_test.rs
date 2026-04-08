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
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
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
}

#[tokio::test]
async fn test_admin_create_product_validation_failure() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({"title": "", "variants": [{"title": "V", "price": -100}]});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_create_product_duplicate_handle() {
    let (app, _) = common::setup_test_app().await;
    create_sample_product(&app).await;
    let payload = json!({"title": "Other", "handle": "classic-t-shirt"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
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
async fn test_admin_get_product_not_found() {
    let (app, _) = common::setup_test_app().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/products/prod_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_list_products() {
    let (app, _) = common::setup_test_app().await;
    create_sample_product(&app).await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/products?offset=0&limit=10")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["count"].as_i64().unwrap() >= 1);
    assert!(body["products"].as_array().unwrap().len() >= 1);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["limit"], 10);
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
async fn test_admin_update_product_not_found() {
    let (app, _) = common::setup_test_app().await;
    let payload = json!({"title": "Nope"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/admin/products/prod_nonexistent")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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
async fn test_admin_delete_product_not_found() {
    let (app, _) = common::setup_test_app().await;
    let req = Request::builder()
        .method(Method::DELETE)
        .uri("/admin/products/prod_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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
async fn test_error_response_format() {
    let (app, _) = common::setup_test_app().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/products/prod_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert!(body["code"].is_string());
    assert!(body["type"].is_string());
    assert!(body["message"].is_string());
    assert_eq!(body["code"], "invalid_request_error");
    assert_eq!(body["type"], "not_found");
    assert_eq!(body.as_object().unwrap().keys().count(), 3);
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
