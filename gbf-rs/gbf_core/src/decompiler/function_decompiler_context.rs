#![deny(missing_docs)]

use crate::basic_block::BasicBlockId;
use crate::instruction::Instruction;
use crate::opcode::Opcode;
use std::backtrace::Backtrace;
use std::collections::HashMap;

use super::ast::array_kind::ArrayKind;
use super::ast::array_node::ArrayNode;
use super::ast::expr::ExprKind;
use super::ast::identifier::IdentifierNode;
use super::ast::phi::PhiNode;
use super::ast::phi_array::PhiArrayNode;
use super::ast::ptr::P;
use super::ast::ssa::SsaContext;
use super::ast::{AstKind, new_phi};
use super::execution_frame::ExecutionFrame;
use super::function_decompiler::{FunctionDecompilerError, FunctionDecompilerErrorContext};
use super::handlers::{OpcodeHandler, global_opcode_handlers};
use super::{ProcessedInstruction, ProcessedInstructionBuilder};

/// Manages the state of the decompiler, including per-block AST stacks and current processing context.
pub struct FunctionDecompilerContext {
    /// AST node stacks for each basic block.
    pub block_ast_node_stack: HashMap<BasicBlockId, Vec<ExecutionFrame>>,
    /// The number of phi nodes in each basic block.
    pub block_ast_phi_count: HashMap<BasicBlockId, usize>,
    /// The current basic block being processed.
    pub current_block_id: BasicBlockId,
    /// The handlers for each opcode.
    pub opcode_handlers: HashMap<Opcode, Box<dyn OpcodeHandler>>,
    /// The SSA Context
    pub ssa_context: SsaContext,
    /// The current instruction being processed.
    pub current_instruction: Instruction,
    /// Register mapping for the current function
    pub register_mapping: HashMap<usize, ExprKind>,
}

impl FunctionDecompilerContext {
    /// Creates a new, empty context. We initialize with the starting block ID and region ID.
    pub fn new(start_block_id: BasicBlockId) -> Self {
        Self {
            block_ast_node_stack: HashMap::new(),
            current_block_id: start_block_id,
            opcode_handlers: HashMap::new(),
            ssa_context: SsaContext::new(),
            current_instruction: Instruction::default(),
            register_mapping: HashMap::new(),
            block_ast_phi_count: HashMap::new(),
        }
    }

    /// Starts processing a new basic block.
    ///
    /// # Arguments
    /// - `block_id`: The ID of the basic block to start processing.
    /// - `region_id`: The ID of the region the basic block belongs to.
    ///
    /// # Errors
    /// - Returns `FunctionDecompilerError` if there is an issue initializing the block stack.
    pub fn start_block_processing(
        &mut self,
        block_id: BasicBlockId,
    ) -> Result<(), FunctionDecompilerError> {
        self.current_block_id = block_id;
        self.block_ast_node_stack.insert(block_id, Vec::new());
        Ok(())
    }

    /// Processes an instruction and updates the AST stack.
    ///
    /// # Arguments
    /// - `instr`: The instruction to process.
    ///
    /// # Errors
    /// - Returns `FunctionDecompilerError` for invalid or unexpected instructions.
    pub fn process_instruction(
        &mut self,
        instr: &Instruction,
    ) -> Result<ProcessedInstruction, FunctionDecompilerError> {
        self.current_instruction = instr.clone();

        let current_block_id = self.current_block_id;

        // TODO: Better handle PushArray
        if instr.opcode == Opcode::PushArray {
            let stack = self
                .block_ast_node_stack
                .get_mut(&current_block_id)
                .expect("Critical error: stack should always be set for each basic block");

            stack.push(ExecutionFrame::BuildingArray(Vec::new()));
            return Ok(ProcessedInstructionBuilder::new().build());
        }
        // TODO: Better handle swap
        if instr.opcode == Opcode::Swap {
            let error_context = self.get_error_context();

            let last = {
                let stack = self
                    .block_ast_node_stack
                    .get_mut(&current_block_id)
                    .expect("Critical error: stack should always be set for each basic block");
                stack.pop()
            }
            .unwrap_or_else(|| {
                ExecutionFrame::StandaloneNode(AstKind::Expression(self.new_phi().into()))
            });

            let second_last = {
                let stack = self
                    .block_ast_node_stack
                    .get_mut(&current_block_id)
                    .expect("Critical error: stack should always be set for each basic block");
                stack.pop()
            }
            .unwrap_or_else(|| {
                ExecutionFrame::StandaloneNode(AstKind::Expression(self.new_phi().into()))
            });

            let stack = self
                .block_ast_node_stack
                .get_mut(&current_block_id)
                .expect("Critical error: stack should always be set for each basic block");

            // If `last` is a BuildingArray, merge it with `second_last`. Otherwise,
            // push both back onto the stack.

            match last {
                ExecutionFrame::BuildingArray(mut last_array) => match second_last {
                    ExecutionFrame::BuildingArray(mut second_last_array) => {
                        last_array.append(&mut second_last_array);
                        stack.push(ExecutionFrame::BuildingArray(last_array));
                    }
                    ExecutionFrame::StandaloneNode(n) => {
                        // Convert n from AstKind to ExprKind
                        let expr = match n {
                            AstKind::Expression(e) => e,
                            _ => {
                                return Err(FunctionDecompilerError::UnexpectedNodeType {
                                    expected: "Expression".to_string(),
                                    context: error_context,
                                    backtrace: Backtrace::capture(),
                                });
                            }
                        };
                        last_array.push(expr);
                        stack.push(ExecutionFrame::BuildingArray(last_array));
                    }
                },
                _ => {
                    stack.push(last);
                    stack.push(second_last);
                }
            }

            return Ok(ProcessedInstructionBuilder::new().build());
        }

        let handlers = global_opcode_handlers();

        let handler =
            handlers
                .get(&instr.opcode)
                .ok_or(FunctionDecompilerError::UnimplementedOpcode {
                    opcode: instr.opcode,
                    context: self.get_error_context(),
                    backtrace: Backtrace::capture(),
                })?;

        // Handle the instruction
        // TODO: Since we have the instruction in the context, we may delete it from the
        // TODO: arguments to avoid passing it around everywhere
        let op = handler.handle_instruction(self, instr)?;

        // Push the SSA ID onto the stack if it exists
        if let Some(ssa_id) = &op.ssa_id {
            self.push_one_node(ssa_id.clone().into())?;
        }

        Ok(op)
    }

