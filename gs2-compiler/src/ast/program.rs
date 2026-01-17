//! Program AST node
//!
//! The Program is the root of the AST for a GS2 source file.

use crate::ast::Statement;

/// A GS2 program (source file)
///
/// Contains a list of top-level statements (function declarations, etc.)
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// The statements in the program
    pub statements: Vec<Statement>,
}

impl Program {
    /// Create a new empty program
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Create a program with the given statements
    pub fn with_statements(statements: Vec<Statement>) -> Self {
        Self { statements }
    }

    /// Add a statement to the program
    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    /// Check if the program is empty
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    /// Get the number of statements
    pub fn len(&self) -> usize {
        self.statements.len()
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
