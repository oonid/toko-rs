use super::models::PaymentRecord;
use crate::db::DbPool;
use crate::db::DbTransaction;
use crate::error::AppError;
use crate::types::generate_entity_id;

#[derive(Clone)]
pub struct PaymentRepository {
    pool: DbPool,
}

impl PaymentRepository {
    pub fn new(pool: DbPool) -> Self {
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
            VALUES ($1, $2, $3, $4, 'pending', 'manual')
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

    pub async fn create_with_tx(
        tx: &mut DbTransaction<'_>,
        order_id: &str,
        amount: i64,
        currency_code: &str,
    ) -> Result<PaymentRecord, AppError> {
        let id = generate_entity_id("pay");
        sqlx::query_as::<_, PaymentRecord>(
            r#"
            INSERT INTO payment_records (id, order_id, amount, currency_code, status, provider)
            VALUES ($1, $2, $3, $4, 'pending', 'manual')
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(order_id)
        .bind(amount)
        .bind(currency_code)
        .fetch_one(&mut **tx)
        .await
        .map_err(AppError::DatabaseError)
    }

    pub async fn find_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<PaymentRecord>, AppError> {
        sqlx::query_as::<_, PaymentRecord>("SELECT * FROM payment_records WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::DatabaseError)
    }

    pub async fn cancel_by_order_id(&self, order_id: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE payment_records SET status = 'canceled', updated_at = CURRENT_TIMESTAMP WHERE order_id = $1 AND status NOT IN ('captured', 'refunded')",
        )
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    pub async fn capture_by_order_id(&self, order_id: &str) -> Result<PaymentRecord, AppError> {
        let result = sqlx::query_as::<_, PaymentRecord>(
            "UPDATE payment_records SET status = 'captured', captured_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE order_id = $1 AND status IN ('pending', 'authorized') RETURNING *",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        result.ok_or_else(|| {
            AppError::InvalidData("Payment cannot be captured".to_string())
        })
    }
}
