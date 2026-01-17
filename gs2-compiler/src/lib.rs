//! GS2 Compiler Library
//!
//! A complete GS2 to bytecode compiler for the Graal Online scripting language.

pub mod ast;
pub mod codegen;
pub mod error;
pub mod opcode;
pub mod parser;

pub use ast::{expression::BinaryOp, identifier::Identifier, literal::Literal, program::Program, statement::Statement};
pub use error::{CompileError, Result, SourceLocation};
pub use opcode::Opcode;

/// Main entry point for compiling GS2 source code
pub fn compile(source: &str) -> crate::error::Result<Vec<u8>> {
    let ast = parser::parse(source)?;
    codegen::compile(&ast)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_compile_simple_function() {
        let source = r#"
            function test() {
                return 42;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        // Should have non-empty bytecode
        assert!(!bytecode.is_empty());

        // First section should be Gs1Flags (type 1)
        // Using Graal encoding: 1 becomes (1 + 32) = 33
        assert_eq!(bytecode[0], 33); // Section type 1 (Gs1Flags) Graal-encoded
    }

    #[test]
    fn test_compile_function_with_string() {
        let source = r#"
            function test() {
                temp.msg = "hello";
                return temp.msg;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_if_statement() {
        let source = r#"
            function test(x) {
                if (x > 0) {
                    return 1;
                } else {
                    return 0;
                }
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_while_loop() {
        let source = r#"
            function test() {
                temp.i = 0;
                while (temp.i < 10) {
                    temp.i = temp.i + 1;
                }
                return temp.i;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_array_literal() {
        let source = r#"
            function test() {
                temp.arr = [1, 2, 3];
                return temp.arr;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_object_literal() {
        let source = r#"
            function test() {
                temp.obj = {name: "test", value: 42};
                return temp.obj;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_function_call() {
        let source = r#"
            function test() {
                return foo(1, 2, 3);
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_ternary() {
        let source = r#"
            function test(x) {
                return x > 0 ? 1 : 0;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_compile_error_invalid_syntax() {
        let source = r#"
            function test() {
                return 42
            "#; // Missing closing brace

        let result = compile(source);
        assert!(result.is_err());
    }
}
