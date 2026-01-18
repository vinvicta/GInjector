//! GS2 Bytecode Decompiler
//!
//! Converts GS2 bytecode back into readable GS2 source code.

use crate::{
    function::Function,
    instruction::Instruction,
    opcode::Opcode,
    utils::Gs2BytecodeAddress,
};
use std::collections::{HashMap, HashSet};

/// Decompiler for converting GS2 bytecode to source code
pub struct Decompiler {
    /// String table from the bytecode
    string_table: Vec<String>,
    /// Function table for resolving function names
    function_table: HashMap<Gs2BytecodeAddress, String>,
    /// Current indentation level
    indent: usize,
}

impl Decompiler {
    /// Create a new decompiler
    pub fn new() -> Self {
        Self {
            string_table: Vec::new(),
            function_table: HashMap::new(),
            indent: 0,
        }
    }

    /// Set the string table
    pub fn with_string_table(mut self, strings: Vec<String>) -> Self {
        self.string_table = strings;
        self
    }

    /// Set the function table
    pub fn with_function_table(mut self, functions: HashMap<Gs2BytecodeAddress, String>) -> Self {
        self.function_table = functions;
        self
    }

    /// Decompile a function to GS2 source code
    pub fn decompile_function(&mut self, function: &Function) -> String {
        let mut result = String::new();

        // Get function name
        let name = function.id.name.as_deref().unwrap_or("{entry}");

        // Function declaration
        if name != "{entry}" {
            result.push_str(&format!("function {}(", name));

            // TODO: Extract parameters from the function prologue
            result.push_str(") {\n");
            self.indent += 1;
        }

        // Find the entry block (first block by address)
        let entry_block = function
            .iter()
            .min_by_key(|b| b.id.address)
            .expect("Function should have at least one block");

        // Track visited blocks to handle control flow
        let mut visited = HashSet::new();
        self.decompile_block_recursive(function, entry_block.id.address, &mut visited, &mut result);

        // Close function
        if name != "{entry}" {
            self.indent -= 1;
            result.push_str("}\n\n");
        }

        result
    }

    /// Recursively decompile a basic block and its successors
    fn decompile_block_recursive(
        &mut self,
        function: &Function,
        block_addr: Gs2BytecodeAddress,
        visited: &mut HashSet<Gs2BytecodeAddress>,
        result: &mut String,
    ) {
        if visited.contains(&block_addr) {
            return;
        }
        visited.insert(block_addr);

        // Get the block
        let block = match function.get_basic_block_by_start_address(block_addr) {
            Ok(b) => b,
            Err(_) => return,
        };

        // Skip module end blocks
        if block.id.block_type == crate::basic_block::BasicBlockType::ModuleEnd {
            return;
        }

        // Emit indentation
        for _ in 0..self.indent {
            result.push_str("    ");
        }

        // Process instructions in the block
        let mut i = 0;
        let instructions = &block.instructions;

        while i < instructions.len() {
            let instr = &instructions[i];

            // Handle control flow instructions
            match instr.opcode {
                Opcode::Jmp => {
                    // Unconditional jump - for while loops, labels, etc.
                    if let Some(target) = instr.get_jump_target() {
                        result.push_str(&format!("// jmp to 0x{:X}\n", target));
                    }
                    break;
                }
                Opcode::Jeq => {
                    // Conditional jump - if statement
                    if let Some(target) = instr.get_jump_target() {
                        // Check if this is an if-else or if-then pattern
                        let has_else = self.has_else_block(function, block_addr);

                        result.push_str("if (/* TODO: condition */) {\n");
                        self.indent += 1;

                        // Recursively decompile the true branch
                        let next_block = self.get_next_block_address(function, block_addr);
                        if let Some(next_addr) = next_block {
                            self.decompile_block_recursive(function, next_addr, visited, result);
                        }

                        self.indent -= 1;
                        for _ in 0..self.indent {
                            result.push_str("    ");
                        }
                        result.push_str("}");

                        if has_else {
                            result.push_str(" else {\n");
                            self.indent += 1;
                            self.decompile_block_recursive(function, target, visited, result);
                            self.indent -= 1;
                            for _ in 0..self.indent {
                                result.push_str("    ");
                            }
                            result.push_str("}");
                        }
                        result.push_str("\n");
                    }
                    break;
                }
                Opcode::Ret => {
                    result.push_str("return;\n");
                    i += 1;
                    continue;
                }
                _ => {
                    // Regular instruction - emit as expression or statement
                    if let Some(stmt) = self.instruction_to_statement(instr) {
                        result.push_str(&stmt);
                        result.push('\n');
                    }
                }
            }

            i += 1;
        }
    }

