//! Auto Environment Generator Library
//!
//! A fast Rust library for scanning .rs files to detect environment variable usage
//! and generating .env files with parallel processing and efficient pattern matching.

use aho_corasick::AhoCorasick;
use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Configuration for the environment generator
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Name of the output file (default: ".env")
    pub output: Option<String>,
    /// Whether to merge with existing file without overwriting values
    pub merge_existing: Option<bool>,
    /// List of variable names to ignore
    pub ignore: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: Some(".env".to_string()),
            merge_existing: Some(true),
            ignore: Some(vec![]),
        }
    }
}

/// Environment variable scanner with efficient pattern matching
pub struct EnvScanner {
    patterns: AhoCorasick,
    extract_regex: Regex,
    config: Config,
}

impl EnvScanner {
    /// Create a new scanner with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(Config::default())
    }

    /// Create a scanner with custom configuration
    pub fn with_config(config: Config) -> Result<Self> {
        // Patterns to search for environment variable calls
        let patterns = vec![
            "std::env::var(",
            "env::var(",
            "dotenv::var(",
            "std::env::var_os(",
            "env::var_os(",
            "dotenv::var_os(",
        ];

        let ac = AhoCorasick::new(patterns).context("Failed to create Aho-Corasick automaton")?;

        // Regex to extract string literals from env var calls (more strict)
        let extract_regex = Regex::new(
            r#"(?:std::env::var|env::var|dotenv::var)(?:_os)?\s*\(\s*"([^"\n\r]*)"\s*\)"#,
        )
        .context("Failed to compile extraction regex")?;

        Ok(Self {
            patterns: ac,
            extract_regex,
            config,
        })
    }

    /// Load configuration from a TOML file
    pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<Config> {
        let content = fs::read_to_string(config_path).context("Failed to read config file")?;
        let config: Config = toml::from_str(&content).context("Failed to parse TOML config")?;
        Ok(config)
    }

    /// Scan a single file for environment variable usage
    fn scan_file<P: AsRef<Path>>(&self, path: P) -> Result<HashSet<String>> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;

        let mut variables = HashSet::new();

        // Process the entire file content to handle multiline cases
        for line in content.lines() {
            let trimmed_line = line.trim();

            // Skip comments and empty lines
            if trimmed_line.starts_with("//") || trimmed_line.is_empty() {
                continue;
            }

            // Check if this is inside a string literal (basic check)
            if let Some(comment_pos) = line.find("//") {
                let before_comment = &line[..comment_pos];
                // Only process the part before the comment
                if self.patterns.is_match(before_comment) {
                    for cap in self.extract_regex.captures_iter(before_comment) {
                        if let Some(var_name) = cap.get(1) {
                            let var_name = var_name.as_str().to_string();

                            // Check if variable should be ignored
                            if let Some(ignore_list) = &self.config.ignore {
                                if !ignore_list.contains(&var_name) {
                                    variables.insert(var_name);
                                }
                            } else {
                                variables.insert(var_name);
                            }
                        }
                    }
                }
            } else {
                // Fast pattern search using Aho-Corasick
                if self.patterns.is_match(&line) {
                    // Extract variable names using regex
                    for cap in self.extract_regex.captures_iter(&line) {
                        if let Some(var_name) = cap.get(1) {
                            let var_name = var_name.as_str().to_string();

                            // Check if variable should be ignored
                            if let Some(ignore_list) = &self.config.ignore {
                                if !ignore_list.contains(&var_name) {
                                    variables.insert(var_name);
                                }
                            } else {
                                variables.insert(var_name);
                            }
                        }
                    }
                }
            }
        }

        // Handle multiline patterns by normalizing whitespace
        let normalized_content = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.starts_with("//") && !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        if self.patterns.is_match(&normalized_content) {
            for cap in self.extract_regex.captures_iter(&normalized_content) {
                if let Some(var_name) = cap.get(1) {
                    let var_name = var_name.as_str().to_string();

                    // Check if variable should be ignored
                    if let Some(ignore_list) = &self.config.ignore {
                        if !ignore_list.contains(&var_name) {
                            variables.insert(var_name);
                        }
                    } else {
                        variables.insert(var_name);
                    }
                }
            }
        }

        Ok(variables)
    }

    /// Find all .rs files in a directory recursively
    fn find_rust_files<P: AsRef<Path>>(&self, dir: P) -> Result<Vec<PathBuf>> {
        let mut rust_files = Vec::new();

        fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Skip target and hidden directories
                    if let Some(name) = path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            if name_str.starts_with('.') || name_str == "target" {
                                continue;
                            }
                        }
                    }
                    walk_dir(&path, files)?;
                } else if path.extension().map_or(false, |ext| ext == "rs") {
                    files.push(path);
                }
            }
            Ok(())
        }

        walk_dir(dir.as_ref(), &mut rust_files)?;
        Ok(rust_files)
    }

    /// Scan all .rs files in parallel and collect environment variables
    pub fn scan_directory<P: AsRef<Path>>(&self, dir: P) -> Result<HashSet<String>> {
        let rust_files = self.find_rust_files(dir)?;

        if rust_files.is_empty() {
            return Ok(HashSet::new());
        }

        // Use Mutex to safely collect results from parallel threads
        let all_variables = Mutex::new(HashSet::new());

        // Parallel processing of files
        rust_files.par_iter().try_for_each(|file| -> Result<()> {
            let variables = self.scan_file(file)?;

            if !variables.is_empty() {
                let mut all_vars = all_variables.lock().unwrap();
                all_vars.extend(variables);
            }

            Ok(())
        })?;

        Ok(all_variables.into_inner().unwrap())
    }

    /// Read existing .env file and return variables as HashMap
    fn read_existing_env<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<std::collections::HashMap<String, String>> {
        let mut existing = std::collections::HashMap::new();

        if path.as_ref().exists() {
            let content = fs::read_to_string(path)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim().to_string();
                    let value = line[eq_pos + 1..].trim().to_string();
                    existing.insert(key, value);
                }
            }
        }

        Ok(existing)
    }

    /// Generate .env file with detected variables
    pub fn generate_env_file<P: AsRef<Path>>(
        &self,
        variables: &HashSet<String>,
        output_path: P,
    ) -> Result<()> {
        let output_path = output_path.as_ref();
        let merge_existing = self.config.merge_existing.unwrap_or(true);

        let mut existing_vars = if merge_existing {
            self.read_existing_env(output_path)?
        } else {
            std::collections::HashMap::new()
        };

        // Add new variables with empty values if they don't exist
        for var in variables {
            existing_vars.entry(var.clone()).or_insert_with(String::new);
        }

        // Sort variables for consistent output
        let mut sorted_vars: Vec<_> = existing_vars.iter().collect();
        sorted_vars.sort_by(|a, b| a.0.cmp(b.0));

        // Write to file
        let mut file = File::create(output_path)
            .with_context(|| format!("Failed to create file: {:?}", output_path))?;

        writeln!(file, "# Auto-generated environment variables")?;
        writeln!(file, "# Add your values below")?;
        writeln!(file)?;

        for (key, value) in sorted_vars {
            if value.is_empty() {
                writeln!(file, "{}=", key)?;
            } else {
                writeln!(file, "{}={}", key, value)?;
            }
        }

        Ok(())
    }
}