    /// If an error happens, this helper function will return the context of the error.
    pub fn get_error_context(&self) -> Box<FunctionDecompilerErrorContext> {
        Box::new(FunctionDecompilerErrorContext {
            current_block_id: self.current_block_id,
            current_instruction: self.current_instruction.clone(),
            current_ast_node_stack: self
                .block_ast_node_stack
                .get(&self.current_block_id)
                .unwrap_or(&Vec::new())
                .to_vec(),
        })
    }

    /// Retrieves the AST stack for a basic block.
    pub fn get_stack(&self, block_id: &BasicBlockId) -> Option<&Vec<ExecutionFrame>> {
        self.block_ast_node_stack.get(block_id)
    }

    /// Create a new phi node, increment the phi count for the current block, and return the new phi node.
    pub fn new_phi(&mut self) -> PhiNode {
        let block_id = self.current_block_id;
        let phi_count = *self.block_ast_phi_count.entry(block_id).or_insert(0);
        let phi = new_phi(phi_count);
        self.block_ast_phi_count.insert(block_id, phi_count + 1);
        phi
    }

    /// Pops an array from the current basic block's stack and returns it as an ArrayKind.
    /// - If no frame is available, returns a PhiArray with empty elements.
    /// - If the last frame is a BuildingArray, returns it as a MergedArray.
    /// - If the last frame is a StandaloneNode, pops additional nodes from the stack
    ///   (both StandaloneNode and BuildingArray frames) and merges them into a PhiArray
    ///   with a newly created phi node.
    pub fn pop_building_array(&mut self) -> Result<ArrayKind, FunctionDecompilerError> {
        let block_id = self.current_block_id;
        let frame_opt = {
            // Scope the mutable borrow so that it ends after pop.
            let stack = self
                .block_ast_node_stack
                .get_mut(&block_id)
                .expect("Critical error: stack should always be set for each basic block");
            stack.pop()
        };

        match frame_opt {
            // No frame in the current block: return an empty PhiArray.
            None => {
                let phi = self.new_phi();
                Ok(ArrayKind::PhiArray(
                    PhiArrayNode::new(phi.into(), Vec::new()).into(),
                ))
            }
            Some(frame) => match frame {
                ExecutionFrame::BuildingArray(array) => {
                    // If we popped a building array, return it as a merged array.
                    let rev: Vec<ExprKind> = array.into_iter().rev().collect();
                    Ok(ArrayKind::MergedArray(ArrayNode::new(rev).into()))
                }
                ExecutionFrame::StandaloneNode(node) => {
                    // Convert node to ExprKind or return an error
                    let node = match node {
                        AstKind::Expression(e) => e,
                        _ => {
                            return Err(FunctionDecompilerError::UnexpectedNodeType {
                                expected: "Expression".to_string(),
                                context: self.get_error_context(),
                                backtrace: Backtrace::capture(),
                            });
                        }
                    };
                    // We encountered a standalone node, which indicates that
                    // the array’s construction started in a predecessor block.
                    // To merge the values, collect this node and any other nodes
                    // currently on the stack.
                    let mut nodes: Vec<ExprKind> = vec![node];

                    // Continue popping nodes until the stack is exhausted or we hit a boundary.
                    {
                        let stack = self.block_ast_node_stack.get_mut(&block_id).expect(
                            "Critical error: stack should always be set for each basic block",
                        );
                        while let Some(frame) = stack.pop() {
                            match frame {
                                ExecutionFrame::StandaloneNode(n) => {
                                    // Convert n from AstKind to ExprKind, or return an error
                                    let expr = match n {
                                        AstKind::Expression(e) => e,
                                        _ => {
                                            return Err(
                                                FunctionDecompilerError::UnexpectedNodeType {
                                                    expected: "Expression".to_string(),
                                                    context: self.get_error_context(),
                                                    backtrace: Backtrace::capture(),
                                                },
                                            );
                                        }
                                    };
                                    nodes.push(expr);
                                }
                                ExecutionFrame::BuildingArray(_) => {
                                    return Err(FunctionDecompilerError::UnexpectedNodeType {
                                        expected: "StandaloneNode".to_string(),
                                        context: self.get_error_context(),
                                        backtrace: Backtrace::capture(),
                                    });
                                }
                            }
                        }
                    }

                    // Create a new phi node to merge these values.
                    let phi = self.new_phi();
                    Ok(ArrayKind::PhiArray(
                        PhiArrayNode::new(phi.into(), nodes).into(),
                    ))
                }
            },
        }
    }

