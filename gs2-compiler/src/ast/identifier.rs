//! Identifier AST node
//!
//! Represents variable names, function names, and other identifiers in GS2.

use crate::error::SourceLocation;

/// An identifier in GS2 source code
#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    /// The name of the identifier
    pub name: String,
    /// Source location for error reporting
    pub location: SourceLocation,
}

impl Identifier {
    /// Create a new identifier
    pub fn new(name: String, location: SourceLocation) -> Self {
        Self { name, location }
    }

    /// Create an identifier with a default location
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: SourceLocation {
                line: 0,
                column: 0,
                offset: 0,
            },
        }
    }
}

impl From<&str> for Identifier {
    fn from(s: &str) -> Self {
        Self::with_name(s)
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Self::with_name(s)
    }
}
