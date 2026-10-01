use std::fs;
use std::path::Path;
use std::io;

/// Represents a module replacement directive
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleReplacement {
    /// Path components in the format ["directory", "file", "submod"]
    pub path: Vec<String>,
    /// The new module content to insert
    pub content: String,
}

/// Parse module replacements from a feature's mod.rs file
///
/// # Example
///
/// Given a `mod.rs` file containing:
///
/// ```rust,ignore
/// #[path = "operations::fetch_many::Config"]
/// mod Config {
///     pub const BUFFER_SIZE: usize = 2048;
/// }
/// ```
///
/// This function will parse it into a `ModuleReplacement` with:
/// - `path`: `["operations", "fetch_many", "Config"]`
/// - `content`: The full module content as a string
pub fn parse_module_replacements(mod_rs_path: &Path) -> io::Result<Vec<ModuleReplacement>> {
    let content = fs::read_to_string(mod_rs_path)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut replacements = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Look for #[path = "..."] or #[parh = "..."] (support typo)
        if line.starts_with("#[path") || line.starts_with("#[parh") {
            if let Some(path_value) = extract_path_value(line) {
                let mut module_content = Vec::new();
                let mut brace_count = 0;
                let mut found_mod = false;
                
                // Skip to the next line with actual content
                i += 1;
                while i < lines.len() {
                    let current = lines[i].trim();
                    
                    // Look for module declaration (case-insensitive)
                    if current.to_lowercase().starts_with("mod ") {
                        found_mod = true;
                    }
                    
                    if found_mod {
                        module_content.push(lines[i]);
                        
                        // Count braces to find the end of the module
                        for ch in current.chars() {
                            match ch {
                                '{' => brace_count += 1,
                                '}' => brace_count -= 1,
                                _ => {}
                            }
                        }
                        
                        // If we've closed all braces, we're done
                        if brace_count == 0 && current.contains('}') {
                            break;
                        }
                    }
                    
                    i += 1;
                }
                
                if found_mod {
                    let path_parts: Vec<String> = path_value
                        .split("::")
                        .map(|s| s.to_string())
                        .collect();
                    
                    replacements.push(ModuleReplacement {
                        path: path_parts,
                        content: module_content.join("\n"),
                    });
                }
            }
        }
        
        i += 1;
    }
    
    Ok(replacements)
}

/// Extract the path value from a #[path = "..."] attribute
///
/// Supports both `#[path = "..."]` and `#[parh = "..."]` (typo)
fn extract_path_value(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')?;
    Some(line[start + 1..start + 1 + end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_extract_path_value() {
        assert_eq!(
            extract_path_value(r#"#[path = "directory::file::submod"]"#),
            Some("directory::file::submod".to_string())
        );
        
        assert_eq!(
            extract_path_value(r#"#[parh = "a::b::c"]"#),
            Some("a::b::c".to_string())
        );
        
        assert_eq!(
            extract_path_value(r#"#[path="no::spaces"]"#),
            Some("no::spaces".to_string())
        );
    }
    
    #[test]
    fn test_parse_module_replacements() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"#[path = "operations::fetch::Config"]"#).unwrap();
        writeln!(temp_file, "mod Config {{").unwrap();
        writeln!(temp_file, "    pub const SIZE: usize = 1024;").unwrap();
        writeln!(temp_file, "}}").unwrap();
        temp_file.flush().unwrap();
        
        let replacements = parse_module_replacements(temp_file.path()).unwrap();
        
        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].path, vec!["operations", "fetch", "Config"]);
        assert!(replacements[0].content.contains("pub const SIZE: usize = 1024;"));
    }
    
    #[test]
    fn test_parse_multiple_replacements() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"#[path = "mod1::Config"]"#).unwrap();
        writeln!(temp_file, "mod Config {{ }}").unwrap();
        writeln!(temp_file, "").unwrap();
        writeln!(temp_file, r#"#[path = "mod2::Settings"]"#).unwrap();
        writeln!(temp_file, "mod Settings {{ }}").unwrap();
        temp_file.flush().unwrap();
        
        let replacements = parse_module_replacements(temp_file.path()).unwrap();
        
        assert_eq!(replacements.len(), 2);
        assert_eq!(replacements[0].path, vec!["mod1", "Config"]);
        assert_eq!(replacements[1].path, vec!["mod2", "Settings"]);
    }
}
