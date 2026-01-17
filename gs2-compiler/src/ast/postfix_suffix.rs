//! Postfix suffix AST nodes
//!
//! Internal representation for postfix operations during parsing.

use crate::ast::{Expression, Identifier};

/// A suffix that can be applied to an expression (function call, member access, array index)
#[derive(Debug, Clone, PartialEq)]
pub enum PostfixSuffix {
    /// Function call: expr(args)
    Call(Vec<Expression>),
    /// Member access: expr.prop
    Access(Identifier),
    /// Array access: expr[index]
    Index(Expression),
}
