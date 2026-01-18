//! GS2 Decompiler Library
//!
//! This library provides functionality for disassembling and decompiling GS2 bytecode.
//! GS2 is the scripting language used by Graal Online.

/// Basic block representation and types
pub mod basic_block;

/// Control flow graph visualization
pub mod cfg_dot;

/// Bytecode decompiler
pub mod decompiler;

/// Function representation and management
pub mod function;

/// Graal-specific I/O encoding/decoding
pub mod graal_io;

/// Instruction representation
pub mod instruction;

/// Module representation and loading
pub mod module;

/// Opcode definitions and parsing
pub mod opcode;

/// Operand types and values
pub mod operand;

/// Utility functions and constants
pub mod utils;

pub use bytecode_loader::{BytecodeLoader, BytecodeLoaderBuilder};
mod bytecode_loader;

use std::io::Cursor;

/// Disassemble GS2 bytecode into a human-readable string format.
///
/// # Arguments
/// - `cursor`: A mutable reference to a cursor containing the bytecode data.
///
/// # Returns
/// - A `Result` containing the disassembly as a `String`, or a `BytecodeLoaderError` if disassembly fails.
///
/// # Example
/// ```no_run
/// use gs2_decompiler::disassemble_bytecode;
/// use std::io::Cursor;
///
/// let bytecode = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04];
/// let mut cursor = Cursor::new(bytecode);
/// let disassembly = disassemble_bytecode(&mut cursor).unwrap();
/// println!("{}", disassembly);
/// ```
pub fn disassemble_bytecode(cursor: &mut Cursor<Vec<u8>>) -> Result<String, bytecode_loader::BytecodeLoaderError> {
    let loader = BytecodeLoaderBuilder::new(cursor).build()?;

    let mut output = String::new();

    for (offset, instruction) in loader.instructions.iter().enumerate() {
        output.push_str(&format!("{:04X}: {}\n", offset, instruction));
    }

    Ok(output)
}

/// Re-exports for commonly used types
pub use basic_block::{BasicBlock, BasicBlockId, BasicBlockType};
pub use decompiler::Decompiler;
pub use function::{Function, FunctionId};
pub use module::{Module, ModuleBuilder};
