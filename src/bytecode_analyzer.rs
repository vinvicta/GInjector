//! Bytecode analysis module for decompiling and disassembling GS2 bytecode
//!
//! Provides functionality to:
//! - Decompile bytecode to readable GS2 code
//! - Disassemble bytecode to instruction listings
//! - Clean up decompiled output using regex patterns

use regex::Regex;
use std::io::Cursor;
use gs2_decompiler::{ModuleBuilder, disassemble_bytecode as decompiler_disassemble};

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
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::LoadFailed(msg) => write!(f, "Failed to load bytecode: {}", msg),
            AnalysisError::DecompileFailed(msg) => write!(f, "Failed to decompile: {}", msg),
            AnalysisError::DisassembleFailed(msg) => write!(f, "Failed to disassemble: {}", msg),
            AnalysisError::NoBytecode => write!(f, "No bytecode provided"),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Result type for analysis operations
pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// Decompile GS2 bytecode to readable GS2 code
///
/// Uses the gs2_decompiler library to decompile bytecode
///
/// # Note
/// Decompilation is not yet implemented in the new library.
/// This function will return an error until the decompiler modules are added.
pub fn decompile_bytecode(bytecode: &[u8]) -> AnalysisResult<String> {
    if bytecode.is_empty() {
        return Err(AnalysisError::NoBytecode);
    }

    // Convert to Vec<u8> to owned data for 'static lifetime requirement
    let bytecode_vec = bytecode.to_vec();
    let cursor = Cursor::new(bytecode_vec);

    // Build the module from bytecode
    let _module = ModuleBuilder::new()
        .name("input.gs2")
        .reader(Box::new(cursor))
        .build()
        .map_err(|e| AnalysisError::LoadFailed(e.to_string()))?;

    // TODO: Implement decompiler modules
    // For now, return an error since decompilation is not yet implemented
    Err(AnalysisError::DecompileFailed(
        "Decompilation is not yet implemented. Please use disassembly instead.".to_string()
    ))
}

/// Disassemble GS2 bytecode to instruction listings
///
/// Uses the gs2_decompiler library to disassemble bytecode
pub fn disassemble_bytecode(bytecode: &[u8]) -> AnalysisResult<String> {
    if bytecode.is_empty() {
        return Err(AnalysisError::NoBytecode);
    }

    let mut cursor = Cursor::new(bytecode.to_vec());

    // Use the disassemble function from gs2_decompiler
    let result = decompiler_disassemble(&mut cursor)
        .map_err(|e| AnalysisError::DisassembleFailed(e.to_string()))?;

    Ok(result)
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

    // Remove log lines from gbf_driver output
    let log_re = Regex::new(r"^\d{4}-\d{2}-\d{2}.*?(INFO|ERROR|WARN).*?$\n?").unwrap();
    code = log_re.replace_all(&code, "").to_string();

    // Remove INFO separator lines
    let sep_re = Regex::new(r"^---.*?---\s*$").unwrap();
    code = sep_re.replace_all(&code, "").to_string();

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

    // Pattern 14: Remove SSA version annotations (lit#2 -> lit)
    let re14 = Regex::new(r#"lit#\d+"#).unwrap();
    code = re14.replace_all(&code, "lit").to_string();

    // Pattern 15: Remove temp SSA annotations (temp.var#3 -> temp.var)
    let re15 = Regex::new(r"temp\\.([a-zA-Z_]+)\\#\\d+").unwrap();
    code = re15.replace_all(&code, "temp.$1").to_string();

    // Pattern 16: Remove builtin_fn_call SSA annotations
    let re16 = Regex::new(r"builtin_fn_call#\\d+").unwrap();
    code = re16.replace_all(&code, "builtin_fn_call").to_string();

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
