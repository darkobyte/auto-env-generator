use auto_env_generator::{Config, EnvScanner};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_large_rust_file(path: &Path, name: &str, var_count: usize) -> std::io::Result<()> {
    let mut content = String::new();
    content.push_str("use std::env;\n\n");
    content.push_str("fn main() {\n");

    for i in 0..var_count {
        content.push_str(&format!(
            "    let var_{} = std::env::var(\"ENV_VAR_{}\").unwrap_or_default();\n",
            i, i
        ));
    }

    // Add some non-env variable code to make it realistic
    content.push_str("\n    // Some regular code\n");
    content.push_str("    let x = 42;\n");
    content.push_str("    let y = x * 2;\n");
    content.push_str("    println!(\"Result: {}\", y);\n");

    // Add some dotenv calls too
    for i in 0..(var_count / 4) {
        content.push_str(&format!(
            "    let dotenv_var_{} = dotenv::var(\"DOTENV_VAR_{}\").ok();\n",
            i, i
        ));
    }

    content.push_str("}\n");

    fs::write(path.join(name), content)
}

fn create_test_project(temp_dir: &Path, files: usize, vars_per_file: usize) -> std::io::Result<()> {
    // Create src directory
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Create main.rs
    create_large_rust_file(&src_dir, "main.rs", vars_per_file)?;

    // Create lib.rs
    create_large_rust_file(&src_dir, "lib.rs", vars_per_file)?;

    // Create multiple module files
    for i in 0..files.saturating_sub(2) {
        create_large_rust_file(&src_dir, &format!("module_{}.rs", i), vars_per_file)?;
    }

    // Create nested directories with files
    let nested_dir = src_dir.join("nested");
    fs::create_dir_all(&nested_dir)?;

    for i in 0..5 {
        create_large_rust_file(&nested_dir, &format!("nested_{}.rs", i), vars_per_file / 2)?;
    }

    // Create tests directory
    let tests_dir = temp_dir.join("tests");
    fs::create_dir_all(&tests_dir)?;

    for i in 0..3 {
        create_large_rust_file(&tests_dir, &format!("test_{}.rs", i), vars_per_file / 3)?;
    }

    // Create examples directory
    let examples_dir = temp_dir.join("examples");
    fs::create_dir_all(&examples_dir)?;

    for i in 0..2 {
        create_large_rust_file(
            &examples_dir,
            &format!("example_{}.rs", i),
            vars_per_file / 4,
        )?;
    }

    // Create target directory (should be ignored)
    let target_dir = temp_dir.join("target").join("debug");
    fs::create_dir_all(&target_dir)?;
    create_large_rust_file(&target_dir, "ignored.rs", vars_per_file)?;

    Ok(())
}

fn bench_scan_small_project(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path(), 5, 10).unwrap();

    let scanner = EnvScanner::new().unwrap();

    c.bench_function("scan_small_project", |b| {
        b.iter(|| {
            let variables = scanner.scan_directory(black_box(temp_dir.path())).unwrap();
            black_box(variables);
        })
    });
}

fn bench_scan_medium_project(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path(), 20, 25).unwrap();

    let scanner = EnvScanner::new().unwrap();

    c.bench_function("scan_medium_project", |b| {
        b.iter(|| {
            let variables = scanner.scan_directory(black_box(temp_dir.path())).unwrap();
            black_box(variables);
        })
    });
}

fn bench_scan_large_project(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path(), 50, 50).unwrap();

    let scanner = EnvScanner::new().unwrap();

    c.bench_function("scan_large_project", |b| {
        b.iter(|| {
            let variables = scanner.scan_directory(black_box(temp_dir.path())).unwrap();
            black_box(variables);
        })
    });
}

fn bench_scan_by_file_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_by_file_count");

    for file_count in [10, 25, 50, 100].iter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), *file_count, 20).unwrap();

        let scanner = EnvScanner::new().unwrap();

        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            file_count,
            |b, _| {
                b.iter(|| {
                    let variables = scanner.scan_directory(black_box(temp_dir.path())).unwrap();
                    black_box(variables);
                })
            },
        );
    }
    group.finish();
}

fn bench_scan_by_vars_per_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_by_vars_per_file");

    for var_count in [5, 20, 50, 100, 200].iter() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), 20, *var_count).unwrap();

        let scanner = EnvScanner::new().unwrap();

        group.throughput(Throughput::Elements(*var_count as u64 * 20));
        group.bench_with_input(BenchmarkId::from_parameter(var_count), var_count, |b, _| {
            b.iter(|| {
                let variables = scanner.scan_directory(black_box(temp_dir.path())).unwrap();
                black_box(variables);
            })
        });
    }
    group.finish();
}

