mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

fn request(method: Method, uri: &str, payload: &serde_json::Value) -> Request<Body> {
    let is_body = method == Method::POST || method == Method::PUT || method == Method::PATCH;
    let mut builder = Request::builder().method(method).uri(uri);
    if is_body {
        builder = builder.header("content-type", "application/json");
        builder.body(Body::from(payload.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    }
}

async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn setup_seeded_app() -> (axum::Router, toko_rs::db::AppDb) {
    let (app, db) = common::setup_test_app().await;
    toko_rs::seed::run_seed(&db).await.unwrap();
    (app, db)
}

#[tokio::test]
async fn test_full_browse_cart_checkout_flow() {
    let (app, _db) = setup_seeded_app().await;

    let res = app
        .clone()
        .oneshot(request(Method::GET, "/store/products", &json!(null)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let products = body["products"].as_array().unwrap();
    assert_eq!(products.len(), 3, "should list 3 seed products");
    assert_eq!(body["count"], 3);
    for p in products {
        assert_eq!(p["status"], "published");
    }

    let res = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/store/products/prod_seed_kaos_polos",
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let variants = body["product"]["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 4, "kaos polos should have 4 size variants");
    let variant_m = variants
        .iter()
        .find(|v| v["title"] == "Kaos Polos - M")
        .unwrap();
    let variant_m_id = variant_m["id"].as_str().unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"email": "buyer@example.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let cart_id = body["cart"]["id"].as_str().unwrap();
    assert_eq!(body["cart"]["email"], "buyer@example.com");
    assert_eq!(body["cart"]["items"].as_array().unwrap().len(), 0);
    assert_eq!(body["cart"]["item_total"], 0);

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": variant_m_id, "quantity": 2}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let items = body["cart"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["variant_id"], variant_m_id);
    assert_eq!(items[0]["quantity"], 2);
    assert_eq!(items[0]["unit_price"], 75000);
    assert_eq!(body["cart"]["item_total"], 150000);
    assert_eq!(body["cart"]["total"], 150000);

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["type"], "order");
    assert!(body["order"]["id"].as_str().unwrap().starts_with("order_"));
    assert_eq!(body["order"]["display_id"], 1);
    assert_eq!(body["order"]["status"], "pending");
    assert_eq!(body["order"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["order"]["items"][0]["quantity"], 2);
    assert_eq!(body["order"]["items"][0]["unit_price"], 75000);
    assert_eq!(body["order"]["item_total"], 150000);
    assert_eq!(body["order"]["total"], 150000);
    assert_eq!(body["payment"]["status"], "pending");
    assert_eq!(body["payment"]["amount"], 150000);
    assert_eq!(body["payment"]["currency_code"], "idr");
}

#[tokio::test]
async fn test_customer_browse_order_history_flow() {
    let (app, _db) = setup_seeded_app().await;

    let res = app
        .clone()
        .oneshot(request(Method::GET, "/store/products", &json!(null)))
        .await
        .unwrap();
    let body = body_json(res).await;
    let sneaker_variants = body["products"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "prod_seed_sneakers")
        .unwrap()["variants"]
        .as_array()
        .unwrap();
    let var_41 = sneaker_variants
        .iter()
        .find(|v| v["title"] == "Sneakers - 41")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/store/carts",
            &json!({"customer_id": "cus_seed_budi", "currency_code": "idr"}),
        ))
        .await
        .unwrap();
    let cart_id = body_json(res).await["cart"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    app.clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/line-items", cart_id),
            &json!({"variant_id": var_41, "quantity": 1}),
        ))
        .await
        .unwrap();

    let res = app
        .clone()
        .oneshot(request(
            Method::POST,
            &format!("/store/carts/{}/complete", cart_id),
            &json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let order_id = body_json(res).await["order"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/store/orders")
                .header("X-Customer-Id", "cus_seed_budi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 1);
    assert_eq!(body["orders"].as_array().unwrap().len(), 1);
    assert_eq!(body["orders"][0]["id"], order_id);

    let res = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/store/orders/{}", order_id))
                .header("X-Customer-Id", "cus_seed_budi")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["order"]["id"], order_id);
    assert_eq!(body["order"]["items"][0]["unit_price"], 450000);
    assert_eq!(body["payment"]["amount"], 450000);
}
