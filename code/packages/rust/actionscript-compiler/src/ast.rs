//! ActionScript Abstract Syntax Tree definitions.

#![deny(unsafe_code)]

/// Visibility modifier in AS2 and AS3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Type annotation in AS2 and AS3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnotation {
    Simple(String),
    Vector(Box<TypeAnnotation>),
    Any,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Ushr,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LogAnd,
    LogOr,
    // AS3 runtime type operators
    Is,
    As,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
    BitNot,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
    Typeof,
    Delete,
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

/// Expressions in ActionScript.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    StringLit(String),
    BoolLit(bool),
    Null,
    Undefined,
    This,
    Root,
    Parent,
    Global,
    Ident(String),
    ArrayLit(Vec<Expr>),
    ObjectLit(Vec<(String, Expr)>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Conditional {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    VectorLit {
        item_type: TypeAnnotation,
        items: Vec<Expr>,
    },
    Eval(Box<Expr>),
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
}

impl Param {
    pub fn param_new(name: String, type_ann: Option<TypeAnnotation>) -> Self {
        Self { name, type_ann }
    }
}

/// Function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Option<TypeAnnotation>,
    pub body: Vec<Stmt>,
    pub is_static: bool,
    pub visibility: Visibility,
}

impl FunctionDef {
    pub fn function_def_new(
        name: String,
        params: Vec<Param>,
        ret_type: Option<TypeAnnotation>,
        body: Vec<Stmt>,
        is_static: bool,
        visibility: Visibility,
    ) -> Self {
        Self {
            name,
            params,
            ret_type,
            body,
            is_static,
            visibility,
        }
    }
}

/// Field definition in class.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub init: Option<Expr>,
    pub is_static: bool,
    pub visibility: Visibility,
    pub is_const: bool,
}

impl FieldDef {
    pub fn field_def_new(
        name: String,
        type_ann: Option<TypeAnnotation>,
        init: Option<Expr>,
        is_static: bool,
        visibility: Visibility,
        is_const: bool,
    ) -> Self {
        Self {
            name,
            type_ann,
            init,
            is_static,
            visibility,
            is_const,
        }
    }
}

/// Class definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<FunctionDef>,
    pub is_final: bool,
    pub is_dynamic: bool,
    pub visibility: Visibility,
}

impl ClassDef {
    #[allow(clippy::too_many_arguments)]
    pub fn class_def_new(
        name: String,
        extends: Option<String>,
        implements: Vec<String>,
        fields: Vec<FieldDef>,
        methods: Vec<FunctionDef>,
        is_final: bool,
        is_dynamic: bool,
        visibility: Visibility,
    ) -> Self {
        Self {
            name,
            extends,
            implements,
            fields,
            methods,
            is_final,
            is_dynamic,
            visibility,
        }
    }
}

/// Interface definition.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDef {
    pub name: String,
    pub extends: Vec<String>,
    pub methods: Vec<(String, Vec<Param>, Option<TypeAnnotation>)>,
}

impl InterfaceDef {
    pub fn interface_def_new(
        name: String,
        extends: Vec<String>,
        methods: Vec<(String, Vec<Param>, Option<TypeAnnotation>)>,
    ) -> Self {
        Self {
            name,
            extends,
            methods,
        }
    }
}

/// Catch clause in AS3 try/catch.
#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub param: String,
    pub param_type: Option<TypeAnnotation>,
    pub body: Vec<Stmt>,
}

impl CatchClause {
    pub fn catch_clause_new(
        param: String,
        param_type: Option<TypeAnnotation>,
        body: Vec<Stmt>,
    ) -> Self {
        Self {
            param,
            param_type,
            body,
        }
    }
}

/// Package definition in AS3.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageDef {
    pub name: String,
    pub statements: Vec<Stmt>,
}

impl PackageDef {
    pub fn package_def_new(name: String, statements: Vec<Stmt>) -> Self {
        Self { name, statements }
    }
}

/// Statements in ActionScript.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    VarDecl {
        name: String,
        type_ann: Option<TypeAnnotation>,
        init: Option<Expr>,
    },
    ConstDecl {
        name: String,
        type_ann: Option<TypeAnnotation>,
        init: Expr,
    },
    Block(Vec<Stmt>),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    DoWhile {
        body: Box<Stmt>,
        cond: Expr,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        step: Option<Expr>,
        body: Box<Stmt>,
    },
    ForIn {
        var_name: String,
        obj: Expr,
        body: Box<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    FunctionDecl(FunctionDef),
    ClassDecl(ClassDef),
    InterfaceDecl(InterfaceDef),
    PackageDecl(PackageDef),
    ImportDecl(String),
    NamespaceDecl(String),
    TryCatchFinally {
        try_block: Vec<Stmt>,
        catches: Vec<CatchClause>,
        finally_block: Option<Vec<Stmt>>,
    },
    Throw(Expr),
    // Flash Timeline Verbs
    Stop,
    Play,
    GotoAndPlay(Expr),
    GotoAndStop(Expr),
    TellTarget {
        target: Expr,
        body: Vec<Stmt>,
    },
    CallAction(Expr),
    OnClipEvent {
        event: String,
        body: Vec<Stmt>,
    },
    Trace(Expr),
}

/// A parsed program comprising statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Program {
    pub fn program_new(statements: Vec<Stmt>) -> Self {
        Self { statements }
    }
}