    /// Check if a block has an else branch
    fn has_else_block(&self, function: &Function, block_addr: Gs2BytecodeAddress) -> bool {
        // Simple heuristic: check if there's another block at a different address
        // that could be an else branch
        function.iter().filter(|b| b.id.address != block_addr).count() > 1
    }

    /// Get the next block address (fallthrough)
    fn get_next_block_address(
        &self,
        function: &Function,
        current_addr: Gs2BytecodeAddress,
    ) -> Option<Gs2BytecodeAddress> {
        // Find the block with the next highest address
        function
            .iter()
            .map(|b| b.id.address)
            .filter(|&addr| addr > current_addr)
            .min()
    }

    /// Convert an instruction to a GS2 statement/expression
    fn instruction_to_statement(&self, instr: &Instruction) -> Option<String> {
        match instr.opcode {
            // Type pushes - these are likely part of an expression
            Opcode::PushNumber => {
                if let Some(operand) = &instr.operand {
                    if let Ok(num) = operand.get_number_value() {
                        return Some(format!("temp.value = {};", num));
                    }
                }
                Some(format!("// {}", instr.opcode))
            }
            Opcode::PushString => {
                if let Some(operand) = &instr.operand {
                    if let Ok(idx) = operand.get_number_value() {
                        if let Some(s) = self.string_table.get(idx as usize) {
                            return Some(format!("temp.value = \"{}\";", s));
                        }
                    }
                }
                Some(format!("// {}", instr.opcode))
            }
            Opcode::PushTrue => Some("temp.value = true;".to_string()),
            Opcode::PushFalse => Some("temp.value = false;".to_string()),
            Opcode::PushNull => Some("temp.value = null;".to_string()),

            // Variable operations
            Opcode::PushVariable => {
                if let Some(operand) = &instr.operand {
                    if let Ok(idx) = operand.get_number_value() {
                        if let Some(s) = self.string_table.get(idx as usize) {
                            return Some(format!("// var {}", s));
                        }
                    }
                }
                Some(format!("// {}", instr.opcode))
            }

            // Binary operations
            Opcode::Add => Some("// + operation".to_string()),
            Opcode::Subtract => Some("// - operation".to_string()),
            Opcode::Multiply => Some("// * operation".to_string()),
            Opcode::Divide => Some("// / operation".to_string()),
            Opcode::Assign => Some("// = assignment".to_string()),

            // Control flow (handled separately)
            Opcode::Jmp | Opcode::Jeq | Opcode::Ret => None,

            // Function calls
            Opcode::Call => {
                if let Some(operand) = &instr.operand {
                    if let Ok(idx) = operand.get_number_value() {
                        if let Some(func_name) = self.function_table.get(&(idx as Gs2BytecodeAddress)) {
                            return Some(format!("{}();", func_name));
                        }
                    }
                }
                Some("// function call".to_string())
            }

            // Default - use Display for opcode name
            _ => Some(format!("// {}", instr.opcode)),
        }
    }
}

impl Default for Decompiler {
    fn default() -> Self {
        Self::new()
    }
}

// Add helper methods to Instruction
pub trait InstructionExt {
    /// Get the jump target address if this instruction has one
    fn get_jump_target(&self) -> Option<Gs2BytecodeAddress>;
}

impl InstructionExt for Instruction {
    fn get_jump_target(&self) -> Option<Gs2BytecodeAddress> {
        if self.opcode.is_conditional_jump() || self.opcode == Opcode::Jmp {
            if let Some(operand) = &self.operand {
                if let Ok(target) = operand.get_number_value() {
                    return Some(target as Gs2BytecodeAddress);
                }
            }
        }
        None
    }
}
