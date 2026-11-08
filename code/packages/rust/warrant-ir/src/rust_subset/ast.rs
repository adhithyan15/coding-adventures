//! Abstract Syntax Tree (AST) for the Confined Rust Subset (VER25a, VER-P022).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    PublicSuper,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub is_inner: bool, // true for #![...], false for #[...]
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustType {
    U8,
    U16,
    U32,
    U64,
    Usize,
    I8,
    I16,
    I32,
    I64,
    Isize,
    Bool,
    Unit,
    Ref(Box<RustType>),
    RefMut(Box<RustType>),
    Slice(Box<RustType>),
    Array(Box<RustType>, usize),
    ConstArray(Box<RustType>, String), // e.g. [u8; N]
    Vec(Box<RustType>),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseTree {
    Simple(String),
    Group(Vec<UseTree>),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDecl {
    pub path: Vec<String>,
    pub tree: UseTree,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstGenericParam {
    pub name: String,
    pub ty: RustType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub is_mut: bool,
    pub name: String,
    pub ty: RustType,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
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
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
    Deref,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    BitOrAssign,
    BitAndAssign,
    BitXorAssign,
    AddAssign,
    SubAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit {
    Bool(bool),
    Int(u128, Option<String>),
    ByteString(Vec<u8>),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Lit(Lit, Span),
    Path(Vec<String>, Span),
    Unary(UnOp, Box<Expr>, Span),
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    MethodCall(Box<Expr>, String, Vec<Expr>, Span),
    Cast(Box<Expr>, RustType, Span),
    Block(Block, Span),
    If(Box<Expr>, Block, Option<Block>, Span),
    Array(Vec<Expr>, Span),
    ArrayRepeat(Box<Expr>, Box<Expr>, Span), // [val; len]
    MacroCall(String, Vec<Expr>, Span),
    Range(Option<Box<Expr>>, Option<Box<Expr>>, bool, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit(_, s)
            | Expr::Path(_, s)
            | Expr::Unary(_, _, s)
            | Expr::Binary(_, _, _, s)
            | Expr::Index(_, _, s)
            | Expr::Call(_, _, s)
            | Expr::MethodCall(_, _, _, s)
            | Expr::Cast(_, _, s)
            | Expr::Block(_, s)
            | Expr::If(_, _, _, s)
            | Expr::Array(_, s)
            | Expr::ArrayRepeat(_, _, s)
            | Expr::MacroCall(_, _, s)
            | Expr::Range(_, _, _, s) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let {
        name: String,
        is_mut: bool,
        ty: Option<RustType>,
        init: Option<Expr>,
        span: Span,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
        span: Span,
    },
    Expr(Expr, Span),
    Return(Option<Expr>, Span),
    For {
        var: String,
        iter: Expr,
        body: Block,
        span: Span,
    },
    Unsafe(Block, Span),
    Item(Box<Item>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub is_unsafe: bool,
    pub is_inline_always: bool,
    pub is_inline_never: bool,
    pub name: String,
    pub const_generics: Vec<ConstGenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<RustType>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDecl {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Use(UseDecl),
    Module(ModuleDecl),
    Function(FnDecl),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceFile {
    pub inner_attrs: Vec<Attribute>,
    pub items: Vec<Item>,
}
