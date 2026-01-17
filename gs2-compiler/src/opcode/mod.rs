//! GS2 Bytecode Opcodes
//!
//! Aligned with gs2-decompiler opcode definitions

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    // Control flow (0x00 - 0x0b)
    Jmp = 0x1,
    Jeq = 0x2,
    ShortCircuitOr = 0x3,
    Jne = 0x4,
    ShortCircuitAnd = 0x5,
    Call = 0x6,
    Ret = 0x7,
    Sleep = 0x8,
    IncreaseLoopCounter = 0x9,
    FunctionStart = 0xa,
    WaitFor = 0xb,

    // Type push literals (0x14 - 0x1b)
    PushNumber = 0x14,
    PushString = 0x15,
    PushVariable = 0x16,
    PushArray = 0x17,
    PushTrue = 0x18,
    PushFalse = 0x19,
    PushNull = 0x1a,
    Pi = 0x1b,

    // Stack operations (0x1e - 0x2f)
    Copy = 0x1e,
    Swap = 0x1f,
    Pop = 0x20,
    ConvertToFloat = 0x21,
    ConvertToString = 0x22,
    AccessMember = 0x23,
    ConvertToObject = 0x24,
    EndArray = 0x25,
    NewUninitializedArray = 0x26,
    SetArray = 0x27,
    New = 0x28,
    MakeVar = 0x29,
    NewObject = 0x2a,
    ConvertToVariable = 0x2b,
    ShortCircuitEnd = 0x2c,
    SetRegister = 0x2d,
    GetRegister = 0x2e,
    MarkRegisterVariable = 0x2f,

    // Assignment and operations (0x32 - 0x35)
    Assign = 0x32,
    EndParams = 0x33,
    Inc = 0x34,
    Dec = 0x35,

    // Arithmetic (0x3c - 0x45)
    Add = 0x3c,
    Subtract = 0x3d,
    Multiply = 0x3e,
    Divide = 0x3f,
    Modulo = 0x40,
    Power = 0x41,
    LogicalNot = 0x44,
    UnarySubtract = 0x45,

    // Comparison (0x46 - 0x4b)
    Equal = 0x46,
    NotEqual = 0x47,
    LessThan = 0x48,
    GreaterThan = 0x49,
    LessThanOrEqual = 0x4a,
    GreaterThanOrEqual = 0x4b,

    // Bitwise (0x4c - 0x4f)
    BitwiseOr = 0x4c,
    BitwiseAnd = 0x4d,
    BitwiseXor = 0x4e,
    BitwiseInvert = 0x4f,

    // Range and object (0x50 - 0x53)
    InRange = 0x50,
    In = 0x51,
    ObjIndex = 0x52,
    ObjType = 0x53,

    // Math functions (0x54 - 0x64)
    Format = 0x54,
    Int = 0x55,
    Abs = 0x56,
    Random = 0x57,
    Sin = 0x58,
    Cos = 0x59,
    ArcTan = 0x5a,
    Exp = 0x5b,
    Log = 0x5c,
    Min = 0x5d,
    Max = 0x5e,
    GetAngle = 0x5f,
    GetDir = 0x60,
    VecX = 0x61,
    VecY = 0x62,
    ObjIndices = 0x63,
    ObjLink = 0x64,

    // Shift and char (0x65 - 0x67)
    ShiftLeft = 0x65,
    ShiftRight = 0x66,
    Char = 0x67,

    // String operations (0x6e - 0x78)
    ObjTrim = 0x6e,
    ObjLength = 0x6f,
    ObjPos = 0x70,
    Join = 0x71,
    ObjCharAt = 0x72,
    ObjSubstring = 0x73,
    ObjStarts = 0x74,
    ObjEnds = 0x75,
    ObjTokenize = 0x76,
    GetTranslation = 0x77,
    ObjPositions = 0x78,

    // Array operations (0x82 - 0x8e)
    ObjSize = 0x82,
    Array = 0x83,
    ArrayAssign = 0x84,
    ArrayMultidim = 0x85,
    ArrayMultidimAssign = 0x86,
    ObjSubarray = 0x87,
    ObjAddString = 0x88,
    ObjDeleteString = 0x89,
    ObjRemoveString = 0x8a,
    ObjReplaceString = 0x8b,
    ObjInsertString = 0x8c,
    ObjClear = 0x8d,
    ArrayNewMultidim = 0x8e,

    // Control structures (0x96 - 0xa3)
    With = 0x96,
    WithEnd = 0x97,
    ForEach = 0xa3,

    // Reserved identifiers (0xb4 - 0xbe)
    This = 0xb4,
    ThisO = 0xb5,
    Player = 0xb6,
    PlayerO = 0xb7,
    Level = 0xb8,
    Temp = 0xbd,
    Params = 0xbe,

    // Immediate operand encoders (0xf0 - 0xf6)
    // These are NOT standalone opcodes - they encode operands for the previous instruction
    ImmStringByte = 0xf0,
    ImmStringShort = 0xf1,
    ImmStringInt = 0xf2,
    ImmByte = 0xf3,
    ImmShort = 0xf4,
    ImmInt = 0xf5,
    ImmFloat = 0xf6,
}

