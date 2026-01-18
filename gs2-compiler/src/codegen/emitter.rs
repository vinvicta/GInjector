//! GS2 Bytecode Emitter
//!
//! Emits Graal bytecode from an AST, matching the official compiler format.
//!
//! Key implementation notes from studying the official C++ compiler:
//! - Opcode values match exactly from opcodes.h
//! - Immediate encoders (0xF0-0xF6) depend on previous opcode:
//!   - After OP_TYPE_NUMBER: offset = 3 -> 0xF3-0xF5 for numbers
//!   - After OP_TYPE_VAR/OP_TYPE_STRING: offset = 0 -> 0xF0-0xF2 for strings/vars
//! - Function addresses use operation index (opIndex) NOT byte offset
//! - Jump targets are encoded as big-endian int16 to operation index
//! - Section format: big-endian u32 type, big-endian u32 length, data
//! - RET is emitted at the very end

#![allow(non_camel_case_types)]

use crate::ast::{Expression, Identifier, Literal, Program, Statement};
use crate::error::Result;
use std::collections::{HashMap, HashSet};

// ==============================================================================
// Opcodes - exact values from official compiler opcodes.h
// ==============================================================================

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    OP_NONE = 0,
    OP_SET_INDEX = 1,
    OP_SET_INDEX_TRUE = 2,
    OP_OR = 3,
    OP_IF = 4,
    OP_AND = 5,
    OP_CALL = 6,
    OP_RET = 7,
    OP_SLEEP = 8,
    OP_CMD_CALL = 9,
    OP_JMP = 10,
    OP_WAITFOR = 11,

    OP_TYPE_NUMBER = 20,
    OP_TYPE_STRING = 21,
    OP_TYPE_VAR = 22,
    OP_TYPE_ARRAY = 23,
    OP_TYPE_TRUE = 24,
    OP_TYPE_FALSE = 25,
    OP_TYPE_NULL = 26,
    OP_PI = 27,

    OP_COPY_LAST_OP = 30,
    OP_SWAP_LAST_OPS = 31,
    OP_INDEX_DEC = 32,
    OP_CONV_TO_FLOAT = 33,
    OP_CONV_TO_STRING = 34,
    OP_MEMBER_ACCESS = 35,
    OP_CONV_TO_OBJECT = 36,
    OP_ARRAY_END = 37,
    OP_ARRAY_NEW = 38,
    OP_SETARRAY = 39,
    OP_INLINE_NEW = 40,
    OP_MAKEVAR = 41,
    OP_NEW_OBJECT = 42,
    OP_OBJ_FROM_STR = 43,
    OP_INLINE_CONDITIONAL = 44,
    OP_UNKNOWN_45 = 45,
    OP_UNKNOWN_46 = 46,
    OP_UNKNOWN_47 = 47,

    OP_ASSIGN = 50,
    OP_FUNC_PARAMS_END = 51,
    OP_INC = 52,
    OP_DEC = 53,
    OP_UNKNOWN_54 = 54,

    OP_ADD = 60,
    OP_SUB = 61,
    OP_MUL = 62,
    OP_DIV = 63,
    OP_MOD = 64,
    OP_POW = 65,
    OP_UNKNOWN_66 = 66,
    OP_UNKNOWN_67 = 67,
    OP_NOT = 68,
    OP_UNARYSUB = 69,
    OP_EQ = 70,
    OP_NEQ = 71,
    OP_LT = 72,
    OP_GT = 73,
    OP_LTE = 74,
    OP_GTE = 75,
    OP_BWO = 76,
    OP_BWA = 77,
    OP_BWX = 78,
    OP_BWI = 79,
    OP_IN_RANGE = 80,
    OP_IN_OBJ = 81,
    OP_OBJ_INDEX = 82,
    OP_OBJ_TYPE = 83,
    OP_FORMAT = 84,
    OP_INT = 85,
    OP_ABS = 86,
    OP_RANDOM = 87,
    OP_SIN = 88,
    OP_COS = 89,
    OP_ARCTAN = 90,
    OP_EXP = 91,
    OP_LOG = 92,
    OP_MIN = 93,
    OP_MAX = 94,
    OP_GETANGLE = 95,
    OP_GETDIR = 96,
    OP_VECX = 97,
    OP_VECY = 98,
    OP_OBJ_INDICES = 99,
    OP_OBJ_LINK = 100,
    OP_BW_LEFTSHIFT = 101,
    OP_BW_RIGHTSHIFT = 102,
    OP_CHAR = 103,
    OP_OBJ_COMPARE = 104,

    OP_OBJ_TRIM = 110,
    OP_OBJ_LENGTH = 111,
    OP_OBJ_POS = 112,
    OP_JOIN = 113,
    OP_OBJ_CHARAT = 114,
    OP_OBJ_SUBSTR = 115,
    OP_OBJ_STARTS = 116,
    OP_OBJ_ENDS = 117,
    OP_OBJ_TOKENIZE = 118,
    OP_TRANSLATE = 119,
    OP_OBJ_POSITIONS = 120,

    OP_OBJ_SIZE = 130,
    OP_ARRAY = 131,
    OP_ARRAY_ASSIGN = 132,
    OP_ARRAY_MULTIDIM = 133,
    OP_ARRAY_MULTIDIM_ASSIGN = 134,
    OP_OBJ_SUBARRAY = 135,
    OP_OBJ_ADDSTRING = 136,
    OP_OBJ_DELETESTRING = 137,
    OP_OBJ_REMOVESTRING = 138,
    OP_OBJ_REPLACESTRING = 139,
    OP_OBJ_INSERTSTRING = 140,
    OP_OBJ_CLEAR = 141,
    OP_ARRAY_NEW_MULTIDIM = 142,
    OP_WITH = 150,
    OP_WITHEND = 151,
    OP_FOREACH = 163,
    OP_THIS = 180,
    OP_THISO = 181,
    OP_PLAYER = 182,
    OP_PLAYERO = 183,
    OP_LEVEL = 184,
    OP_TEMP = 189,
    OP_PARAMS = 190,
}

impl Opcode {
    pub fn is_boolean_returning(self) -> bool {
        matches!(
            self,
            Self::OP_NOT
                | Self::OP_EQ
                | Self::OP_NEQ
                | Self::OP_LT
                | Self::OP_GT
                | Self::OP_LTE
                | Self::OP_GTE
                | Self::OP_IN_RANGE
                | Self::OP_IN_OBJ
        )
    }

    pub fn is_reserved_ident(self) -> bool {
        matches!(
            self,
            Self::OP_THIS
                | Self::OP_THISO
                | Self::OP_PLAYER
                | Self::OP_PLAYERO
                | Self::OP_LEVEL
                | Self::OP_TEMP
        )
    }

    pub fn is_object_returning(self) -> bool {
        matches!(
            self,
            Self::OP_THIS
                | Self::OP_THISO
                | Self::OP_PLAYER
                | Self::OP_PLAYERO
                | Self::OP_LEVEL
                | Self::OP_TEMP
        )
    }
}

// ==============================================================================
// Label and Jump Tracking
// ==============================================================================

type LabelId = u32;
type JumpAddress = u32;

// ==============================================================================
// Bytecode Emitter
// ==============================================================================

pub struct BytecodeEmitter {
    // Output buffers
    bytecode: Vec<u8>,

