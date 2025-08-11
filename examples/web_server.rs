//! Example web server that uses environment variables
//!
//! This example demonstrates how auto-env-generator can detect environment
//! variables used in a typical Rust web application.

use std::env;

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: Option<String>,
    pub jwt_secret: String,
    pub api_key: String,
    pub port: u16,
    pub host: String,
    pub log_level: String,
    pub cors_origin: String,
    pub session_secret: String,
    pub smtp_server: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Config {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),

            redis_url: env::var("REDIS_URL").ok(),

            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),

            api_key: env::var("API_KEY").expect("API_KEY must be set"),

            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,

            host: env::var("HOST").unwrap_or_else(|_| "localhost".to_string()),

            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),

            cors_origin: env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string()),

            session_secret: env::var("SESSION_SECRET").expect("SESSION_SECRET must be set"),

            smtp_server: env::var("SMTP_SERVER").ok(),
            smtp_username: env::var("SMTP_USERNAME").ok(),
            smtp_password: env::var("SMTP_PASSWORD").ok(),
        })
    }
}

pub struct Database {
    url: String,
}

impl Database {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL")?;
        let max_connections: u32 = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()?;

        println!(
            "Connecting to database with {} max connections",
            max_connections
        );

        Ok(Database { url: database_url })
    }

    pub fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Simulate database connection
        println!("Connected to database: {}", self.url);
        Ok(())
    }
}

pub struct Cache {
    redis_url: Option<String>,
}

impl Cache {
    pub fn new() -> Self {
        let redis_url = env::var("REDIS_URL").ok();
        let cache_ttl = env::var("CACHE_TTL").unwrap_or_else(|_| "3600".to_string());

        println!("Cache TTL set to: {} seconds", cache_ttl);

        Cache { redis_url }
    }

    pub fn is_enabled(&self) -> bool {
        self.redis_url.is_some()
    }
}

pub fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let log_format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    println!(
        "Setting up logging: level={}, format={}",
        log_level, log_format
    );
    Ok(())
}

pub fn setup_metrics() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(metrics_url) = env::var("METRICS_URL") {
        let metrics_interval = env::var("METRICS_INTERVAL").unwrap_or_else(|_| "60".to_string());

        println!(
            "Metrics enabled: url={}, interval={}s",
            metrics_url, metrics_interval
        );
    }

    Ok(())
}

pub fn setup_security() -> Result<(), Box<dyn std::error::Error>> {
    let allowed_origins = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());

    let rate_limit = env::var("RATE_LIMIT").unwrap_or_else(|_| "100".to_string());

    let enable_https = env::var("ENABLE_HTTPS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    println!(
        "Security config: origins={}, rate_limit={}/min, https={}",
        allowed_origins, rate_limit, enable_https
    );

    if enable_https {
        let cert_path =
            env::var("TLS_CERT_PATH").expect("TLS_CERT_PATH required when HTTPS is enabled");
        let key_path =
            env::var("TLS_KEY_PATH").expect("TLS_KEY_PATH required when HTTPS is enabled");

        println!("TLS configured: cert={}, key={}", cert_path, key_path);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting web server...");

    // Load configuration
    let config = Config::from_env()?;
    println!("Configuration loaded: {:?}", config);

    // Setup components
    setup_logging()?;
    setup_metrics()?;
    setup_security()?;

    // Initialize database
    let db = Database::new()?;
    db.connect()?;

    // Initialize cache
    let cache = Cache::new();
    if cache.is_enabled() {
        println!("Redis cache enabled");
    } else {
        println!("Redis cache disabled (no REDIS_URL provided)");
    }

    // Check for optional features
    if let Ok(webhook_url) = env::var("WEBHOOK_URL") {
        let webhook_secret =
            env::var("WEBHOOK_SECRET").unwrap_or_else(|_| "default-secret".to_string());
        println!("Webhooks enabled: url={}, secret=***", webhook_url);
    }

    if let Ok(sentry_dsn) = env::var("SENTRY_DSN") {
        let sentry_env =
            env::var("SENTRY_ENVIRONMENT").unwrap_or_else(|_| "production".to_string());
        println!("Sentry enabled: env={}", sentry_env);
    }

    // Feature flags
    let enable_admin = env::var("ENABLE_ADMIN_PANEL")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if enable_admin {
        let admin_token =
            env::var("ADMIN_TOKEN").expect("ADMIN_TOKEN required when admin panel is enabled");
        println!("Admin panel enabled with token");
    }

    println!("Server running on {}:{}", config.host, config.port);
    println!("Press Ctrl+C to shutdown");

    // Simulate server running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        break; // Just for example, don't actually run forever
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_with_env_vars() {
        // Test environment variables in tests
        std::env::set_var("TEST_DATABASE_URL", "sqlite::memory:");
        std::env::set_var("TEST_JWT_SECRET", "test-secret");
        std::env::set_var("TEST_API_KEY", "test-key");
        std::env::set_var("TEST_SESSION_SECRET", "test-session");

        let test_db_url = env::var("TEST_DATABASE_URL").unwrap();
        assert_eq!(test_db_url, "sqlite::memory:");

        // Cleanup
        std::env::remove_var("TEST_DATABASE_URL");
        std::env::remove_var("TEST_JWT_SECRET");
        std::env::remove_var("TEST_API_KEY");
        std::env::remove_var("TEST_SESSION_SECRET");
    }

    #[test]
    fn test_optional_features() {
        // Test optional feature flags
        if let Ok(feature_flag) = env::var("EXPERIMENTAL_FEATURES") {
            println!("Experimental features enabled: {}", feature_flag);
        }

        let debug_mode = env::var("DEBUG_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        assert!(!debug_mode); // Should be false by default
    }
}
