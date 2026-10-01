# Using rust-branch with Your Existing Project

Quick guide to integrate rust-branch into your project.

## Step 1: Add rust-branch

Add to your `Cargo.toml`:

```toml
[build-dependencies]
rust-branch = { git = "https://github.com/Karam2B/linked-ql", package = "rust-branch" }

[features]
feature-less-cloning = []
```

## Step 2: Create build.rs

Create `build.rs` in your project root:

```rust
fn main() {
    rust_branch::process("feature-less-cloning");
}
```

**That's the only line you need!**

## Step 3: Create Feature Directory

```bash
mkdir -p src/feature-less-cloning
```

## Step 4: Add Alternative Implementation

Copy a module you want to replace:

```bash
cp -r src/operations src/feature-less-cloning/
```

Then edit `src/feature-less-cloning/operations/*` with your alternative implementation.

## Step 5: Use Conditional Compilation in lib.rs

```rust
#[cfg(not(feature = "feature-less-cloning"))]
pub mod operations;

#[cfg(feature = "feature-less-cloning")]
#[path = "feature-less-cloning/operations/mod.rs"]
pub mod operations;
```

Or use a macro if you have one:

```rust
feature_mod!(operations, "feature-less-cloning");
```

## Step 6: Build

```bash
# Default implementation
cargo build

# Feature implementation
cargo build --features feature-less-cloning
```

## Done!

rust-branch will:
- ✅ Detect when the feature is enabled
- ✅ Validate the feature directory exists  
- ✅ Parse module replacement directives
- ✅ Provide build warnings
- ✅ Work automatically with all cargo commands

## Troubleshooting

**Build warning "Feature 'X' is enabled" not showing?**
- The feature is not enabled - add `--features X` to your cargo command

**"Feature directory does not exist" error?**
- Create the directory: `mkdir -p src/your-feature-name`

**Changes not taking effect?**
- Run `cargo clean` and rebuild

## Advanced: Module Replacements

In `src/feature-less-cloning/mod.rs`:

```rust
#[path = "operations::fetch::Config"]
mod Config {
    pub const USE_CLONING: bool = false;
}
```

This replaces just the `Config` module in `src/operations/fetch.rs` without replacing the entire file.

## Next Steps

- Read [EXAMPLES.md](EXAMPLES.md) for complete working examples
- Check [README.md](README.md) for full API documentation
- See the parent repository for real-world usage
