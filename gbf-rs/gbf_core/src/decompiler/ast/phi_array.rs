#![deny(missing_docs)]

use gbf_macros::AstNodeTransform;
use serde::{Deserialize, Serialize};

use super::{
    AstKind, AstVisitable, array_kind::ArrayKind, expr::ExprKind, phi::PhiNode, ptr::P,
    visitors::AstVisitor,
};

/// Represents a function call
#[derive(Debug, Clone, Serialize, Deserialize, Eq, AstNodeTransform)]
#[convert_to(ArrayKind::PhiArray, ExprKind::Array, AstKind::Expression)]
pub struct PhiArrayNode {
    /// The phi node that merges the array.
    pub phi: P<PhiNode>,
    /// The elements in the array
    pub elements: Vec<ExprKind>,
}

impl PhiArrayNode {
    /// Creates a new array.
    ///
    /// # Arguments
    /// - `elements`: The elements of the array.
    pub fn new(phi: P<PhiNode>, elements: Vec<ExprKind>) -> Self {
        Self { phi, elements }
    }
}

impl AstVisitable for P<PhiArrayNode> {
    fn accept<V: AstVisitor>(&self, visitor: &mut V) -> V::Output {
        visitor.visit_phi_array(self)
    }
}

// == Other implementations for unary operations ==
impl PartialEq for PhiArrayNode {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements && self.phi == other.phi
    }
}
