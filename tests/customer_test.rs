mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use common::setup_test_app;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_register_customer_success() {
    let (app, _) = setup_test_app().await;
    let payload = json!({
        "first_name": "Budi",
        "last_name": "Santoso",
        "email": "budi@example.com",
        "phone": "+6281234567890"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let c = &body["customer"];
    assert!(c["id"].as_str().unwrap().starts_with("cus_"));
    assert_eq!(c["first_name"], "Budi");
    assert_eq!(c["last_name"], "Santoso");
    assert_eq!(c["email"], "budi@example.com");
    assert_eq!(c["phone"], "+6281234567890");
    assert_eq!(c["has_account"], true);
}

#[tokio::test]
async fn test_register_customer_without_email() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["customer"]["first_name"], "Budi");
    assert!(body["customer"]["email"].is_null());
}

#[tokio::test]
async fn test_get_profile_with_valid_header() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "budi@example.com", "first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    let cus_id = body["customer"]["id"].as_str().unwrap();

    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/store/customers/me")
        .header("X-Customer-Id", cus_id)
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    assert_eq!(body2["customer"]["email"], "budi@example.com");
}

#[tokio::test]
async fn test_get_profile_not_found() {
    let (app, _) = setup_test_app().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/store/customers/me")
        .header("X-Customer-Id", "cus_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_customer_profile() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "budi@example.com", "first_name": "Budi"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    let cus_id = body["customer"]["id"].as_str().unwrap();

    let update = json!({"phone": "+6289876543210"});
    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/store/customers/me")
        .header("content-type", "application/json")
        .header("X-Customer-Id", cus_id)
        .body(Body::from(update.to_string()))
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    assert_eq!(body2["customer"]["phone"], "+6289876543210");
    assert_eq!(body2["customer"]["first_name"], "Budi");
}

#[tokio::test]
async fn test_update_customer_without_header() {
    let (app, _) = setup_test_app().await;
    let update = json!({"phone": "+6289876543210"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers/me")
        .header("content-type", "application/json")
        .body(Body::from(update.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_customer_with_company_name() {
    let (app, _) = setup_test_app().await;
    let payload = json!({
        "email": "biz@example.com",
        "first_name": "Biz",
        "company_name": "PT Toko Maju"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["customer"]["company_name"], "PT Toko Maju");
}

#[tokio::test]
async fn test_update_customer_company_name() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "updco@example.com", "first_name": "Upd"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let cus_id = body_json(resp).await["customer"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let update = json!({"company_name": "CV Berkah Jaya"});
    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/store/customers/me")
        .header("content-type", "application/json")
        .header("X-Customer-Id", &cus_id)
        .body(Body::from(update.to_string()))
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = body_json(resp2).await;
    assert_eq!(body2["customer"]["company_name"], "CV Berkah Jaya");
}

#[tokio::test]
async fn test_customer_company_name_absent_when_not_set() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "nocomp@example.com", "first_name": "No"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["customer"]["company_name"].is_null());
}

#[tokio::test]
async fn test_customer_created_by_field_present() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "cb@example.com", "first_name": "CB"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["customer"]["created_by"].is_null());
}

#[tokio::test]
async fn test_customer_update_email() {
    let (app, _) = setup_test_app().await;
    let payload = json!({"email": "old@example.com", "first_name": "Email"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    let customer_id = body["customer"]["id"].as_str().unwrap();

    let update = json!({"email": "new@example.com"});
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers/me")
        .header("content-type", "application/json")
        .header("X-Customer-Id", customer_id)
        .body(Body::from(update.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["customer"]["email"], "new@example.com");
}

async fn create_test_customer(
    app: &axum::Router,
    email: &str,
    first_name: &str,
    last_name: &str,
    phone: &str,
    company_name: Option<&str>,
) -> String {
    let mut payload = json!({
        "email": email,
        "first_name": first_name,
        "last_name": last_name,
        "phone": phone
    });
    if let Some(cn) = company_name {
        payload["company_name"] = json!(cn);
    }
    let req = Request::builder()
        .method(Method::POST)
        .uri("/store/customers")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    body["customer"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_admin_list_customers() {
    let (app, _) = setup_test_app().await;
    create_test_customer(&app, "list1@example.com", "List1", "Test", "+1111", None).await;
    create_test_customer(&app, "list2@example.com", "List2", "Test", "+2222", None).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["customers"].is_array());
    assert!(body["count"].as_i64().unwrap() >= 2);
    assert_eq!(body["offset"], 0);
    assert!(body["limit"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_admin_list_customers_search_by_q() {
    let (app, _) = setup_test_app().await;
    create_test_customer(
        &app,
        "qsearch@example.com",
        "UniqueQName",
        "SearchQTest",
        "+3333",
        Some("PT QSearch Corp"),
    )
    .await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?q=UniqueQName")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let customers = body["customers"].as_array().unwrap();
    assert!(!customers.is_empty());
    assert_eq!(customers[0]["first_name"], "UniqueQName");
}

#[tokio::test]
async fn test_admin_list_customers_search_q_by_company_name() {
    let (app, _) = setup_test_app().await;
    create_test_customer(
        &app,
        "compq@example.com",
        "CompQ",
        "Test",
        "+4444",
        Some("PT QSearch Corp"),
    )
    .await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?q=QSearch")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let customers = body["customers"].as_array().unwrap();
    assert!(!customers.is_empty());
    assert_eq!(customers[0]["company_name"], "PT QSearch Corp");
}

#[tokio::test]
async fn test_admin_list_customers_filter_by_email() {
    let (app, _) = setup_test_app().await;
    create_test_customer(
        &app,
        "filteremail@example.com",
        "FilterEmail",
        "Test",
        "+5555",
        None,
    )
    .await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?email=filteremail")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let customers = body["customers"].as_array().unwrap();
    assert!(!customers.is_empty());
    assert!(customers[0]["email"]
        .as_str()
        .unwrap()
        .contains("filteremail"));
}

#[tokio::test]
async fn test_admin_list_customers_filter_by_first_name() {
    let (app, _) = setup_test_app().await;
    create_test_customer(
        &app,
        "fnfilter@example.com",
        "FilterFirst",
        "Test",
        "+6666",
        None,
    )
    .await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?first_name=FilterFirst")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let customers = body["customers"].as_array().unwrap();
    assert!(!customers.is_empty());
    assert_eq!(customers[0]["first_name"], "FilterFirst");
}

#[tokio::test]
async fn test_admin_list_customers_filter_by_last_name() {
    let (app, _) = setup_test_app().await;
    create_test_customer(
        &app,
        "lnfilter@example.com",
        "Test",
        "FilterLast",
        "+7777",
        None,
    )
    .await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?last_name=FilterLast")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let customers = body["customers"].as_array().unwrap();
    assert!(!customers.is_empty());
    assert_eq!(customers[0]["last_name"], "FilterLast");
}

#[tokio::test]
async fn test_admin_list_customers_filter_by_has_account() {
    let (app, _) = setup_test_app().await;
    create_test_customer(&app, "ha@example.com", "HasAcct", "Test", "+8888", None).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?has_account=true")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let customers = body["customers"].as_array().unwrap();
    assert!(!customers.is_empty());
    assert_eq!(customers[0]["has_account"], true);
}

#[tokio::test]
async fn test_admin_list_customers_pagination() {
    let (app, _) = setup_test_app().await;
    create_test_customer(&app, "page1@example.com", "Page1", "Test", "+9001", None).await;
    create_test_customer(&app, "page2@example.com", "Page2", "Test", "+9002", None).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers?offset=0&limit=1")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["customers"].as_array().unwrap().len(), 1);
    assert!(body["count"].as_i64().unwrap() >= 2);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["limit"], 1);
}

#[tokio::test]
async fn test_admin_get_customer() {
    let (app, _) = setup_test_app().await;
    let cus_id =
        create_test_customer(&app, "getcus@example.com", "GetCus", "Test", "+1010", None).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri(&format!("/admin/customers/{}", cus_id))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["customer"]["id"], cus_id);
    assert_eq!(body["customer"]["email"], "getcus@example.com");
    assert_eq!(body["customer"]["first_name"], "GetCus");
    assert!(body["customer"]["addresses"].is_array());
    assert!(body["customer"]["default_billing_address_id"].is_null());
    assert!(body["customer"]["default_shipping_address_id"].is_null());
}

#[tokio::test]
async fn test_admin_get_customer_not_found() {
    let (app, _) = setup_test_app().await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/admin/customers/cus_nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert!(body["message"].as_str().unwrap().contains("not found"));
}
