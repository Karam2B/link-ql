use std::path::Path;

/// Generate conditional compilation code for module replacements
///
/// This module provides utilities for generating Rust code that conditionally
/// compiles different module implementations based on enabled features.

/// Generate cfg attribute for a feature
pub fn cfg_feature(feature_name: &str) -> String {
    format!(r#"#[cfg(feature = "{}")]"#, feature_name)
}

/// Generate cfg attribute for NOT having a feature
pub fn cfg_not_feature(feature_name: &str) -> String {
    format!(r#"#[cfg(not(feature = "{}"))]"#, feature_name)
}

/// Generate a module path attribute
pub fn path_attribute(path: &str) -> String {
    format!(r#"#[path = "{}"]"#, path)
}

/// Generate conditional module declaration
///
/// # Example
///
/// ```rust,ignore
/// let code = conditional_module("operations", "feature-less-cloning", "feature-less-cloning/operations.rs");
/// ```
///
/// Generates:
///
/// ```rust,ignore
/// #[cfg(not(feature = "feature-less-cloning"))]
/// pub mod operations;
///
/// #[cfg(feature = "feature-less-cloning")]
/// #[path = "feature-less-cloning/operations.rs"]
/// pub mod operations;
/// ```
pub fn conditional_module(module_name: &str, feature_name: &str, feature_path: &str) -> String {
    format!(
        r#"{cfg_not}
pub mod {module};

{cfg}
{path}
pub mod {module};"#,
        cfg_not = cfg_not_feature(feature_name),
        cfg = cfg_feature(feature_name),
        path = path_attribute(feature_path),
        module = module_name
    )
}

/// Convert a path to use forward slashes (for cross-platform compatibility)
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cfg_feature() {
        assert_eq!(
            cfg_feature("test-feature"),
            r#"#[cfg(feature = "test-feature")]"#
        );
    }
    
    #[test]
    fn test_cfg_not_feature() {
        assert_eq!(
            cfg_not_feature("test-feature"),
            r#"#[cfg(not(feature = "test-feature"))]"#
        );
    }
    
    #[test]
    fn test_path_attribute() {
        assert_eq!(
            path_attribute("feature/mod.rs"),
            r#"#[path = "feature/mod.rs"]"#
        );
    }
    
    #[test]
    fn test_conditional_module() {
        let code = conditional_module("operations", "my-feature", "my-feature/operations.rs");
        
        assert!(code.contains(r#"#[cfg(not(feature = "my-feature"))]"#));
        assert!(code.contains(r#"#[cfg(feature = "my-feature")]"#));
        assert!(code.contains(r#"#[path = "my-feature/operations.rs"]"#));
        assert!(code.contains("pub mod operations;"));
    }
}
