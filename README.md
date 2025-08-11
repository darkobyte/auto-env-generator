# Auto Environment Generator

[![Crates.io](https://img.shields.io/crates/v/auto-env-generator.svg)](https://crates.io/crates/auto-env-generator)
[![Documentation](https://docs.rs/auto-env-generator/badge.svg)](https://docs.rs/auto-env-generator)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)


A fast, parallel Rust tool for automatically scanning your codebase and generating `.env` files based on detected environment variable usage. Uses efficient pattern matching with Aho-Corasick and memchr for sub-second performance even on large projects.

## Features

- 🚀 **Blazing Fast**: Parallel scanning with Rayon + efficient pattern matching
- 🔍 **Smart Detection**: Finds `std::env::var()`, `env::var()`, and `dotenv::var()` calls
- 🛡️ **Safe Merging**: Preserves existing values when merging with existing `.env` files
- ⚙️ **Configurable**: TOML configuration with ignore lists and custom output paths
- 📦 **Library + CLI**: Use as a library or standalone command-line tool
- 🎯 **Zero Dependencies**: Minimal runtime dependencies, maximum performance
- 🧪 **Well Tested**: Comprehensive test suite with benchmarks

## Quick Start

### CLI Usage

Install the CLI tool:

```bash
cargo install auto-env-generator
```

Generate a `.env` file for your project:

```bash
# Scan current directory
autoenv generate

# Scan specific directory
autoenv generate ./my-rust-project

# Generate .env.example instead
autoenv generate -o .env.example

# Scan and list variables without generating file
autoenv scan

# Show verbose output
autoenv generate --verbose
```

### Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
auto-env-generator = "0.1"
```

Basic usage:

```rust
use auto_env_generator::generate_env_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate .env file for current directory
    generate_env_file(".")?;

    // Or scan a specific directory
    generate_env_file("./my-project")?;

    Ok(())
}
```

Advanced usage with configuration:

```rust
use auto_env_generator::{generate_env_file_with_config, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        output: Some(".env.example".to_string()),
        merge_existing: Some(false),
        ignore: Some(vec!["DEBUG".to_string(), "TEST_MODE".to_string()]),
    };

    generate_env_file_with_config("./my-project", config)?;
    Ok(())
}
```

## What It Detects

The tool efficiently scans for these patterns:

```rust
// Standard library calls
let db_url = std::env::var("DATABASE_URL").unwrap();
let api_key = env::var("API_KEY").unwrap();

// dotenv crate calls
let secret = dotenv::var("SECRET_KEY").unwrap();

// With _os variants
let path = std::env::var_os("PATH").unwrap();
let home = env::var_os("HOME").unwrap();
let config = dotenv::var_os("CONFIG_PATH").unwrap();

// Various formatting styles
let var1 = std::env::var("VAR_1").unwrap();
let var2=env::var("VAR_2").ok();
let var3 = dotenv::var(  "VAR_3"  ).unwrap_or_default();

// Multi-line calls
let var4 = std::env::var(
    "MULTILINE_VAR"
).unwrap();
```

## Configuration

Create an `autoenv.toml` file in your project root:

```toml
# Output file name (default: ".env")
output = ".env.example"

# Whether to merge with existing file without overwriting values (default: true)
merge_existing = true

# List of variable names to ignore
ignore = [
    "HOME",
    "PATH",
    "USER",
    "DEBUG",
    "TEST_MODE"
]
```

Generate a sample config file:

```bash
autoenv init-config
```

## CLI Commands

### `generate`

Generate a `.env` file by scanning Rust source files:

```bash
autoenv generate [DIRECTORY] [OPTIONS]

Options:
  -o, --output <FILE>        Output file name (default: .env)
  -c, --config <CONFIG>      Configuration file path
      --no-merge             Don't merge with existing file (overwrite instead)
      --ignore <VARIABLE>    Variables to ignore (can be used multiple times)
  -v, --verbose              Verbose output
```

Examples:

```bash
# Basic usage
autoenv generate

# Generate .env.example file
autoenv generate -o .env.example

# Ignore specific variables
autoenv generate --ignore DEBUG --ignore TEST_MODE

# Use custom config file
autoenv generate -c custom-config.toml

# Scan different directory with verbose output
autoenv generate ./backend --verbose
```

### `scan`

List found environment variables without generating a file:

```bash
autoenv scan [DIRECTORY] [OPTIONS]

Options:
  -c, --config <CONFIG>      Configuration file path
      --ignore <VARIABLE>    Variables to ignore
      --show-locations       Show file locations where variables were found
```

### `config`

Show current configuration:

```bash
autoenv config [OPTIONS]

Options:
  -c, --config <CONFIG>      Configuration file path
```

### `init-config`

Create a sample configuration file:

```bash
autoenv init-config [FILE]
```

## Library API

### Core Functions

```rust
use auto_env_generator::*;

// Generate .env file with default settings
generate_env_file(path: &str) -> Result<()>

// Generate with custom configuration
generate_env_file_with_config(path: &str, config: Config) -> Result<()>

// Generate to custom output path
generate_env_file_to(scan_path: &str, output_path: &str) -> Result<()>

// Just scan and return found variables
scan_for_env_vars(path: &str) -> Result<HashSet<String>>
```

### Advanced Usage

```rust
use auto_env_generator::{EnvScanner, Config};

// Create scanner with custom configuration
let config = Config {
    output: Some(".env.production".to_string()),
    merge_existing: Some(true),
    ignore: Some(vec!["DEBUG".to_string()]),
};

let scanner = EnvScanner::with_config(config)?;

// Scan directory
let variables = scanner.scan_directory("./src")?;

// Generate file
scanner.generate_env_file(&variables, ".env.production")?;

// Load configuration from file
let config = EnvScanner::load_config("autoenv.toml")?;
```

## Performance

Auto Environment Generator is designed for speed:

- **Parallel Processing**: Uses Rayon for parallel file processing
- **Efficient Pattern Matching**: Aho-Corasick algorithm for fast pattern detection
- **Zero-Copy Reading**: BufReader with minimal allocations
- **Smart Filtering**: Skips target directories and non-Rust files

### Benchmarks

On a typical Rust project (500 files, ~50,000 lines):

- **Scan Time**: ~100-300ms
- **Memory Usage**: <50MB peak
- **Throughput**: >1000 files/second

Run benchmarks yourself:

```bash
cargo bench
```

## How It Works

1. **File Discovery**: Recursively finds all `.rs` files, skipping `target/` and hidden directories
2. **Parallel Scanning**: Uses Rayon to process files in parallel
3. **Pattern Matching**: Aho-Corasick automaton quickly finds potential env var calls
4. **Extraction**: Regex extracts variable names from string literals
5. **Deduplication**: HashSet ensures no duplicate variables
6. **Merge Logic**: Intelligently merges with existing `.env` files
7. **Output**: Generates sorted, commented `.env` file

## Integration

### CI/CD

Use in GitHub Actions to ensure your `.env.example` stays up-to-date:

```yaml
name: Update .env.example

on: [push, pull_request]

jobs:
  update-env:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install auto-env-generator
        run: cargo install auto-env-generator
      - name: Generate .env.example
        run: autoenv generate -o .env.example
      - name: Check for changes
        run: git diff --exit-code .env.example
```

### Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: auto-env-generator
        name: Update .env.example
        entry: autoenv generate -o .env.example
        language: system
        files: '\.rs$'
        pass_filenames: false
```

### Build Scripts

Use in `build.rs`:

```rust
fn main() {
    auto_env_generator::generate_env_file(".").unwrap();
    println!("cargo:rerun-if-changed=src/");
}
```

## Examples

### Basic Web Server

For a typical web application:

```rust
// src/main.rs
use std::env;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").unwrap_or_default();
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // ... rest of your app
}
```

Running `autoenv generate` produces:

```bash
# Auto-generated environment variables
# Add your values below

DATABASE_URL=
JWT_SECRET=
PORT=
REDIS_URL=
```

### With Configuration

Create `autoenv.toml`:

```toml
output = ".env.example"
merge_existing = true
ignore = ["HOME", "PATH", "USER"]
```

Running `autoenv generate` will ignore system variables and create `.env.example`.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
git clone https://github.com/yourusername/auto-env-generator
cd auto-env-generator
cargo build
cargo test
cargo bench
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# Benchmarks
cargo bench

# Test CLI
cargo run --bin autoenv -- generate --help
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Alternatives

- **Manual scanning**: Grep-based approaches are slower and less accurate
- **IDE extensions**: Limited to specific editors and don't integrate with CI/CD
- **Custom scripts**: Brittle and require maintenance

Auto Environment Generator provides a robust, fast, and reliable solution that integrates seamlessly into any Rust workflow.
