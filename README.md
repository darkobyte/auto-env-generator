# autoenv-gen
A Rust tool that scans your project source code for environment variable usage and automatically generates a .env file with the required keys.
Never forget a missing environment variable again!

## ✨ Features
- Automatic detection of std::env::var() and dotenv::var() calls in your Rust code.
- Generates .env or .env.example with placeholder values.
- Prevents missing variable errors at runtime.
- CLI integration for easy use.
- Optionally merges with existing .env files without overwriting values.
- Supports ignoring variables via config file.
