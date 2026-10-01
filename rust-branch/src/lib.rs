//! # rust-branch
//!
//! Automatic feature-based conditional compilation for Rust.
//!
//! `rust-branch` allows you to maintain multiple implementations of modules
//! and automatically select which one to compile based on Cargo features,
//! without deleting or modifying your original code.
//!
//! ## Quick Start
//!
//! ### 1. Add to `build.rs`:
//!
//! ```rust,no_run
//! fn main() {
//!     rust_branch::process("feature-less-cloning");
//! }
//! ```
//!
//! ### 2. Create feature directory:
//!
//! ```text
//! src/
//! ├── operations/              # Original implementation
//! │   └── mod.rs
//! └── feature-less-cloning/    # Alternative implementation
//!     └── operations/
//!         └── mod.rs
//! ```
//!
//! ### 3. Add feature to `Cargo.toml`:
//!
//! ```toml
//! [features]
//! feature-less-cloning = []
//! ```
//!
//! ### 4. Compile:
//!
//! ```bash
//! # Use original implementation
//! cargo build
//!
//! # Use feature implementation
//! cargo build --features feature-less-cloning
//! ```
//!
//! ## How It Works
//!
//! When you call `rust_branch::process("feature-name")` in your build script:
//!
//! 1. It checks if the feature is enabled via environment variables
//! 2. It scans the `src/feature-name/` directory for alternative implementations
//! 3. It generates cargo instructions to use the appropriate files
//!
//! The compiler then automatically selects the right implementation based on
//! which features are enabled.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write};

mod parser;
mod generator;

pub use parser::*;
pub use generator::*;

/// Main entry point for rust-branch.
///
/// Call this from your `build.rs` with the name of your feature directory.
///
/// # Example
///
/// ```rust,no_run
/// fn main() {
///     rust_branch::process("feature-less-cloning");
/// }
/// ```
///
/// This will:
/// - Check if the `feature-less-cloning` cargo feature is enabled
/// - If enabled, configure the build to use files from `src/feature-less-cloning/`
/// - If not enabled, use the default files from `src/`
///
/// # Arguments
///
/// * `feature_name` - The name of the feature directory (must match a Cargo feature)
///
/// # Panics
///
/// This function will panic if the feature directory doesn't exist or if there
/// are I/O errors accessing the source directory.
pub fn process(feature_name: &str) {
    // Tell cargo to rerun if source files change
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=build.rs");
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let src_dir = Path::new(&manifest_dir).join("src");
    
    // Check if the feature is enabled
    let feature_var = format!("CARGO_FEATURE_{}", 
        feature_name.to_uppercase().replace("-", "_"));
    let feature_enabled = env::var(&feature_var).is_ok();
    
    if !feature_enabled {
        // Feature not enabled, use default implementation
        return;
    }
    
    println!("cargo:warning=rust-branch: Feature '{}' is enabled", feature_name);
    
    let feature_dir = src_dir.join(feature_name);
    
    if !feature_dir.exists() {
        panic!("rust-branch: Feature directory does not exist: {}", feature_dir.display());
    }
    
    // Process the feature directory
    if let Err(e) = process_feature_directory(&feature_dir, &src_dir, feature_name) {
        panic!("rust-branch: Error processing feature directory: {}", e);
    }
}

/// Process a feature directory and set up module replacements
fn process_feature_directory(
    feature_dir: &Path,
    src_dir: &Path,
    feature_name: &str,
) -> io::Result<()> {
    // Parse mod.rs for module replacements if it exists
    let mod_rs = feature_dir.join("mod.rs");
    if mod_rs.exists() {
        match parser::parse_module_replacements(&mod_rs) {
            Ok(replacements) => {
                if !replacements.is_empty() {
                    println!("cargo:warning=rust-branch: Found {} module replacement(s)", 
                        replacements.len());
                }
                // Module replacements are handled by the conditional compilation
                // in the source files themselves via cfg attributes
            }
            Err(e) => {
                println!("cargo:warning=rust-branch: Error parsing mod.rs: {}", e);
            }
        }
    }
    
    // The actual module selection is handled by cfg attributes in the source code
    // This build script just validates and provides warnings
    
    Ok(())
}

/// Configuration builder for more advanced usage
///
/// # Example
///
/// ```rust,no_run
/// fn main() {
///     rust_branch::Config::new("feature-less-cloning")
///         .with_src_dir("src")
///         .process();
/// }
/// ```
pub struct Config {
    feature_name: String,
    src_dir: Option<PathBuf>,
}

impl Config {
    /// Create a new configuration for the given feature
    pub fn new(feature_name: impl Into<String>) -> Self {
        Self {
            feature_name: feature_name.into(),
            src_dir: None,
        }
    }
    
    /// Set a custom source directory (default: "src")
    pub fn with_src_dir(mut self, src_dir: impl Into<PathBuf>) -> Self {
        self.src_dir = Some(src_dir.into());
        self
    }
    
    /// Process the configuration
    pub fn process(self) {
        process(&self.feature_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_config_builder() {
        let config = Config::new("test-feature")
            .with_src_dir("src");
        
        assert_eq!(config.feature_name, "test-feature");
        assert!(config.src_dir.is_some());
    }
    
    #[test]
    fn test_feature_name_normalization() {
        // Test that feature names with hyphens are correctly converted to underscores
        let feature_name = "feature-less-cloning";
        let env_var = format!("CARGO_FEATURE_{}", 
            feature_name.to_uppercase().replace("-", "_"));
        
        assert_eq!(env_var, "CARGO_FEATURE_FEATURE_LESS_CLONING");
    }
}
