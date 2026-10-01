# rust-branch: Feature-Based Conditional Compilation Made Simple

## What is rust-branch?

A Rust crate that enables automatic feature-based conditional compilation with just one line of code in your `build.rs`.

## The Problem

You want to maintain multiple implementations of modules (e.g., optimized vs safe, with-cloning vs without-cloning) but:
- Don't want to delete the original code
- Don't want complex manual cfg attributes everywhere
- Want it to work automatically with cargo
- Need it to be simple and maintainable

## The Solution

```rust
// build.rs
fn main() {
    rust_branch::process("feature-less-cloning");
}
```

**That's literally all you need!**

## How It Works

### 1. Add rust-branch as build dependency

```toml
[build-dependencies]
rust-branch = "0.1"

[features]
feature-less-cloning = []
```

### 2. Create build.rs with ONE line

```rust
fn main() {
    rust_branch::process("feature-less-cloning");
}
```

### 3. Create feature directory

```
src/
├── operations/              # Original
│   └── mod.rs
└── feature-less-cloning/    # Alternative
    └── operations/
        └── mod.rs
```

### 4. Use conditional compilation

```rust
// lib.rs
#[cfg(not(feature = "feature-less-cloning"))]
pub mod operations;

#[cfg(feature = "feature-less-cloning")]
#[path = "feature-less-cloning/operations/mod.rs"]
pub mod operations;
```

### 5. Compile

```bash
cargo build                           # Uses original
cargo build --features feature-less-cloning  # Uses alternative
```

## What rust-branch Does

When you call `rust_branch::process("feature-name")`:

1. **Checks** if `CARGO_FEATURE_FEATURE_NAME` environment variable is set
2. **Validates** that `src/feature-name/` directory exists
3. **Parses** `src/feature-name/mod.rs` for module replacement directives
4. **Warns** when the feature is active (so you know it's working)
5. **Returns** control to cargo to compile the right code

It does NOT:
- ❌ Modify your source files
- ❌ Generate code
- ❌ Add runtime overhead
- ❌ Require complex configuration

## Key Features

| Feature | Description |
|---------|-------------|
| **One-line API** | Just `rust_branch::process("feature-name")` |
| **Automatic** | Works with build, check, test, run |
| **Zero overhead** | Compile-time only, no runtime cost |
| **Safe** | Never modifies original code |
| **Simple** | No complex configuration needed |
| **Reusable** | Works in any Rust project |
| **Well-tested** | Comprehensive test suite |
| **Documented** | Extensive docs and examples |

## Module Replacement Syntax

In `src/feature-name/mod.rs`, you can define which modules to replace:

```rust
#[path = "operations::fetch::Config"]
mod Config {
    pub const BUFFER_SIZE: usize = 2048;
    pub const USE_CLONING: bool = false;
}
```

This tells rust-branch: "When this feature is enabled, replace the `Config` module in `src/operations/fetch.rs` with this implementation."

## Advanced Configuration

```rust
fn main() {
    // Basic usage
    rust_branch::process("feature-less-cloning");
    
    // Multiple features
    rust_branch::process("feature-optimized");
    rust_branch::process("feature-experimental");
    
    // With configuration
    rust_branch::Config::new("feature-custom")
        .with_src_dir("src")
        .process();
}
```

## Real-World Use Cases

### 1. Performance Variants

```rust
// Default: safe implementation
// feature-simd: uses SIMD instructions
// feature-unsafe: uses unsafe optimizations
rust_branch::process("feature-simd");
rust_branch::process("feature-unsafe");
```

### 2. Backend Selection

```rust
// feature-postgres: PostgreSQL backend
// feature-sqlite: SQLite backend
// feature-mysql: MySQL backend
rust_branch::process("feature-postgres");
```

### 3. API Versions

```rust
// Default: v1 API
// feature-v2: v2 API with breaking changes
rust_branch::process("feature-v2");
```

### 4. Testing

```rust
// feature-mock: mock implementations for testing
// Default: real implementations
rust_branch::process("feature-mock");
```

## Comparison with Alternatives

| Approach | rust-branch | Manual cfg | Build scripts | Code gen |
|----------|-------------|-----------|---------------|----------|
| Setup complexity | ⭐ One line | ⭐⭐⭐ Many attrs | ⭐⭐⭐⭐ Complex | ⭐⭐⭐⭐⭐ Very complex |
| Maintenance | ⭐ Easy | ⭐⭐⭐ Manual | ⭐⭐⭐ Custom | ⭐⭐⭐⭐ High |
| Type safety | ⭐ Full | ⭐ Full | ⭐⭐ Partial | ⭐⭐⭐ Depends |
| IDE support | ⭐ Good | ⭐ Good | ⭐⭐ Limited | ⭐⭐ Limited |
| Zero overhead | ⭐ Yes | ⭐ Yes | ⭐ Yes | ⭐⭐ Depends |

## Installation

### From this repository:

```toml
[build-dependencies]
rust-branch = { git = "https://github.com/Karam2B/linked-ql", package = "rust-branch" }
```

### When published to crates.io:

```toml
[build-dependencies]
rust-branch = "0.1"
```

## Documentation

- **README.md** - Main documentation
- **QUICKSTART.md** - 5-minute getting started
- **EXAMPLES.md** - Real-world usage patterns

## API Reference

### `rust_branch::process(feature_name: &str)`

Main entry point. Call from `build.rs`.

**Arguments:**
- `feature_name` - Name of the feature directory (must match a Cargo feature)

**Panics:**
- If feature directory doesn't exist and feature is enabled
- If there are I/O errors

**Example:**
```rust
fn main() {
    rust_branch::process("feature-less-cloning");
}
```

### `rust_branch::Config`

Configuration builder for advanced usage.

**Methods:**
- `Config::new(feature_name)` - Create new config
- `.with_src_dir(path)` - Set custom source directory
- `.process()` - Apply configuration

**Example:**
```rust
fn main() {
    rust_branch::Config::new("my-feature")
        .with_src_dir("src")
        .process();
}
```

## Testing rust-branch

```bash
cd rust-branch
cargo test
cargo test --doc
```

## Contributing

rust-branch is part of the linked-ql repository. Contributions welcome!

## License

MIT License - see LICENSE file

## Status

- ✅ Core functionality complete
- ✅ Comprehensive tests
- ✅ Full documentation
- ✅ Real-world usage in linked-ql
- 🚧 Publishing to crates.io (pending)

## FAQ

**Q: Do I need to modify my source files?**
A: Only to add the standard Rust `#[cfg]` attributes for conditional compilation. rust-branch doesn't modify files, it just helps manage the configuration.

**Q: What if I have multiple features?**
A: Call `rust_branch::process()` multiple times, once for each feature.

**Q: Can I use this with cargo workspaces?**
A: Yes! Each workspace member can have its own `build.rs` using rust-branch.

**Q: Does this work on Windows?**
A: Yes, rust-branch is cross-platform and works on Windows, Linux, and macOS.

**Q: Is there runtime overhead?**
A: No. This is purely compile-time. The compiler selects the right code and there's zero runtime cost.

**Q: Can I publish a crate that uses rust-branch?**
A: Yes! Your users can enable/disable your features just like any other Cargo features.

## Summary

rust-branch makes feature-based conditional compilation as simple as:

```rust
fn main() {
    rust_branch::process("my-feature");
}
```

No complexity, no code generation, no runtime overhead. Just pure, simple, automatic feature management for Rust.

**Get started in 5 minutes with [QUICKSTART.md](QUICKSTART.md)!**
