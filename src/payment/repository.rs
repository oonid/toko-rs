use super::models::PaymentRecord;
use crate::error::AppError;
use crate::types::generate_entity_id;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct PaymentRepository {
    pool: SqlitePool,
}

impl PaymentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        order_id: &str,
        amount: i64,
        currency_code: &str,
    ) -> Result<PaymentRecord, AppError> {
        let id = generate_entity_id("pay");
        sqlx::query_as::<_, PaymentRecord>(
            r#"
            INSERT INTO payment_records (id, order_id, amount, currency_code, status, provider)
            VALUES (?, ?, ?, ?, 'pending', 'manual')
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(order_id)
        .bind(amount)
        .bind(currency_code)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn find_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<PaymentRecord>, AppError> {
        sqlx::query_as::<_, PaymentRecord>("SELECT * FROM payment_records WHERE order_id = ?")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::DatabaseError)
    }
}
