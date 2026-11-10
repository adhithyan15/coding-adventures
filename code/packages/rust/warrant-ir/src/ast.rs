//! Abstract syntax tree, types, and labels for the Warrant Intermediate Representation (WIR).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowMode {
    Wrap,
    Trap,
    Saturate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WirType {
    Unit,
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    ByteSlice,
    Ptr(Box<WirType>),
    Tuple(Vec<WirType>),
    Dynamic,
}

impl WirType {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            WirType::U8
                | WirType::U16
                | WirType::U32
                | WirType::U64
                | WirType::I8
                | WirType::I16
                | WirType::I32
                | WirType::I64
        )
    }

    pub fn is_signed_integer(&self) -> bool {
        matches!(
            self,
            WirType::I8 | WirType::I16 | WirType::I32 | WirType::I64
        )
    }

    pub fn is_unsigned_integer(&self) -> bool {
        matches!(
            self,
            WirType::U8 | WirType::U16 | WirType::U32 | WirType::U64
        )
    }

    pub fn bit_width(&self) -> Option<u32> {
        match self {
            WirType::U8 | WirType::I8 => Some(8),
            WirType::U16 | WirType::I16 => Some(16),
            WirType::U32 | WirType::I32 => Some(32),
            WirType::U64 | WirType::I64 => Some(64),
            _ => None,
        }
    }

    pub fn byte_size(&self) -> Option<usize> {
        match self {
            WirType::Unit => Some(0),
            WirType::Bool | WirType::U8 | WirType::I8 => Some(1),
            WirType::U16 | WirType::I16 => Some(2),
            WirType::U32 | WirType::I32 => Some(4),
            WirType::U64 | WirType::I64 => Some(8),
            WirType::Ptr(_) => Some(8),
            WirType::Tuple(elems) => {
                let mut total = 0;
                for e in elems {
                    total += e.byte_size()?;
                }
                Some(total)
            }
            WirType::ByteSlice | WirType::Dynamic => None,
        }
    }

    pub fn min_value(&self) -> Option<i128> {
        match self {
            WirType::U8 | WirType::U16 | WirType::U32 | WirType::U64 => Some(0),
            WirType::I8 => Some(i8::MIN as i128),
            WirType::I16 => Some(i16::MIN as i128),
            WirType::I32 => Some(i32::MIN as i128),
            WirType::I64 => Some(i64::MIN as i128),
            _ => None,
        }
    }

    pub fn max_value(&self) -> Option<i128> {
        match self {
            WirType::U8 => Some(u8::MAX as i128),
            WirType::U16 => Some(u16::MAX as i128),
            WirType::U32 => Some(u32::MAX as i128),
            WirType::U64 => Some(u64::MAX as i128),
            WirType::I8 => Some(i8::MAX as i128),
            WirType::I16 => Some(i16::MAX as i128),
            WirType::I32 => Some(i32::MAX as i128),
            WirType::I64 => Some(i64::MAX as i128),
            _ => None,
        }
    }
}

/// Information flow and secrecy label lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label {
    pub is_secret: bool,
    pub is_hostile: bool,
}

impl Label {
    pub const PUBLIC: Self = Self {
        is_secret: false,
        is_hostile: false,
    };
    pub const SECRET: Self = Self {
        is_secret: true,
        is_hostile: false,
    };
    pub const HOSTILE: Self = Self {
        is_secret: false,
        is_hostile: true,
    };
    pub const HOSTILE_SECRET: Self = Self {
        is_secret: true,
        is_hostile: true,
    };

    pub fn join(self, other: Self) -> Self {
        Self {
            is_secret: self.is_secret || other.is_secret,
            is_hostile: self.is_hostile || other.is_hostile,
        }
    }

    pub fn flows_to(self, target: Self) -> bool {
        (!self.is_secret || target.is_secret) && (!self.is_hostile || target.is_hostile)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProvenanceId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarId(pub String);

impl From<&str> for VarId {
    fn from(s: &str) -> Self {
        VarId(s.to_string())
    }
}

impl From<String> for VarId {
    fn from(s: String) -> Self {
        VarId(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,
    Neg(OverflowMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add(OverflowMode),
    Sub(OverflowMode),
    Mul(OverflowMode),
    Div(OverflowMode),
    Rem(OverflowMode),
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WirExpr {
    ConstInt { val: i128, ty: WirType },
    ConstBool(bool),
    ConstBytes(Vec<u8>),
    ConstUnit,
    Var(VarId),
    Unary { op: UnaryOp, expr: Box<WirExpr> },
    Binary { op: BinaryOp, lhs: Box<WirExpr>, rhs: Box<WirExpr> },
    Trunc { expr: Box<WirExpr>, target_ty: WirType },
    ZExt { expr: Box<WirExpr>, target_ty: WirType },
    SExt { expr: Box<WirExpr>, target_ty: WirType },
    TupleMake(Vec<WirExpr>),
    TupleGet { expr: Box<WirExpr>, index: usize },
    PtrOffset { ptr: Box<WirExpr>, offset_bytes: Box<WirExpr> },
    PtrLen(Box<WirExpr>),
    MakeDynamic { expr: Box<WirExpr>, tag: String },
    CheckDynamic { expr: Box<WirExpr>, expected_tag: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WirInst {
    Let {
        var: VarId,
        ty: WirType,
        label: Label,
        expr: WirExpr,
    },
    Alloc {
        var: VarId,
        size_expr: WirExpr,
    },
    Load {
        var: VarId,
        ptr_expr: WirExpr,
        ty: WirType,
    },
    Store {
        ptr_expr: WirExpr,
        val_expr: WirExpr,
    },
    Free {
        ptr_expr: WirExpr,
    },
    CallCapability {
        var: Option<VarId>,
        capability: String,
        args: Vec<WirExpr>,
    },
    CallFunction {
        var: Option<VarId>,
        func_name: String,
        args: Vec<WirExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WirTerminator {
    Return(Option<WirExpr>),
    Jump(BlockId),
    Branch {
        cond: WirExpr,
        then_block: BlockId,
        else_block: BlockId,
    },
    Trap(String),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<WirInst>,
    pub terminator: WirTerminator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<(VarId, WirType, Label)>,
    pub return_type: WirType,
    pub blocks: Vec<BasicBlock>,
    pub ranking_metric: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

impl Program {
    pub fn new(functions: Vec<Function>) -> Self {
        Self { functions }
    }

    pub fn find_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }
}
