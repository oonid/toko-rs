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

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_rust_log")]
    pub rust_log: String,
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

        let config = AppConfig::load().unwrap();
        assert_eq!(config.database_url, "sqlite::memory:");
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 4242);
        assert_eq!(config.rust_log, "debug");

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

        std::env::set_var("DATABASE_URL", "sqlite::memory:");
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
        std::env::remove_var("RUST_LOG");
        dotenvy::dotenv().ok();

        let config = AppConfig::load().unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);

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
    }
}
