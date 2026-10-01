# rust-branch Examples

## Basic Example

This example demonstrates the simplest usage of rust-branch.

### Project Structure

```
my-project/
├── Cargo.toml
├── build.rs
└── src/
    ├── lib.rs
    ├── operations/
    │   └── mod.rs
    └── feature-less-cloning/
        └── operations/
            └── mod.rs
```

### `Cargo.toml`

```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"

[build-dependencies]
rust-branch = "0.1"

[features]
feature-less-cloning = []
```

### `build.rs`

```rust
fn main() {
    rust_branch::process("feature-less-cloning");
}
```

That's it! Just one line.

### `src/lib.rs`

```rust
// Conditional module loading
#[cfg(not(feature = "feature-less-cloning"))]
pub mod operations;

#[cfg(feature = "feature-less-cloning")]
#[path = "feature-less-cloning/operations/mod.rs"]
pub mod operations;

// Or use with a macro (if you have one):
// feature_mod!(operations, "feature-less-cloning");
```

### `src/operations/mod.rs` (Original)

```rust
pub fn process_data(data: Vec<i32>) -> Vec<i32> {
    // Original implementation uses cloning
    let cloned = data.clone();
    cloned.into_iter().map(|x| x * 2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process() {
        let result = process_data(vec![1, 2, 3]);
        assert_eq!(result, vec![2, 4, 6]);
    }
}
```

### `src/feature-less-cloning/operations/mod.rs` (Alternative)

```rust
pub fn process_data(data: Vec<i32>) -> Vec<i32> {
    // Feature implementation avoids cloning
    data.into_iter().map(|x| x * 2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process() {
        let result = process_data(vec![1, 2, 3]);
        assert_eq!(result, vec![2, 4, 6]);
    }
}
```

### Compiling

```bash
# Use original implementation (with cloning)
$ cargo build
   Compiling my-project v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)

# Use feature implementation (without cloning)
$ cargo build --features feature-less-cloning
   Compiling my-project v0.1.0
    warning: rust-branch: Feature 'feature-less-cloning' is enabled
    Finished dev [unoptimized + debuginfo] target(s)
```

### Testing Both Variants

```bash
cargo test
cargo test --features feature-less-cloning
```

## Advanced Example: Multiple Features

### `build.rs`

```rust
fn main() {
    // Process multiple features
    rust_branch::process("feature-less-cloning");
    rust_branch::process("feature-optimized");
    
    // Or with configuration
    rust_branch::Config::new("feature-experimental")
        .with_src_dir("src")
        .process();
}
```

### `Cargo.toml`

```toml
[features]
less-cloning = []
optimized = []
experimental = []

# Combined features
all-features = ["less-cloning", "optimized", "experimental"]
```

### Module Structure

```
src/
├── operations/
│   └── mod.rs
├── feature-less-cloning/
│   └── operations/
│       └── mod.rs
├── feature-optimized/
│   └── operations/
│       └── mod.rs
└── feature-experimental/
    └── operations/
        └── mod.rs
```

### Testing All Combinations

```bash
# Create a shell script to test all combinations
for features in "" "less-cloning" "optimized" "less-cloning,optimized"; do
    echo "Testing with features: $features"
    cargo test --features "$features"
done
```

## Module Replacement Example

### `src/feature-less-cloning/mod.rs`

```rust
// Replace specific modules within files
#[path = "operations::fetch::Config"]
mod Config {
    pub const BUFFER_SIZE: usize = 2048;
    pub const USE_CLONING: bool = false;
}

#[path = "utils::helpers::Settings"]
mod Settings {
    pub type ConnectionType = String;
    
    pub fn default_timeout() -> u64 {
        30000  // Different timeout for this feature
    }
}
```

This replaces:
- The `Config` module in `src/operations/fetch.rs`
- The `Settings` module in `src/utils/helpers.rs`

Without modifying those files directly!

## CI/CD Example

### GitHub Actions

```yaml
name: Test All Features

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features:
          - ""
          - "feature-less-cloning"
          - "feature-optimized"
          - "feature-less-cloning,feature-optimized"
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Test
        run: |
          if [ -z "${{ matrix.features }}" ]; then
            cargo test
          else
            cargo test --features ${{ matrix.features }}
          fi
```

## Benchmarking Different Implementations

### `benches/compare.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use my_project::operations;

fn benchmark_operations(c: &mut Criterion) {
    let data = vec![1, 2, 3, 4, 5];
    
    c.bench_function("process_data", |b| {
        b.iter(|| operations::process_data(black_box(data.clone())))
    });
}

criterion_group!(benches, benchmark_operations);
criterion_main!(benches);
```

Run benchmarks for each feature:

```bash
cargo bench
cargo bench --features feature-less-cloning
cargo bench --features feature-optimized
```

## Real-World Example: Database Backends

```rust
// build.rs
fn main() {
    rust_branch::process("postgres");
    rust_branch::process("sqlite");
    rust_branch::process("mysql");
}
```

```toml
[features]
default = ["sqlite"]
postgres = []
sqlite = []
mysql = []
```

```
src/
├── database/
│   └── mod.rs         # Generic interface
├── feature-postgres/
│   └── database/
│       └── mod.rs     # PostgreSQL implementation
├── feature-sqlite/
│   └── database/
│       └── mod.rs     # SQLite implementation
└── feature-mysql/
    └── database/
        └── mod.rs     # MySQL implementation
```

Users of your library can then choose their backend:

```bash
cargo build --features postgres
cargo build --features mysql
```
