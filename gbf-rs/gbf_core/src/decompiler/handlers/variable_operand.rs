#![deny(missing_docs)]

use std::backtrace::Backtrace;

use crate::{
    decompiler::{
        ProcessedInstruction, ProcessedInstructionBuilder,
        ast::{new_assignment, new_fn_call, new_id_with_version},
        function_decompiler::FunctionDecompilerError,
        function_decompiler_context::FunctionDecompilerContext,
    },
    instruction::Instruction,
    opcode::Opcode,
};

use super::OpcodeHandler;

/// Handles other instructions.
pub struct VariableOperandHandler;

impl OpcodeHandler for VariableOperandHandler {
    fn handle_instruction(
        &self,
        context: &mut FunctionDecompilerContext,
        instruction: &Instruction,
    ) -> Result<ProcessedInstruction, FunctionDecompilerError> {
        let arr = context.pop_building_array()?;

        match instruction.opcode {
            Opcode::Call => {
                // Now build the function call node.
                let function_call_node = new_fn_call(arr);

                // Create a new SSA id for this function call.
                let var = context.ssa_context.new_ssa_version_for("fn_call");
                let ssa_id = new_id_with_version("fn_call", var);
                let stmt = new_assignment(ssa_id.clone(), function_call_node);

                Ok(ProcessedInstructionBuilder::new()
                    .ssa_id(ssa_id.into())
                    .push_to_region(stmt.into())
                    .build())
            }
            Opcode::EndParams => Ok(ProcessedInstructionBuilder::new()
                .function_parameters(arr)
                .build()),
            Opcode::EndArray => {
                context.push_one_node(arr.into())?;

                Ok(ProcessedInstructionBuilder::new().build())
            }
            _ => Err(FunctionDecompilerError::UnimplementedOpcode {
                opcode: instruction.opcode,
                context: context.get_error_context(),
                backtrace: Backtrace::capture(),
            }),
        }
    }
}