fn bench_single_file_scanning(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    // Create a file with various patterns
    let content = r#"
use std::env;
use dotenv;

fn main() {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let api_key = env::var("API_KEY").unwrap();
    let debug = dotenv::var("DEBUG_MODE").unwrap_or_default();
    let port = std::env::var_os("PORT").unwrap();
    let host = env::var_os("HOST").unwrap();
    let secret = dotenv::var_os("SECRET_KEY").unwrap();

    // Some non-env code
    let x = 42;
    let y = format!("Hello {}", x);
    println!("{}", y);

    // More env vars
    let cache_url = std::env::var("CACHE_URL").ok();
    let log_level = env::var("LOG_LEVEL").unwrap_or("info".to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let test_var = std::env::var("TEST_VARIABLE").unwrap();
        assert!(!test_var.is_empty());
    }
}
"#;

    fs::write(&file_path, content).unwrap();

    let scanner = EnvScanner::new().unwrap();

    c.bench_function("single_file_scan", |b| {
        b.iter(|| {
            let variables = scanner.scan_directory(black_box(temp_dir.path())).unwrap();
            black_box(variables);
        })
    });
}

fn bench_pattern_matching_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_matching");

    let test_line = r#"    let db_url = std::env::var("DATABASE_URL").unwrap();"#;

    // Benchmark our Aho-Corasick + Regex approach
    let _scanner = EnvScanner::new().unwrap();
    group.bench_function("aho_corasick_regex", |b| {
        b.iter(|| {
            // Simulate what happens in our scanner
            let patterns =
                aho_corasick::AhoCorasick::new(&["std::env::var(", "env::var(", "dotenv::var("])
                    .unwrap();

            if patterns.is_match(black_box(test_line)) {
                let regex =
                    regex::Regex::new(r#"(?:std::env::var|env::var|dotenv::var)\s*\(\s*"([^"]+)""#)
                        .unwrap();
                for cap in regex.captures_iter(test_line) {
                    if let Some(var_name) = cap.get(1) {
                        black_box(var_name.as_str());
                    }
                }
            }
        })
    });

    // Compare with regex-only approach
    group.bench_function("regex_only", |b| {
        b.iter(|| {
            let regex =
                regex::Regex::new(r#"(?:std::env::var|env::var|dotenv::var)\s*\(\s*"([^"]+)""#)
                    .unwrap();
            for cap in regex.captures_iter(black_box(test_line)) {
                if let Some(var_name) = cap.get(1) {
                    black_box(var_name.as_str());
                }
            }
        })
    });

    // Compare with simple string contains
    group.bench_function("string_contains", |b| {
        b.iter(|| {
            let line = black_box(test_line);
            if line.contains("env::var(") {
                // Simple extraction (less accurate but faster)
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        black_box(&line[start + 1..start + 1 + end]);
                    }
                }
            }
        })
    });

    group.finish();
}

fn bench_env_file_generation(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let scanner = EnvScanner::new().unwrap();

    // Create a large set of variables
    let mut variables = std::collections::HashSet::new();
    for i in 0..1000 {
        variables.insert(format!("VAR_{}", i));
    }

    c.bench_function("generate_env_file_1000_vars", |b| {
        b.iter(|| {
            let output_path = temp_dir
                .path()
                .join(format!("test_{}.env", rand::random::<u32>()));
            scanner
                .generate_env_file(black_box(&variables), &output_path)
                .unwrap();
        })
    });
}

fn bench_config_with_ignore_list(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path(), 30, 30).unwrap();

    let mut group = c.benchmark_group("scan_with_ignore_list");

    // Benchmark with no ignore list
    let config_no_ignore = Config::default();
    let scanner_no_ignore = EnvScanner::with_config(config_no_ignore).unwrap();

    group.bench_function("no_ignore", |b| {
        b.iter(|| {
            let variables = scanner_no_ignore
                .scan_directory(black_box(temp_dir.path()))
                .unwrap();
            black_box(variables);
        })
    });

    // Benchmark with small ignore list
    let config_small_ignore = Config {
        ignore: Some(vec!["DEBUG".to_string(), "TEST".to_string()]),
        ..Default::default()
    };
    let scanner_small_ignore = EnvScanner::with_config(config_small_ignore).unwrap();

    group.bench_function("small_ignore_list", |b| {
        b.iter(|| {
            let variables = scanner_small_ignore
                .scan_directory(black_box(temp_dir.path()))
                .unwrap();
            black_box(variables);
        })
    });

    // Benchmark with large ignore list
    let large_ignore_list: Vec<String> = (0..100).map(|i| format!("IGNORE_{}", i)).collect();
    let config_large_ignore = Config {
        ignore: Some(large_ignore_list),
        ..Default::default()
    };
    let scanner_large_ignore = EnvScanner::with_config(config_large_ignore).unwrap();

    group.bench_function("large_ignore_list", |b| {
        b.iter(|| {
            let variables = scanner_large_ignore
                .scan_directory(black_box(temp_dir.path()))
                .unwrap();
            black_box(variables);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_scan_small_project,
    bench_scan_medium_project,
    bench_scan_large_project,
    bench_scan_by_file_count,
    bench_scan_by_vars_per_file,
    bench_single_file_scanning,
    bench_pattern_matching_comparison,
    bench_env_file_generation,
    bench_config_with_ignore_list,
);

criterion_main!(benches);
