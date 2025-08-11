//! Auto Environment Generator CLI
//!
//! A command-line tool for automatically scanning Rust projects and generating .env files
//! based on detected environment variable usage.

use anyhow::{Context, Result};
use auto_env_generator::{Config, EnvScanner};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "autoenv")]
#[command(about = "Automatically generate .env files from Rust source code")]
#[command(version = "0.1.0")]
#[command(long_about = "
Auto Environment Generator scans your Rust project for environment variable usage
and generates .env files with all detected variables. It uses parallel processing
and efficient pattern matching to handle large codebases quickly.

Examples:
  autoenv generate                    # Scan current directory
  autoenv generate ./my-project      # Scan specific directory
  autoenv generate -o .env.example   # Generate .env.example file
  autoenv scan                       # Just list found variables
  autoenv config                     # Show current configuration
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate .env file by scanning Rust source files
    Generate {
        /// Directory to scan (default: current directory)
        #[arg(value_name = "DIRECTORY")]
        path: Option<PathBuf>,

        /// Output file name (default: .env)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Configuration file path
        #[arg(short, long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        /// Don't merge with existing file (overwrite instead)
        #[arg(long)]
        no_merge: bool,

        /// Variables to ignore (can be used multiple times)
        #[arg(long, value_name = "VARIABLE")]
        ignore: Vec<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Scan and list found environment variables without generating file
    Scan {
        /// Directory to scan (default: current directory)
        #[arg(value_name = "DIRECTORY")]
        path: Option<PathBuf>,

        /// Configuration file path
        #[arg(short, long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        /// Variables to ignore (can be used multiple times)
        #[arg(long, value_name = "VARIABLE")]
        ignore: Vec<String>,

        /// Show file locations where variables were found
        #[arg(long)]
        show_locations: bool,
    },

    /// Show current configuration
    Config {
        /// Configuration file path
        #[arg(short, long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },

    /// Create a sample configuration file
    InitConfig {
        /// Output path for config file (default: autoenv.toml)
        #[arg(value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            path,
            output,
            config,
            no_merge,
            ignore,
            verbose,
        } => {
            let scan_path = path.unwrap_or_else(|| PathBuf::from("."));

            if verbose {
                println!("Scanning directory: {}", scan_path.display());
            }

            // Load configuration
            let mut config_obj = if let Some(config_path) = config {
                if verbose {
                    println!("Loading config from: {}", config_path.display());
                }
                EnvScanner::load_config(config_path).context("Failed to load configuration file")?
            } else {
                // Try to load default config file if it exists
                let default_config = scan_path.join("autoenv.toml");
                if default_config.exists() {
                    if verbose {
                        println!("Loading default config: {}", default_config.display());
                    }
                    EnvScanner::load_config(default_config)?
                } else {
                    Config::default()
                }
            };

            // Override config with command line arguments
            if let Some(output_file) = output {
                config_obj.output = Some(output_file);
            }

            if no_merge {
                config_obj.merge_existing = Some(false);
            }

            if !ignore.is_empty() {
                let mut ignore_list = config_obj.ignore.unwrap_or_default();
                ignore_list.extend(ignore);
                config_obj.ignore = Some(ignore_list);
            }

            // Create scanner and scan directory
            let scanner = EnvScanner::with_config(config_obj.clone())?;

            if verbose {
                println!("Scanning for environment variables...");
            }

            let variables = scanner
                .scan_directory(&scan_path)
                .context("Failed to scan directory")?;

            if variables.is_empty() {
                println!("No environment variables found in Rust files.");
                return Ok(());
            }

            if verbose {
                println!("Found {} environment variables:", variables.len());
                let mut sorted_vars: Vec<_> = variables.iter().collect();
                sorted_vars.sort();
                for var in sorted_vars {
                    println!("  - {}", var);
                }
            }

            // Generate .env file
            let output_file = config_obj.output.unwrap_or_else(|| ".env".to_string());
            let output_path = scan_path.join(&output_file);

            scanner
                .generate_env_file(&variables, &output_path)
                .context("Failed to generate .env file")?;

            println!(
                "Generated {} with {} variables",
                output_file,
                variables.len()
            );
            if verbose {
                println!("Output path: {}", output_path.display());
            }

            Ok(())
        }

        Commands::Scan {
            path,
            config,
            ignore,
            show_locations,
        } => {
            let scan_path = path.unwrap_or_else(|| PathBuf::from("."));

            // Load configuration
            let mut config_obj = if let Some(config_path) = config {
                EnvScanner::load_config(config_path)?
            } else {
                let default_config = scan_path.join("autoenv.toml");
                if default_config.exists() {
                    EnvScanner::load_config(default_config)?
                } else {
                    Config::default()
                }
            };

            if !ignore.is_empty() {
                let mut ignore_list = config_obj.ignore.unwrap_or_default();
                ignore_list.extend(ignore);
                config_obj.ignore = Some(ignore_list);
            }

            let scanner = EnvScanner::with_config(config_obj)?;
            let variables = scanner.scan_directory(&scan_path)?;

            if variables.is_empty() {
                println!("No environment variables found in Rust files.");
                return Ok(());
            }

            println!("Found {} environment variables:", variables.len());
            let mut sorted_vars: Vec<_> = variables.iter().collect();
            sorted_vars.sort();

            for var in sorted_vars {
                if show_locations {
                    // TODO: Implement location tracking for detailed output
                    println!("  {}", var);
                } else {
                    println!("  {}", var);
                }
            }

            Ok(())
        }

        Commands::Config { config } => {
            let config_path = config.unwrap_or_else(|| PathBuf::from("autoenv.toml"));

            if config_path.exists() {
                let config_obj = EnvScanner::load_config(&config_path)?;
                println!("Configuration from: {}", config_path.display());
                println!();

                let toml_content = toml::to_string_pretty(&config_obj)
                    .context("Failed to serialize configuration")?;
                println!("{}", toml_content);
            } else {
                println!("Configuration file not found: {}", config_path.display());
                println!("Using default configuration:");
                println!();

                let default_config = Config::default();
                let toml_content = toml::to_string_pretty(&default_config)
                    .context("Failed to serialize default configuration")?;
                println!("{}", toml_content);

                println!();
                println!("To create a configuration file, run:");
                println!("  autoenv init-config");
            }

            Ok(())
        }

        Commands::InitConfig { output } => {
            let config_path = output.unwrap_or_else(|| PathBuf::from("autoenv.toml"));

            if config_path.exists() {
                println!(
                    "Configuration file already exists: {}",
                    config_path.display()
                );
                println!("Use --force to overwrite (not implemented yet)");
                return Ok(());
            }

            let default_config = Config {
                output: Some(".env".to_string()),
                merge_existing: Some(true),
                ignore: Some(vec![
                    // Common variables that might not need to be in .env
                    "HOME".to_string(),
                    "PATH".to_string(),
                    "USER".to_string(),
                ]),
            };

            let toml_content = toml::to_string_pretty(&default_config)
                .context("Failed to serialize configuration")?;

            std::fs::write(
                &config_path,
                format!(
                    "# Auto Environment Generator Configuration\n\
                 # See https://github.com/your-repo/auto-env-generator for documentation\n\n{}",
                    toml_content
                ),
            )
            .context("Failed to write configuration file")?;

            println!("Created configuration file: {}", config_path.display());
            println!();
            println!("Edit the file to customize your settings:");
            println!("  - output: Name of the generated file");
            println!("  - merge_existing: Whether to preserve existing values");
            println!("  - ignore: List of variables to skip");

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert()
    }

    #[test]
    fn test_generate_command_parsing() {
        let cmd = Cli::try_parse_from(&[
            "autoenv",
            "generate",
            "./test-dir",
            "--output",
            ".env.example",
            "--ignore",
            "DEBUG",
            "--ignore",
            "TEST_VAR",
            "--verbose",
        ]);

        assert!(cmd.is_ok());

        if let Commands::Generate {
            path,
            output,
            ignore,
            verbose,
            ..
        } = cmd.unwrap().command
        {
            assert_eq!(path, Some(PathBuf::from("./test-dir")));
            assert_eq!(output, Some(".env.example".to_string()));
            assert_eq!(ignore, vec!["DEBUG".to_string(), "TEST_VAR".to_string()]);
            assert!(verbose);
        } else {
            panic!("Expected Generate command");
        }
    }

    #[test]
    fn test_scan_command_parsing() {
        let cmd = Cli::try_parse_from(&["autoenv", "scan", "--show-locations"]);

        assert!(cmd.is_ok());

        if let Commands::Scan { show_locations, .. } = cmd.unwrap().command {
            assert!(show_locations);
        } else {
            panic!("Expected Scan command");
        }
    }
}
