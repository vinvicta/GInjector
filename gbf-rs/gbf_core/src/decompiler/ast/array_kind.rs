#![deny(missing_docs)]

use gbf_macros::AstNodeTransform;
use serde::{Deserialize, Serialize};

use super::{
    AstKind, AstVisitable, array_node::ArrayNode, expr::ExprKind, phi_array::PhiArrayNode, ptr::P,
    visitors::AstVisitor,
};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, AstNodeTransform)]
#[convert_to(AstKind::Expression, ExprKind::Array)]
/// Represents the kind of an array node in the AST.
pub enum ArrayKind {
    /// Represents an UnmergedArray
    PhiArray(P<PhiArrayNode>),
    /// Represents a MergedArray
    MergedArray(P<ArrayNode>),
}

impl AstVisitable for ArrayKind {
    fn accept<V: AstVisitor>(&self, visitor: &mut V) -> V::Output {
        visitor.visit_array(self)
    }
}

impl PartialEq for ArrayKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ArrayKind::PhiArray(p1), ArrayKind::PhiArray(p2)) => p1 == p2,
            (ArrayKind::MergedArray(a1), ArrayKind::MergedArray(a2)) => a1 == a2,
            _ => false,
        }
    }
}