    /// Pops an AST node from the current basic block's stack.
    pub fn pop_one_node(&mut self) -> Result<AstKind, FunctionDecompilerError> {
        let block_id = self.current_block_id;
        let frame_opt = {
            let stack = self
                .block_ast_node_stack
                .get_mut(&block_id)
                .expect("Critical error: stack should always be set for each basic block");
            stack.pop()
        };

        // If there's not a frame to pop, return a new phi node.
        let mut last_frame = match frame_opt {
            Some(frame) => frame,
            None => return Ok(AstKind::Expression(self.new_phi().into())),
        };

        let result = match &mut last_frame {
            ExecutionFrame::BuildingArray(array) => {
                // Pop the node from the array
                if let Some(node) = array.pop() {
                    Ok(AstKind::Expression(node))
                } else {
                    Ok(AstKind::Expression(self.new_phi().into()))
                }
            }
            ExecutionFrame::StandaloneNode(node) => Ok(node.clone()),
        };

        // Push the last frame back onto the stack, even if it's empty
        if let ExecutionFrame::BuildingArray(_) = last_frame {
            let stack = self
                .block_ast_node_stack
                .get_mut(&block_id)
                .expect("Critical error: stack should always be set for each basic block");
            stack.push(last_frame);
        }

        result
    }

    /// Pops an expression from the current basic block's stack.
    pub fn pop_expression(&mut self) -> Result<ExprKind, FunctionDecompilerError> {
        let node = self.pop_one_node()?;
        match node {
            AstKind::Expression(expr) => Ok(expr.clone()),
            _ => Err(FunctionDecompilerError::UnexpectedNodeType {
                expected: "Expression".to_string(),
                context: self.get_error_context(),
                backtrace: Backtrace::capture(),
            }),
        }
    }

    /// Pops an identifier from the current basic block's stack.
    pub fn pop_identifier(&mut self) -> Result<P<IdentifierNode>, FunctionDecompilerError> {
        let node = self.pop_expression()?;

        match node {
            ExprKind::Identifier(ident) => Ok(ident.clone()),
            _ => Err(FunctionDecompilerError::UnexpectedNodeType {
                expected: "Identifier".to_string(),
                context: self.get_error_context(),
                backtrace: Backtrace::capture(),
            }),
        }
    }

    /// Pushes an AST node to the current basic block's stack.
    pub fn push_one_node(&mut self, node: AstKind) -> Result<(), FunctionDecompilerError> {
        let block_id = self.current_block_id;

        let stack = self
            .block_ast_node_stack
            .get_mut(&block_id)
            .expect("Critical error: stack should always be set for each basic block");

        // Check if we're in a frame that requires special handling
        if let Some(last_frame) = stack.last_mut() {
            match last_frame {
                ExecutionFrame::BuildingArray(array) => {
                    // Ensure the node is an expression before adding to the array
                    if let AstKind::Expression(expr) = node {
                        array.push(expr.clone());
                        return Ok(());
                    } else {
                        return Err(FunctionDecompilerError::UnexpectedNodeType {
                            expected: "Expression".to_string(),
                            context: self.get_error_context(),
                            backtrace: Backtrace::capture(),
                        });
                    }
                }
                ExecutionFrame::StandaloneNode(_) => {
                    // Do nothing; let it fall through to the standalone push below
                }
            }
        }

        // If no special frame handling is required, push the node directly onto the stack
        stack.push(ExecutionFrame::StandaloneNode(node));
        Ok(())
    }
}