impl Default for EnvScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create default EnvScanner")
    }
}

/// Main API function for programmatic use
pub fn generate_env_file<P: AsRef<Path>>(path: P) -> Result<()> {
    generate_env_file_with_config(path, Config::default())
}

/// Generate .env file with custom configuration
pub fn generate_env_file_with_config<P: AsRef<Path>>(path: P, config: Config) -> Result<()> {
    let scanner = EnvScanner::with_config(config.clone())?;
    let variables = scanner.scan_directory(&path)?;

    let output_file = config.output.unwrap_or_else(|| ".env".to_string());
    let output_path = path.as_ref().join(output_file);

    scanner.generate_env_file(&variables, output_path)?;
    Ok(())
}

/// Generate .env file with custom output path
pub fn generate_env_file_to<P: AsRef<Path>, O: AsRef<Path>>(
    scan_path: P,
    output_path: O,
) -> Result<()> {
    let scanner = EnvScanner::new()?;
    let variables = scanner.scan_directory(scan_path)?;
    scanner.generate_env_file(&variables, output_path)?;
    Ok(())
}

/// Scan directory and return found environment variables
pub fn scan_for_env_vars<P: AsRef<Path>>(path: P) -> Result<HashSet<String>> {
    let scanner = EnvScanner::new()?;
    scanner.scan_directory(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> Result<()> {
        let file_path = dir.join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
        Ok(())
    }

    #[test]
    fn test_scan_single_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content = r#"
use std::env;

fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let api_key = env::var("API_KEY").unwrap();
    let debug = dotenv::var("DEBUG_MODE").unwrap_or_default();
}
"#;

        create_test_file(temp_dir.path(), "main.rs", content)?;

        let scanner = EnvScanner::new()?;
        let variables = scanner.scan_file(temp_dir.path().join("main.rs"))?;

        assert_eq!(variables.len(), 3);
        assert!(variables.contains("DATABASE_URL"));
        assert!(variables.contains("API_KEY"));
        assert!(variables.contains("DEBUG_MODE"));

        Ok(())
    }

    #[test]
    fn test_ignore_variables() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content = r#"
fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let api_key = std::env::var("API_KEY").unwrap();
}
"#;

        create_test_file(temp_dir.path(), "main.rs", content)?;

        let config = Config {
            ignore: Some(vec!["API_KEY".to_string()]),
            ..Default::default()
        };

        let scanner = EnvScanner::with_config(config)?;
        let variables = scanner.scan_directory(temp_dir.path())?;

        assert_eq!(variables.len(), 1);
        assert!(variables.contains("DATABASE_URL"));
        assert!(!variables.contains("API_KEY"));

        Ok(())
    }

    #[test]
    fn test_merge_existing_env() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create existing .env file
        let existing_env = "EXISTING_VAR=existing_value\nDATABASE_URL=\n";
        fs::write(temp_dir.path().join(".env"), existing_env)?;

        // Create Rust file with new variable
        let content = r#"
fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let new_var = std::env::var("NEW_VARIABLE").unwrap();
}
"#;
        create_test_file(temp_dir.path(), "main.rs", content)?;

        let scanner = EnvScanner::new()?;
        let variables = scanner.scan_directory(temp_dir.path())?;
        scanner.generate_env_file(&variables, temp_dir.path().join(".env"))?;

        let result = fs::read_to_string(temp_dir.path().join(".env"))?;

        // Should contain existing value and new empty variable
        assert!(result.contains("EXISTING_VAR=existing_value"));
        assert!(result.contains("NEW_VARIABLE="));
        assert!(result.contains("DATABASE_URL="));

        Ok(())
    }

    #[test]
    fn test_parallel_scanning() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create multiple files in different directories
        create_test_file(
            temp_dir.path(),
            "src/main.rs",
            r#"
fn main() {
    let var1 = std::env::var("VAR_1").unwrap();
}
"#,
        )?;

        create_test_file(
            temp_dir.path(),
            "src/lib.rs",
            r#"
pub fn test() {
    let var2 = env::var("VAR_2").unwrap();
}
"#,
        )?;

        create_test_file(
            temp_dir.path(),
            "tests/integration.rs",
            r#"
#[test]
fn test_something() {
    let var3 = dotenv::var("VAR_3").unwrap();
}
"#,
        )?;

        let scanner = EnvScanner::new()?;
        let variables = scanner.scan_directory(temp_dir.path())?;

        assert_eq!(variables.len(), 3);
        assert!(variables.contains("VAR_1"));
        assert!(variables.contains("VAR_2"));
        assert!(variables.contains("VAR_3"));

        Ok(())
    }

    #[test]
    fn test_skip_target_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create file in target directory (should be ignored)
        create_test_file(
            temp_dir.path(),
            "target/debug/build/main.rs",
            r#"
fn main() {
    let var1 = std::env::var("SHOULD_BE_IGNORED").unwrap();
}
"#,
        )?;

        // Create normal file
        create_test_file(
            temp_dir.path(),
            "src/main.rs",
            r#"
fn main() {
    let var2 = std::env::var("SHOULD_BE_FOUND").unwrap();
}
"#,
        )?;

        let scanner = EnvScanner::new()?;
        let variables = scanner.scan_directory(temp_dir.path())?;

        assert_eq!(variables.len(), 1);
        assert!(variables.contains("SHOULD_BE_FOUND"));
        assert!(!variables.contains("SHOULD_BE_IGNORED"));

        Ok(())
    }
}
