//! GS2 Compiler Library
//!
//! This library will eventually contain a full GS2 to bytecode compiler.
//! For now, it provides the basic structures and opcodes.

pub mod error;
pub mod opcode;
pub mod parser;

pub use error::{CompileError, Result};
pub use opcode::Opcode;
