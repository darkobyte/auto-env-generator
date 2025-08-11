use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple Web Server Example");

    // Required environment variables
    let database_url = env::var("DATABASE_URL")?;
    let api_key = env::var("API_KEY")?;
    let jwt_secret = env::var("JWT_SECRET")?;

    // Optional environment variables with defaults
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    // Feature flags
    let enable_debug = env::var("DEBUG_MODE").unwrap_or_else(|_| "false".to_string());

    println!("Configuration:");
    println!("  Database URL: {}", database_url);
    println!("  API Key: ***");
    println!("  JWT Secret: ***");
    println!("  Host: {}", host);
    println!("  Port: {}", port);
    println!("  Log Level: {}", log_level);
    println!("  Debug Mode: {}", enable_debug);

    Ok(())
}
