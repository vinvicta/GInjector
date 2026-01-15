//! Error types for the GS2 compiler

use std::fmt;

pub type Result<T> = std::result::Result<T, CompileError>;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub enum CompileError {
    Lexer {
        message: String,
        location: SourceLocation,
    },
    Parse {
        message: String,
        location: SourceLocation,
    },
    Compiler {
        message: String,
        location: Option<SourceLocation>,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Lexer { message, location } => {
                write!(f, "Lexer error at {}: {}", location, message)
            }
            CompileError::Parse { message, location } => {
                write!(f, "Parse error at {}: {}", location, message)
            }
            CompileError::Compiler { message, location } => {
                if let Some(loc) = location {
                    write!(f, "Compiler error at {}: {}", loc, message)
                } else {
                    write!(f, "Compiler error: {}", message)
                }
            }
            CompileError::Io { message } => {
                write!(f, "I/O error: {}", message)
            }
        }
    }
}

impl std::error::Error for CompileError {}
