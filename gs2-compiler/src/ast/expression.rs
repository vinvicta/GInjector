//! Expression AST nodes
//!
//! Represents all types of expressions in GS2 source code.

use crate::ast::{Identifier, Literal};
use crate::error::SourceLocation;

/// A binary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,

    // Logical
    LogicalAnd,
    LogicalOr,

    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,

    // Assignment
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    PowerAssign,
    LeftShiftAssign,
    RightShiftAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
}

impl BinaryOp {
    /// Returns true if this operator is an assignment operator
    pub fn is_assignment(&self) -> bool {
        matches!(
            self,
            BinaryOp::Assign
                | BinaryOp::AddAssign
                | BinaryOp::SubtractAssign
                | BinaryOp::MultiplyAssign
                | BinaryOp::DivideAssign
                | BinaryOp::ModuloAssign
                | BinaryOp::PowerAssign
                | BinaryOp::LeftShiftAssign
                | BinaryOp::RightShiftAssign
                | BinaryOp::BitwiseAndAssign
                | BinaryOp::BitwiseOrAssign
                | BinaryOp::BitwiseXorAssign
        )
    }

    /// Returns the precedence of this operator (higher = tighter binding)
    pub fn precedence(&self) -> u8 {
        match self {
            // Assignment (lowest)
            BinaryOp::Assign
            | BinaryOp::AddAssign
            | BinaryOp::SubtractAssign
            | BinaryOp::MultiplyAssign
            | BinaryOp::DivideAssign
            | BinaryOp::ModuloAssign
            | BinaryOp::PowerAssign
            | BinaryOp::LeftShiftAssign
            | BinaryOp::RightShiftAssign
            | BinaryOp::BitwiseAndAssign
            | BinaryOp::BitwiseOrAssign
            | BinaryOp::BitwiseXorAssign => 1,

            // Logical OR
            BinaryOp::LogicalOr => 2,

            // Logical AND
            BinaryOp::LogicalAnd => 3,

            // Bitwise OR
            BinaryOp::BitwiseOr => 4,

            // Bitwise XOR
            BinaryOp::BitwiseXor => 5,

            // Bitwise AND
            BinaryOp::BitwiseAnd => 6,

            // Equality
            BinaryOp::Equal | BinaryOp::NotEqual => 7,

            // Comparison
            BinaryOp::LessThan
            | BinaryOp::GreaterThan
            | BinaryOp::LessThanOrEqual
            | BinaryOp::GreaterThanOrEqual => 8,

            // Shift
            BinaryOp::LeftShift | BinaryOp::RightShift => 9,

            // Additive
            BinaryOp::Add | BinaryOp::Subtract => 10,

            // Multiplicative
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => 11,

            // Power (highest)
            BinaryOp::Power => 12,
        }
    }

    /// Returns true if this operator is left-associative
    pub fn is_left_associative(&self) -> bool {
        !matches!(self, BinaryOp::Assign | BinaryOp::Power)
    }
}

/// A unary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    // Arithmetic
    Negate,
    // Logical
    LogicalNot,
    // Bitwise
    BitwiseInvert,
}

/// An expression in GS2 source code
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A literal value
    Literal {
        literal: Box<Literal>,
        location: SourceLocation,
    },

    /// An identifier (variable reference)
    Identifier {
        identifier: Box<Identifier>,
        location: SourceLocation,
    },

    /// A binary operation: `left op right`
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
        location: SourceLocation,
    },

    /// A unary operation: `op expr`
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expression>,
        location: SourceLocation,
    },

    /// A function call: `func(arg1, arg2, ...)`
    FunctionCall {
        target: Box<Expression>,
        args: Vec<Expression>,
        location: SourceLocation,
    },

    /// Member access: `object.property`
    MemberAccess {
        object: Box<Expression>,
        property: Identifier,
        location: SourceLocation,
    },

    /// Array access: `array[index]`
    ArrayAccess {
        array: Box<Expression>,
        index: Box<Expression>,
        location: SourceLocation,
    },

    /// An array literal: `[elem1, elem2, ...]`
    ArrayLiteral {
        elements: Vec<Expression>,
        location: SourceLocation,
    },

    /// An object literal: `{key: value, ...}`
    ObjectLiteral {
        properties: Vec<(Identifier, Expression)>,
        location: SourceLocation,
    },

    /// A ternary expression: `condition ? true_expr : false_expr`
    Ternary {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
        location: SourceLocation,
    },
}

impl Expression {
    /// Get the source location of this expression
    pub fn location(&self) -> &SourceLocation {
        match self {
            Expression::Literal { location, .. }
            | Expression::Identifier { location, .. }
            | Expression::BinaryOp { location, .. }
            | Expression::UnaryOp { location, .. }
            | Expression::FunctionCall { location, .. }
            | Expression::MemberAccess { location, .. }
            | Expression::ArrayAccess { location, .. }
            | Expression::ArrayLiteral { location, .. }
            | Expression::ObjectLiteral { location, .. }
            | Expression::Ternary { location, .. } => location,
        }
    }

    /// Create a literal expression
    pub fn literal(literal: Literal, location: SourceLocation) -> Self {
        Expression::Literal {
            literal: Box::new(literal),
            location,
        }
    }

    /// Create an identifier expression
    pub fn identifier(identifier: Identifier, location: SourceLocation) -> Self {
        Expression::Identifier {
            identifier: Box::new(identifier),
            location,
        }
    }

    /// Create a binary operation expression
    pub fn binary_op(
        left: Expression,
        op: BinaryOp,
        right: Expression,
        location: SourceLocation,
    ) -> Self {
        Expression::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
            location,
        }
    }

    /// Create a unary operation expression
    pub fn unary_op(op: UnaryOp, expr: Expression, location: SourceLocation) -> Self {
        Expression::UnaryOp {
            op,
            expr: Box::new(expr),
            location,
        }
    }

    /// Create a function call expression
    pub fn function_call(
        target: Expression,
        args: Vec<Expression>,
        location: SourceLocation,
    ) -> Self {
        Expression::FunctionCall {
            target: Box::new(target),
            args,
            location,
        }
    }

    /// Create a member access expression
    pub fn member_access(object: Expression, property: Identifier, location: SourceLocation) -> Self {
        Expression::MemberAccess {
            object: Box::new(object),
            property,
            location,
        }
    }

    /// Create an array access expression
    pub fn array_access(array: Expression, index: Expression, location: SourceLocation) -> Self {
        Expression::ArrayAccess {
            array: Box::new(array),
            index: Box::new(index),
            location,
        }
    }
}
