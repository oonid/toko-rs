use crate::config::InvoiceConfig;

#[derive(Clone)]
pub struct InvoiceRepository {
    pub config: InvoiceConfig,
}

impl InvoiceRepository {
    pub fn new(config: InvoiceConfig) -> Self {
        Self { config }
    }
}
