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
            INSERT INTO customers (id, first_name, last_name, email, phone, company_name, has_account, created_by, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, TRUE, NULL, $7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.company_name)
        .bind(metadata_to_json(input.metadata))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if crate::db::is_unique_violation(&e) {
                return AppError::DuplicateError(format!(
                    "Customer with email '{}' already exists",
                    input.email.as_deref().unwrap_or("(none)")
                ));
            }
            AppError::DatabaseError(e)
        })?;

        self.wrap_with_addresses(customer).await
    }

    pub async fn find_by_id(&self, id: &str) -> Result<CustomerWithAddresses, AppError> {
        let customer = sqlx::query_as::<_, Customer>(
            "SELECT * FROM customers WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Customer with id {} was not found", id)))?;

        self.wrap_with_addresses(customer).await
    }

    pub async fn list(
        &self,
        params: &AdminCustomerListParams,
    ) -> Result<(Vec<CustomerWithAddresses>, i64), AppError> {
        let limit = params.capped_limit();

        let q_pattern: Option<String> = params.q.as_ref().map(|q| format!("%{}%", q));
        let email_pattern: Option<String> = params.email.as_ref().map(|v| format!("%{}%", v));
        let first_name_pattern: Option<String> =
            params.first_name.as_ref().map(|v| format!("%{}%", v));
        let last_name_pattern: Option<String> =
            params.last_name.as_ref().map(|v| format!("%{}%", v));
        let has_account_val = params.has_account;

        let mut conditions: Vec<String> = vec!["c.deleted_at IS NULL".to_string()];
        let mut param_idx = 1u32;

        if q_pattern.is_some() {
            conditions.push(format!(
                "(c.first_name ILIKE ${param_idx} OR c.last_name ILIKE ${param_idx} OR c.email ILIKE ${param_idx} OR c.phone ILIKE ${param_idx} OR c.company_name ILIKE ${param_idx})"
            ));
            param_idx += 1;
        }
        if email_pattern.is_some() {
            conditions.push(format!("c.email ILIKE ${param_idx}"));
            param_idx += 1;
        }
        if first_name_pattern.is_some() {
            conditions.push(format!("c.first_name ILIKE ${param_idx}"));
            param_idx += 1;
        }
        if last_name_pattern.is_some() {
            conditions.push(format!("c.last_name ILIKE ${param_idx}"));
            param_idx += 1;
        }
        if has_account_val.is_some() {
            conditions.push(format!("c.has_account = ${param_idx}"));
            param_idx += 1;
        }

        let where_clause = conditions.join(" AND ");
        let off_idx = param_idx;
        let lim_idx = param_idx + 1;

        let count_sql = format!("SELECT COUNT(*) FROM customers c WHERE {}", where_clause);
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref v) = q_pattern {
            count_q = count_q.bind(v.as_str());
        }
        if let Some(ref v) = email_pattern {
            count_q = count_q.bind(v.as_str());
        }
        if let Some(ref v) = first_name_pattern {
            count_q = count_q.bind(v.as_str());
        }
        if let Some(ref v) = last_name_pattern {
            count_q = count_q.bind(v.as_str());
        }
        if let Some(v) = has_account_val {
            count_q = count_q.bind(v);
        }
        let count = count_q.fetch_one(&self.pool).await?;

        let data_sql = format!(
            "SELECT c.* FROM customers c WHERE {} ORDER BY c.created_at DESC OFFSET ${off_idx} LIMIT ${lim_idx}",
            where_clause
        );
        let mut data_q = sqlx::query_as::<_, Customer>(&data_sql);
        if let Some(ref v) = q_pattern {
            data_q = data_q.bind(v.as_str());
        }
        if let Some(ref v) = email_pattern {
            data_q = data_q.bind(v.as_str());
        }
        if let Some(ref v) = first_name_pattern {
            data_q = data_q.bind(v.as_str());
        }
        if let Some(ref v) = last_name_pattern {
            data_q = data_q.bind(v.as_str());
        }
        if let Some(v) = has_account_val {
            data_q = data_q.bind(v);
        }
        let customers = data_q
            .bind(params.offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        let mut wrapped = Vec::with_capacity(customers.len());
        for c in customers {
            wrapped.push(self.wrap_with_addresses(c).await?);
        }

        Ok((wrapped, count))
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
                email = COALESCE($3, email),
                phone = COALESCE($4, phone),
                company_name = COALESCE($5, company_name),
                metadata = COALESCE($6, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $7 AND deleted_at IS NULL
            "#,
        )
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.company_name)
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

    async fn wrap_with_addresses(
        &self,
        customer: Customer,
    ) -> Result<CustomerWithAddresses, AppError> {
        let addresses = self.list_addresses(&customer.id).await?;
        let default_billing_address_id = addresses
            .iter()
            .find(|a| a.is_default_billing)
            .map(|a| a.id.clone());
        let default_shipping_address_id = addresses
            .iter()
            .find(|a| a.is_default_shipping)
            .map(|a| a.id.clone());

        Ok(CustomerWithAddresses {
            customer,
            addresses,
            default_billing_address_id,
            default_shipping_address_id,
        })
    }
}
