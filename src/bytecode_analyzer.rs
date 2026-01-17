//! Bytecode analysis module for decompiling and disassembling GS2 bytecode
//!
//! Provides functionality to:
//! - Decompile bytecode to readable GS2 code
//! - Disassemble bytecode to instruction listings
//! - Clean up decompiled output using regex patterns

use regex::Regex;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Errors that can occur during bytecode analysis
#[derive(Debug, Clone)]
pub enum AnalysisError {
    /// Failed to load bytecode
    LoadFailed(String),
    /// Failed to decompile
    DecompileFailed(String),
    /// Failed to disassemble
    DisassembleFailed(String),
    /// No bytecode provided
    NoBytecode,
    /// Driver not found
    DriverNotFound,
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::LoadFailed(msg) => write!(f, "Failed to load bytecode: {}", msg),
            AnalysisError::DecompileFailed(msg) => write!(f, "Failed to decompile: {}", msg),
            AnalysisError::DisassembleFailed(msg) => write!(f, "Failed to disassemble: {}", msg),
            AnalysisError::NoBytecode => write!(f, "No bytecode provided"),
            AnalysisError::DriverNotFound => write!(f, "GBF driver not found. Please build with: cargo build --manifest-path gbf-rs/gbf_driver/Cargo.toml"),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Result type for analysis operations
pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// Get the path to the gbf_driver binary
fn get_driver_path() -> Option<std::path::PathBuf> {
    // Try multiple possible locations for the driver binary
    let possible_paths = vec![
        std::path::PathBuf::from("gbf-rs/target/release/gbf_driver"),
        std::path::PathBuf::from("../gbf-rs/target/release/gbf_driver"),
        std::path::PathBuf::from("target/release/gbf_driver"),
        // Also try debug builds
        std::path::PathBuf::from("gbf-rs/target/debug/gbf_driver"),
        std::path::PathBuf::from("../gbf-rs/target/debug/gbf_driver"),
        std::path::PathBuf::from("target/debug/gbf_driver"),
    ];

    for path in possible_paths {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Decompile GS2 bytecode to readable GS2 code
///
/// Uses the gbf_driver binary to decompile bytecode
pub fn decompile_bytecode(bytecode: &[u8]) -> AnalysisResult<String> {
    if bytecode.is_empty() {
        return Err(AnalysisError::NoBytecode);
    }

    let driver_path = get_driver_path().ok_or(AnalysisError::DriverNotFound)?;

    // Write bytecode to a temp file
    let mut temp_file = NamedTempFile::new()
        .map_err(|e| AnalysisError::LoadFailed(format!("Failed to create temp file: {}", e)))?;

    temp_file.write_all(bytecode)
        .map_err(|e| AnalysisError::LoadFailed(format!("Failed to write bytecode: {}", e)))?;

    let temp_path = temp_file.path();

    // Run gbf_driver to decompile
    let output = Command::new(&driver_path)
        .arg(temp_path)
        .arg("--minified")
        .output()
        .map_err(|e| AnalysisError::DecompileFailed(format!("Failed to run driver: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AnalysisError::DecompileFailed(format!("Driver error: {}", stderr)));
    }

    let decompiled = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(clean_decompiled_code(&decompiled))
}

/// Disassemble GS2 bytecode to instruction listings
///
/// Uses the gbf_driver binary to disassemble bytecode
pub fn disassemble_bytecode(bytecode: &[u8]) -> AnalysisResult<String> {
    if bytecode.is_empty() {
        return Err(AnalysisError::NoBytecode);
    }

    let driver_path = get_driver_path().ok_or(AnalysisError::DriverNotFound)?;

    // Write bytecode to a temp file
    let mut temp_file = NamedTempFile::new()
        .map_err(|e| AnalysisError::LoadFailed(format!("Failed to create temp file: {}", e)))?;

    temp_file.write_all(bytecode)
        .map_err(|e| AnalysisError::LoadFailed(format!("Failed to write bytecode: {}", e)))?;

    let temp_path = temp_file.path();

    // Run gbf_driver in disassembly mode (module mode with minified output gives us raw output)
    let output = Command::new(&driver_path)
        .arg(temp_path)
        .arg("--function")
        .arg("join")
        .arg("--minified")
        .output()
        .map_err(|e| AnalysisError::DisassembleFailed(format!("Failed to run driver: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AnalysisError::DisassembleFailed(format!("Driver error: {}", stderr)));
    }

    let disassembled = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(disassembled)
}

/// Clean up decompiled code using regex patterns
///
/// Applies common transformations to make the decompiled code more readable:
/// - Simplify echo(lit) patterns
/// - Simplify member access calls with literals
/// - Fix function declarations
/// - Remove redundant statements
pub fn clean_decompiled_code(code: &str) -> String {
    let mut code = code.to_string();

    // Pattern 1: echo with literal that gets returned
    // lit = "text"; fn_call = echo(lit); lit = 0; return lit;
    // -> echo("text");
    let re1 = Regex::new(r#"lit\s*=\s*"([^"]+)"\s*;\s*fn_call\s*=\s*echo\s*\(lit\)\s*;\s*lit\s*=\s*0;\s*return\s+lit;"#).unwrap();
    code = re1.replace_all(&code, r#"echo("$1");"#).to_string();

    // Pattern 2: echo with literal (no return)
    // lit = "text"; fn_call = echo(lit);
    // -> echo("text");
    let re2 = Regex::new(r#"lit\s*=\s*"([^"]+)"\s*;\s*fn_call\s*=\s*echo\s*\(lit\)\s*;"#).unwrap();
    code = re2.replace_all(&code, r#"echo("$1");"#).to_string();

    // Pattern 3: member access with string literal
    // lit = "text"; fn_call = obj.method(lit);
    // -> obj.method("text");
    let re3 = Regex::new(r#"lit\s*=\s*"([^"]+)"\s*;\s*fn_call\s*=\s*([^(]+)\.([^(]+)\(lit\)\s*;"#).unwrap();
    code = re3.replace_all(&code, r#"$2.$3("$1");"#).to_string();

    // Pattern 4: boolean true member call
    // lit = true; fn_call = obj.method(lit);
    // -> obj.method(true);
    let re4 = Regex::new(r#"lit\s*=\s*true\s*;\s*fn_call\s*=\s*([^.]+)\.([^(]+)\(lit\)\s*;"#).unwrap();
    code = re4.replace_all(&code, r#"$1.$2(true);"#).to_string();

    // Pattern 5: boolean false member call
    // lit = false; fn_call = obj.method(lit);
    // -> obj.method(false);
    let re5 = Regex::new(r#"lit\s*=\s*false\s*;\s*fn_call\s*=\s*([^.]+)\.([^(]+)\(lit\)\s*;"#).unwrap();
    code = re5.replace_all(&code, r#"$1.$2(false);"#).to_string();

    // Pattern 6: boolean true function call
    // lit = true; fn_call = func(lit);
    // -> func(true);
    let re6 = Regex::new(r#"lit\s*=\s*true\s*;\s*fn_call\s*=\s*([^(]+)\(lit\)\s*;"#).unwrap();
    code = re6.replace_all(&code, r#"$1(true);"#).to_string();

    // Pattern 7: boolean false function call
    // lit = false; fn_call = func(lit);
    // -> func(false);
    let re7 = Regex::new(r#"lit\s*=\s*false\s*;\s*fn_call\s*=\s*([^(]+)\(lit\)\s*;"#).unwrap();
    code = re7.replace_all(&code, r#"$1(false);"#).to_string();

    // Pattern 8: visible assignment with false
    // lit = false; obj.visible = lit;
    // -> obj.visible = false;
    let re8 = Regex::new(r#"lit\s*=\s*false\s*;\s*([^=]+\.visible\s*=\s*)lit\s*;"#).unwrap();
    code = re8.replace_all(&code, r#"$1false;"#).to_string();

    // Pattern 9: visible assignment with true
    // lit = true; obj.visible = lit;
    // -> obj.visible = true;
    let re9 = Regex::new(r#"lit\s*=\s*true\s*;\s*([^=]+\.visible\s*=\s*)lit\s*;"#).unwrap();
    code = re9.replace_all(&code, r#"$1true;"#).to_string();

    // Pattern 10: Remove redundant fn_call assignments
    // fn_call = ...; -> (nothing, just keep the statement)
    let re10 = Regex::new(r#"fn_call\s*=\s*"#).unwrap();
    code = re10.replace_all(&code, "").to_string();

    // Pattern 11: Fix function declarations
    // function public.name -> public function name
    let re11 = Regex::new(r#"function\s+(public|private)\.(\w+)"#).unwrap();
    code = re11.replace_all(&code, r#"$1 function $2"#).to_string();

    // Pattern 12: Fix closing braces with semicolons
    // } ; -> };
    let re12 = Regex::new(r##"}\s*;"##).unwrap();
    code = re12.replace_all(&code, "};").to_string();

    // Pattern 13: Remove excessive blank lines
    let re13 = Regex::new(r"\n\s*\n\s*\n").unwrap();
    code = re13.replace_all(&code, "\n\n").to_string();

    code
}

/// Convert hex string to raw bytecode
pub fn hex_to_bytecode(hex: &str) -> AnalysisResult<Vec<u8>> {
    let hex = hex.trim();
    if hex.is_empty() {
        return Ok(Vec::new());
    }

    // Remove 0x prefix if present
    let hex = hex.strip_prefix("0x").unwrap_or(hex);

    // Handle space-separated hex
    let hex_parts: Vec<&str> = hex.split_whitespace().collect();

    let mut bytecode = Vec::new();
    for part in hex_parts {
        let byte = u8::from_str_radix(part, 16)
            .map_err(|_| AnalysisError::LoadFailed(format!("Invalid hex: {}", part)))?;
        bytecode.push(byte);
    }

    Ok(bytecode)
}

/// Check if the gbf_driver is available
pub fn is_driver_available() -> bool {
    get_driver_path().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_echo_literal() {
        let input = r#"lit = "Hello"; fn_call = echo(lit); lit = 0; return lit;"#;
        let output = clean_decompiled_code(input);
        assert_eq!(output, r#"echo("Hello");"#);
    }

    #[test]
    fn test_clean_member_access() {
        let input = r#"lit = "test"; fn_call = player.chat(lit);"#;
        let output = clean_decompiled_code(input);
        assert_eq!(output, r#"player.chat("test");"#);
    }

    #[test]
    fn test_clean_boolean_call() {
        let input = r#"lit = true; fn_call = obj.show(lit);"#;
        let output = clean_decompiled_code(input);
        assert_eq!(output, r#"obj.show(true);"#);
    }

    #[test]
    fn test_fix_function_decl() {
        let input = r#"function public.test"#;
        let output = clean_decompiled_code(input);
        assert_eq!(output, r#"public function test"#);
    }

    #[test]
    fn test_hex_to_bytecode() {
        let result = hex_to_bytecode("00 01 02 FF").unwrap();
        assert_eq!(result, vec![0x00, 0x01, 0x02, 0xFF]);
    }

    #[test]
    fn test_hex_to_bytecode_no_spaces() {
        let result = hex_to_bytecode("000102FF").unwrap();
        assert_eq!(result, vec![0x00, 0x01, 0x02, 0xFF]);
    }
}
