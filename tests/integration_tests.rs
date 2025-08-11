//! Integration tests for auto-env-generator
//!
//! Tests the complete functionality including CLI, library API, configuration,
//! and edge cases for environment variable detection and .env file generation.

use auto_env_generator::{generate_env_file, generate_env_file_with_config, Config, EnvScanner};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn create_test_file(dir: &Path, path: &str, content: &str) -> std::io::Result<()> {
    let file_path = dir.join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file_path, content)
}

fn read_env_file(path: &Path) -> std::collections::HashMap<String, String> {
    let mut env_vars = std::collections::HashMap::new();

    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                env_vars.insert(key, value);
            }
        }
    }

    env_vars
}

#[test]
fn test_detection_correctness_comprehensive() {
    let temp_dir = TempDir::new().unwrap();

    // Create a complex Rust file with various patterns
    let complex_content = r#"
use std::env;
use dotenv;

pub struct Config {
    database_url: String,
    redis_url: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            redis_url: env::var("REDIS_URL").ok(),
        }
    }

    pub fn load_secrets() {
        let api_key = dotenv::var("API_KEY").unwrap();
        let secret_key = std::env::var_os("SECRET_KEY").unwrap();
        let jwt_secret = env::var_os("JWT_SECRET").unwrap();
        let oauth_secret = dotenv::var_os("OAUTH_SECRET").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let test_var = std::env::var("TEST_VARIABLE").unwrap();
        let debug_mode = env::var("DEBUG_MODE").unwrap_or_default();

        // This should not be detected (not a direct env::var call)
        let other_var = format!("{}_{}", "PREFIX", std::env::var("SUFFIX_VAR").unwrap());
    }
}

fn main() {
    // Various spacing and formatting
    let var1 = std::env::var("VAR_WITH_SPACES").unwrap();
    let var2=env::var("VAR_NO_SPACES").unwrap();
    let var3 = dotenv::var(  "VAR_EXTRA_SPACES"  ).unwrap();

    // Multi-line (should still work)
    let var4 = std::env::var(
        "MULTILINE_VAR"
    ).unwrap();

    // With comments
    let var5 = env::var("COMMENTED_VAR").unwrap(); // This has a comment

    // In conditionals
    if let Ok(optional_var) = std::env::var("OPTIONAL_VAR") {
        println!("Optional var: {}", optional_var);
    }

    // In match expressions
    match env::var("MATCH_VAR") {
        Ok(val) => println!("Got: {}", val),
        Err(_) => println!("Not found"),
    }
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", complex_content).unwrap();

    let scanner = EnvScanner::new().unwrap();
    let variables = scanner.scan_directory(temp_dir.path()).unwrap();

    // Expected variables (should be detected)
    let expected = vec![
        "DATABASE_URL",
        "REDIS_URL",
        "API_KEY",
        "SECRET_KEY",
        "JWT_SECRET",
        "OAUTH_SECRET",
        "TEST_VARIABLE",
        "DEBUG_MODE",
        "SUFFIX_VAR", // This should be detected as it's a direct call
        "VAR_WITH_SPACES",
        "VAR_NO_SPACES",
        "VAR_EXTRA_SPACES",
        "MULTILINE_VAR",
        "COMMENTED_VAR",
        "OPTIONAL_VAR",
        "MATCH_VAR",
    ];

    assert_eq!(variables.len(), expected.len());

    for var in expected {
        assert!(
            variables.contains(var),
            "Variable '{}' should be detected",
            var
        );
    }
}

#[test]
fn test_detection_edge_cases() {
    let temp_dir = TempDir::new().unwrap();

    let edge_cases_content = r#"
use std::env;

fn main() {
    // These should be detected
    let var1 = std::env::var("SIMPLE_VAR").unwrap();
    let var2 = env::var("ANOTHER_VAR").ok();
    let var3 = dotenv::var("DOTENV_VAR").unwrap_or_default();

    // These should NOT be detected (not literal strings)
    let dynamic_key = "DYNAMIC_KEY";
    let var4 = std::env::var(dynamic_key).unwrap(); // Variable key
    let var5 = std::env::var(format!("PREFIX_{}", "SUFFIX")).unwrap(); // Computed key

    // These should NOT be detected (not env::var calls)
    let not_env = some_other_function("NOT_ENV_VAR");
    let comment_var = "This contains std::env::var(\"FAKE_VAR\") in a string";

    // Edge case: Special characters in variable names
    let special_var = env::var("VAR_WITH-DASH").unwrap();
    let underscore_var = env::var("VAR_WITH_UNDERSCORE").unwrap();
    let number_var = env::var("VAR123").unwrap();
}

fn some_other_function(key: &str) -> String {
    format!("Not env: {}", key)
}
"#;

    create_test_file(temp_dir.path(), "src/edge_cases.rs", edge_cases_content).unwrap();

    let scanner = EnvScanner::new().unwrap();
    let variables = scanner.scan_directory(temp_dir.path()).unwrap();

    // Should detect these
    assert!(variables.contains("SIMPLE_VAR"));
    assert!(variables.contains("ANOTHER_VAR"));
    assert!(variables.contains("DOTENV_VAR"));
    assert!(variables.contains("VAR_WITH-DASH"));
    assert!(variables.contains("VAR_WITH_UNDERSCORE"));
    assert!(variables.contains("VAR123"));

    // Should NOT detect these
    assert!(!variables.contains("DYNAMIC_KEY"));
    assert!(!variables.contains("NOT_ENV_VAR"));
    assert!(!variables.contains("FAKE_VAR"));

    // Should be exactly 6 variables
    assert_eq!(variables.len(), 6);
}

