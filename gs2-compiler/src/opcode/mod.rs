//! GS2 Bytecode Opcodes
//!
//! Ported from gs2-parser/src/opcodes.h

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    // Control flow
    None = 0,
    SetIndex = 1,
    SetIndexTrue = 2,
    Or = 3,
    If = 4,
    And = 5,
    Call = 6,
    Ret = 7,
    Sleep = 8,
    CmdCall = 9,
    Jmp = 10,
    Waitfor = 11,

    // Type literals
    TypeNumber = 20,
    TypeString = 21,
    TypeVar = 22,
    TypeArray = 23,
    TypeTrue = 24,
    TypeFalse = 25,
    TypeNull = 26,
    Pi = 27,

    // Stack and object operations
    CopyLastOp = 30,
    SwapLastOps = 31,
    IndexDec = 32,
    ConvToFloat = 33,
    ConvToString = 34,
    MemberAccess = 35,
    ConvToObject = 36,
    ArrayEnd = 37,
    ArrayNew = 38,
    Setarray = 39,
    InlineNew = 40,
    Makevar = 41,
    NewObject = 42,
    ObjFromStr = 43,
    InlineConditional = 44,
    Unknown45 = 45,
    Unknown46 = 46,
    Unknown47 = 47,

    // Assignment and operations
    Assign = 50,
    FuncParamsEnd = 51,
    Inc = 52,
    Dec = 53,
    Unknown54 = 54,

    // Arithmetic
    Add = 60,
    Sub = 61,
    Mul = 62,
    Div = 63,
    Mod = 64,
    Pow = 65,
    Unknown66 = 66,
    Unknown67 = 67,
    Not = 68,
    UnarySub = 69,

    // Comparison
    Eq = 70,
    Neq = 71,
    Lt = 72,
    Gt = 73,
    Lte = 74,
    Gte = 75,

    // Bitwise
    Bwo = 76,
    Bwa = 77,
    Bwx = 78,
    Bwi = 79,

    // Range and object
    InRange = 80,
    InObj = 81,
    ObjIndex = 82,
    ObjType = 83,

    // Math functions
    Format = 84,
    Int = 85,
    Abs = 86,
    Random = 87,
    Sin = 88,
    Cos = 89,
    Arctan = 90,
    Exp = 91,
    Log = 92,
    Min = 93,
    Max = 94,
    GetAngle = 95,
    GetDir = 96,
    VecX = 97,
    VecY = 98,
    ObjIndices = 99,
    ObjLink = 100,

    // More bitwise
    BwLeftshift = 101,
    BwRightshift = 102,

    // More functions
    Char = 103,
    ObjCompare = 104,

    // String operations
    ObjTrim = 110,
    ObjLength = 111,
    ObjPos = 112,
    Join = 113,
    ObjCharAt = 114,
    ObjSubstr = 115,
    ObjStarts = 116,
    ObjEnds = 117,
    ObjTokenize = 118,
    Translate = 119,
    ObjPositions = 120,

    // Array operations
    ObjSize = 130,
    Array = 131,
    ArrayAssign = 132,
    ArrayMultidim = 133,
    ArrayMultidimAssign = 134,
    ObjSubarray = 135,
    ObjAddstring = 136,
    ObjDeletestring = 137,
    ObjRemovestring = 138,
    ObjReplacestring = 139,
    ObjInsertstring = 140,
    ObjClear = 141,
    ArrayNewMultidim = 142,

    // Control structures
    With = 150,
    WithEnd = 151,
    ForEach = 163,

    // Reserved identifiers
    This = 180,
    Thiso = 181,
    Player = 182,
    Playero = 183,
    Level = 184,
    Temp = 189,
    Params = 190,
}

impl Opcode {
    /// Check if this opcode returns a boolean value
    pub fn is_boolean_returning(self) -> bool {
        matches!(
            self,
            Self::Not
                | Self::Eq
                | Self::Neq
                | Self::Lt
                | Self::Gt
                | Self::Lte
                | Self::Gte
                | Self::InRange
                | Self::InObj
        )
    }

    /// Check if this is a reserved identifier opcode
    pub fn is_reserved_ident(self) -> bool {
        matches!(
            self,
            Self::This
                | Self::Thiso
                | Self::Player
                | Self::Playero
                | Self::Level
                | Self::Temp
        )
    }

    /// Check if this opcode returns an object
    pub fn is_object_returning(self) -> bool {
        matches!(
            self,
            Self::This
                | Self::Thiso
                | Self::Player
                | Self::Playero
                | Self::Level
                | Self::Temp
        )
    }

