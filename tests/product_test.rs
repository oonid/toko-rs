mod common;

use axum::{body::Body, http::{Method, Request, StatusCode}};
use tower::ServiceExt; 
use serde_json::json;

#[tokio::test]
async fn test_admin_create_product_success() {
    let (app, _db) = common::setup_test_app().await;

    let payload = json!({
        "title": "Classic T-Shirt",
        "handle": "classic-t-shirt",
        "description": "A comfortable tee",
        "options": [
            {"title": "Size", "values": ["S", "M", "L"]}
        ],
        "variants": [
            {
                "title": "Small",
                "price": 2500,
                "options": {"Size": "S"}
            }
        ]
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    if response.status() != StatusCode::OK {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        panic!("API Error Response: {:?}", String::from_utf8_lossy(&body_bytes));
    }
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_create_product_validation_failure() {
    let (app, _db) = common::setup_test_app().await;

    // Provide valid JSON shape but invalid data (e.g. negative price) to hit our custom validator
    let payload = json!({
        "title": "", // Empty title might fail validate(length(min = 1))
        "handle": "fail",
        "description": "Validation test",
        "options": [],
        "variants": [
            {
                "title": "Small",
                "price": -500, // Negative price fails validation
                "options": {"Size": "S"}
            }
        ]
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/admin/products")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // It should hit AppError::InvalidData -> 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_product_endpoint_stubs() {
    let endpoints = vec![
        (Method::GET, "/admin/products"),
        (Method::GET, "/admin/products/prod_123"),
        (Method::PUT, "/admin/products/prod_123"),
        (Method::DELETE, "/admin/products/prod_123"),
        (Method::POST, "/admin/products/prod_123/variants"),
        (Method::GET, "/store/products"),
        (Method::GET, "/store/products/prod_123"),
    ];

    for (method, uri) in endpoints {
        let (app, _db) = common::setup_test_app().await;
        
        let path = if uri.contains("?") { uri } else { &format!("{}?limit=10&offset=0", uri) };

        let request = Request::builder()
            .method(method.clone())
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "title": "Stub variant",
                "price": 0,
                "options": {}
            }).to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Since these are stubbed, they should eventually hit 501 NOT IMPLEMENTED
        // If axum extractors fail first (e.g. missing body for PUT), it will be 4xx.
        // As long as it is an expected 4xx or 501, it counts as handled.
        let status = response.status();
        assert!(status.is_client_error() || status == StatusCode::NOT_IMPLEMENTED, "Expected handled status for {} {}, got {}", method, uri, status);
    }
}
