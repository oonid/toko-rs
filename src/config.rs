use serde::Deserialize;

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_rust_log() -> String {
    "toko_rs=debug,tower_http=debug".to_string()
}

fn default_currency_code() -> String {
    "idr".to_string()
}

fn default_cors_origins() -> String {
    "*".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_rust_log")]
    pub rust_log: String,
    #[serde(default = "default_currency_code")]
    pub default_currency_code: String,
    #[serde(default = "default_cors_origins")]
    pub cors_origins: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, envy::Error> {
        dotenvy::dotenv().ok();
        envy::from_env::<AppConfig>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_load_with_env_vars() {
        let orig = std::env::var("DATABASE_URL").ok();
        std::env::set_var("DATABASE_URL", "sqlite::memory:");
        let orig_host = std::env::var("HOST").ok();
        std::env::set_var("HOST", "0.0.0.0");
        let orig_port = std::env::var("PORT").ok();
        std::env::set_var("PORT", "4242");
        let orig_log = std::env::var("RUST_LOG").ok();
        std::env::set_var("RUST_LOG", "debug");

        let orig_cc = std::env::var("DEFAULT_CURRENCY_CODE").ok();
        std::env::remove_var("DEFAULT_CURRENCY_CODE");

        let config = AppConfig::load().unwrap();
        assert_eq!(config.database_url, "sqlite::memory:");
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 4242);
        assert_eq!(config.rust_log, "debug");
        assert_eq!(config.default_currency_code, "idr");

        match orig_cc {
            Some(v) => std::env::set_var("DEFAULT_CURRENCY_CODE", v),
            None => std::env::remove_var("DEFAULT_CURRENCY_CODE"),
        }

        match orig {
            Some(v) => std::env::set_var("DATABASE_URL", v),
            None => std::env::remove_var("DATABASE_URL"),
        }
        match orig_host {
            Some(v) => std::env::set_var("HOST", v),
            None => std::env::remove_var("HOST"),
        }
        match orig_port {
            Some(v) => std::env::set_var("PORT", v),
            None => std::env::remove_var("PORT"),
        }
        match orig_log {
            Some(v) => std::env::set_var("RUST_LOG", v),
            None => std::env::remove_var("RUST_LOG"),
        }
    }

    #[test]
    #[serial]
    fn test_defaults_when_not_set() {
        let orig_db = std::env::var("DATABASE_URL").ok();
        let orig_host = std::env::var("HOST").ok();
        let orig_port = std::env::var("PORT").ok();
        let orig_log = std::env::var("RUST_LOG").ok();
        let orig_cors = std::env::var("CORS_ORIGINS").ok();

        std::env::set_var("DATABASE_URL", "sqlite::memory:");
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
        std::env::set_var("RUST_LOG", "toko_rs=debug,tower_http=debug");
        std::env::remove_var("DEFAULT_CURRENCY_CODE");
        std::env::remove_var("CORS_ORIGINS");

        let config = AppConfig::load().unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.default_currency_code, "idr");
        assert_eq!(config.rust_log, "toko_rs=debug,tower_http=debug");
        assert_eq!(config.cors_origins, "*");

        match orig_db {
            Some(v) => std::env::set_var("DATABASE_URL", v),
            None => std::env::remove_var("DATABASE_URL"),
        }
        match orig_host {
            Some(v) => std::env::set_var("HOST", v),
            None => std::env::remove_var("HOST"),
        }
        match orig_port {
            Some(v) => std::env::set_var("PORT", v),
            None => std::env::remove_var("PORT"),
        }
        match orig_log {
            Some(v) => std::env::set_var("RUST_LOG", v),
            None => std::env::remove_var("RUST_LOG"),
        }
        match orig_cors {
            Some(v) => std::env::set_var("CORS_ORIGINS", v),
            None => std::env::remove_var("CORS_ORIGINS"),
        }
    }

    #[test]
    #[serial]
    fn test_cors_origins_from_env() {
        let orig_db = std::env::var("DATABASE_URL").ok();
        let orig_cors = std::env::var("CORS_ORIGINS").ok();

        std::env::set_var("DATABASE_URL", "sqlite::memory:");
        std::env::set_var("CORS_ORIGINS", "http://localhost:3000");

        let config = AppConfig::load().unwrap();
        assert_eq!(config.cors_origins, "http://localhost:3000");

        match orig_db {
            Some(v) => std::env::set_var("DATABASE_URL", v),
            None => std::env::remove_var("DATABASE_URL"),
        }
        match orig_cors {
            Some(v) => std::env::set_var("CORS_ORIGINS", v),
            None => std::env::remove_var("CORS_ORIGINS"),
        }
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_host(), "0.0.0.0");
        assert_eq!(default_port(), 3000);
        assert_eq!(default_rust_log(), "toko_rs=debug,tower_http=debug");
        assert_eq!(default_currency_code(), "idr");
        assert_eq!(default_cors_origins(), "*");
    }
}
