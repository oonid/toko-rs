use std::sync::Arc;
use tokio::task::JoinHandle;

#[allow(dead_code)]
pub struct E2eContext {
    pub base_url: String,
    pub client: reqwest::Client,
    pub pool: sqlx::PgPool,
    pub server: JoinHandle<Result<(), std::io::Error>>,
}

impl E2eContext {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(self.url(path)).send().await.unwrap()
    }

    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_json_with_header(
        &self,
        path: &str,
        body: &serde_json::Value,
        header_name: &str,
        header_value: &str,
    ) -> reqwest::Response {
        self.client
            .post(self.url(path))
            .header(header_name, header_value)
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn get_with_header(
        &self,
        path: &str,
        header_name: &str,
        header_value: &str,
    ) -> reqwest::Response {
        self.client
            .get(self.url(path))
            .header(header_name, header_value)
            .send()
            .await
            .unwrap()
    }

    pub async fn delete(&self, path: &str) -> reqwest::Response {
        self.client.delete(self.url(path)).send().await.unwrap()
    }

    pub async fn body(&self, resp: reqwest::Response) -> serde_json::Value {
        resp.json().await.unwrap()
    }
}

impl Drop for E2eContext {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub async fn setup_e2e() -> E2eContext {
    let db_url = resolve_db_url().await;

    let (app_db, repos) = toko_rs::db::create_db(&db_url, "idr")
        .await
        .expect("Failed to create PG pool");
    toko_rs::db::run_migrations(&app_db)
        .await
        .expect("Failed to run migrations");

    let pool = match &app_db {
        toko_rs::db::AppDb::Postgres(p) => p.clone(),
    };

    clean_all_tables(&pool).await;
    seed(&pool).await;

    let state = toko_rs::AppState {
        db: app_db,
        repos: Arc::new(repos),
    };
    let app = toko_rs::app_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    let client = reqwest::Client::new();

    E2eContext {
        base_url,
        client,
        pool,
        server,
    }
}

async fn resolve_db_url() -> String {
    match std::env::var("E2E_DATABASE_URL").as_deref() {
        Ok("testcontainers://") => start_testcontainers_pg().await,
        Ok(url) => url.to_string(),
        Err(_) => "postgres://postgres:postgres@localhost:5432/toko_e2e".to_string(),
    }
}

async fn start_testcontainers_pg() -> String {
    use testcontainers::runners::AsyncRunner as _;
    use testcontainers_modules::postgres::Postgres;

    let container = Postgres::default()
        .with_user("postgres")
        .with_password("postgres")
        .with_db_name("toko_e2e")
        .start()
        .await
        .expect("Failed to start testcontainers PG");

    let host = container.get_host().await.expect("Failed to get host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");
    format!("postgres://postgres:postgres@{host}:{port}/toko_e2e")
}

async fn clean_all_tables(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM payment_records")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM order_line_items")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM orders")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM cart_line_items")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM carts")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM customer_addresses")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM customers")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_variant_option")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_option_values")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_options")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_variants")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM products")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM idempotency_keys")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE _sequences SET value = 0 WHERE name = 'order_display_id'")
        .execute(pool)
        .await
        .unwrap();
}

async fn seed(pool: &sqlx::PgPool) {
    let app_db = toko_rs::db::AppDb::Postgres(pool.clone());
    toko_rs::seed::run_seed(&app_db).await.expect("Seed failed");
}
