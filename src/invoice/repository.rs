use super::models::InvoiceConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::types::generate_entity_id;

#[derive(Clone)]
pub struct InvoiceRepository {
    pool: DbPool,
}

impl InvoiceRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn get_config(&self) -> Result<InvoiceConfig, AppError> {
        let config = sqlx::query_as::<_, InvoiceConfig>("SELECT * FROM invoice_config LIMIT 1")
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice config not found".into()))?;
        Ok(config)
    }

    pub async fn upsert_config(
        &self,
        company_name: Option<String>,
        company_address: Option<String>,
        company_phone: Option<String>,
        company_email: Option<String>,
        company_logo: Option<String>,
        notes: Option<String>,
    ) -> Result<InvoiceConfig, AppError> {
        let existing = sqlx::query_as::<_, InvoiceConfig>("SELECT * FROM invoice_config LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        match existing {
            Some(mut config) => {
                let company_name = company_name.unwrap_or(config.company_name);
                let company_address = company_address.unwrap_or(config.company_address);
                let company_phone = company_phone.unwrap_or(config.company_phone);
                let company_email = company_email.unwrap_or(config.company_email);
                let company_logo = company_logo.or(config.company_logo);
                let notes = notes.or(config.notes);

                sqlx::query(
                    r#"UPDATE invoice_config
                       SET company_name = $1, company_address = $2, company_phone = $3,
                           company_email = $4, company_logo = $5, notes = $6,
                           updated_at = CURRENT_TIMESTAMP
                       WHERE id = $7"#,
                )
                .bind(&company_name)
                .bind(&company_address)
                .bind(&company_phone)
                .bind(&company_email)
                .bind(&company_logo)
                .bind(&notes)
                .bind(&config.id)
                .execute(&self.pool)
                .await?;

                config.company_name = company_name;
                config.company_address = company_address;
                config.company_phone = company_phone;
                config.company_email = company_email;
                config.company_logo = company_logo;
                config.notes = notes;

                Ok(config)
            }
            None => {
                let id = generate_entity_id("invcfg");
                let config = sqlx::query_as::<_, InvoiceConfig>(
                    r#"INSERT INTO invoice_config (id, company_name, company_address, company_phone, company_email, company_logo, notes)
                       VALUES ($1, $2, $3, $4, $5, $6, $7)
                       RETURNING *"#,
                )
                .bind(&id)
                .bind(company_name.unwrap_or_default())
                .bind(company_address.unwrap_or_default())
                .bind(company_phone.unwrap_or_default())
                .bind(company_email.unwrap_or_default())
                .bind(company_logo)
                .bind(notes)
                .fetch_one(&self.pool)
                .await?;
                Ok(config)
            }
        }
    }
}