#[test]
fn test_merge_logic_comprehensive() {
    let temp_dir = TempDir::new().unwrap();

    // Create existing .env file with some variables
    let existing_env = r#"# Existing environment file
# This is a comment

EXISTING_VAR=existing_value
SHARED_VAR=original_value
EMPTY_EXISTING=

# Another comment
DATABASE_URL=postgres://localhost/mydb
"#;

    fs::write(temp_dir.path().join(".env"), existing_env).unwrap();

    // Create Rust file with some overlapping and new variables
    let rust_content = r#"
fn main() {
    let shared = std::env::var("SHARED_VAR").unwrap();
    let new_var = std::env::var("NEW_VARIABLE").unwrap();
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let api_key = std::env::var("API_KEY").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    // Test with merge enabled (default)
    let config = Config {
        output: Some(".env".to_string()),
        merge_existing: Some(true),
        ignore: None,
    };

    generate_env_file_with_config(temp_dir.path(), config).unwrap();

    let result_vars = read_env_file(&temp_dir.path().join(".env"));

    // Should preserve existing values
    assert_eq!(
        result_vars.get("EXISTING_VAR"),
        Some(&"existing_value".to_string())
    );
    assert_eq!(
        result_vars.get("SHARED_VAR"),
        Some(&"original_value".to_string())
    );
    assert_eq!(
        result_vars.get("DATABASE_URL"),
        Some(&"postgres://localhost/mydb".to_string())
    );
    assert_eq!(result_vars.get("EMPTY_EXISTING"), Some(&"".to_string()));

    // Should add new variables with empty values
    assert_eq!(result_vars.get("NEW_VARIABLE"), Some(&"".to_string()));
    assert_eq!(result_vars.get("API_KEY"), Some(&"".to_string()));

    // Should have all variables
    assert_eq!(result_vars.len(), 6);
}

#[test]
fn test_no_merge_behavior() {
    let temp_dir = TempDir::new().unwrap();

    // Create existing .env file
    let existing_env = "EXISTING_VAR=existing_value\nSHARED_VAR=original_value\n";
    fs::write(temp_dir.path().join(".env"), existing_env).unwrap();

    // Create Rust file
    let rust_content = r#"
fn main() {
    let shared = std::env::var("SHARED_VAR").unwrap();
    let new_var = std::env::var("NEW_VARIABLE").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    // Test with merge disabled
    let config = Config {
        output: Some(".env".to_string()),
        merge_existing: Some(false),
        ignore: None,
    };

    generate_env_file_with_config(temp_dir.path(), config).unwrap();

    let result_vars = read_env_file(&temp_dir.path().join(".env"));

    // Should only have detected variables with empty values
    assert_eq!(result_vars.get("SHARED_VAR"), Some(&"".to_string()));
    assert_eq!(result_vars.get("NEW_VARIABLE"), Some(&"".to_string()));

    // Should NOT have the existing variable that wasn't detected
    assert!(!result_vars.contains_key("EXISTING_VAR"));

    assert_eq!(result_vars.len(), 2);
}

#[test]
fn test_ignore_list_behavior() {
    let temp_dir = TempDir::new().unwrap();

    let rust_content = r#"
fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let api_key = std::env::var("API_KEY").unwrap();
    let debug = std::env::var("DEBUG_MODE").unwrap();
    let secret = std::env::var("SECRET_KEY").unwrap();
    let port = std::env::var("PORT").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    // Test with ignore list
    let config = Config {
        output: Some(".env".to_string()),
        merge_existing: Some(false),
        ignore: Some(vec!["DEBUG_MODE".to_string(), "SECRET_KEY".to_string()]),
    };

    generate_env_file_with_config(temp_dir.path(), config).unwrap();

    let result_vars = read_env_file(&temp_dir.path().join(".env"));

    // Should have variables that are not ignored
    assert!(result_vars.contains_key("DATABASE_URL"));
    assert!(result_vars.contains_key("API_KEY"));
    assert!(result_vars.contains_key("PORT"));

    // Should NOT have ignored variables
    assert!(!result_vars.contains_key("DEBUG_MODE"));
    assert!(!result_vars.contains_key("SECRET_KEY"));

    assert_eq!(result_vars.len(), 3);
}

#[test]
fn test_custom_output_file() {
    let temp_dir = TempDir::new().unwrap();

    let rust_content = r#"
fn main() {
    let var = std::env::var("TEST_VAR").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    // Test with custom output file
    let config = Config {
        output: Some(".env.example".to_string()),
        merge_existing: Some(false),
        ignore: None,
    };

    generate_env_file_with_config(temp_dir.path(), config).unwrap();

    // Should create .env.example, not .env
    assert!(temp_dir.path().join(".env.example").exists());
    assert!(!temp_dir.path().join(".env").exists());

    let result_vars = read_env_file(&temp_dir.path().join(".env.example"));
    assert!(result_vars.contains_key("TEST_VAR"));
}

#[test]
fn test_nested_directory_scanning() {
    let temp_dir = TempDir::new().unwrap();

    // Create files in various nested directories
    create_test_file(
        temp_dir.path(),
        "src/main.rs",
        r#"fn main() { let var1 = std::env::var("VAR_1").unwrap(); }"#,
    )
    .unwrap();

    create_test_file(
        temp_dir.path(),
        "src/lib.rs",
        r#"pub fn lib_fn() { let var2 = std::env::var("VAR_2").unwrap(); }"#,
    )
    .unwrap();

    create_test_file(
        temp_dir.path(),
        "src/modules/auth.rs",
        r#"pub fn auth() { let var3 = std::env::var("VAR_3").unwrap(); }"#,
    )
    .unwrap();

    create_test_file(
        temp_dir.path(),
        "tests/integration.rs",
        r#"#[test] fn test() { let var4 = std::env::var("VAR_4").unwrap(); }"#,
    )
    .unwrap();

    create_test_file(
        temp_dir.path(),
        "examples/example.rs",
        r#"fn main() { let var5 = std::env::var("VAR_5").unwrap(); }"#,
    )
    .unwrap();

    // Create file in target directory (should be ignored)
    create_test_file(
        temp_dir.path(),
        "target/debug/main.rs",
        r#"fn main() { let ignored = std::env::var("IGNORED_VAR").unwrap(); }"#,
    )
    .unwrap();

    generate_env_file(temp_dir.path()).unwrap();

    let result_vars = read_env_file(&temp_dir.path().join(".env"));

    // Should find variables from all non-target directories
    assert!(result_vars.contains_key("VAR_1"));
    assert!(result_vars.contains_key("VAR_2"));
    assert!(result_vars.contains_key("VAR_3"));
    assert!(result_vars.contains_key("VAR_4"));
    assert!(result_vars.contains_key("VAR_5"));

    // Should ignore target directory
    assert!(!result_vars.contains_key("IGNORED_VAR"));

    assert_eq!(result_vars.len(), 5);
}

#[test]
fn test_library_api_functions() {
    let temp_dir = TempDir::new().unwrap();

    let rust_content = r#"
fn main() {
    let var1 = std::env::var("LIB_VAR_1").unwrap();
    let var2 = std::env::var("LIB_VAR_2").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    // Test basic generate_env_file function
    generate_env_file(temp_dir.path()).unwrap();
    assert!(temp_dir.path().join(".env").exists());

    // Test generate_env_file_to with custom output path
    let custom_output = temp_dir.path().join("custom.env");
    auto_env_generator::generate_env_file_to(temp_dir.path(), &custom_output).unwrap();
    assert!(custom_output.exists());

    // Test scan_for_env_vars function
    let variables = auto_env_generator::scan_for_env_vars(temp_dir.path()).unwrap();
    assert_eq!(variables.len(), 2);
    assert!(variables.contains("LIB_VAR_1"));
    assert!(variables.contains("LIB_VAR_2"));
}

#[test]
fn test_config_loading() {
    let temp_dir = TempDir::new().unwrap();

    // Create a TOML config file with correct format:
    let correct_config_content = r#"
output = ".env.development"
merge_existing = false
ignore = ["DEBUG", "TEST_MODE", "DEVELOPMENT"]
"#;

    fs::write(temp_dir.path().join("autoenv.toml"), correct_config_content).unwrap();

    let config = EnvScanner::load_config(temp_dir.path().join("autoenv.toml")).unwrap();

    assert_eq!(config.output, Some(".env.development".to_string()));
    assert_eq!(config.merge_existing, Some(false));
    assert_eq!(
        config.ignore,
        Some(vec![
            "DEBUG".to_string(),
            "TEST_MODE".to_string(),
            "DEVELOPMENT".to_string()
        ])
    );
}

#[test]
fn test_empty_project() {
    let temp_dir = TempDir::new().unwrap();

    // Create empty src directory
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "fn main() { println!(\"Hello, world!\"); }",
    )
    .unwrap();

    generate_env_file(temp_dir.path()).unwrap();

    let result_vars = read_env_file(&temp_dir.path().join(".env"));
    assert_eq!(result_vars.len(), 0);

    // File should still be created with header
    let content = fs::read_to_string(temp_dir.path().join(".env")).unwrap();
    assert!(content.contains("Auto-generated environment variables"));
}

#[test]
fn test_file_with_no_env_vars() {
    let temp_dir = TempDir::new().unwrap();

    let rust_content = r#"
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");

    println!("Hello, world!");

    let x = 42;
    let y = x * 2;

    // This contains the word "var" but is not an env var call
    let my_var = "not an env var";

    // This contains env::var in a comment but should not be detected
    // std::env::var("COMMENT_VAR") - this is just a comment
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    let scanner = EnvScanner::new().unwrap();
    let variables = scanner.scan_directory(temp_dir.path()).unwrap();

    // Should not detect any variables since none are actual env::var calls
    assert_eq!(variables.len(), 0);
}

#[test]
fn test_malformed_env_calls() {
    let temp_dir = TempDir::new().unwrap();

    let rust_content = r#"
fn main() {
    // Valid calls
    let var1 = std::env::var("VALID_VAR").unwrap();

    // Invalid/malformed calls that should not be detected
    let var2 = std::env::var(; // Syntax error, but we should handle gracefully
    let var3 = std::env::var("UNCLOSED_QUOTE;
    let var4 = std::env::var('SINGLE_QUOTES').unwrap(); // Wrong quote type
    let var5 = std::env::var().unwrap(); // No argument

    // These should be detected despite formatting
    let var6 = std::env::var("VALID_VAR_2").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    let scanner = EnvScanner::new().unwrap();
    let variables = scanner.scan_directory(temp_dir.path()).unwrap();

    // Should only detect the valid calls
    assert_eq!(variables.len(), 2);
    assert!(variables.contains("VALID_VAR"));
    assert!(variables.contains("VALID_VAR_2"));
}

#[test]
fn test_cli_integration() {
    let temp_dir = TempDir::new().unwrap();

    let rust_content = r#"
fn main() {
    let var = std::env::var("CLI_TEST_VAR").unwrap();
}
"#;

    create_test_file(temp_dir.path(), "src/main.rs", rust_content).unwrap();

    // Build the CLI binary first
    let output = Command::new("cargo")
        .args(&["build", "--bin", "autoenv"])
        .current_dir(".")
        .output()
        .expect("Failed to build CLI");

    if !output.status.success() {
        panic!(
            "Failed to build CLI: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Test the CLI generate command
    let output = Command::new("./target/debug/autoenv")
        .args(&["generate", temp_dir.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute CLI");

    if !output.status.success() {
        panic!(
            "CLI command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Check that .env file was created
    assert!(temp_dir.path().join(".env").exists());

    let result_vars = read_env_file(&temp_dir.path().join(".env"));
    assert!(result_vars.contains_key("CLI_TEST_VAR"));
}

#[test]
fn test_large_file_performance() {
    let temp_dir = TempDir::new().unwrap();

    // Create a large file with many environment variables
    let mut large_content = String::new();
    large_content.push_str("fn main() {\n");

    for i in 0..1000 {
        large_content.push_str(&format!(
            "    let var_{} = std::env::var(\"LARGE_VAR_{}\").unwrap();\n",
            i, i
        ));
    }

    large_content.push_str("}\n");

    create_test_file(temp_dir.path(), "src/main.rs", &large_content).unwrap();

    let start = std::time::Instant::now();

    let scanner = EnvScanner::new().unwrap();
    let variables = scanner.scan_directory(temp_dir.path()).unwrap();

    let duration = start.elapsed();

    // Should complete quickly (under 1 second for 1000 variables)
    assert!(duration.as_secs() < 1);
    assert_eq!(variables.len(), 1000);

    // Verify some variables were detected correctly
    assert!(variables.contains("LARGE_VAR_0"));
    assert!(variables.contains("LARGE_VAR_500"));
    assert!(variables.contains("LARGE_VAR_999"));
}