    // String table
    string_table: Vec<String>,
    string_table_map: HashMap<String, i32>,

    // Function table
    function_table: Vec<FunctionEntry>,
    function_set: HashSet<String>,

    // Constants table (const declarations)
    constants_table: HashMap<String, Expression>,

    // Label management
    label_counter: LabelId,
    label_locs: HashMap<LabelId, Vec<usize>>,
    label_addr: HashMap<LabelId, JumpAddress>,

    // Current state
    op_index: u32,
    last_op: Opcode,

    // Control flow labels (like in official compiler)
    success_label: LabelId,
    fail_label: LabelId,
    exit_label: LabelId,
    break_label: LabelId,
    continue_label: LabelId,

    // Expression state
    is_inline_conditional: bool,
    is_inside_expression: bool,
    is_copy_assignment: bool,
}

#[derive(Debug, Clone)]
struct FunctionEntry {
    function_name: String,
    op_index: u32,
    jmp_loc: usize,
}

impl BytecodeEmitter {
    pub fn new() -> Self {
        let exit_label = 1;
        let mut emitter = Self {
            bytecode: Vec::new(),
            string_table: Vec::new(),
            string_table_map: HashMap::new(),
            function_table: Vec::new(),
            function_set: HashSet::new(),
            constants_table: HashMap::new(),
            label_counter: 1,
            label_locs: HashMap::new(),
            label_addr: HashMap::new(),
            op_index: 0,
            last_op: Opcode::OP_NONE,
            success_label: exit_label,
            fail_label: exit_label,
            exit_label,
            break_label: 0,
            continue_label: 0,
            is_inline_conditional: true,
            is_inside_expression: false,
            is_copy_assignment: false,
        };
        // Initialize exit_label address
        emitter.label_addr.insert(exit_label, 0);
        emitter
    }

    // ========================================================================
    // Public API
    // ========================================================================

    pub fn emit_program(&mut self, program: &Program) -> Result<Vec<u8>> {
        // First pass: collect all const declarations
        self.collect_constants(program)?;

        // Second pass: emit all statements (consts are skipped, just stored in table)
        for statement in &program.statements {
            self.emit_statement(statement)?;
        }

        // Get the final bytecode
        Ok(self.finalize_bytecode())
    }

    // Collect all const declarations into the constants table
    fn collect_constants(&mut self, program: &Program) -> Result<()> {
        for statement in &program.statements {
            if let Statement::Const { name, value, .. } = statement {
                // Check for redefinition
                if self.constants_table.contains_key(&name.name) {
                    return Err(crate::error::CompileError::ConstRedefinition {
                        name: name.name.clone(),
                        location: statement.location().clone(),
                    });
                }
                // Resolve the value to get the actual literal (handling const references)
                let resolved_value = self.resolve_const_value(value)?;
                self.constants_table.insert(name.name.clone(), resolved_value);
            } else if let Statement::Enum { name, members, .. } = statement {
                // Process enum members as constants
                let prefix = name.as_ref().map(|n| n.name.clone());
                for member in members {
                    let const_name = if let Some(ref enum_name) = prefix {
                        format!("{}::{}", enum_name, member.name.name)
                    } else {
                        member.name.name.clone()
                    };

                    // Check for redefinition
                    if self.constants_table.contains_key(&const_name) {
                        return Err(crate::error::CompileError::ConstRedefinition {
                            name: const_name.clone(),
                            location: statement.location().clone(),
                        });
                    }

                    // Add the member value to the constants table
                    if let Some(ref value) = member.value {
                        let resolved_value = self.resolve_const_value(value)?;
                        self.constants_table.insert(const_name, resolved_value);
                    }
                }
            }
        }
        Ok(())
    }

    // Resolve a const value, following const references until we get a literal
    fn resolve_const_value(&self, expr: &Expression) -> Result<Expression> {
        match expr {
            Expression::Literal { .. } => Ok(expr.clone()),
            Expression::Identifier { identifier, .. } => {
                // Look up the const in the table
                if let Some(value) = self.constants_table.get(&identifier.name) {
                    // Recursively resolve (in case const refers to another const)
                    self.resolve_const_value(value)
                } else {
                    Err(crate::error::CompileError::UndefinedConst {
                        name: identifier.name.clone(),
                        location: expr.location().clone(),
                    })
                }
            }
            _ => Err(crate::error::CompileError::InvalidConstValue {
                location: expr.location().clone(),
            }),
        }
    }

    // ========================================================================
    // Label Management (from official compiler)
    // ========================================================================

    fn create_label(&mut self) -> LabelId {
        let id = self.label_counter;
        self.label_counter += 1;
        id
    }

    fn add_location(&mut self, label: LabelId, loc: usize) {
        self.label_locs.entry(label).or_default().push(loc);
    }

    fn set_location(&mut self, label: LabelId, addr: JumpAddress) {
        self.label_addr.insert(label, addr);
    }

    fn write_labels(&mut self) {
        for (&label, locs) in &self.label_locs {
            if label == self.exit_label {
                continue;
            }

            if let Some(&write_addr) = self.label_addr.get(&label) {
                for &loc in locs {
                    // Write the jump address as big-endian int16
                    let bytes = (write_addr as i16).to_be_bytes();
                    self.bytecode[loc] = bytes[0];
                    self.bytecode[loc + 1] = bytes[1];
                }
            }
        }
    }

    // ========================================================================
    // Bytecode Emission
    // ========================================================================

    fn emit(&mut self, op: Opcode) {
        self.bytecode.push(op as u8);
        self.last_op = op;
        self.op_index += 1;
    }

    fn emit_byte(&mut self, v: u8) {
        self.bytecode.push(v);
    }

    fn emit_short(&mut self, v: i16) {
        self.bytecode.extend_from_slice(&v.to_be_bytes());
    }

    fn emit_int(&mut self, v: i32) {
        self.bytecode.extend_from_slice(&v.to_be_bytes());
    }

    fn emit_string(&mut self, s: &str) {
        self.bytecode.extend_from_slice(s.as_bytes());
        self.bytecode.push(0);
    }

    // ========================================================================
    // Dynamic Number Encoding (from official compiler's emitDynamicNumber)
    // ========================================================================

    fn emit_dynamic_number(&mut self, val: i32) {
        // Strings use 0xF0 -> 0xF2, numbers use 0xF3 -> 0xF5
        let offset = match self.last_op {
            Opcode::OP_SET_INDEX | Opcode::OP_SET_INDEX_TRUE | Opcode::OP_TYPE_NUMBER => 3,
            Opcode::OP_TYPE_VAR | Opcode::OP_TYPE_STRING => 0,
            _ => {
                // This should not happen in well-formed code
                0
            }
        };

        if val >= i8::MIN as i32 && val <= i8::MAX as i32 {
            self.emit_byte(0xF0 + offset);
            self.emit_byte(val as u8);
        } else if val >= i16::MIN as i32 && val <= i16::MAX as i32 {
            self.emit_byte(0xF1 + offset);
            self.emit_short(val as i16);
        } else {
            self.emit_byte(0xF2 + offset);
            self.emit_int(val);
        }
    }

