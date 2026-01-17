#![deny(missing_docs)]

use gbf_macros::AstNodeTransform;
use serde::{Deserialize, Serialize};

use crate::decompiler::structure_analysis::{ControlFlowEdgeType, region::RegionId};

use super::{AstKind, AstVisitable, expr::ExprKind, ptr::P, visitors::AstVisitor};

/// Represents a Phi node in SSA form.
///
/// Phi nodes are used to merge values coming from different control-flow paths.
/// Initially, the phi node has no arguments (i.e. no predecessor regions), but you
/// can add them later using the [`add_region`] method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, AstNodeTransform)]
#[convert_to(AstKind::Expression, ExprKind::Phi)]
pub struct PhiNode {
    resolved: bool,
    region_ids: Vec<(RegionId, ControlFlowEdgeType)>,
    idx: usize,
}

impl PhiNode {
    /// Creates a new unresolved `PhiNode`
    ///
    /// # Returns
    ///
    /// A new phi node with no predecessor regions.
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            resolved: false,
            region_ids: Vec::new(),
        }
    }

    /// Returns the index of the phi node.
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Adds a predecessor `RegionId` to this phi node.
    ///
    /// This method allows the phi node to record a region (i.e. a basic block ID)
    /// from which a value is coming.
    ///
    /// # Arguments
    /// * `region` - The identifier of the predecessor region.
    pub fn add_region(&mut self, region: RegionId, edge_type: ControlFlowEdgeType) {
        self.region_ids.push((region, edge_type));
    }

    /// Adds predecessor `RegionId`s to this phi node.
    ///
    /// This method allows the phi node to record multiple regions (i.e. basic block IDs)
    /// from which values are coming.
    ///
    /// # Arguments
    /// * `regions` - The identifiers of the predecessor regions.
    pub fn add_regions(&mut self, regions: Vec<(RegionId, ControlFlowEdgeType)>) {
        self.region_ids.extend(regions);
    }

    /// Returns a reference to the list of region IDs associated with this phi node.
    ///
    /// # Returns
    ///
    /// A slice containing all the predecessor region IDs added so far.
    pub fn regions(&self) -> &[(RegionId, ControlFlowEdgeType)] {
        &self.region_ids
    }
}

impl AstVisitable for P<PhiNode> {
    fn accept<V: AstVisitor>(&self, visitor: &mut V) -> V::Output {
        visitor.visit_phi(self)
    }
}
