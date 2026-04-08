use super::models::Customer;
use super::types::*;
use crate::error::AppError;
use crate::types::generate_entity_id;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct CustomerRepository {
    pool: SqlitePool,
}

impl CustomerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, input: CreateCustomerInput) -> Result<Customer, AppError> {
        let id = generate_entity_id("cus");
        let customer = sqlx::query_as::<_, Customer>(
            r#"
            INSERT INTO customers (id, first_name, last_name, email, phone, has_account, metadata)
            VALUES (?, ?, ?, ?, ?, TRUE, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(input.metadata.map(sqlx::types::Json))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.message().contains("UNIQUE") {
                    return AppError::DuplicateError(format!(
                        "Customer with email '{}' already exists",
                        input.email
                    ));
                }
            }
            AppError::DatabaseError(e)
        })?;

        Ok(customer)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Customer, AppError> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Customer with id {} was not found", id)))
    }

    pub async fn update(
        &self,
        id: &str,
        input: &UpdateCustomerInput,
    ) -> Result<Customer, AppError> {
        let _existing = self.find_by_id(id).await?;

        sqlx::query(
            r#"
            UPDATE customers SET
                first_name = COALESCE(?, first_name),
                last_name = COALESCE(?, last_name),
                phone = COALESCE(?, phone),
                metadata = COALESCE(?, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.phone)
        .bind(input.metadata.clone().map(sqlx::types::Json))
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id).await
    }
}
