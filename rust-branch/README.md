# rust-branch

Automatic feature-based conditional compilation for Rust.

`rust-branch` allows you to maintain multiple implementations of modules and automatically select which one to compile based on Cargo features, without deleting or modifying your original code.

## Quick Start

### 1. Add to your `Cargo.toml`:

```toml
[build-dependencies]
rust-branch = { path = "../rust-branch" }

[features]
feature-less-cloning = []
```

### 2. Create a `build.rs`:

```rust
fn main() {
    rust_branch::process("feature-less-cloning");
}
```

### 3. Create feature directory structure:

```
src/
├── operations/              # Original implementation
│   └── mod.rs
└── feature-less-cloning/    # Alternative implementation
    └── operations/
        └── mod.rs
```

### 4. Use conditional module loading in `lib.rs`:

```rust
#[cfg(not(feature = "feature-less-cloning"))]
pub mod operations;

#[cfg(feature = "feature-less-cloning")]
#[path = "feature-less-cloning/operations/mod.rs"]
pub mod operations;
```

Or use the macro from `linked-sql-macros`:

```rust
linked_sql_macros::feature_mod!(operations, "feature-less-cloning");
```

### 5. Compile:

```bash
# Use original implementation
cargo build

# Use feature implementation
cargo build --features feature-less-cloning
```

## How It Works

When you call `rust_branch::process("feature-name")` in your build script:

1. It checks if the feature is enabled via `CARGO_FEATURE_*` environment variables
2. It validates that the feature directory exists
3. It parses `mod.rs` in the feature directory for module replacement directives
4. It provides build warnings and validation

The actual module selection happens through standard Rust `#[cfg]` attributes in your source code.

## Module Replacement Syntax

In `src/feature-less-cloning/mod.rs`, you can define module replacements:

```rust
#[path = "operations::fetch_many::Config"]
mod Config {
    pub const USE_CLONING: bool = false;
    pub const BUFFER_SIZE: usize = 2048;
}
```

This indicates that when the feature is enabled, the `Config` module in `src/operations/fetch_many.rs` should use this alternative implementation.

## Advanced Configuration

```rust
fn main() {
    rust_branch::Config::new("feature-less-cloning")
        .with_src_dir("src")
        .process();
}
```

## Features

- ✅ **Automatic**: Works with `cargo build`, `cargo check`, `cargo test`, `cargo run`
- ✅ **Zero overhead**: Compile-time selection only
- ✅ **No code deletion**: Original and feature implementations coexist
- ✅ **Type-safe**: Compiler validates all variants
- ✅ **Simple API**: One line in `build.rs`
- ✅ **Cross-platform**: Works on Windows, Linux, macOS

## Examples

See the parent repository for complete examples of using rust-branch.

## License

MIT
