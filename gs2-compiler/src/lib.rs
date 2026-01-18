//! GS2 Compiler Library
//!
//! A complete GS2 to bytecode compiler for the Graal Online scripting language.

pub mod ast;
pub mod codegen;
pub mod error;
pub mod opcode;
pub mod parser;

pub use ast::{expression::{BinaryOp, CastType}, identifier::Identifier, literal::Literal, program::Program, statement::Statement};
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
        // Section headers use raw big-endian u32
        assert_eq!(bytecode[0], 0); // First byte of section type (big-endian u32)
        assert_eq!(bytecode[1], 0);
        assert_eq!(bytecode[2], 0);
        assert_eq!(bytecode[3], 1); // Section type 1 (Gs1Flags)
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

    #[test]
    fn test_compile_builtin_sleep() {
        let source = r#"
            function test() {
                sleep(5);
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        // Should contain OP_SLEEP (0x08)
        assert!(bytecode.contains(&0x08), "Bytecode should contain OP_SLEEP");
    }

    #[test]
    fn test_compile_builtin_sin() {
        let source = r#"
            function test() {
                return sin(1.5);
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        // Should contain OP_SIN (0x58)
        assert!(bytecode.contains(&0x58), "Bytecode should contain OP_SIN");
    }

    #[test]
    fn test_compile_builtin_format() {
        let source = r#"
            function test() {
                return format("Hello %s", "World");
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        // Should contain OP_FORMAT (0x54)
        assert!(bytecode.contains(&0x54), "Bytecode should contain OP_FORMAT");
    }

    #[test]
    fn test_compile_builtin_string_trim() {
        let source = r#"
            function test() {
                return " hello ".trim();
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        // Should contain OP_OBJ_TRIM (0x6E)
        assert!(bytecode.contains(&0x6E), "Bytecode should contain OP_OBJ_TRIM");
    }

    #[test]
    fn test_compile_builtin_array_size() {
        let source = r#"
            function test() {
                temp.arr = [1, 2, 3];
                return temp.arr.size();
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok());

        let bytecode = result.unwrap();
        // Should contain OP_OBJ_SIZE (0x82)
        assert!(bytecode.contains(&0x82), "Bytecode should contain OP_OBJ_SIZE");
    }

    #[test]
    fn test_compile_foreach_with_braces() {
        let source = r#"
            function test() {
                for (temp.x: [1, 2, 3]) {
                    temp.x = x;
                }
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("for-each with braces should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_foreach_single_statement() {
        let source = r#"
            function test() {
                for (temp.p: allplayers) p.chat = "hello";
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("for-each with single statement should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain OP_FOREACH (0xA3)
        assert!(bytecode.contains(&0xA3), "Bytecode should contain OP_FOREACH");
    }

    #[test]
    fn test_compile_if_single_statement() {
        let source = r#"
            function test() {
                if (true) return 1;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok(), "if with single statement should compile");
    }

    #[test]
    fn test_compile_if_else_single_statements() {
        let source = r#"
            function test() {
                if (true) return 1; else return 0;
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok(), "if-else with single statements should compile");
    }

    #[test]
    fn test_compile_while_single_statement() {
        let source = r#"
            function test() {
                while (true) sleep(1);
            }
        "#;

        let result = compile(source);
        assert!(result.is_ok(), "while with single statement should compile");
    }

    #[test]
    fn test_compile_for_single_statement() {
        let source = r#"
            function test() {
                for (temp.i = 0; temp.i < 10; temp.i++) sleep(1);
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("for with single statement should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_builtin_type_conversions() {
        let source = r#"
            function test() {
                // sin() takes a float and returns a float
                // should convert argument to float
                temp.result = sin(temp.x);

                // arraylen() takes object and returns float
                temp.size = arraylen(temp.arr);

                // format() takes string and variable args
                temp.msg = format("Hello %s", temp.name);

                // String methods require string conversion
                temp.str = "hello".trim();
                temp.len = temp.str.length();
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("type conversion test should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain conversion opcodes
        assert!(bytecode.contains(&0x21), "Bytecode should contain OP_CONV_TO_FLOAT (0x21)");
        assert!(bytecode.contains(&0x22), "Bytecode should contain OP_CONV_TO_STRING (0x22)");
    }

    #[test]
    fn test_compile_in_array() {
        let source = r#"
            function test() {
                temp.found = 5 in [1, 2, 3, 4, 5];
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("in array test should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain OP_IN_OBJ (0x51)
        assert!(bytecode.contains(&0x51), "Bytecode should contain OP_IN_OBJ (0x51)");
    }

    #[test]
    fn test_compile_in_range_pipe() {
        let source = r#"
            function test() {
                temp.in_range = temp.x in |0, 100|;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("in range test should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain OP_IN_RANGE (0x50)
        assert!(bytecode.contains(&0x50), "Bytecode should contain OP_IN_RANGE (0x50)");
    }

    #[test]
    fn test_compile_in_range_angle() {
        let source = r#"
            function test() {
                temp.in_range = temp.x in <0, 100>;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("in range angle test should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain OP_IN_RANGE (0x50)
        assert!(bytecode.contains(&0x50), "Bytecode should contain OP_IN_RANGE (0x50)");
    }

    #[test]
    fn test_compile_cast_int() {
        let source = r#"
            function test() {
                temp.value = cast_int(temp.float);
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("cast_int test should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain OP_INT (0x55)
        assert!(bytecode.contains(&0x55), "Bytecode should contain OP_INT (0x55)");
    }

    #[test]
    fn test_compile_cast_float() {
        let source = r#"
            function test() {
                temp.value = cast_float(temp.int);
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("cast_float test should compile, but got error: {}", e);
        }

        let bytecode = result.unwrap();
        // Should contain OP_CONV_TO_FLOAT (0x21)
        assert!(bytecode.contains(&0x21), "Bytecode should contain OP_CONV_TO_FLOAT (0x21)");
    }

    #[test]
    fn test_compile_elseif() {
        let source = r#"
            function test() {
                if (temp.x == 1) return 1;
                elseif (temp.x == 2) return 2;
                else return 0;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("elseif test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_const_number() {
        let source = r#"
            const MAX_HEALTH = 100;
            function test() {
                temp.health = MAX_HEALTH;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("const number test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_const_float() {
        let source = r#"
            const PI = 3.14159;
            function test() {
                temp.pi = PI;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("const float test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_const_string() {
        let source = r#"
            const DEFAULT_NAME = "Player";
            function test() {
                temp.name = DEFAULT_NAME;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("const string test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_const_negative() {
        let source = r#"
            const NEGATIVE_NUM = -5;
            function test() {
                temp.value = NEGATIVE_NUM;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("const negative test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_const_ref() {
        let source = r#"
            const PI = 3.14159;
            const TWO_PI = PI;
            function test() {
                temp.value = TWO_PI;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("const reference test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_const_redefinition_error() {
        let source = r#"
            const FOO = 1;
            const FOO = 2;
            function test() {
                temp.value = FOO;
            }
        "#;

        let result = compile(source);
        assert!(result.is_err(), "const redefinition should fail");
    }

    #[test]
    fn test_compile_const_undefined_in_const_decl_error() {
        let source = r#"
            const B = A;
            function test() {
                temp.value = B;
            }
        "#;

        let result = compile(source);
        assert!(result.is_err(), "undefined const in const declaration should fail");
    }

    #[test]
    fn test_compile_enum_anonymous() {
        let source = r#"
            enum {
                IDLE,
                WALKING,
                RUNNING
            };
            function test() {
                temp.state = IDLE;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("anonymous enum test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_enum_named() {
        let source = r#"
            enum State {
                IDLE,
                WALKING,
                RUNNING
            };
            function test() {
                temp.state = State::IDLE;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("named enum test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_enum_with_values() {
        let source = r#"
            enum Flags {
                FLAG_NONE = 0,
                FLAG_READ = 1,
                FLAG_WRITE = 2
            };
            function test() {
                temp.flags = Flags::FLAG_READ;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("enum with explicit values test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_enum_negative() {
        let source = r#"
            enum Signed {
                NEGATIVE = -5,
                ZERO = 0,
                POSITIVE = 5
            };
            function test() {
                temp.value = Signed::NEGATIVE;
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("enum with negative values test should compile, but got error: {}", e);
        }
    }

    #[test]
    fn test_compile_enum_auto_increment_after_value() {
        let source = r#"
            enum Test {
                FIRST,
                SECOND = 10,
                THIRD,  // Should be 11
                FOURTH  // Should be 12
            };
            function test() {
                temp.a = Test::FIRST;   // 0
                temp.b = Test::SECOND;  // 10
                temp.c = Test::THIRD;   // 11
                temp.d = Test::FOURTH;  // 12
            }
        "#;

        let result = compile(source);
        if let Err(e) = result {
            panic!("enum auto-increment test should compile, but got error: {}", e);
        }
    }
}