    fn emit_dynamic_number_unsigned(&mut self, val: u32) {
        let offset = match self.last_op {
            Opcode::OP_SET_INDEX | Opcode::OP_SET_INDEX_TRUE | Opcode::OP_TYPE_NUMBER => 3,
            Opcode::OP_TYPE_VAR | Opcode::OP_TYPE_STRING => 0,
            _ => 0,
        };

        if val <= u8::MAX as u32 {
            self.emit_byte(0xF0 + offset);
            self.emit_byte(val as u8);
        } else if val <= u16::MAX as u32 {
            self.emit_byte(0xF1 + offset);
            self.emit_short(val as i16);
        } else {
            self.emit_byte(0xF2 + offset);
            self.emit_int(val as i32);
        }
    }

    fn emit_double_number(&mut self, num: &str) {
        self.emit_byte(0xF6);
        self.emit_string(num);
    }

    // ========================================================================
    // String Table Management
    // ========================================================================

    fn get_string_const(&mut self, s: &str) -> i32 {
        if let Some(&idx) = self.string_table_map.get(s) {
            return idx;
        }
        let idx = self.string_table.len() as i32;
        self.string_table.push(s.to_string());
        self.string_table_map.insert(s.to_string(), idx);
        idx
    }

    // ========================================================================
    // Function Table Management
    // ========================================================================

    fn add_function(&mut self, function_name: String, op_idx: u32, jmp_loc: usize) {
        if self.function_set.insert(function_name.clone()) {
            self.function_table.push(FunctionEntry {
                function_name,
                op_index: op_idx,
                jmp_loc,
            });
        }
    }

    // ========================================================================
    // Statement Emission
    // ========================================================================

