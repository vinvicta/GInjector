//! Bytecode emitter for GS2
//!
//! Emits Graal bytecode from an AST.

use crate::ast::{Expression, Literal, Program, Statement};
use crate::error::Result;
use crate::opcode::Opcode;

/// Bytecode emitter
pub struct BytecodeEmitter {
    /// The output bytes
    output: Vec<u8>,
    /// String table
    strings: Vec<String>,
    /// Function table (function locations)
    functions: Vec<(String, u32)>,
    /// Current instruction address
    current_address: u32,
    /// Break label stack for loops
    break_stack: Vec<usize>,
    /// Continue label stack for loops
    continue_stack: Vec<usize>,
}

impl BytecodeEmitter {
    /// Create a new bytecode emitter
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
            strings: Vec::new(),
            functions: Vec::new(),
            current_address: 0,
            break_stack: Vec::new(),
            continue_stack: Vec::new(),
        }
    }

    /// Emit a complete program
    pub fn emit_program(&mut self, program: &Program) {
        // First pass: collect strings and functions
        self.collect_strings_and_functions(program);

        // Second pass: emit bytecode
        let _ = self.emit_gs1_flags();
        let _ = self.emit_functions();
        let _ = self.emit_strings();
        let _ = self.emit_instructions(program);
    }

    /// Collect strings and function information from the program
    fn collect_strings_and_functions(&mut self, program: &Program) {
        for statement in &program.statements {
            self.collect_from_statement(statement);
        }
    }

    /// Collect strings and functions from a statement
    fn collect_from_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::FunctionDeclaration { name, body, .. } => {
                let location = self.current_address;
                self.functions.push((name.name.clone(), location));
                self.collect_from_statement(body);
            }
            Statement::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_from_statement(stmt);
                }
            }
            Statement::Expression { expr, .. } => {
                self.collect_from_expression(expr);
            }
            Statement::If { condition, true_block, false_block, .. } => {
                self.collect_from_expression(condition);
                self.collect_from_statement(true_block);
                if let Some(fb) = false_block {
                    self.collect_from_statement(fb);
                }
            }
            Statement::While { condition, body, .. } => {
                self.collect_from_expression(condition);
                self.collect_from_statement(body);
            }
            Statement::For { init, condition, increment, body, .. } => {
                if let Some(i) = init {
                    self.collect_from_statement(i);
                }
                if let Some(c) = condition {
                    self.collect_from_expression(c);
                }
                if let Some(i) = increment {
                    self.collect_from_statement(i);
                }
                self.collect_from_statement(body);
            }
            Statement::Return { expr, .. } => {
                if let Some(e) = expr {
                    self.collect_from_expression(e);
                }
            }
            Statement::With { obj, body, .. } => {
                self.collect_from_expression(obj);
                self.collect_from_statement(body);
            }
            Statement::Switch { expr, cases, default_case, .. } => {
                self.collect_from_expression(expr);
                for (case_expr, body) in cases {
                    self.collect_from_expression(case_expr);
                    self.collect_from_statement(body);
                }
                if let Some(dc) = default_case {
                    self.collect_from_statement(dc);
                }
            }
            Statement::ForEach { array, body, .. } => {
                self.collect_from_expression(array);
                self.collect_from_statement(body);
            }
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    /// Collect strings from an expression
    fn collect_from_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal { literal, .. } => {
                if let Literal::String { value, .. } = &**literal {
                    if !self.strings.contains(value) {
                        self.strings.push(value.clone());
                    }
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.collect_from_expression(left);
                self.collect_from_expression(right);
            }
            Expression::UnaryOp { expr, .. } => {
                self.collect_from_expression(expr);
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                self.collect_from_expression(condition);
                self.collect_from_expression(true_expr);
                self.collect_from_expression(false_expr);
            }
            Expression::FunctionCall { target, args, .. } => {
                self.collect_from_expression(target);
                for arg in args {
                    self.collect_from_expression(arg);
                }
            }
            Expression::ArrayAccess { array, index, .. } => {
                self.collect_from_expression(array);
                self.collect_from_expression(index);
            }
            Expression::MemberAccess { object, .. } => {
                self.collect_from_expression(object);
            }
            Expression::ArrayLiteral { elements, .. } => {
                for elem in elements {
                    self.collect_from_expression(elem);
                }
            }
            Expression::ObjectLiteral { properties, .. } => {
                for (_, value) in properties {
                    self.collect_from_expression(value);
                }
            }
            Expression::Identifier { .. } => {}
        }
    }

    /// Emit the GS1 flags section
    fn emit_gs1_flags(&mut self) -> Result<()> {
        self.write_u32(1)?; // Section type: Gs1Flags
        self.write_u32(4)?; // Section length: 4
        self.write_u32(0)?; // Flags: 0
        Ok(())
    }

    /// Emit the functions section
    fn emit_functions(&mut self) -> Result<()> {
        self.write_u32(2)?; // Section type: Functions

        let mut length = 0usize;
        for (name, _location) in &self.functions {
            length += 4; // function location
            length += name.len() + 1; // name + null terminator
        }

        self.write_u32(length as u32)?;

        for (name, location) in self.functions.clone() {
            self.write_u32(location)?;
            self.write_string(&name)?;
        }

        Ok(())
    }

    /// Emit the strings section
    fn emit_strings(&mut self) -> Result<()> {
        self.write_u32(3)?; // Section type: Strings

        let mut length = 0usize;
        for string in &self.strings {
            length += string.len() + 1;
        }

        self.write_u32(length as u32)?;

        for string in self.strings.clone() {
            self.write_string(&string)?;
        }

        Ok(())
    }

    /// Emit the instructions section
    fn emit_instructions(&mut self, program: &Program) -> Result<()> {
        let instructions_start = self.output.len();

        self.write_u32(4)?; // Section type: Instructions
        let length_offset = self.output.len();
        self.write_u32(0)?; // Section length placeholder

        // Emit instructions for each statement
        for statement in &program.statements {
            let _ = self.emit_statement(statement);
        }

        let instructions_end = self.output.len();
        let instructions_length = (instructions_end - instructions_start - 8) as u32;
        let length_bytes = instructions_length.to_be_bytes();
        self.output[length_offset..length_offset + 4].copy_from_slice(&length_bytes);

        Ok(())
    }

    /// Emit a statement
    fn emit_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::FunctionDeclaration { body, .. } => {
                // Emit function start marker
                self.emit_opcode(Opcode::FunctionStart)?;
                let _ = self.emit_statement(body);
                self.emit_opcode(Opcode::Ret)?;
            }
            Statement::Block { statements, .. } => {
                for stmt in statements {
                    let _ = self.emit_statement(stmt);
                }
            }
            Statement::Expression { expr, .. } => {
                let _ = self.emit_expression(expr)?;
                // Pop the result if it's not used
                // self.emit_opcode(Opcode::Pop)?;
            }
            Statement::Return { expr, .. } => {
                if let Some(e) = expr {
                    let _ = self.emit_expression(e)?;
                }
                self.emit_opcode(Opcode::Ret)?;
            }
            Statement::If { condition, true_block, false_block, .. } => {
                let _ = self.emit_if(condition, true_block, false_block.as_deref());
            }
            Statement::While { condition, body, .. } => {
                let _ = self.emit_while(condition, body);
            }
            Statement::For { init, condition, increment, body, .. } => {
                let _ = self.emit_for(init, condition, increment, body);
            }
            Statement::ForEach { item, array, body, .. } => {
                let _ = self.emit_for_each(item, array, body);
            }
            Statement::Break { .. } => {
                if let Some(target) = self.break_stack.last().copied() {
                    self.emit_opcode(Opcode::Jmp)?;
                    self.write_u32(target as u32)?;
                }
            }
            Statement::Continue { .. } => {
                if let Some(target) = self.continue_stack.last().copied() {
                    self.emit_opcode(Opcode::Jmp)?;
                    self.write_u32(target as u32)?;
                }
            }
            Statement::With { obj, body, .. } => {
                let _ = self.emit_with(obj, body);
            }
            Statement::Switch { .. } => {
                // TODO: Implement switch statements
            }
        }
        Ok(())
    }

    /// Emit an if statement
    fn emit_if(
        &mut self,
        condition: &Expression,
        true_block: &Statement,
        false_block: Option<&Statement>,
    ) -> Result<()> {
        let _ = self.emit_expression(condition)?;
        self.emit_opcode(Opcode::Jne)?;
        let else_addr_offset = self.output.len();
        self.write_u32(0)?;

        let _ = self.emit_statement(true_block);

        if let Some(false_block) = false_block {
            self.emit_opcode(Opcode::Jmp)?;
            let end_addr_offset = self.output.len();
            self.write_u32(0)?;

            let else_addr = self.current_address as u32;
            self.update_u32_at(else_addr_offset, else_addr)?;

            let _ = self.emit_statement(false_block);

            let end_addr = self.current_address as u32;
            self.update_u32_at(end_addr_offset, end_addr)?;
        } else {
            let else_addr = self.current_address as u32;
            self.update_u32_at(else_addr_offset, else_addr)?;
        }

        Ok(())
    }

    /// Emit a while loop
    fn emit_while(&mut self, condition: &Expression, body: &Statement) -> Result<()> {
        let loop_start = self.output.len();

        let _ = self.emit_expression(condition)?;
        self.emit_opcode(Opcode::Jne)?;
        let end_addr_offset = self.output.len();
        self.write_u32(0)?;

        // Set up break/continue targets
        let loop_end = self.output.len(); // placeholder
        self.break_stack.push(loop_end);
        self.continue_stack.push(loop_start);

        let _ = self.emit_statement(body);

        // Pop break/continue targets
        self.break_stack.pop();
        self.continue_stack.pop();

        self.emit_opcode(Opcode::Jmp)?;
        self.write_u32(loop_start as u32)?;

        let end_addr = self.current_address as u32;
        self.update_u32_at(end_addr_offset, end_addr)?;

        Ok(())
    }

    /// Emit a for loop
    fn emit_for(
        &mut self,
        init: &Option<Box<Statement>>,
        condition: &Option<Expression>,
        increment: &Option<Box<Statement>>,
        body: &Statement,
    ) -> Result<()> {
        // Emit initialization
        if let Some(init) = init {
            let _ = self.emit_statement(init)?;
        }

        let loop_start = self.output.len();
        let increment_addr = self.output.len(); // Will be updated

        // Emit condition
        if let Some(condition) = condition {
            let _ = self.emit_expression(condition)?;
        } else {
            // No condition means always true - push true
            self.emit_opcode(Opcode::PushTrue)?;
        }
        self.emit_opcode(Opcode::Jne)?;
        let end_addr_offset = self.output.len();
        self.write_u32(0)?;

        // Set up break/continue targets
        self.break_stack.push(self.output.len());
        self.continue_stack.push(increment_addr);

        let _ = self.emit_statement(body)?;

        // Increment
        let _actual_increment_addr = self.output.len();
        if let Some(increment) = increment {
            let _ = self.emit_statement(increment)?;
        }

        // Jump back to condition
        self.emit_opcode(Opcode::Jmp)?;
        self.write_u32(loop_start as u32)?;

        // End of loop
        let end_addr = self.current_address as u32;
        self.update_u32_at(end_addr_offset, end_addr)?;

        self.break_stack.pop();
        self.continue_stack.pop();

        Ok(())
    }

    /// Emit a for-each loop
    fn emit_for_each(&mut self, _item: &crate::ast::Identifier, array: &Expression, body: &Statement) -> Result<()> {
        // Emit array expression
        let _ = self.emit_expression(array)?;

        // ForEach opcode expects the array on the stack
        self.emit_opcode(Opcode::ForEach)?;
        let end_addr_offset = self.output.len();
        self.write_u32(0)?;

        // The loop variable is available as a special variable
        // For now, we'll emit the body assuming the item is accessible
        let _ = self.emit_statement(body)?;

        // Jump back to ForEach
        self.emit_opcode(Opcode::Jmp)?;
        self.write_u32((end_addr_offset - 5) as u32)?;

        let end_addr = self.current_address as u32;
        self.update_u32_at(end_addr_offset, end_addr)?;

        Ok(())
    }

    /// Emit a with statement
    fn emit_with(&mut self, obj: &Expression, body: &Statement) -> Result<()> {
        let _ = self.emit_expression(obj)?;
        self.emit_opcode(Opcode::With)?;

        let _ = self.emit_statement(body)?;

        self.emit_opcode(Opcode::WithEnd)?;
        Ok(())
    }

    /// Emit an expression
    fn emit_expression(&mut self, expr: &Expression) -> Result<()> {
        match expr {
            Expression::Literal { literal, .. } => {
                self.emit_literal(literal)?;
            }
            Expression::Identifier { identifier, .. } => {
                self.emit_identifier(&identifier.name)?;
            }
            Expression::BinaryOp { left, op, right, .. } => {
                self.emit_binary_op(left, *op, right)?;
            }
            Expression::UnaryOp { op, expr, .. } => {
                self.emit_unary_op(*op, expr)?;
            }
            Expression::FunctionCall { target, args, .. } => {
                self.emit_function_call(target, args)?;
            }
            Expression::ArrayAccess { array, index, .. } => {
                let _ = self.emit_expression(array)?;
                let _ = self.emit_expression(index)?;
                self.emit_opcode(Opcode::ObjIndex)?;
            }
            Expression::MemberAccess { object, property, .. } => {
                let _ = self.emit_expression(object)?;
                self.emit_member_access_name(&property.name)?;
            }
            Expression::Ternary { condition, true_expr, false_expr, .. } => {
                self.emit_ternary(condition, true_expr, false_expr)?;
            }
            Expression::ArrayLiteral { elements, .. } => {
                self.emit_array_literal(elements)?;
            }
            Expression::ObjectLiteral { properties, .. } => {
                self.emit_object_literal(properties)?;
            }
        }
        Ok(())
    }

    /// Emit a literal
    fn emit_literal(&mut self, literal: &Literal) -> Result<()> {
        match literal {
            Literal::Number { value, .. } => {
                if let Ok(n) = value.parse::<i32>() {
                    // Emit PushNumber opcode followed by the value
                    self.emit_opcode(Opcode::PushNumber)?;
                    self.write_i32(n)?;
                } else {
                    // Handle floating point
                    self.emit_opcode(Opcode::PushNumber)?;
                    // For floats, we'd need to handle them differently
                    // For now, parse as f32 and write as bytes
                    if let Ok(f) = value.parse::<f32>() {
                        self.write_f32(f)?;
                    }
                }
            }
            Literal::String { value, .. } => {
                let index = self.strings.iter().position(|s| s == value)
                    .unwrap_or(0);
                self.emit_opcode(Opcode::PushString)?;
                self.emit_u8(index as u8)?;
            }
            Literal::Boolean { value, .. } => {
                if *value {
                    self.emit_opcode(Opcode::PushTrue)?;
                } else {
                    self.emit_opcode(Opcode::PushFalse)?;
                }
            }
            Literal::Null { .. } => {
                self.emit_opcode(Opcode::PushNull)?;
            }
        }
        Ok(())
    }

    /// Emit an identifier reference
    fn emit_identifier(&mut self, name: &str) -> Result<()> {
        if name.starts_with("temp.") {
            self.emit_opcode(Opcode::Temp)?;
            let prop = &name[5..];
            self.emit_member_access_name(prop)?;
        } else if name.starts_with("this.") {
            self.emit_opcode(Opcode::This)?;
            let prop = &name[5..];
            self.emit_member_access_name(prop)?;
        } else if name.starts_with("player.") {
            self.emit_opcode(Opcode::Player)?;
            let prop = &name[7..];
            self.emit_member_access_name(prop)?;
        } else if name.starts_with("level.") {
            self.emit_opcode(Opcode::Level)?;
            let prop = &name[6..];
            self.emit_member_access_name(prop)?;
        } else {
            // Regular variable access through temp
            self.emit_opcode(Opcode::Temp)?;
            self.emit_member_access_name(name)?;
        }
        Ok(())
    }

    /// Emit a binary operation
    fn emit_binary_op(&mut self, left: &Expression, op: crate::ast::expression::BinaryOp, right: &Expression) -> Result<()> {
        // Check if this is an assignment
        if op.is_assignment() {
            return self.emit_assignment(left, op, right);
        }

        let _ = self.emit_expression(left)?;
        let _ = self.emit_expression(right)?;

        let opcode = match op {
            crate::ast::expression::BinaryOp::Add => Opcode::Add,
            crate::ast::expression::BinaryOp::Subtract => Opcode::Subtract,
            crate::ast::expression::BinaryOp::Multiply => Opcode::Multiply,
            crate::ast::expression::BinaryOp::Divide => Opcode::Divide,
            crate::ast::expression::BinaryOp::Modulo => Opcode::Modulo,
            crate::ast::expression::BinaryOp::Power => Opcode::Power,
            crate::ast::expression::BinaryOp::Equal => Opcode::Equal,
            crate::ast::expression::BinaryOp::NotEqual => Opcode::NotEqual,
            crate::ast::expression::BinaryOp::LessThan => Opcode::LessThan,
            crate::ast::expression::BinaryOp::GreaterThan => Opcode::GreaterThan,
            crate::ast::expression::BinaryOp::LessThanOrEqual => Opcode::LessThanOrEqual,
            crate::ast::expression::BinaryOp::GreaterThanOrEqual => Opcode::GreaterThanOrEqual,
            crate::ast::expression::BinaryOp::LogicalAnd => Opcode::ShortCircuitAnd,
            crate::ast::expression::BinaryOp::LogicalOr => Opcode::ShortCircuitOr,
            crate::ast::expression::BinaryOp::BitwiseAnd => Opcode::BitwiseAnd,
            crate::ast::expression::BinaryOp::BitwiseOr => Opcode::BitwiseOr,
            crate::ast::expression::BinaryOp::BitwiseXor => Opcode::BitwiseXor,
            crate::ast::expression::BinaryOp::LeftShift => Opcode::ShiftLeft,
            crate::ast::expression::BinaryOp::RightShift => Opcode::ShiftRight,
            _ => return Ok(()),
        };

        self.emit_opcode(opcode)?;
        Ok(())
    }

    /// Emit an assignment operation
    fn emit_assignment(&mut self, target: &Expression, op: crate::ast::expression::BinaryOp, value: &Expression) -> Result<()> {
        // For simple assignment, we need to:
        // 1. Emit the target (to get the object/reference)
        // 2. Emit the value
        // 3. Emit Assign opcode

        // For compound assignment (+=, -=, etc.), we need to:
        // 1. Emit the target (to get the current value)
        // 2. Emit the value
        // 3. Emit the operation
        // 4. Store back

        let is_compound = !matches!(op, crate::ast::expression::BinaryOp::Assign);

        if is_compound {
            // For compound assignment, we first need to read the current value
            match target {
                Expression::Identifier { identifier, .. } => {
                    self.emit_identifier(&identifier.name)?;
                }
                Expression::MemberAccess { object, property, .. } => {
                    let _ = self.emit_expression(object)?;
                    self.emit_member_access_name(&property.name)?;
                }
                Expression::ArrayAccess { array, index, .. } => {
                    let _ = self.emit_expression(array)?;
                    let _ = self.emit_expression(index)?;
                }
                _ => {}
            }
        }

        // Emit the value
        let _ = self.emit_expression(value)?;

        if is_compound {
            // Emit the operation
            let opcode = match op {
                crate::ast::expression::BinaryOp::AddAssign => Opcode::Add,
                crate::ast::expression::BinaryOp::SubtractAssign => Opcode::Subtract,
                crate::ast::expression::BinaryOp::MultiplyAssign => Opcode::Multiply,
                crate::ast::expression::BinaryOp::DivideAssign => Opcode::Divide,
                crate::ast::expression::BinaryOp::ModuloAssign => Opcode::Modulo,
                crate::ast::expression::BinaryOp::PowerAssign => Opcode::Power,
                crate::ast::expression::BinaryOp::LeftShiftAssign => Opcode::ShiftLeft,
                crate::ast::expression::BinaryOp::RightShiftAssign => Opcode::ShiftRight,
                crate::ast::expression::BinaryOp::BitwiseAndAssign => Opcode::BitwiseAnd,
                crate::ast::expression::BinaryOp::BitwiseOrAssign => Opcode::BitwiseOr,
                crate::ast::expression::BinaryOp::BitwiseXorAssign => Opcode::BitwiseXor,
                _ => return Ok(()),
            };
            self.emit_opcode(opcode)?;
        }

        // Emit the target again for the assignment
        match target {
            Expression::Identifier { identifier, .. } => {
                self.emit_identifier(&identifier.name)?;
            }
            Expression::MemberAccess { object, property, .. } => {
                let _ = self.emit_expression(object)?;
                self.emit_member_access_name(&property.name)?;
            }
            Expression::ArrayAccess { array, index, .. } => {
                let _ = self.emit_expression(array)?;
                let _ = self.emit_expression(index)?;
            }
            _ => {}
        }

        self.emit_opcode(Opcode::Assign)?;
        Ok(())
    }

    /// Emit a unary operation
    fn emit_unary_op(&mut self, op: crate::ast::expression::UnaryOp, expr: &Expression) -> Result<()> {
        let _ = self.emit_expression(expr)?;

        let opcode = match op {
            crate::ast::expression::UnaryOp::LogicalNot => Opcode::LogicalNot,
            crate::ast::expression::UnaryOp::Negate => Opcode::UnarySubtract,
            crate::ast::expression::UnaryOp::BitwiseInvert => Opcode::BitwiseInvert,
        };

        self.emit_opcode(opcode)?;
        Ok(())
    }

    /// Emit a function call
    fn emit_function_call(&mut self, target: &Expression, args: &[Expression]) -> Result<()> {
        // Push the target first
        let _ = self.emit_expression(target)?;

        // Push arguments (left to right)
        for arg in args {
            let _ = self.emit_expression(arg)?;
        }

        // Emit Call opcode with argument count
        self.emit_opcode(Opcode::Call)?;
        self.emit_u8(args.len() as u8)?;

        Ok(())
    }

    /// Emit member access by name
    fn emit_member_access_name(&mut self, name: &str) -> Result<()> {
        // Add the name to strings if not already present
        if !self.strings.iter().any(|s| s == name) {
            self.strings.push(name.to_string());
        }

        let index = self.strings.iter().position(|s| s == name)
            .unwrap_or(0);

        // Emit AccessMember opcode followed by string index
        self.emit_opcode(Opcode::AccessMember)?;
        self.emit_u8(index as u8)?;

        Ok(())
    }

    /// Emit a ternary expression
    fn emit_ternary(&mut self, condition: &Expression, true_expr: &Expression, false_expr: &Expression) -> Result<()> {
        let _ = self.emit_expression(condition)?;
        self.emit_opcode(Opcode::Jne)?;
        let false_addr_offset = self.output.len();
        self.write_u32(0)?;

        let _ = self.emit_expression(true_expr)?;

        self.emit_opcode(Opcode::Jmp)?;
        let end_addr_offset = self.output.len();
        self.write_u32(0)?;

        let false_addr = self.current_address as u32;
        self.update_u32_at(false_addr_offset, false_addr)?;

        let _ = self.emit_expression(false_expr)?;

        let end_addr = self.current_address as u32;
        self.update_u32_at(end_addr_offset, end_addr)?;

        Ok(())
    }

    /// Emit an array literal
    fn emit_array_literal(&mut self, elements: &[Expression]) -> Result<()> {
        // Emit NewUninitializedArray
        self.emit_opcode(Opcode::NewUninitializedArray)?;

        // Push each element
        for elem in elements {
            let _ = self.emit_expression(elem)?;
        }

        // Emit EndArray
        self.emit_opcode(Opcode::EndArray)?;

        Ok(())
    }

    /// Emit an object literal
    fn emit_object_literal(&mut self, properties: &[(crate::ast::Identifier, Expression)]) -> Result<()> {
        // Emit NewObject
        self.emit_opcode(Opcode::NewObject)?;

        // For each property, emit the property name and value
        for (ident, value) in properties {
            // Push property name as string
            let _ = self.emit_expression(&Expression::Literal {
                literal: Box::new(Literal::String {
                    value: ident.name.clone(),
                    location: Default::default(),
                }),
                location: Default::default(),
            })?;

            // Push value
            let _ = self.emit_expression(value)?;

            // Emit Assign to set the property
            self.emit_opcode(Opcode::Assign)?;
        }

        Ok(())
    }

    /// Emit an opcode
    fn emit_opcode(&mut self, opcode: Opcode) -> Result<()> {
        self.output.push(opcode as u8);
        self.current_address += 1;
        Ok(())
    }

    /// Write a u8
    fn emit_u8(&mut self, value: u8) -> Result<()> {
        self.output.push(value);
        self.current_address += 1;
        Ok(())
    }

    /// Write a u32 (Graal encoding)
    fn write_u32(&mut self, value: u32) -> Result<()> {
        let mut encoded = Vec::new();
        GraalWriter::encode_u32(value, &mut encoded);
        self.output.extend_from_slice(&encoded);
        self.current_address += encoded.len() as u32;
        Ok(())
    }

    /// Write an i32 as big-endian bytes
    fn write_i32(&mut self, value: i32) -> Result<()> {
        self.output.extend_from_slice(&value.to_be_bytes());
        self.current_address += 4;
        Ok(())
    }

    /// Write an f32 as big-endian bytes
    fn write_f32(&mut self, value: f32) -> Result<()> {
        self.output.extend_from_slice(&value.to_be_bytes());
        self.current_address += 4;
        Ok(())
    }

    /// Update a u32 at a specific offset
    fn update_u32_at(&mut self, offset: usize, value: u32) -> Result<()> {
        let mut encoded = Vec::new();
        GraalWriter::encode_u32(value, &mut encoded);
        self.output[offset..offset + encoded.len()].copy_from_slice(&encoded);
        Ok(())
    }

    /// Write a string
    fn write_string(&mut self, s: &str) -> Result<()> {
        self.output.extend_from_slice(s.as_bytes());
        self.output.push(0);
        Ok(())
    }

    /// Convert to bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.output
    }
}

/// Minimal Graal writer for encoding
struct GraalWriter;

impl GraalWriter {
    fn encode_u32(value: u32, buffer: &mut Vec<u8>) {
        if value <= 0xDF {
            buffer.push((value + 32) as u8);
        } else if value <= 0x705F {
            let byte1 = ((value >> 7) & 0x7F) + 32;
            let byte2 = (value & 0x7F) + 32;
            buffer.push(byte1 as u8);
            buffer.push(byte2 as u8);
        } else if value <= 0x38305F {
            let byte1 = ((value >> 14) & 0x7F) + 32;
            let byte2 = ((value >> 7) & 0x7F) + 32;
            let byte3 = (value & 0x7F) + 32;
            buffer.push(byte1 as u8);
            buffer.push(byte2 as u8);
            buffer.push(byte3 as u8);
        } else {
            let byte1 = ((value >> 21) & 0x7F) + 32;
            let byte2 = ((value >> 14) & 0x7F) + 32;
            let byte3 = ((value >> 7) & 0x7F) + 32;
            let byte4 = (value & 0x7F) + 32;
            buffer.push(byte1 as u8);
            buffer.push(byte2 as u8);
            buffer.push(byte3 as u8);
            buffer.push(byte4 as u8);
        }
    }
}
