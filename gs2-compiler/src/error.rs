//! Error types for the GS2 compiler

use std::fmt;

pub type Result<T> = std::result::Result<T, CompileError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl SourceLocation {
    pub fn new() -> Self {
        Self::default()
    }
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
    ConstRedefinition {
        name: String,
        location: SourceLocation,
    },
    UndefinedConst {
        name: String,
        location: SourceLocation,
    },
    InvalidConstValue {
        location: SourceLocation,
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
            CompileError::ConstRedefinition { name, location } => {
                write!(f, "Compiler error at {}: redefinition of const {}", location, name)
            }
            CompileError::UndefinedConst { name, location } => {
                write!(f, "Compiler error at {}: undefined const {}", location, name)
            }
            CompileError::InvalidConstValue { location } => {
                write!(f, "Compiler error at {}: const value must be a literal or reference to another const", location)
            }
        }
    }
}

impl std::error::Error for CompileError {}

impl CompileError {
    /// Get the source location of this error, if available
    pub fn source_location(&self) -> Option<&SourceLocation> {
        match self {
            CompileError::Lexer { location, .. } => Some(location),
            CompileError::Parse { location, .. } => Some(location),
            CompileError::Compiler { location, .. } => location.as_ref(),
            CompileError::Io { .. } => None,
            CompileError::ConstRedefinition { location, .. } => Some(location),
            CompileError::UndefinedConst { location, .. } => Some(location),
            CompileError::InvalidConstValue { location } => Some(location),
        }
    }
}
