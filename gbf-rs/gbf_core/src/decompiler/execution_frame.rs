#![deny(missing_docs)]

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::ast::{AstKind, expr::ExprKind};

/// Represents the state of execution for the decompiler.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ExecutionFrame {
    /// The decompiler is currently building a standalone node.
    StandaloneNode(AstKind),
    /// The decompiler is currently building an array.
    BuildingArray(Vec<ExprKind>),
}

impl Display for ExecutionFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionFrame::BuildingArray(_) => write!(f, "BuildingArray"),
            ExecutionFrame::StandaloneNode(_) => write!(f, "StandaloneNode"),
        }
    }
}
