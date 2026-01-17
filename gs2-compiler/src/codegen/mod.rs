//! Code generator for GS2 bytecode
//!
//! Compiles an AST into Graal bytecode.

use crate::ast::Program;
use crate::error::Result;

pub mod emitter;

use emitter::BytecodeEmitter;

/// Compile a GS2 program into bytecode
pub fn compile(program: &Program) -> Result<Vec<u8>> {
    let mut emitter = BytecodeEmitter::new();
    emitter.emit_program(program);
    Ok(emitter.into_bytes())
}
