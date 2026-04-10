use super::models::{Customer, CustomerAddress};
use super::types::*;
use crate::db::DbPool;
use crate::error::AppError;
use crate::types::{generate_entity_id, metadata_to_json};

#[derive(Clone)]
pub struct CustomerRepository {
    pool: DbPool,
}

impl CustomerRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        input: CreateCustomerInput,
    ) -> Result<CustomerWithAddresses, AppError> {
        let id = generate_entity_id("cus");
        let customer = sqlx::query_as::<_, Customer>(
            r#"
            INSERT INTO customers (id, first_name, last_name, email, phone, has_account, metadata)
            VALUES ($1, $2, $3, $4, $5, TRUE, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(metadata_to_json(input.metadata))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if crate::db::is_unique_violation(&e) {
                return AppError::DuplicateError(format!(
                    "Customer with email '{}' already exists",
                    input.email
                ));
            }
            AppError::DatabaseError(e)
        })?;

        Ok(self.wrap_with_addresses(customer).await)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<CustomerWithAddresses, AppError> {
        let customer = sqlx::query_as::<_, Customer>(
            "SELECT * FROM customers WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Customer with id {} was not found", id)))?;

        Ok(self.wrap_with_addresses(customer).await)
    }

    pub async fn update(
        &self,
        id: &str,
        input: &UpdateCustomerInput,
    ) -> Result<CustomerWithAddresses, AppError> {
        let _existing = self.find_by_id(id).await?;

        sqlx::query(
            r#"
            UPDATE customers SET
                first_name = COALESCE($1, first_name),
                last_name = COALESCE($2, last_name),
                phone = COALESCE($3, phone),
                metadata = COALESCE($4, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $5 AND deleted_at IS NULL
            "#,
        )
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.phone)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id).await
    }

    pub async fn list_addresses(
        &self,
        customer_id: &str,
    ) -> Result<Vec<CustomerAddress>, AppError> {
        let addresses = sqlx::query_as::<_, CustomerAddress>(
            "SELECT * FROM customer_addresses WHERE customer_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC",
        )
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(addresses)
    }

    async fn wrap_with_addresses(&self, customer: Customer) -> CustomerWithAddresses {
        let addresses = self.list_addresses(&customer.id).await.unwrap_or_default();
        let default_billing_address_id = addresses
            .iter()
            .find(|a| a.is_default_billing)
            .map(|a| a.id.clone());
        let default_shipping_address_id = addresses
            .iter()
            .find(|a| a.is_default_shipping)
            .map(|a| a.id.clone());

        CustomerWithAddresses {
            customer,
            addresses,
            default_billing_address_id,
            default_shipping_address_id,
        }
    }
}