impl Opcode {
    /// Check if this opcode returns a boolean value
    pub fn is_boolean_returning(self) -> bool {
        matches!(
            self,
            Self::LogicalNot
                | Self::Equal
                | Self::NotEqual
                | Self::LessThan
                | Self::GreaterThan
                | Self::LessThanOrEqual
                | Self::GreaterThanOrEqual
                | Self::InRange
                | Self::In
        )
    }

    /// Check if this is a reserved identifier opcode
    pub fn is_reserved_ident(self) -> bool {
        matches!(
            self,
            Self::This
                | Self::ThisO
                | Self::Player
                | Self::PlayerO
                | Self::Level
                | Self::Temp
        )
    }

    /// Check if this opcode returns an object
    pub fn is_object_returning(self) -> bool {
        matches!(
            self,
            Self::This
                | Self::ThisO
                | Self::Player
                | Self::PlayerO
                | Self::Level
                | Self::Temp
        )
    }

    /// Get the name of this opcode for debugging
    pub fn name(self) -> &'static str {
        match self {
            Self::Jmp => "JMP",
            Self::Jeq => "JEQ",
            Self::ShortCircuitOr => "SHORT_CIRCUIT_OR",
            Self::Jne => "JNE",
            Self::ShortCircuitAnd => "SHORT_CIRCUIT_AND",
            Self::Call => "CALL",
            Self::Ret => "RET",
            Self::Sleep => "SLEEP",
            Self::IncreaseLoopCounter => "INC_LOOP_CNT",
            Self::FunctionStart => "FUNCTION_START",
            Self::WaitFor => "WAIT_FOR",
            Self::PushNumber => "PUSH_NUMBER",
            Self::PushString => "PUSH_STRING",
            Self::PushVariable => "PUSH_VARIABLE",
            Self::PushArray => "PUSH_ARRAY",
            Self::PushTrue => "PUSH_TRUE",
            Self::PushFalse => "PUSH_FALSE",
            Self::PushNull => "PUSH_NULL",
            Self::Pi => "PI",
            Self::Copy => "COPY",
            Self::Swap => "SWAP",
            Self::Pop => "POP",
            Self::ConvertToFloat => "CONV_TO_FLOAT",
            Self::ConvertToString => "CONV_TO_STRING",
            Self::AccessMember => "ACCESS_MEMBER",
            Self::ConvertToObject => "CONV_TO_OBJECT",
            Self::EndArray => "END_ARRAY",
            Self::NewUninitializedArray => "NEW_ARRAY",
            Self::SetArray => "SET_ARRAY",
            Self::New => "NEW",
            Self::MakeVar => "MAKE_VAR",
            Self::NewObject => "NEW_OBJECT",
            Self::ConvertToVariable => "CONV_TO_VAR",
            Self::ShortCircuitEnd => "SHORT_CIRCUIT_END",
            Self::SetRegister => "SET_REG",
            Self::GetRegister => "GET_REG",
            Self::MarkRegisterVariable => "MARK_REG_VAR",
            Self::Assign => "ASSIGN",
            Self::EndParams => "END_PARAMS",
            Self::Inc => "INC",
            Self::Dec => "DEC",
            Self::Add => "ADD",
            Self::Subtract => "SUB",
            Self::Multiply => "MUL",
            Self::Divide => "DIV",
            Self::Modulo => "MOD",
            Self::Power => "POW",
            Self::LogicalNot => "NOT",
            Self::UnarySubtract => "UNARY_SUB",
            Self::Equal => "EQ",
            Self::NotEqual => "NEQ",
            Self::LessThan => "LT",
            Self::GreaterThan => "GT",
            Self::LessThanOrEqual => "LTE",
            Self::GreaterThanOrEqual => "GTE",
            Self::BitwiseOr => "BW_OR",
            Self::BitwiseAnd => "BW_AND",
            Self::BitwiseXor => "BW_XOR",
            Self::BitwiseInvert => "BW_INVERT",
            Self::InRange => "IN_RANGE",
            Self::In => "IN",
            Self::ObjIndex => "OBJ_INDEX",
            Self::ObjType => "OBJ_TYPE",
            Self::Format => "FORMAT",
            Self::Int => "INT",
            Self::Abs => "ABS",
            Self::Random => "RANDOM",
            Self::Sin => "SIN",
            Self::Cos => "COS",
            Self::ArcTan => "ARCTAN",
            Self::Exp => "EXP",
            Self::Log => "LOG",
            Self::Min => "MIN",
            Self::Max => "MAX",
            Self::GetAngle => "GET_ANGLE",
            Self::GetDir => "GET_DIR",
            Self::VecX => "VEC_X",
            Self::VecY => "VEC_Y",
            Self::ObjIndices => "OBJ_INDICES",
            Self::ObjLink => "OBJ_LINK",
            Self::ShiftLeft => "SHIFT_LEFT",
            Self::ShiftRight => "SHIFT_RIGHT",
            Self::Char => "CHAR",
            Self::ObjTrim => "OBJ_TRIM",
            Self::ObjLength => "OBJ_LENGTH",
            Self::ObjPos => "OBJ_POS",
            Self::Join => "JOIN",
            Self::ObjCharAt => "OBJ_CHAR_AT",
            Self::ObjSubstring => "OBJ_SUBSTRING",
            Self::ObjStarts => "OBJ_STARTS",
            Self::ObjEnds => "OBJ_ENDS",
            Self::ObjTokenize => "OBJ_TOKENIZE",
            Self::GetTranslation => "GET_TRANSLATION",
            Self::ObjPositions => "OBJ_POSITIONS",
            Self::ObjSize => "OBJ_SIZE",
            Self::Array => "ARRAY",
            Self::ArrayAssign => "ARRAY_ASSIGN",
            Self::ArrayMultidim => "ARRAY_MULTIDIM",
            Self::ArrayMultidimAssign => "ARRAY_MULTIDIM_ASSIGN",
            Self::ObjSubarray => "OBJ_SUBARRAY",
            Self::ObjAddString => "OBJ_ADD_STRING",
            Self::ObjDeleteString => "OBJ_DELETE_STRING",
            Self::ObjRemoveString => "OBJ_REMOVE_STRING",
            Self::ObjReplaceString => "OBJ_REPLACE_STRING",
            Self::ObjInsertString => "OBJ_INSERT_STRING",
            Self::ObjClear => "OBJ_CLEAR",
            Self::ArrayNewMultidim => "ARRAY_NEW_MULTIDIM",
            Self::With => "WITH",
            Self::WithEnd => "WITH_END",
            Self::ForEach => "FOR_EACH",
            Self::This => "THIS",
            Self::ThisO => "THIS_O",
            Self::Player => "PLAYER",
            Self::PlayerO => "PLAYER_O",
            Self::Level => "LEVEL",
            Self::Temp => "TEMP",
            Self::Params => "PARAMS",
            Self::ImmStringByte => "IMM_STRING_BYTE",
            Self::ImmStringShort => "IMM_STRING_SHORT",
            Self::ImmStringInt => "IMM_STRING_INT",
            Self::ImmByte => "IMM_BYTE",
            Self::ImmShort => "IMM_SHORT",
            Self::ImmInt => "IMM_INT",
            Self::ImmFloat => "IMM_FLOAT",
        }
    }

    /// Check if this is an immediate operand encoder (not a real opcode)
    pub fn is_immediate_encoder(self) -> bool {
        matches!(
            self,
            Self::ImmStringByte
                | Self::ImmStringShort
                | Self::ImmStringInt
                | Self::ImmByte
                | Self::ImmShort
                | Self::ImmInt
                | Self::ImmFloat
        )
    }

    /// Convert from u8 to Opcode, returning None for invalid values
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x1 => Some(Self::Jmp),
            0x2 => Some(Self::Jeq),
            0x3 => Some(Self::ShortCircuitOr),
            0x4 => Some(Self::Jne),
            0x5 => Some(Self::ShortCircuitAnd),
            0x6 => Some(Self::Call),
            0x7 => Some(Self::Ret),
            0x8 => Some(Self::Sleep),
            0x9 => Some(Self::IncreaseLoopCounter),
            0xa => Some(Self::FunctionStart),
            0xb => Some(Self::WaitFor),
            0x14 => Some(Self::PushNumber),
            0x15 => Some(Self::PushString),
            0x16 => Some(Self::PushVariable),
            0x17 => Some(Self::PushArray),
            0x18 => Some(Self::PushTrue),
            0x19 => Some(Self::PushFalse),
            0x1a => Some(Self::PushNull),
            0x1b => Some(Self::Pi),
            0x1e => Some(Self::Copy),
            0x1f => Some(Self::Swap),
            0x20 => Some(Self::Pop),
            0x21 => Some(Self::ConvertToFloat),
            0x22 => Some(Self::ConvertToString),
            0x23 => Some(Self::AccessMember),
            0x24 => Some(Self::ConvertToObject),
            0x25 => Some(Self::EndArray),
            0x26 => Some(Self::NewUninitializedArray),
            0x27 => Some(Self::SetArray),
            0x28 => Some(Self::New),
            0x29 => Some(Self::MakeVar),
            0x2a => Some(Self::NewObject),
            0x2b => Some(Self::ConvertToVariable),
            0x2c => Some(Self::ShortCircuitEnd),
            0x2d => Some(Self::SetRegister),
            0x2e => Some(Self::GetRegister),
            0x2f => Some(Self::MarkRegisterVariable),
            0x32 => Some(Self::Assign),
            0x33 => Some(Self::EndParams),
            0x34 => Some(Self::Inc),
            0x35 => Some(Self::Dec),
            0x3c => Some(Self::Add),
            0x3d => Some(Self::Subtract),
            0x3e => Some(Self::Multiply),
            0x3f => Some(Self::Divide),
            0x40 => Some(Self::Modulo),
            0x41 => Some(Self::Power),
            0x44 => Some(Self::LogicalNot),
            0x45 => Some(Self::UnarySubtract),
            0x46 => Some(Self::Equal),
            0x47 => Some(Self::NotEqual),
            0x48 => Some(Self::LessThan),
            0x49 => Some(Self::GreaterThan),
            0x4a => Some(Self::LessThanOrEqual),
            0x4b => Some(Self::GreaterThanOrEqual),
            0x4c => Some(Self::BitwiseOr),
            0x4d => Some(Self::BitwiseAnd),
            0x4e => Some(Self::BitwiseXor),
            0x4f => Some(Self::BitwiseInvert),
            0x50 => Some(Self::InRange),
            0x51 => Some(Self::In),
            0x52 => Some(Self::ObjIndex),
            0x53 => Some(Self::ObjType),
            0x54 => Some(Self::Format),
            0x55 => Some(Self::Int),
            0x56 => Some(Self::Abs),
            0x57 => Some(Self::Random),
            0x58 => Some(Self::Sin),
            0x59 => Some(Self::Cos),
            0x5a => Some(Self::ArcTan),
            0x5b => Some(Self::Exp),
            0x5c => Some(Self::Log),
            0x5d => Some(Self::Min),
            0x5e => Some(Self::Max),
            0x5f => Some(Self::GetAngle),
            0x60 => Some(Self::GetDir),
            0x61 => Some(Self::VecX),
            0x62 => Some(Self::VecY),
            0x63 => Some(Self::ObjIndices),
            0x64 => Some(Self::ObjLink),
            0x65 => Some(Self::ShiftLeft),
            0x66 => Some(Self::ShiftRight),
            0x67 => Some(Self::Char),
            0x6e => Some(Self::ObjTrim),
            0x6f => Some(Self::ObjLength),
            0x70 => Some(Self::ObjPos),
            0x71 => Some(Self::Join),
            0x72 => Some(Self::ObjCharAt),
            0x73 => Some(Self::ObjSubstring),
            0x74 => Some(Self::ObjStarts),
            0x75 => Some(Self::ObjEnds),
            0x76 => Some(Self::ObjTokenize),
            0x77 => Some(Self::GetTranslation),
            0x78 => Some(Self::ObjPositions),
            0x82 => Some(Self::ObjSize),
            0x83 => Some(Self::Array),
            0x84 => Some(Self::ArrayAssign),
            0x85 => Some(Self::ArrayMultidim),
            0x86 => Some(Self::ArrayMultidimAssign),
            0x87 => Some(Self::ObjSubarray),
            0x88 => Some(Self::ObjAddString),
            0x89 => Some(Self::ObjDeleteString),
            0x8a => Some(Self::ObjRemoveString),
            0x8b => Some(Self::ObjReplaceString),
            0x8c => Some(Self::ObjInsertString),
            0x8d => Some(Self::ObjClear),
            0x8e => Some(Self::ArrayNewMultidim),
            0x96 => Some(Self::With),
            0x97 => Some(Self::WithEnd),
            0xa3 => Some(Self::ForEach),
            0xb4 => Some(Self::This),
            0xb5 => Some(Self::ThisO),
            0xb6 => Some(Self::Player),
            0xb7 => Some(Self::PlayerO),
            0xb8 => Some(Self::Level),
            0xbd => Some(Self::Temp),
            0xbe => Some(Self::Params),
            0xf0 => Some(Self::ImmStringByte),
            0xf1 => Some(Self::ImmStringShort),
            0xf2 => Some(Self::ImmStringInt),
            0xf3 => Some(Self::ImmByte),
            0xf4 => Some(Self::ImmShort),
            0xf5 => Some(Self::ImmInt),
            0xf6 => Some(Self::ImmFloat),
            _ => None,
        }
    }

    /// Get the immediate encoder opcode for a string index
    pub fn string_index_to_immediate(index: u32) -> Self {
        if index <= 0xFF {
            Self::ImmStringByte
        } else if index <= 0xFFFF {
            Self::ImmStringShort
        } else {
            Self::ImmStringInt
        }
    }

    /// Get the immediate encoder opcode for a number value
    pub fn number_to_immediate(value: i32) -> Self {
        if value >= i8::MIN as i32 && value <= i8::MAX as i32 {
            Self::ImmByte
        } else if value >= i16::MIN as i32 && value <= i16::MAX as i32 {
            Self::ImmShort
        } else {
            Self::ImmInt
        }
    }
}