    /// Get the name of this opcode for debugging
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "OP_NONE",
            Self::Assign => "OP_ASSIGN",
            Self::SetIndex => "OP_SET_INDEX",
            Self::SetIndexTrue => "OP_SET_INDEX_TRUE",
            Self::If => "OP_IF",
            Self::TypeTrue => "OP_TRUE",
            Self::TypeFalse => "OP_FALSE",
            Self::TypeNull => "OP_NULL",
            Self::Add => "OP_ADD",
            Self::Sub => "OP_SUB",
            Self::Mul => "OP_MUL",
            Self::Div => "OP_DIV",
            Self::Mod => "OP_MOD",
            Self::Pow => "OP_POW",
            Self::Inc => "OP_INC",
            Self::Dec => "OP_DEC",
            Self::UnarySub => "OP_UNARYSUB",
            Self::TypeNumber => "OP_TYPE_NUMBER",
            Self::Format => "OP_FORMAT",
            Self::TypeString => "OP_TYPE_STRING",
            Self::TypeVar => "OP_TYPE_VAR",
            Self::TypeArray => "OP_TYPE_ARRAY",
            Self::ArrayEnd => "OP_ARRAY_END",
            Self::ConvToFloat => "OP_CONV_TO_FLOAT",
            Self::ConvToString => "OP_CONV_TO_STRING",
            Self::MemberAccess => "OP_MEMBER_ACCESS",
            Self::ConvToObject => "OP_CONV_TO_OBJECT",
            Self::NewObject => "OP_NEW_OBJECT",
            Self::FuncParamsEnd => "OP_FUNC_PARAMS_END",
            Self::Call => "OP_CALL",
            Self::CmdCall => "OP_CMD_CALL",
            Self::Jmp => "OP_JMP",
            Self::IndexDec => "OP_INDEX_DEC",
            Self::Ret => "OP_RET",
            Self::Eq => "OP_EQ",
            Self::Neq => "OP_NEQ",
            Self::Lt => "OP_LT",
            Self::Gt => "OP_GT",
            Self::Lte => "OP_LTE",
            Self::Gte => "OP_GTE",
            Self::Not => "OP_NOT",
            Self::And => "OP_AND",
            Self::Or => "OP_OR",
            Self::Array => "OP_ARRAY[]",
            Self::ObjCharAt => "OP_OBJ_CHARAT",
            Self::ObjClear => "OP_OBJ_CLEAR",
            Self::ObjEnds => "OP_OBJ_ENDS",
            Self::InRange => "OP_IN_RANGE",
            Self::InObj => "OP_IN_OBJ",
            Self::ObjIndex => "OP_OBJ_INDEX",
            Self::ObjIndices => "OP_OBJ_INDICES",
            Self::ObjLength => "OP_OBJ_LENGTH",
            Self::ObjLink => "OP_OBJ_LINK",
            Self::ObjPos => "OP_OBJ_POS",
            Self::ObjPositions => "OP_OBJ_POSITIONS",
            Self::ObjSize => "OP_OBJ_SIZE",
            Self::ObjStarts => "OP_OBJ_STARTS",
            Self::ObjSubarray => "OP_OBJ_SUBARRAY",
            Self::ObjSubstr => "OP_OBJ_SUBSTR",
            Self::ObjTokenize => "OP_OBJ_TOKENIZE",
            Self::ObjTrim => "OP_OBJ_TRIM",
            Self::ObjType => "OP_OBJ_TYPE",
            Self::Join => "OP_JOIN",
            Self::This => "OP_THIS",
            Self::Thiso => "OP_THISO",
            Self::Player => "OP_PLAYER",
            Self::Playero => "OP_PLAYERO",
            Self::Level => "OP_LEVEL",
            Self::Temp => "OP_TEMP",
            _ => "OP_UNKNOWN",
        }
    }

    /// Convert from u8 to Opcode, returning None for invalid values
    pub fn from_u8(value: u8) -> Option<Self> {
        // Only implement valid opcode ranges
        match value {
            0 => Some(Self::None),
            1 => Some(Self::SetIndex),
            2 => Some(Self::SetIndexTrue),
            3 => Some(Self::Or),
            4 => Some(Self::If),
            5 => Some(Self::And),
            6 => Some(Self::Call),
            7 => Some(Self::Ret),
            8 => Some(Self::Sleep),
            9 => Some(Self::CmdCall),
            10 => Some(Self::Jmp),
            11 => Some(Self::Waitfor),
            20 => Some(Self::TypeNumber),
            21 => Some(Self::TypeString),
            22 => Some(Self::TypeVar),
            23 => Some(Self::TypeArray),
            24 => Some(Self::TypeTrue),
            25 => Some(Self::TypeFalse),
            26 => Some(Self::TypeNull),
            27 => Some(Self::Pi),
            30 => Some(Self::CopyLastOp),
            31 => Some(Self::SwapLastOps),
            32 => Some(Self::IndexDec),
            33 => Some(Self::ConvToFloat),
            34 => Some(Self::ConvToString),
            35 => Some(Self::MemberAccess),
            36 => Some(Self::ConvToObject),
            37 => Some(Self::ArrayEnd),
            38 => Some(Self::ArrayNew),
            39 => Some(Self::Setarray),
            40 => Some(Self::InlineNew),
            41 => Some(Self::Makevar),
            42 => Some(Self::NewObject),
            43 => Some(Self::ObjFromStr),
            44 => Some(Self::InlineConditional),
            45 => Some(Self::Unknown45),
            46 => Some(Self::Unknown46),
            47 => Some(Self::Unknown47),
            50 => Some(Self::Assign),
            51 => Some(Self::FuncParamsEnd),
            52 => Some(Self::Inc),
            53 => Some(Self::Dec),
            54 => Some(Self::Unknown54),
            60 => Some(Self::Add),
            61 => Some(Self::Sub),
            62 => Some(Self::Mul),
            63 => Some(Self::Div),
            64 => Some(Self::Mod),
            65 => Some(Self::Pow),
            66 => Some(Self::Unknown66),
            67 => Some(Self::Unknown67),
            68 => Some(Self::Not),
            69 => Some(Self::UnarySub),
            70 => Some(Self::Eq),
            71 => Some(Self::Neq),
            72 => Some(Self::Lt),
            73 => Some(Self::Gt),
            74 => Some(Self::Lte),
            75 => Some(Self::Gte),
            76 => Some(Self::Bwo),
            77 => Some(Self::Bwa),
            78 => Some(Self::Bwx),
            79 => Some(Self::Bwi),
            80 => Some(Self::InRange),
            81 => Some(Self::InObj),
            82 => Some(Self::ObjIndex),
            83 => Some(Self::ObjType),
            84 => Some(Self::Format),
            85 => Some(Self::Int),
            86 => Some(Self::Abs),
            87 => Some(Self::Random),
            88 => Some(Self::Sin),
            89 => Some(Self::Cos),
            90 => Some(Self::Arctan),
            91 => Some(Self::Exp),
            92 => Some(Self::Log),
            93 => Some(Self::Min),
            94 => Some(Self::Max),
            95 => Some(Self::GetAngle),
            96 => Some(Self::GetDir),
            97 => Some(Self::VecX),
            98 => Some(Self::VecY),
            99 => Some(Self::ObjIndices),
            100 => Some(Self::ObjLink),
            101 => Some(Self::BwLeftshift),
            102 => Some(Self::BwRightshift),
            103 => Some(Self::Char),
            104 => Some(Self::ObjCompare),
            110 => Some(Self::ObjTrim),
            111 => Some(Self::ObjLength),
            112 => Some(Self::ObjPos),
            113 => Some(Self::Join),
            114 => Some(Self::ObjCharAt),
            115 => Some(Self::ObjSubstr),
            116 => Some(Self::ObjStarts),
            117 => Some(Self::ObjEnds),
            118 => Some(Self::ObjTokenize),
            119 => Some(Self::Translate),
            120 => Some(Self::ObjPositions),
            130 => Some(Self::ObjSize),
            131 => Some(Self::Array),
            132 => Some(Self::ArrayAssign),
            133 => Some(Self::ArrayMultidim),
            134 => Some(Self::ArrayMultidimAssign),
            135 => Some(Self::ObjSubarray),
            136 => Some(Self::ObjAddstring),
            137 => Some(Self::ObjDeletestring),
            138 => Some(Self::ObjRemovestring),
            139 => Some(Self::ObjReplacestring),
            140 => Some(Self::ObjInsertstring),
            141 => Some(Self::ObjClear),
            142 => Some(Self::ArrayNewMultidim),
            150 => Some(Self::With),
            151 => Some(Self::WithEnd),
            163 => Some(Self::ForEach),
            180 => Some(Self::This),
            181 => Some(Self::Thiso),
            182 => Some(Self::Player),
            183 => Some(Self::Playero),
            184 => Some(Self::Level),
            189 => Some(Self::Temp),
            190 => Some(Self::Params),
            _ => None,
        }
    }
}
