//! Literal value AST nodes
//!
//! Represents literal values in GS2 source code.

use crate::error::SourceLocation;

/// A literal value in GS2 source code
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Number literal (integer or float)
    Number {
        value: String,
        location: SourceLocation,
    },
    /// String literal
    String {
        value: String,
        location: SourceLocation,
    },
    /// Boolean literal
    Boolean {
        value: bool,
        location: SourceLocation,
    },
    /// Null literal
    Null {
        location: SourceLocation,
    },
}

impl Literal {
    /// Get the source location of this literal
    pub fn location(&self) -> &SourceLocation {
        match self {
            Literal::Number { location, .. }
            | Literal::String { location, .. }
            | Literal::Boolean { location, .. }
            | Literal::Null { location } => location,
        }
    }

    /// Create a number literal
    pub fn number(value: impl Into<String>, location: SourceLocation) -> Self {
        Literal::Number {
            value: value.into(),
            location,
        }
    }

    /// Create a string literal
    pub fn string(value: impl Into<String>, location: SourceLocation) -> Self {
        Literal::String {
            value: value.into(),
            location,
        }
    }

    /// Create a boolean literal
    pub fn boolean(value: bool, location: SourceLocation) -> Self {
        Literal::Boolean { value, location }
    }

    /// Create a null literal
    pub fn null(location: SourceLocation) -> Self {
        Literal::Null { location }
    }
}