    fn emit_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Block { statements, .. } => {
                for s in statements {
                    self.emit_statement(s)?;
                }
            }
            Statement::Expression { expr, .. } => {
                self.emit_expression(expr)?;
                // If the last op wasn't a boolean-returning op and we're in a statement context,
                // we might need to pop the value
                if !self.last_op.is_boolean_returning() {
                    // Check if we need to pop (for unused return values)
                    self.maybe_pop_unused();
                }
            }
            Statement::FunctionDeclaration {
                name,
                params,
                body,
                is_public,
                ..
            } => {
                self.emit_function_declaration(name, params, body, *is_public)?;
            }
            Statement::If {
                condition,
                true_block,
                false_block,
                ..
            } => {
                self.emit_if(condition, true_block, false_block.as_deref())?;
            }
            Statement::While { condition, body, .. } => {
                self.emit_while(condition, body)?;
            }
            Statement::For {
                init,
                condition,
                increment,
                body,
                ..
            } => {
                self.emit_for(init, condition, increment, body)?;
            }
            Statement::ForEach { item, array, body, .. } => {
                self.emit_for_each(item, array, body)?;
            }
            Statement::Return { expr, .. } => {
                self.emit_return(expr.as_ref())?;
            }
            Statement::With { obj, body, .. } => {
                self.emit_with(obj, body)?;
            }
            Statement::Break { .. } => {
                self.emit_break()?;
            }
            Statement::Continue { .. } => {
                self.emit_continue()?;
            }
            Statement::Switch { .. } => {
                // TODO: Implement switch statements
            }
            Statement::Const { .. } => {
                // Const declarations are handled in collect_constants, no bytecode to emit
            }
            Statement::Enum { .. } => {
                // Enum declarations are handled in collect_constants, no bytecode to emit
            }
        }
        Ok(())
    }

    fn emit_function_declaration(
        &mut self,
        name: &Identifier,
        params: &[Identifier],
        body: &Statement,
        _is_public: bool,
    ) -> Result<()> {
        let mut func_name = String::new();
        // if is_public {
        //     func_name.push_str("public.");
        // }
        func_name.push_str(&name.name);

        // Emit jump to skip over function body
        self.emit(Opcode::OP_SET_INDEX);
        self.emit_byte(0xF4);
        let jmp_loc = self.bytecode.len();
        self.emit_short(0); // placeholder

        // Add to function table
        self.add_function(func_name, self.op_index, jmp_loc);

        // Emit parameters
        self.emit(Opcode::OP_TYPE_ARRAY);
        for param in params.iter().rev() {
            let id = self.get_string_const(&param.name);
            self.emit(Opcode::OP_TYPE_VAR);
            self.emit_dynamic_number_unsigned(id as u32);
        }
        self.emit(Opcode::OP_FUNC_PARAMS_END);

        // Emit function start marker
        self.emit(Opcode::OP_JMP);
        // TODO: Check if function has function calls for OP_CMD_CALL

        // Emit function body
        self.emit_statement(body)?;

        // Emit return if last op wasn't RET
        if self.last_op != Opcode::OP_RET {
            self.emit(Opcode::OP_TYPE_NUMBER);
            self.emit_dynamic_number(0);
            self.emit(Opcode::OP_RET);
        }

        // Fix up the jump at jmp_loc to jump to current op_index
        let bytes = (self.op_index as i16).to_be_bytes();
        self.bytecode[jmp_loc] = bytes[0];
        self.bytecode[jmp_loc + 1] = bytes[1];

        Ok(())
    }

    fn emit_if(
        &mut self,
        condition: &Expression,
        true_block: &Statement,
        false_block: Option<&Statement>,
    ) -> Result<()> {
        let save_success = self.success_label;
        let save_fail = self.fail_label;

        let new_success = self.create_label();
        let new_fail = self.create_label();

        self.success_label = new_success;
        self.fail_label = new_fail;

        // Emit condition
        let was_inline_cond = self.is_inline_conditional;
        self.is_inline_conditional = false;
        self.emit_expression(condition)?;
        self.is_inline_conditional = was_inline_cond;

        // Convert to number if needed
        if !self.last_op.is_boolean_returning() {
            self.maybe_convert_to_number();
        }

        // Set success label to current op index
        self.set_location(new_success, self.op_index);

        // Emit conditional jump
        self.emit(Opcode::OP_IF);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(new_fail, self.bytecode.len() - 2);

        // Emit true block
        self.emit_statement(true_block)?;

        // Calculate next op index
        let next_op = self.op_index + if false_block.is_some() { 1 } else { 0 };
        self.set_location(new_fail, next_op);

        self.success_label = save_success;
        self.fail_label = save_fail;

        // Emit else block if present
        if let Some(fb) = false_block {
            self.emit(Opcode::OP_SET_INDEX);
            self.emit_byte(0xF4);
            self.emit_short(0);
            let else_loc = self.bytecode.len() - 2;

            self.emit_statement(fb)?;

            // Fix up the else jump
            let bytes = (self.op_index as i16).to_be_bytes();
            self.bytecode[else_loc] = bytes[0];
            self.bytecode[else_loc + 1] = bytes[1];
        }

        Ok(())
    }

    fn emit_while(&mut self, condition: &Expression, body: &Statement) -> Result<()> {
        let save = (self.success_label, self.fail_label, self.continue_label, self.break_label);

        let new_break = self.create_label();
        let new_continue = self.create_label();

        self.break_label = new_break;
        self.continue_label = new_continue;

        // Set continue location to start of loop
        self.set_location(new_continue, self.op_index);

        // Emit condition
        let was_inline_cond = self.is_inline_conditional;
        self.is_inline_conditional = false;
        self.emit_expression(condition)?;
        self.is_inline_conditional = was_inline_cond;

        self.maybe_convert_to_number();

        self.emit(Opcode::OP_IF);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(new_break, self.bytecode.len() - 2);

        self.emit(Opcode::OP_CMD_CALL);

        self.emit_statement(body)?;

        // Jump back to condition
        self.emit(Opcode::OP_SET_INDEX);
        self.emit_dynamic_number(self.label_addr[&new_continue] as i32);

        self.set_location(new_break, self.op_index);

        self.success_label = save.0;
        self.fail_label = save.1;
        self.continue_label = save.2;
        self.break_label = save.3;

        Ok(())
    }

    fn emit_for(
        &mut self,
        init: &Option<Box<Statement>>,
        condition: &Option<Expression>,
        increment: &Option<Box<Statement>>,
        body: &Statement,
    ) -> Result<()> {
        // Emit init
        if let Some(i) = init {
            self.emit_statement(i)?;
        }

        let start_loop_op = self.op_index;

        // Emit condition
        if let Some(c) = condition {
            self.emit_expression(c)?;
            self.maybe_convert_to_number();
        } else {
            self.emit(Opcode::OP_TYPE_TRUE);
        }

        let save = (self.success_label, self.fail_label, self.continue_label, self.break_label);

        let new_break = self.create_label();
        let new_continue = self.create_label();

        self.break_label = new_break;
        self.continue_label = new_continue;

        self.emit(Opcode::OP_IF);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(new_break, self.bytecode.len() - 2);

        self.emit(Opcode::OP_CMD_CALL);

        self.emit_statement(body)?;

        self.set_location(new_continue, self.op_index);

        // Emit increment
        if let Some(inc) = increment {
            self.emit_statement(inc)?;
        }

        // Jump back to condition
        self.emit(Opcode::OP_SET_INDEX);
        self.emit_dynamic_number(start_loop_op as i32);

        self.set_location(new_break, self.op_index);

        self.success_label = save.0;
        self.fail_label = save.1;
        self.continue_label = save.2;
        self.break_label = save.3;

        Ok(())
    }

    fn emit_for_each(
        &mut self,
        item: &Identifier,
        array: &Expression,
        body: &Statement,
    ) -> Result<()> {
        // Push array name and expression
        let id = self.get_string_const(&item.name);
        self.emit(Opcode::OP_TYPE_VAR);
        self.emit_dynamic_number_unsigned(id as u32);

        self.emit_expression(array)?;
        self.emit(Opcode::OP_CONV_TO_OBJECT);

        // Push index (starts at 0)
        self.emit(Opcode::OP_TYPE_NUMBER);
        self.emit_dynamic_number(0);

        let save = (self.success_label, self.fail_label, self.continue_label, self.break_label);

        let new_break = self.create_label();
        let new_continue = self.create_label();

        self.break_label = new_break;
        self.continue_label = new_continue;

        let start_loop_op = self.op_index;

        self.emit(Opcode::OP_FOREACH);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(new_break, self.bytecode.len() - 2);

        self.emit(Opcode::OP_CMD_CALL);

        self.emit_statement(body)?;

        self.set_location(new_continue, self.op_index);

        self.emit(Opcode::OP_INC);

        // Jump back to start
        self.emit(Opcode::OP_SET_INDEX);
        self.emit_dynamic_number(start_loop_op as i32);

        self.set_location(new_break, self.op_index);

        self.success_label = save.0;
        self.fail_label = save.1;
        self.continue_label = save.2;
        self.break_label = save.3;

        // Pop the index variable
        self.emit(Opcode::OP_INDEX_DEC);

        Ok(())
    }

    fn emit_return(&mut self, expr: Option<&Expression>) -> Result<()> {
        if let Some(e) = expr {
            let save = (self.success_label, self.fail_label);

            let new_label = self.create_label();
            self.success_label = new_label;
            self.fail_label = new_label;

            self.emit_expression(e)?;

            self.set_location(new_label, self.op_index);

            self.success_label = save.0;
            self.fail_label = save.1;
        } else {
            self.emit(Opcode::OP_TYPE_NUMBER);
            self.emit_dynamic_number(0);
        }

        self.emit(Opcode::OP_RET);
        Ok(())
    }

    fn emit_with(&mut self, obj: &Expression, body: &Statement) -> Result<()> {
        self.emit_expression(obj)?;
        self.emit(Opcode::OP_CONV_TO_OBJECT);

        self.emit(Opcode::OP_WITH);
        self.emit_byte(0xF4);
        self.emit_short(0);
        let with_loc = self.bytecode.len() - 2;

        self.emit_statement(body)?;

        self.emit(Opcode::OP_WITHEND);

        let bytes = (self.op_index as i16).to_be_bytes();
        self.bytecode[with_loc] = bytes[0];
        self.bytecode[with_loc + 1] = bytes[1];

        Ok(())
    }

    fn emit_break(&mut self) -> Result<()> {
        if self.break_label == 0 {
            // TODO: Warning about break outside loop
            return Ok(());
        }
        self.emit(Opcode::OP_SET_INDEX);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(self.break_label, self.bytecode.len() - 2);
        Ok(())
    }

    fn emit_continue(&mut self) -> Result<()> {
        if self.continue_label == 0 {
            // TODO: Warning about continue outside loop
            return Ok(());
        }
        self.emit(Opcode::OP_SET_INDEX);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(self.continue_label, self.bytecode.len() - 2);
        Ok(())
    }

    // ========================================================================
    // Expression Emission
    // ========================================================================

    fn emit_expression(&mut self, expr: &Expression) -> Result<()> {
        match expr {
            Expression::Literal { literal, .. } => {
                self.emit_literal(literal)?;
            }
            Expression::Identifier { identifier, .. } => {
                self.emit_identifier(identifier)?;
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
            Expression::MemberAccess { object, property, .. } => {
                self.emit_member_access(object, property)?;
            }
            Expression::ArrayAccess { array, index, .. } => {
                self.emit_expression(array)?;
                self.emit_expression(index)?;
                self.maybe_convert_to_number();
                self.emit(Opcode::OP_ARRAY);
            }
            Expression::ArrayLiteral { elements, .. } => {
                self.emit_array_literal(elements)?;
            }
            Expression::ObjectLiteral { properties, .. } => {
                self.emit_object_literal(properties)?;
            }
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                self.emit_ternary(condition, true_expr, false_expr)?;
            }
            Expression::In {
                value,
                container,
                upper_bound,
                ..
            } => {
                self.emit_in_expr(value, container, upper_bound.as_deref())?;
            }
            Expression::Cast { expr, target_type, .. } => {
                self.emit_cast_expr(expr, *target_type)?;
            }
        }
        Ok(())
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<()> {
        match literal {
            Literal::Number { value, .. } => {
                // Try to parse as integer first
                if let Ok(n) = value.parse::<i32>() {
                    self.emit(Opcode::OP_TYPE_NUMBER);
                    self.emit_dynamic_number(n);
                } else {
                    // It's a float
                    self.emit(Opcode::OP_TYPE_NUMBER);
                    self.emit_double_number(value);
                }
            }
            Literal::String { value, .. } => {
                let id = self.get_string_const(value);
                self.emit(Opcode::OP_TYPE_STRING);
                self.emit_dynamic_number_unsigned(id as u32);
            }
            Literal::Boolean { value, .. } => {
                if *value {
                    self.emit(Opcode::OP_TYPE_TRUE);
                } else {
                    self.emit(Opcode::OP_TYPE_FALSE);
                }
            }
            Literal::Null { .. } => {
                self.emit(Opcode::OP_TYPE_NULL);
            }
        }
        Ok(())
    }

    fn emit_identifier(&mut self, ident: &Identifier) -> Result<()> {
        // Check if this is a const - if so, substitute the value
        if let Some(const_value) = self.constants_table.get(&ident.name).cloned() {
            // Emit the const value instead of the identifier
            return self.emit_expression(&const_value);
        }

        // Check for reserved identifiers
        let ident_lower = ident.name.to_lowercase();
        let opcode = match ident_lower.as_str() {
            "this" => Some(Opcode::OP_THIS),
            "thiso" => Some(Opcode::OP_THISO),
            "player" => Some(Opcode::OP_PLAYER),
            "playero" => Some(Opcode::OP_PLAYERO),
            "level" => Some(Opcode::OP_LEVEL),
            "temp" => Some(Opcode::OP_TEMP),
            "true" => Some(Opcode::OP_TYPE_TRUE),
            "false" => Some(Opcode::OP_TYPE_FALSE),
            "null" => Some(Opcode::OP_TYPE_NULL),
            "pi" => Some(Opcode::OP_PI),
            _ => None,
        };

        if let Some(op) = opcode {
            self.emit(op);
        } else {
            let id = self.get_string_const(&ident.name);
            self.emit(Opcode::OP_TYPE_VAR);
            self.emit_dynamic_number_unsigned(id as u32);
        }

        Ok(())
    }

    fn emit_binary_op(
        &mut self,
        left: &Expression,
        op: crate::ast::expression::BinaryOp,
        right: &Expression,
    ) -> Result<()> {
        // Handle logical AND and OR specially (from official compiler)
        if matches!(op, crate::ast::expression::BinaryOp::LogicalAnd | crate::ast::expression::BinaryOp::LogicalOr) {
            return self.emit_logical_op(left, op, right);
        }

        // Handle assignments
        if op.is_assignment() {
            return self.emit_assignment(left, op, right);
        }

        // Regular binary operations
        self.emit_expression(left)?;
        self.maybe_convert_to_number();
        self.emit_expression(right)?;
        self.maybe_convert_to_number();

        let opcode = match op {
            crate::ast::expression::BinaryOp::Add => Opcode::OP_ADD,
            crate::ast::expression::BinaryOp::Subtract => Opcode::OP_SUB,
            crate::ast::expression::BinaryOp::Multiply => Opcode::OP_MUL,
            crate::ast::expression::BinaryOp::Divide => Opcode::OP_DIV,
            crate::ast::expression::BinaryOp::Modulo => Opcode::OP_MOD,
            crate::ast::expression::BinaryOp::Power => Opcode::OP_POW,
            crate::ast::expression::BinaryOp::Equal => Opcode::OP_EQ,
            crate::ast::expression::BinaryOp::NotEqual => Opcode::OP_NEQ,
            crate::ast::expression::BinaryOp::LessThan => Opcode::OP_LT,
            crate::ast::expression::BinaryOp::GreaterThan => Opcode::OP_GT,
            crate::ast::expression::BinaryOp::LessThanOrEqual => Opcode::OP_LTE,
            crate::ast::expression::BinaryOp::GreaterThanOrEqual => Opcode::OP_GTE,
            crate::ast::expression::BinaryOp::BitwiseAnd => Opcode::OP_BWA,
            crate::ast::expression::BinaryOp::BitwiseOr => Opcode::OP_BWO,
            crate::ast::expression::BinaryOp::BitwiseXor => Opcode::OP_BWX,
            crate::ast::expression::BinaryOp::LeftShift => Opcode::OP_BW_LEFTSHIFT,
            crate::ast::expression::BinaryOp::RightShift => Opcode::OP_BW_RIGHTSHIFT,
            _ => {
                // Unknown operator
                return Ok(());
            }
        };

        self.emit(opcode);
        Ok(())
    }

    fn emit_logical_op(
        &mut self,
        left: &Expression,
        op: crate::ast::expression::BinaryOp,
        right: &Expression,
    ) -> Result<()> {
        let is_first_binary_expr = !self.is_inside_expression;

        if is_first_binary_expr {
            self.is_inside_expression = true;

            if self.is_inline_conditional {
                let label = self.create_label();
                self.success_label = label;
                self.fail_label = label;
            }
        }

        let tmp_success = self.success_label;
        let tmp_fail = self.fail_label;
        let was_inline_cond = self.is_inline_conditional;

        if matches!(op, crate::ast::expression::BinaryOp::LogicalAnd) {
            let new_success = self.create_label();
            self.success_label = new_success;

            self.emit_expression(left)?;
            self.maybe_convert_to_number();

            self.set_location(new_success, self.op_index);
            self.success_label = tmp_success;
            self.fail_label = tmp_fail;

            if was_inline_cond {
                self.emit(Opcode::OP_AND);
                self.emit_byte(0xF4);
                self.emit_short(0);
                self.add_location(self.fail_label, self.bytecode.len() - 2);
            } else {
                self.emit(Opcode::OP_IF);
                self.emit_byte(0xF4);
                self.emit_short(0);
                self.add_location(self.fail_label, self.bytecode.len() - 2);
            }

            self.emit_expression(right)?;
            self.maybe_convert_to_number();
        } else {
            // Logical OR
            let new_fail = self.create_label();
            self.fail_label = new_fail;

            self.emit_expression(left)?;
            self.maybe_convert_to_number();

            self.emit(Opcode::OP_OR);
            self.emit_byte(0xF4);
            self.emit_short(0);
            self.add_location(self.success_label, self.bytecode.len() - 2);

            self.set_location(new_fail, self.op_index);
            let _success_copy = self.success_label;
            self.success_label = tmp_success;
            self.fail_label = tmp_fail;

            self.emit_expression(right)?;
            self.maybe_convert_to_number();
        }

        if is_first_binary_expr {
            self.set_location(tmp_success, self.op_index);
            self.is_inside_expression = false;

            if was_inline_cond {
                self.emit(Opcode::OP_INLINE_CONDITIONAL);
            }
        }

        Ok(())
    }

    fn emit_assignment(
        &mut self,
        target: &Expression,
        op: crate::ast::expression::BinaryOp,
        value: &Expression,
    ) -> Result<()> {
        let is_compound = !matches!(op, crate::ast::expression::BinaryOp::Assign);

        if is_compound {
            // For compound assignment, read current value first
            self.emit_expression(target)?;
            self.emit(Opcode::OP_COPY_LAST_OP);
            self.maybe_convert_to_number();

            self.emit_expression(value)?;
            self.maybe_convert_to_number();

            // Emit the operation
            let opcode = match op {
                crate::ast::expression::BinaryOp::AddAssign => Opcode::OP_ADD,
                crate::ast::expression::BinaryOp::SubtractAssign => Opcode::OP_SUB,
                crate::ast::expression::BinaryOp::MultiplyAssign => Opcode::OP_MUL,
                crate::ast::expression::BinaryOp::DivideAssign => Opcode::OP_DIV,
                crate::ast::expression::BinaryOp::ModuloAssign => Opcode::OP_MOD,
                crate::ast::expression::BinaryOp::PowerAssign => Opcode::OP_POW,
                _ => Opcode::OP_ADD, // fallback
            };
            self.emit(opcode);

            // Emit target again
            self.emit_expression(target)?;
        } else {
            // Handle chained assignments
            if self.is_copy_assignment {
                self.emit(Opcode::OP_COPY_LAST_OP);
                self.is_copy_assignment = false;
            }

            if matches!(target, Expression::BinaryOp { .. }) {
                self.is_copy_assignment = true;
            }

            self.emit_expression(target)?;
            self.emit_expression(value)?;
        }

        self.emit(Opcode::OP_ASSIGN);
        Ok(())
    }

    fn emit_unary_op(
        &mut self,
        op: crate::ast::expression::UnaryOp,
        expr: &Expression,
    ) -> Result<()> {
        // Handle postfix increment/decrement specially
        match op {
            crate::ast::expression::UnaryOp::PostInc | crate::ast::expression::UnaryOp::PostDec => {
                // For postfix inc/dec, we need to:
                // 1. Emit the target expression (gets the current value on stack)
                // 2. Copy it (so we keep the original value for the result)
                // 3. Increment/decrement
                // 4. Assign back to the target
                self.emit_expression(expr)?;

                let opcode = match op {
                    crate::ast::expression::UnaryOp::PostInc => Opcode::OP_INC,
                    crate::ast::expression::UnaryOp::PostDec => Opcode::OP_DEC,
                    _ => unreachable!(),
                };

                self.emit(opcode);
                return Ok(());
            }
            _ => {}
        }

        let was_inline_cond = self.is_inline_conditional;

        let is_first_binary_expr = !self.is_inside_expression;
        if is_first_binary_expr {
            self.is_inside_expression = true;
            self.is_inline_conditional = true;

            let label = self.create_label();
            self.success_label = label;
            self.fail_label = label;
        }

        self.emit_expression(expr)?;

        if is_first_binary_expr {
            self.is_inside_expression = false;
            self.is_inline_conditional = was_inline_cond;

            // Set the label location
            if let (Some(_success), Some(_fail)) = (
                self.label_addr.get(&self.success_label),
                self.label_addr.get(&self.fail_label),
            ) {
                // Label was already set
            }
        }

        let opcode = match op {
            crate::ast::expression::UnaryOp::Negate => Opcode::OP_UNARYSUB,
            crate::ast::expression::UnaryOp::LogicalNot => Opcode::OP_NOT,
            crate::ast::expression::UnaryOp::BitwiseInvert => Opcode::OP_BWI,
            _ => return Ok(()), // Skip unknown ops
        };

        if !self.last_op.is_boolean_returning() && op == crate::ast::expression::UnaryOp::LogicalNot {
            // Need conversion
        }

        self.emit(opcode);
        Ok(())
    }

    fn emit_in_expr(
        &mut self,
        value: &Expression,
        container: &Expression,
        upper_bound: Option<&Expression>,
    ) -> Result<()> {
        // Emit value
        self.emit_expression(value)?;

        // Emit container (or lower bound for range)
        self.emit_expression(container)?;

        if let Some(upper) = upper_bound {
            // Range check: value in |lower, upper|
            // Convert lower bound to number if needed
            self.maybe_convert_to_number();

            // Emit upper bound
            self.emit_expression(upper)?;

            // Convert upper bound to number if needed
            self.maybe_convert_to_number();

            // Emit OP_IN_RANGE
            self.emit(Opcode::OP_IN_RANGE);
        } else {
            // Array/object membership check: value in array
            // Convert container to object if needed
            if !self.last_op.is_object_returning() {
                self.emit(Opcode::OP_CONV_TO_OBJECT);
            }

            // Emit OP_IN_OBJ
            self.emit(Opcode::OP_IN_OBJ);
        }

        Ok(())
    }

    fn emit_cast_expr(&mut self, expr: &Expression, target_type: crate::ast::expression::CastType) -> Result<()> {
        // Emit the expression first
        self.emit_expression(expr)?;

        // Emit the appropriate cast opcode
        match target_type {
            crate::ast::expression::CastType::Integer => {
                self.emit(Opcode::OP_INT);
            }
            crate::ast::expression::CastType::Float => {
                self.emit(Opcode::OP_CONV_TO_FLOAT);
            }
        }

        Ok(())
    }

    fn emit_function_call(&mut self, target: &Expression, args: &[Expression]) -> Result<()> {
        // Check if this is a built-in function call or method call
        if let Expression::Identifier { identifier, .. } = target {
            // Check for global built-in functions
            if let Some(builtin) = BuiltInFunction::get_global(&identifier.name) {
                return self.emit_builtin_function_call(builtin, args, None);
            }
        } else if let Expression::MemberAccess { object, property, .. } = target {
            // Check for object method built-ins
            if let Some(builtin) = BuiltInFunction::get_method(&property.name) {
                return self.emit_builtin_function_call(builtin, args, Some(object.as_ref()));
            }
        }

        // Regular function call - emit arguments first (left to right)
        for arg in args {
            self.emit_expression(arg)?;
        }

        // Emit target
        self.emit_expression(target)?;

        // If target is a member access, we need to emit MEMBER_ACCESS
        if matches!(target, Expression::MemberAccess { .. }) {
            self.emit(Opcode::OP_MEMBER_ACCESS);
        }

        self.emit(Opcode::OP_CALL);

        // Pop unused return value if needed
        self.emit(Opcode::OP_INDEX_DEC);

        Ok(())
    }

    fn emit_builtin_function_call(
        &mut self,
        builtin: &BuiltInFunction,
        args: &[Expression],
        object: Option<&Expression>,
    ) -> Result<()> {
        // Emit object first if object_first flag is set
        if builtin.flags.object_first {
            if let Some(obj) = object {
                self.emit_expression(obj)?;
                // Convert object if needed
                if let Some(conv_op) = builtin.convert_object_op {
                    self.emit(conv_op);
                }
            } else {
                // For global functions with object_first, use temp
                self.emit(Opcode::OP_TEMP);
            }
        }

        // Get parameter types from signature
        let param_types: Vec<char> = builtin.param_types().take(args.len()).collect();

        // Emit arguments with type conversions
        if builtin.flags.use_array {
            // Start array
            self.emit(Opcode::OP_TYPE_ARRAY);

            // Emit arguments (reversed if reverse_args is set)
            let mut indexed_args: Vec<(usize, &Expression)> = args.iter().enumerate().collect();
            if builtin.flags.reverse_args {
                indexed_args.reverse();
            }

            for (idx, arg) in indexed_args {
                self.emit_expression(arg)?;

                // Apply type conversion based on signature
                if idx < param_types.len() {
                    self.emit_type_conversion(param_types[idx]);
                }
            }

            // End array
            self.emit(Opcode::OP_ARRAY_END);
        } else {
            // Emit arguments directly
            let mut indexed_args: Vec<(usize, &Expression)> = args.iter().enumerate().collect();
            if builtin.flags.reverse_args {
                indexed_args.reverse();
            }

            for (idx, arg) in indexed_args {
                self.emit_expression(arg)?;

                // Apply type conversion based on signature
                if idx < param_types.len() {
                    self.emit_type_conversion(param_types[idx]);
                }
            }
        }

        // Emit object if not already emitted (for methods)
        if !builtin.flags.object_first {
            if let Some(obj) = object {
                self.emit_expression(obj)?;
                // Convert object if needed
                if let Some(conv_op) = builtin.convert_object_op {
                    self.emit(conv_op);
                }
            }
        }

        // Emit the built-in opcode
        self.emit(builtin.opcode);

        // Convert return value if needed
        let return_type = builtin.return_type();
        if return_type != '-' && return_type != 'x' {
            self.emit_type_conversion(return_type);
        }

        // Pop unused return value if function doesn't return
        if !builtin.flags.return_value && return_type == '-' {
            self.emit(Opcode::OP_INDEX_DEC);
        }

        Ok(())
    }

    /// Emit a type conversion opcode based on the signature character
    fn emit_type_conversion(&mut self, type_char: char) {
        match type_char {
            'f' => self.emit(Opcode::OP_CONV_TO_FLOAT),
            's' => self.emit(Opcode::OP_CONV_TO_STRING),
            'o' => self.emit(Opcode::OP_CONV_TO_OBJECT),
            'x' | '-' => {} // No conversion needed
            _ => {} // Unknown type, skip
        }
    }

    fn emit_member_access(&mut self, object: &Expression, property: &Identifier) -> Result<()> {
        self.emit_expression(object)?;

        // If the object isn't an object-returning op and isn't OP_TYPE_VAR, convert to object
        if !self.last_op.is_object_returning() && self.last_op != Opcode::OP_TYPE_VAR {
            self.emit(Opcode::OP_CONV_TO_OBJECT);
        }

        // Get the property string index
        let id = self.get_string_const(&property.name);

        self.emit(Opcode::OP_TYPE_VAR);
        self.emit_dynamic_number_unsigned(id as u32);

        self.emit(Opcode::OP_MEMBER_ACCESS);
        Ok(())
    }

    fn emit_array_literal(&mut self, elements: &[Expression]) -> Result<()> {
        self.emit(Opcode::OP_TYPE_ARRAY);

        for elem in elements.iter().rev() {
            self.emit_expression(elem)?;
        }

        self.emit(Opcode::OP_ARRAY_END);
        Ok(())
    }

    fn emit_object_literal(
        &mut self,
        properties: &[(crate::ast::Identifier, Expression)],
    ) -> Result<()> {
        for (ident, value) in properties {
            self.emit_string_literal(&ident.name)?;
            self.emit_expression(value)?;
            self.emit(Opcode::OP_ASSIGN);
        }
        Ok(())
    }

    fn emit_string_literal(&mut self, s: &str) -> Result<()> {
        let id = self.get_string_const(s);
        self.emit(Opcode::OP_TYPE_STRING);
        self.emit_dynamic_number_unsigned(id as u32);
        Ok(())
    }

    fn emit_ternary(
        &mut self,
        condition: &Expression,
        true_expr: &Expression,
        false_expr: &Expression,
    ) -> Result<()> {
        let save = (self.success_label, self.fail_label);

        let new_fail = self.create_label();
        let new_success = self.create_label();

        self.fail_label = new_fail;
        self.success_label = new_success;

        self.emit_expression(condition)?;
        self.maybe_convert_to_number();

        self.emit(Opcode::OP_IF);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(new_fail, self.bytecode.len() - 2);

        self.emit_expression(true_expr)?;

        self.set_location(new_fail, self.op_index + 1);

        self.emit(Opcode::OP_SET_INDEX);
        self.emit_byte(0xF4);
        self.emit_short(0);
        self.add_location(new_success, self.bytecode.len() - 2);

        self.emit_expression(false_expr)?;

        self.set_location(new_success, self.op_index);

        self.success_label = save.0;
        self.fail_label = save.1;

        Ok(())
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn maybe_convert_to_number(&mut self) {
        // Only convert if the last op wasn't already number-returning
        // This is a simplified version - the full version checks expression types
    }

    fn maybe_pop_unused(&mut self) {
        // Check if we should pop the value (for unused return values)
        // This is context-dependent
    }

    // ========================================================================
    // Final Bytecode Generation (from official compiler's getByteCode)
    // ========================================================================

    fn finalize_bytecode(&mut self) -> Vec<u8> {
        // Emit RET at the very end (as per official compiler)
        self.emit(Opcode::OP_RET);

        // Set exit label location
        self.set_location(self.exit_label, self.op_index);

        // Write all jump labels
        self.write_labels();

        // Fix up function jump locations
        for func in &self.function_table {
            if func.jmp_loc != 0 {
                let bytes = (self.op_index as i16).to_be_bytes();
                self.bytecode[func.jmp_loc] = bytes[0];
                self.bytecode[func.jmp_loc + 1] = bytes[1];
            }
        }

        let mut result = Vec::new();

        // ====================================================================
        // GS1 Flags Section
        // ====================================================================
        result.extend_from_slice(&(1u32).to_be_bytes()); // Section type: GS1Flags
        result.extend_from_slice(&(4u32).to_be_bytes()); // Section length
        result.extend_from_slice(&(0u32).to_be_bytes()); // Flags: 0

        // ====================================================================
        // Functions Section
        // ====================================================================
        let mut functions_buffer = Vec::new();

        // Collect functions in order (for now, just definition order)
        for func in &self.function_table {
            functions_buffer.extend_from_slice(&func.op_index.to_be_bytes());
            functions_buffer.extend_from_slice(func.function_name.as_bytes());
            functions_buffer.push(0);
        }

        result.extend_from_slice(&(2u32).to_be_bytes()); // Section type: Functions
        result.extend_from_slice(&(functions_buffer.len() as u32).to_be_bytes());
        result.extend_from_slice(&functions_buffer);

        // ====================================================================
        // Strings Section
        // ====================================================================
        let mut strings_buffer = Vec::new();
        for s in &self.string_table {
            strings_buffer.extend_from_slice(s.as_bytes());
            strings_buffer.push(0);
        }

        result.extend_from_slice(&(3u32).to_be_bytes()); // Section type: Strings
        result.extend_from_slice(&(strings_buffer.len() as u32).to_be_bytes());
        result.extend_from_slice(&strings_buffer);

        // ====================================================================
        // Bytecode Section
        // ====================================================================
        result.extend_from_slice(&(4u32).to_be_bytes()); // Section type: Bytecode
        result.extend_from_slice(&(self.bytecode.len() as u32).to_be_bytes());
        result.extend_from_slice(&self.bytecode);
        result.push(b'\n');

        result
    }

    pub fn into_bytes(self) -> Vec<u8> {
        // Note: This consumes the emitter and returns bytecode
        // For a proper implementation, we would need to handle the ownership differently
        // For now, this is a placeholder that returns an empty vec since the
        // actual bytecode is returned via emit_program
        Vec::new()
    }
}

impl Default for BytecodeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// Built-in Function Definitions
// ==============================================================================

/// Command flags for built-in functions
#[derive(Debug, Clone, Copy, Default)]
pub struct CmdFlags {
    /// Pass arguments in an array
    pub use_array: bool,
    /// Reverse argument order
    pub reverse_args: bool,
    /// Function returns a value
    pub return_value: bool,
    /// Object comes first (for obj.func() calls)
    pub object_first: bool,
}

impl CmdFlags {
    pub const DEFAULT: Self = Self {
        use_array: true,
        reverse_args: true,
        return_value: true,
        object_first: false,
    };

    pub const DEFAULT_OBJ: Self = Self {
        use_array: true,
        reverse_args: true,
        return_value: true,
        object_first: false,
    };

    pub const NONE: Self = Self {
        use_array: false,
        reverse_args: false,
        return_value: false,
        object_first: false,
    };
}

/// Built-in function definition
#[derive(Debug, Clone)]
pub struct BuiltInFunction {
    pub name: &'static str,
    pub opcode: Opcode,
    pub convert_object_op: Option<Opcode>,
    pub flags: CmdFlags,
    /// Type signature: first char = return type, rest = param types
    /// - '-' = discard (no return)
    /// - 'x' = no conversion
    /// - 'f' = convert to float
    /// - 'o' = convert to object
    /// - 's' = convert to string
    pub signature: &'static str,
}

impl BuiltInFunction {
    pub const fn new(name: &'static str, opcode: Opcode, flags: CmdFlags, signature: &'static str) -> Self {
        Self {
            name,
            opcode,
            convert_object_op: None,
            flags,
            signature,
        }
    }

    pub const fn new_obj(name: &'static str, opcode: Opcode, convert_op: Option<Opcode>, flags: CmdFlags, signature: &'static str) -> Self {
        Self {
            name,
            opcode,
            convert_object_op: convert_op,
            flags,
            signature,
        }
    }

    pub const GLOBAL_FUNCS: &'static [BuiltInFunction] = &[
        BuiltInFunction {
            name: "sleep",
            opcode: Opcode::OP_SLEEP,
            convert_object_op: None,
            flags: CmdFlags::NONE,
            signature: "-f",
        },
        BuiltInFunction::new("sin", Opcode::OP_SIN, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("cos", Opcode::OP_COS, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("arctan", Opcode::OP_ARCTAN, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("vecx", Opcode::OP_VECX, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("vecy", Opcode::OP_VECY, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("abs", Opcode::OP_ABS, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("exp", Opcode::OP_EXP, CmdFlags::DEFAULT, "ff"),
        BuiltInFunction::new("log", Opcode::OP_LOG, CmdFlags::DEFAULT, "fff"),
        BuiltInFunction::new("min", Opcode::OP_MIN, CmdFlags::DEFAULT, "fff"),
        BuiltInFunction::new("max", Opcode::OP_MAX, CmdFlags::DEFAULT, "fff"),
        BuiltInFunction::new("pow", Opcode::OP_POW, CmdFlags {
            return_value: true,
            use_array: true,
            reverse_args: true,
            object_first: false,
        }, "fff"),
        BuiltInFunction::new("random", Opcode::OP_RANDOM, CmdFlags::DEFAULT, "fff"),
        BuiltInFunction::new("arraylen", Opcode::OP_OBJ_SIZE, CmdFlags::DEFAULT, "fo"),
        BuiltInFunction::new("sarraylen", Opcode::OP_OBJ_SIZE, CmdFlags::DEFAULT, "fo"),
        BuiltInFunction::new("setarray", Opcode::OP_SETARRAY, CmdFlags {
            use_array: false,
            reverse_args: false,
            return_value: false,
            object_first: true,
        }, "-of"),
        BuiltInFunction::new("getangle", Opcode::OP_GETANGLE, CmdFlags {
            return_value: true,
            use_array: true,
            reverse_args: true,
            object_first: false,
        }, "fff"),
        BuiltInFunction::new("getdir", Opcode::OP_GETDIR, CmdFlags {
            return_value: true,
            use_array: true,
            reverse_args: true,
            object_first: false,
        }, "fff"),
        BuiltInFunction::new("format", Opcode::OP_FORMAT, CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: false,
        }, "xs"),
        BuiltInFunction::new("makevar", Opcode::OP_MAKEVAR, CmdFlags::DEFAULT, "s"),
        BuiltInFunction::new("waitfor", Opcode::OP_WAITFOR, CmdFlags {
            use_array: false,
            reverse_args: false,
            return_value: true,
            object_first: true,
        }, "xssf"),
    ];

    pub const OBJ_METHODS: &'static [BuiltInFunction] = &[
        BuiltInFunction::new_obj("index", Opcode::OP_OBJ_INDEX, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags {
            use_array: false,
            reverse_args: false,
            return_value: true,
            object_first: true,
        }, "fx"),
        BuiltInFunction::new_obj("type", Opcode::OP_OBJ_TYPE, None, CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("indices", Opcode::OP_OBJ_INDICES, None, CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("link", Opcode::OP_OBJ_LINK, None, CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("trim", Opcode::OP_OBJ_TRIM, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "s"),
        BuiltInFunction::new_obj("length", Opcode::OP_OBJ_LENGTH, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "f"),
        BuiltInFunction::new_obj("pos", Opcode::OP_OBJ_POS, Some(Opcode::OP_CONV_TO_STRING), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "fs"),
        BuiltInFunction::new_obj("charat", Opcode::OP_OBJ_CHARAT, Some(Opcode::OP_CONV_TO_STRING), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "sf"),
        BuiltInFunction::new_obj("substring", Opcode::OP_OBJ_SUBSTR, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "sff"),
        BuiltInFunction::new_obj("starts", Opcode::OP_OBJ_STARTS, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("ends", Opcode::OP_OBJ_ENDS, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("tokenize", Opcode::OP_OBJ_TOKENIZE, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("positions", Opcode::OP_OBJ_POSITIONS, Some(Opcode::OP_CONV_TO_STRING), CmdFlags::DEFAULT_OBJ, "os"),
        BuiltInFunction::new_obj("size", Opcode::OP_OBJ_SIZE, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("subarray", Opcode::OP_OBJ_SUBARRAY, None, CmdFlags::DEFAULT_OBJ, "-"),
        BuiltInFunction::new_obj("clear", Opcode::OP_OBJ_CLEAR, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags::NONE, "-"),
        BuiltInFunction::new_obj("add", Opcode::OP_OBJ_ADDSTRING, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "-"),
        BuiltInFunction::new_obj("delete", Opcode::OP_OBJ_DELETESTRING, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "-"),
        BuiltInFunction::new_obj("insert", Opcode::OP_OBJ_INSERTSTRING, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "-"),
        BuiltInFunction::new_obj("remove", Opcode::OP_OBJ_REMOVESTRING, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "-"),
        BuiltInFunction::new_obj("replace", Opcode::OP_OBJ_REPLACESTRING, Some(Opcode::OP_CONV_TO_OBJECT), CmdFlags {
            use_array: true,
            reverse_args: true,
            return_value: true,
            object_first: true,
        }, "-"),
    ];

    pub fn get_global(name: &str) -> Option<&'static BuiltInFunction> {
        Self::GLOBAL_FUNCS.iter().find(|f| f.name == name)
    }

    pub fn get_method(name: &str) -> Option<&'static BuiltInFunction> {
        Self::OBJ_METHODS.iter().find(|f| f.name == name)
    }

    pub fn is_builtin(name: &str) -> bool {
        Self::get_global(name).is_some() || Self::get_method(name).is_some()
    }

    pub fn return_type(&self) -> char {
        self.signature.chars().next().unwrap_or('-')
    }

    pub fn param_types(&self) -> impl Iterator<Item = char> + '_ {
        self.signature.chars().skip(1).chain(std::iter::repeat('x'))
    }
}
