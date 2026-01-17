//! Statement AST nodes
//!
//! Represents all types of statements in GS2 source code.

use crate::ast::{Expression, Identifier};
use crate::error::SourceLocation;

/// A statement in GS2 source code
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// An expression statement (expression followed by semicolon)
    Expression {
        expr: Expression,
        location: SourceLocation,
    },

    /// A block of statements: `{ stmt1; stmt2; ... }`
    Block {
        statements: Vec<Statement>,
        location: SourceLocation,
    },

    /// A function declaration: `function name(params) { body }` or `public function name(params) { body }`
    FunctionDeclaration {
        name: Identifier,
        params: Vec<Identifier>,
        body: Box<Statement>, // Always a Block
        is_public: bool,
        location: SourceLocation,
    },

    /// An if statement: `if (condition) { true_block } else { false_block }`
    If {
        condition: Expression,
        true_block: Box<Statement>,
        false_block: Option<Box<Statement>>,
        location: SourceLocation,
    },

    /// A while loop: `while (condition) { body }`
    While {
        condition: Expression,
        body: Box<Statement>,
        location: SourceLocation,
    },

    /// A for loop: `for (init; condition; increment) { body }`
    For {
        init: Option<Box<Statement>>,
        condition: Option<Expression>,
        increment: Option<Box<Statement>>,
        body: Box<Statement>,
        location: SourceLocation,
    },

    /// A for-each loop: `for (item: array) { body }`
    ForEach {
        item: Identifier,
        array: Expression,
        body: Box<Statement>,
        location: SourceLocation,
    },

    /// A switch statement
    Switch {
        expr: Expression,
        cases: Vec<(Expression, Box<Statement>)>,
        default_case: Option<Box<Statement>>,
        location: SourceLocation,
    },

    /// A break statement
    Break {
        location: SourceLocation,
    },

    /// A continue statement
    Continue {
        location: SourceLocation,
    },

    /// A return statement: `return expr;` or `return;`
    Return {
        expr: Option<Expression>,
        location: SourceLocation,
    },

    /// A with statement: `with (obj) { body }`
    With {
        obj: Expression,
        body: Box<Statement>,
        location: SourceLocation,
    },
}

impl Statement {
    /// Get the source location of this statement
    pub fn location(&self) -> &SourceLocation {
        match self {
            Statement::Expression { location, .. }
            | Statement::Block { location, .. }
            | Statement::FunctionDeclaration { location, .. }
            | Statement::If { location, .. }
            | Statement::While { location, .. }
            | Statement::For { location, .. }
            | Statement::ForEach { location, .. }
            | Statement::Switch { location, .. }
            | Statement::Break { location, .. }
            | Statement::Continue { location, .. }
            | Statement::Return { location, .. }
            | Statement::With { location, .. } => location,
        }
    }

    /// Create an expression statement
    pub fn expression(expr: Expression, location: SourceLocation) -> Self {
        Statement::Expression { expr, location }
    }

    /// Create a block statement
    pub fn block(statements: Vec<Statement>, location: SourceLocation) -> Self {
        Statement::Block {
            statements,
            location,
        }
    }

    /// Create an empty block
    pub fn empty_block(location: SourceLocation) -> Self {
        Statement::Block {
            statements: Vec::new(),
            location,
        }
    }

    /// Create a function declaration
    pub fn function_declaration(
        name: Identifier,
        params: Vec<Identifier>,
        body: Statement,
        is_public: bool,
        location: SourceLocation,
    ) -> Self {
        Statement::FunctionDeclaration {
            name,
            params,
            body: Box::new(body),
            is_public,
            location,
        }
    }

    /// Create an if statement
    pub fn if_stmt(
        condition: Expression,
        true_block: Statement,
        false_block: Option<Statement>,
        location: SourceLocation,
    ) -> Self {
        Statement::If {
            condition,
            true_block: Box::new(true_block),
            false_block: false_block.map(Box::new),
            location,
        }
    }

    /// Create a while loop
    pub fn while_loop(condition: Expression, body: Statement, location: SourceLocation) -> Self {
        Statement::While {
            condition,
            body: Box::new(body),
            location,
        }
    }

    /// Create a for loop
    pub fn for_loop(
        init: Option<Statement>,
        condition: Option<Expression>,
        increment: Option<Statement>,
        body: Statement,
        location: SourceLocation,
    ) -> Self {
        Statement::For {
            init: init.map(Box::new),
            condition,
            increment: increment.map(Box::new),
            body: Box::new(body),
            location,
        }
    }

    /// Create a break statement
    pub fn break_stmt(location: SourceLocation) -> Self {
        Statement::Break { location }
    }

    /// Create a continue statement
    pub fn continue_stmt(location: SourceLocation) -> Self {
        Statement::Continue { location }
    }

    /// Create a return statement
    pub fn return_stmt(expr: Option<Expression>, location: SourceLocation) -> Self {
        Statement::Return { expr, location }
    }
}
