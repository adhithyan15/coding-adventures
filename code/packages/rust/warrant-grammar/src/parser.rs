//! # Bounded guarded Warrant parser (VER04c / P006b3)
//!
//! Consumes the significant-layout token stream from [`layout`](crate::layout)
//! and constructs a typed, bounded Abstract Syntax Tree (AST) conforming to
//! `warrant.grammar`.
//!
//! Enforces bounded recursion depth (preventing call-stack exhaustion / CWE-674),
//! strict AST node quotas, and work budgets. Errors fail closed with stable code
//! `W0001` and never leak raw source tokens.

use std::fmt;

use crate::fixed::FixedTy;
use crate::layout::{LayoutToken, LayoutTokenKind, TokenSpan};
use crate::source::SourceDocument;

pub const DEFAULT_MAX_NODES: usize = 65_536;
pub const MAX_NODES_CEILING: usize = 1_048_576;

pub const DEFAULT_MAX_DEPTH: usize = 64;
pub const MAX_DEPTH_CEILING: usize = 128;

pub const DEFAULT_MAX_WORK: usize = 196_608;
pub const MAX_WORK_CEILING: usize = 3_145_728;

/// Bounded configuration for the guarded parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserLimits {
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_parse_work: usize,
}

impl Default for ParserLimits {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_NODES,
            max_depth: DEFAULT_MAX_DEPTH,
            max_parse_work: DEFAULT_MAX_WORK,
        }
    }
}

/// Closed parser failure kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserErrorKind {
    InvalidLimits,
    Work,
    NodeBudget,
    RecursionDepth,
    UnexpectedToken,
    UnexpectedEof,
    Allocation,
    /// The second word of the header is not exactly `v1` (VER04j section 1.1).
    InvalidVersionHeader,
    /// A v1 construct (a new literal form) in a v0 document (VER04j section 1.1).
    V1ConstructInV0,
    /// A suffixed literal outside the range of its type (VER04j section 2.2).
    LiteralOutOfRange,
    /// WX-A: a duplicate field, constructor-field or constructor name inside one
    /// `record` or `variant` declaration (VER04j section 14.1, item 13).
    DuplicateName,
    /// WX-E: a `vectors` source path or `sha256` pin that breaks the rules of
    /// VER04j section 5.2 (W0708).
    TableIntegrity,
}

/// Parser error with a stable diagnostic code and safe message: `W0001` for
/// every v0 failure, `W0101` for the v1 header and literal rules.
#[derive(Clone, PartialEq, Eq)]
pub struct ParserError {
    kind: ParserErrorKind,
    span: Option<(usize, usize)>,
}

impl ParserError {
    #[track_caller]
    pub fn new(kind: ParserErrorKind, span: Option<(usize, usize)>) -> Self {
        Self { kind, span }
    }

    pub fn code(&self) -> &'static str {
        match self.kind {
            ParserErrorKind::InvalidVersionHeader
            | ParserErrorKind::V1ConstructInV0
            | ParserErrorKind::LiteralOutOfRange => "W0101",
            ParserErrorKind::DuplicateName => "W0201",
            ParserErrorKind::TableIntegrity => "W0708",
            _ => "W0001",
        }
    }

    pub fn kind(&self) -> ParserErrorKind {
        self.kind
    }

    pub fn span(&self) -> Option<(usize, usize)> {
        self.span
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self.kind {
            ParserErrorKind::InvalidLimits => "invalid parser limits",
            ParserErrorKind::Work => "parse work budget exceeded",
            ParserErrorKind::NodeBudget => "ast node quota exceeded",
            ParserErrorKind::RecursionDepth => "recursion depth limit exceeded",
            ParserErrorKind::UnexpectedToken => "unexpected token in source",
            ParserErrorKind::UnexpectedEof => "unexpected end of file",
            ParserErrorKind::Allocation => "memory allocation failure",
            ParserErrorKind::InvalidVersionHeader => "invalid version header",
            ParserErrorKind::V1ConstructInV0 => "v1 construct in a v0 document",
            ParserErrorKind::LiteralOutOfRange => "literal out of range for its type",
            ParserErrorKind::DuplicateName => "duplicate name in a declaration",
            ParserErrorKind::TableIntegrity => "malformed table source path or sha256 pin",
        };
        write!(f, "{}: {msg}", self.code())
    }
}

impl fmt::Debug for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParserError")
            .field("code", &self.code())
            .field("kind", &self.kind)
            .field("span", &self.span)
            .finish()
    }
}

impl std::error::Error for ParserError {}

// ── AST Definitions ─────────────────────────────────────────────────────────

/// The language version a document declares in its header (VER04j section 1.1).
///
/// ```text
/// warrant gf256        -- a v0 document: means exactly what VER04 says
/// warrant gf256 v1     -- a v1 document: WX-B (fixed-width integers) and
///                         WX-C (bitwise operations) are enabled
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    V0,
    V1,
}

/// A parsed Warrant file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarrantFile<'a> {
    pub name: &'a str,
    pub version: Version,
    pub declarations: Vec<Declaration<'a>>,
    pub span: (usize, usize),
}

/// Top-level declaration in a Warrant file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declaration<'a> {
    Model(ModelDecl<'a>),
    Error(ErrorDecl<'a>),
    Op(OpDecl<'a>),
    Example(ExampleDecl<'a>),
    Claim(ClaimDecl<'a>),
    Assumptions(AssumptionsDecl<'a>),
    Labels(LabelsDecl<'a>),
    Input(InputDecl<'a>),
    Bound(BoundDecl<'a>),
    /// WX-A: `record Name(f1: T1, ...)` (v1 only).
    Record(RecordDecl<'a>),
    /// WX-A: `variant Name = C1(f: T) | C2 | ...` (v1 only).
    Variant(VariantDecl<'a>),
    /// WX-E: `vectors Name:` with a pinned external table (v1 only).
    Vectors(VectorsDecl<'a>),
    /// WX-G: `ensures op:` and one to 32 Bool rows over the parameters and
    /// `result` (v1 only).
    Ensures(EnsuresDecl<'a>),
}

/// `ensures NAME:` (VER04k section 4.2): the postcondition rows of one operation.
///
/// ```text
/// ensures zeroize:
///   len(result) == len(buf)
///   result == zeros(len(buf))
/// ```
///
/// In a row the operation's parameters denote the *pre-state* (the argument
/// values) and the contextual name `result` the operation's return value, which
/// carries the post-state of every `inout` parameter.  The parser keeps the rows
/// as written; the 32-row cap, the typing and the "no operation, no quantifier"
/// rules belong to the checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsuresDecl<'a> {
    pub op: &'a str,
    pub rows: Vec<Expression<'a>>,
    pub span: (usize, usize),
}

/// `vectors NAME:` (VER04j section 5.2): a table of known answers kept in a
/// fixture file whose identity the document pins.
///
/// ```text
/// vectors sha256_cavs_short:
///   source "sha256-cavs-v1/short.tfv"
///   sha256 b"<64 lowercase hex digits>"
///   rows 65
///   column msg: Bytes
///   column md: Bytes<32>
///   holds sha256_digest(msg) == md
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorsDecl<'a> {
    pub name: &'a str,
    /// The path relative to the fixtures root, already checked by
    /// [`valid_table_path`].
    pub source: &'a str,
    /// The 64 lowercase hex digits of the pinned SHA-256.
    pub sha256: &'a str,
    /// The number of records the file must hold.
    pub rows: u64,
    pub columns: Vec<ColumnDecl<'a>>,
    pub holds: Expression<'a>,
    pub span: (usize, usize),
}

/// `column NAME: type` of a `vectors` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDecl<'a> {
    pub name: &'a str,
    pub ty: TypeExpr<'a>,
    pub span: (usize, usize),
}

/// Longest `source` path of a table, in bytes (VER04j section 5.2).
pub const MAX_PATH_BYTES: usize = 256;

/// The path rules of VER04j section 5.2: `[A-Za-z0-9_./-]+`, at most
/// [`MAX_PATH_BYTES`] bytes, no leading `/`, and no empty, `.` or `..` segment.
/// A backslash, a space, a NUL or any non-ASCII byte is outside the alphabet.
pub fn valid_table_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_BYTES
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-'))
        && path
            .split('/')
            // No empty segment and no all-dot segment (`.`, `..`, `...`).
            .all(|seg| !seg.is_empty() && !seg.bytes().all(|b| b == b'.'))
}

/// One `name: type` field of a record or constructor (VER04j section 4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl<'a> {
    pub name: &'a str,
    pub ty: TypeExpr<'a>,
    pub span: (usize, usize),
}

/// `record Name(f1: T1, ..., fn: Tn)`, `1 <= n <= 32`: a nominal type with a
/// single constructor that has the type's own name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDecl<'a> {
    pub name: &'a str,
    pub fields: Vec<FieldDecl<'a>>,
    pub span: (usize, usize),
}

/// One constructor of a `variant`; a nullary constructor has no fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtorDecl<'a> {
    pub name: &'a str,
    pub fields: Vec<FieldDecl<'a>>,
    pub span: (usize, usize),
}

/// `variant Name = C1(fields) | C2 | ...`, `2 <= constructors <= 256`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantDecl<'a> {
    pub name: &'a str,
    pub ctors: Vec<CtorDecl<'a>>,
    pub span: (usize, usize),
}

/// Most fields a record or a constructor may have (VER04j section 4.2).
pub const MAX_FIELDS: usize = 32;
/// Most constructors a variant may have.
pub const MAX_CTORS: usize = 256;
/// Most components of a tuple.
pub const MAX_TUPLE: usize = 8;
/// Deepest nesting of a pattern (VER04j section 4.3).
pub const MAX_PATTERN_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDecl<'a> {
    pub name: &'a str,
    pub params: Vec<Parameter<'a>>,
    pub target_type: TypeExpr<'a>,
    pub body: Option<Expression<'a>>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDecl<'a> {
    pub name: &'a str,
    pub variants: Vec<&'a str>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpDecl<'a> {
    pub name: &'a str,
    pub static_params: Vec<StaticParam<'a>>,
    pub params: Vec<Parameter<'a>>,
    pub return_type: TypeExpr<'a>,
    pub is_fresh: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticParam<'a> {
    pub name: &'a str,
    pub type_name: &'a str,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleDecl<'a> {
    pub name: &'a str,
    pub body: Expression<'a>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimDecl<'a> {
    pub name: &'a str,
    pub body: Expression<'a>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssumptionsDecl<'a> {
    Platform {
        rows: Vec<QualifiedName<'a>>,
        span: (usize, usize),
    },
    Environment {
        rows: Vec<EnvironmentRow<'a>>,
        span: (usize, usize),
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentRow<'a> {
    pub name: &'a str,
    pub expr: Expression<'a>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelsDecl<'a> {
    pub name: &'a str,
    pub rows: Vec<LabelRow<'a>>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRow<'a> {
    pub name: QualifiedName<'a>,
    pub is_secret: bool,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDecl<'a> {
    pub target: QualifiedName<'a>,
    pub is_hostile: bool,
    pub rejections: Vec<Rejection<'a>>,
    pub ensures: Vec<&'a str>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection<'a> {
    pub condition: Expression<'a>,
    pub error_variant: QualifiedName<'a>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundDecl<'a> {
    pub name: &'a str,
    pub rows: Vec<BoundRow<'a>>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundRow<'a> {
    pub name: &'a str,
    pub bound: Expression<'a>,
    pub span: (usize, usize),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Parameter<'a> {
    pub name: &'a str,
    pub param_type: TypeExpr<'a>,
    pub refinement: Option<Expression<'a>>,
    /// WX-G (VER04k section 4.2): the parameter is written `name: inout T`.  Its
    /// value at the call is the pre-state, and the operation's result carries
    /// the post-state.  Only an operation's parameter can be `inout` (v1 only).
    pub inout: bool,
    pub span: (usize, usize),
}

/// `Debug` prints exactly what the derive printed before WX-G added `inout`,
/// and the extra field only when it is set.  The pinned hashes of the two v0
/// specifications (`tests/v0_compat.rs`, VER04j section 1.3) are hashes of this
/// text, so a field that only a v1 document can set must not appear in it.
impl std::fmt::Debug for Parameter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Parameter");
        d.field("name", &self.name)
            .field("param_type", &self.param_type)
            .field("refinement", &self.refinement);
        if self.inout {
            d.field("inout", &self.inout);
        }
        d.field("span", &self.span).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr<'a> {
    Bytes {
        width: Option<Width<'a>>,
        span: (usize, usize),
    },
    List {
        inner: Box<TypeExpr<'a>>,
        span: (usize, usize),
    },
    Option {
        inner: Box<TypeExpr<'a>>,
        span: (usize, usize),
    },
    Result {
        ok_type: Box<TypeExpr<'a>>,
        err_name: &'a str,
        span: (usize, usize),
    },
    Named {
        name: &'a str,
        span: (usize, usize),
    },
    /// WX-A: `(T1, ..., Tn)`, `2 <= n <= 8`, structural (v1 only).
    Tuple {
        elems: Vec<TypeExpr<'a>>,
        span: (usize, usize),
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Width<'a> {
    Literal(u64),
    Named(&'a str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedName<'a> {
    pub segments: Vec<&'a str>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression<'a> {
    Quantified {
        is_forall: bool,
        binders: Vec<Parameter<'a>>,
        body: Box<Expression<'a>>,
        span: (usize, usize),
    },
    Conditional {
        condition: Box<Expression<'a>>,
        then_branch: Box<Expression<'a>>,
        else_branch: Box<Expression<'a>>,
        span: (usize, usize),
    },
    Match {
        target: Box<Expression<'a>>,
        arms: Vec<MatchArm<'a>>,
        span: (usize, usize),
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
        span: (usize, usize),
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expression<'a>>,
        span: (usize, usize),
    },
    Literal {
        lit: Literal<'a>,
        span: (usize, usize),
    },
    List {
        elements: Vec<Expression<'a>>,
        span: (usize, usize),
    },
    Reference {
        name: QualifiedName<'a>,
        widths: Vec<Width<'a>>,
        args: Option<Vec<Expression<'a>>>,
        span: (usize, usize),
    },
    Grouped {
        inner: Box<Expression<'a>>,
        span: (usize, usize),
    },
    /// WX-A: `(e1, ..., en)`, `2 <= n <= 8` (v1 only).
    Tuple {
        elements: Vec<Expression<'a>>,
        span: (usize, usize),
    },
    /// WX-A: postfix projection `base.0` / `base.field` (v1 only).  A projection
    /// whose head is a plain name (`r.field`) is parsed as a qualified name and
    /// resolved by the checker.
    Project {
        base: Box<Expression<'a>>,
        key: ProjKey<'a>,
        span: (usize, usize),
    },
}

impl<'a> Expression<'a> {
    /// The names among `names` that the expression mentions as the first segment
    /// of a reference (a column of a `vectors` table, a parameter of an
    /// operation's bound).  Patterns bind fresh names, which the checker has
    /// already kept distinct from every name in scope.
    pub fn mentioned_names(&self, names: &std::collections::HashSet<&str>, out: &mut std::collections::BTreeSet<String>) {
        match self {
            Expression::Reference { name, args, .. } => {
                if let Some(first) = name.segments.first() {
                    if names.contains(first) {
                        out.insert((*first).to_string());
                    }
                }
                for a in args.iter().flat_map(|l| l.iter()) {
                    a.mentioned_names(names, out);
                }
            }
            Expression::Grouped { inner, .. } => inner.mentioned_names(names, out),
            Expression::Unary { operand, .. } => operand.mentioned_names(names, out),
            Expression::Binary { left, right, .. } => {
                left.mentioned_names(names, out);
                right.mentioned_names(names, out);
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                condition.mentioned_names(names, out);
                then_branch.mentioned_names(names, out);
                else_branch.mentioned_names(names, out);
            }
            Expression::Tuple { elements, .. } | Expression::List { elements, .. } => {
                for x in elements {
                    x.mentioned_names(names, out);
                }
            }
            Expression::Project { base, .. } => base.mentioned_names(names, out),
            Expression::Match { target, arms, .. } => {
                target.mentioned_names(names, out);
                for a in arms {
                    a.body.mentioned_names(names, out);
                }
            }
            Expression::Quantified { binders, body, .. } => {
                for b in binders {
                    if let Some(r) = &b.refinement {
                        r.mentioned_names(names, out);
                    }
                }
                body.mentioned_names(names, out);
            }
            Expression::Literal { .. } => {}
        }
    }
}

/// The selector of an [`Expression::Project`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjKey<'a> {
    /// A tuple component `t.0`.
    Index(usize),
    /// A record field `r.name`.
    Field(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Implies,
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    Add,
    Sub,
    Mul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal<'a> {
    Bool(bool),
    U64(u64),
    U8(u8),
    Bytes(&'a str),
    Int(u64),
    /// A suffixed literal of one of the six types added by WX-B (`5u16`,
    /// `-128i8`, `0xffu32`), holding its canonical in-range value.  `u8` and
    /// `u64` literals keep their v0 variants.
    Fixed(FixedTy, i128),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm<'a> {
    pub pattern: Pattern<'a>,
    pub body: Expression<'a>,
    pub span: (usize, usize),
}

/// A match pattern (VER04j section 4.3).
///
/// ```text
/// None                  Pattern::None
/// Some(x)  Some((a, b)) Pattern::Some(Box<..>)     (likewise Ok and Err)
/// _                     Pattern::Wild              (v1; a v0 binder named `_`)
/// true false            Pattern::Bool
/// (p, q)                Pattern::Tuple
/// Red  Circle(r)  E.X   Pattern::Ctor { name, args }
/// ```
///
/// A bare NAME is `Ctor` with no arguments: the checker decides whether it is a
/// constructor (it names one) or a fresh binder.  In a v0 document the inner
/// pattern of `Some`/`Ok`/`Err` is always such a bare NAME, a binder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern<'a> {
    None,
    Some(Box<Pattern<'a>>),
    Ok(Box<Pattern<'a>>),
    Err(Box<Pattern<'a>>),
    Wild,
    Bool(bool),
    Tuple(Vec<Pattern<'a>>),
    Ctor {
        name: QualifiedName<'a>,
        args: Vec<Pattern<'a>>,
    },
}

impl<'a> Pattern<'a> {
    /// The binder name of a v0-style inner pattern (a bare single NAME).
    pub fn as_binder(&self) -> Option<&'a str> {
        match self {
            Pattern::Ctor { name, args } if args.is_empty() && name.segments.len() == 1 => {
                Some(name.segments[0])
            }
            _ => None,
        }
    }
}

// ── Parser Engine ───────────────────────────────────────────────────────────

struct Parser<'a> {
    tokens: &'a [LayoutToken<'a>],
    pos: usize,
    doc: &'a SourceDocument<'a>,
    limits: ParserLimits,
    nodes_allocated: usize,
    depth: usize,
    work_done: usize,
    version: Version,
}

impl<'a> Parser<'a> {
    fn new(
        tokens: &'a [LayoutToken<'a>],
        doc: &'a SourceDocument<'a>,
        limits: ParserLimits,
    ) -> Result<Self, ParserError> {
        if limits.max_nodes == 0
            || limits.max_nodes > MAX_NODES_CEILING
            || limits.max_depth == 0
            || limits.max_depth > MAX_DEPTH_CEILING
            || limits.max_parse_work == 0
            || limits.max_parse_work > MAX_WORK_CEILING
        {
            return Err(ParserError::new(ParserErrorKind::InvalidLimits, None));
        }

        Ok(Self {
            tokens,
            pos: 0,
            doc,
            limits,
            nodes_allocated: 0,
            depth: 0,
            work_done: 0,
            version: Version::V0,
        })
    }

    fn check_work(&mut self) -> Result<(), ParserError> {
        self.work_done += 1;
        if self.work_done > self.limits.max_parse_work {
            return Err(ParserError::new(
                ParserErrorKind::Work,
                self.current_raw_span(),
            ));
        }
        Ok(())
    }

    fn enter_depth(&mut self) -> Result<(), ParserError> {
        if self.depth >= self.limits.max_depth {
            return Err(ParserError::new(
                ParserErrorKind::RecursionDepth,
                self.current_raw_span(),
            ));
        }
        self.depth += 1;
        Ok(())
    }

    fn leave_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn count_node(&mut self) -> Result<(), ParserError> {
        self.nodes_allocated += 1;
        if self.nodes_allocated > self.limits.max_nodes {
            return Err(ParserError::new(
                ParserErrorKind::NodeBudget,
                self.current_raw_span(),
            ));
        }
        Ok(())
    }

    fn peek(&self) -> Option<&'a LayoutToken<'a>> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<LayoutTokenKind<'a>> {
        self.tokens.get(self.pos).map(|t| t.kind)
    }

    fn advance(&mut self) -> Result<Option<&'a LayoutToken<'a>>, ParserError> {
        self.check_work()?;
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            Ok(Some(tok))
        } else {
            Ok(None)
        }
    }

    fn current_raw_span(&self) -> Option<(usize, usize)> {
        self.peek().and_then(|tok| match tok.span {
            TokenSpan::Real { start, end } => {
                let orig = self.doc.original_span(start, end)?;
                Some((orig.start.byte_offset, orig.end.byte_offset))
            }
            TokenSpan::Synthetic { at } => {
                let orig = self.doc.original_span(at, at)?;
                Some((orig.start.byte_offset, orig.end.byte_offset))
            }
        })
    }

    fn token_span_range(&self, tok: &LayoutToken<'a>) -> (usize, usize) {
        match tok.span {
            TokenSpan::Real { start, end } => {
                let orig = self.doc.original_span(start, end);
                orig.map(|o| (o.start.byte_offset, o.end.byte_offset))
                    .unwrap_or((start, end))
            }
            TokenSpan::Synthetic { at } => {
                let orig = self.doc.original_span(at, at);
                orig.map(|o| (o.start.byte_offset, o.end.byte_offset))
                    .unwrap_or((at, at))
            }
        }
    }

    fn expect_word(&mut self, expected: &str) -> Result<&'a LayoutToken<'a>, ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Word(w)) if w == expected => Ok(self.advance()?.unwrap()),
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    fn expect_name(&mut self) -> Result<(&'a str, (usize, usize)), ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Word(w)) => {
                let tok = self.advance()?.unwrap();
                Ok((w, self.token_span_range(tok)))
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    fn expect_symbol(&mut self, sym: &str) -> Result<&'a LayoutToken<'a>, ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Symbol(s)) if s == sym => Ok(self.advance()?.unwrap()),
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    fn is_widths_ahead(&mut self) -> Result<bool, ParserError> {
        let mut i = self.pos + 1;
        while i < self.tokens.len() {
            self.check_work()?;
            match self.tokens[i].kind {
                LayoutTokenKind::Number(_) | LayoutTokenKind::Word(_) => {
                    i += 1;
                    if i < self.tokens.len()
                        && matches!(self.tokens[i].kind, LayoutTokenKind::Symbol(","))
                    {
                        i += 1;
                        continue;
                    }
                    if i < self.tokens.len()
                        && matches!(self.tokens[i].kind, LayoutTokenKind::Symbol(">"))
                    {
                        return Ok(true);
                    }
                    return Ok(false);
                }
                _ => return Ok(false),
            }
        }
        Ok(false)
    }

    fn expect_newline(&mut self) -> Result<(), ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Newline) => {
                self.advance()?;
                Ok(())
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Ok(()), // EOF at end of line can accept
        }
    }

    fn expect_indent(&mut self) -> Result<(), ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Indent) => {
                self.advance()?;
                Ok(())
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    fn expect_dedent(&mut self) -> Result<(), ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Dedent) => {
                self.advance()?;
                Ok(())
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    // ── Parse Top-Level File ────────────────────────────────────────────────

    fn parse_file(&mut self) -> Result<WarrantFile<'a>, ParserError> {
        self.count_node()?;
        let start_tok = self.expect_word("warrant")?;
        let start_span = self.token_span_range(start_tok);

        let (file_name, _) = self.expect_name()?;
        // VER04j section 1.1: `warrant NAME [ NAME ]`; the optional second
        // name must be exactly `v1`.  Anything else is W0101, never ignored.
        if let Some(LayoutTokenKind::Word(word)) = self.peek_kind() {
            if word != "v1" {
                return Err(ParserError::new(
                    ParserErrorKind::InvalidVersionHeader,
                    self.current_raw_span(),
                ));
            }
            self.advance()?;
            self.version = Version::V1;
        }
        self.expect_newline()?;
        self.expect_indent()?;

        let mut declarations = Vec::new();
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, LayoutTokenKind::Dedent) {
                break;
            }
            if matches!(tok.kind, LayoutTokenKind::Newline) {
                self.advance()?;
                continue;
            }
            declarations.push(self.parse_declaration()?);
        }

        self.expect_dedent()?;

        // Skip any trailing newlines.
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, LayoutTokenKind::Newline) {
                self.advance()?;
            } else {
                return Err(ParserError::new(
                    ParserErrorKind::UnexpectedToken,
                    self.current_raw_span(),
                ));
            }
        }

        let end_span = declarations
            .last()
            .map(|d| match d {
                Declaration::Model(m) => m.span,
                Declaration::Error(e) => e.span,
                Declaration::Op(o) => o.span,
                Declaration::Example(e) => e.span,
                Declaration::Claim(c) => c.span,
                Declaration::Assumptions(a) => match a {
                    AssumptionsDecl::Platform { span, .. } => *span,
                    AssumptionsDecl::Environment { span, .. } => *span,
                },
                Declaration::Labels(l) => l.span,
                Declaration::Input(i) => i.span,
                Declaration::Bound(b) => b.span,
                Declaration::Record(r) => r.span,
                Declaration::Variant(v) => v.span,
                Declaration::Vectors(v) => v.span,
                Declaration::Ensures(e) => e.span,
            })
            .unwrap_or(start_span);

        Ok(WarrantFile {
            name: file_name,
            version: self.version,
            declarations,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_declaration(&mut self) -> Result<Declaration<'a>, ParserError> {
        self.count_node()?;
        self.enter_depth()?;
        let result = match self.peek_kind() {
            Some(LayoutTokenKind::Word("model")) => self.parse_model_decl().map(Declaration::Model),
            Some(LayoutTokenKind::Word("error")) => self.parse_error_decl().map(Declaration::Error),
            Some(LayoutTokenKind::Word("op")) => self.parse_op_decl().map(Declaration::Op),
            Some(LayoutTokenKind::Word("example")) => {
                self.parse_example_decl().map(Declaration::Example)
            }
            Some(LayoutTokenKind::Word("claim")) => self.parse_claim_decl().map(Declaration::Claim),
            Some(LayoutTokenKind::Word("assume")) => {
                self.parse_assumptions_decl().map(Declaration::Assumptions)
            }
            Some(LayoutTokenKind::Word("labels")) => {
                self.parse_labels_decl().map(Declaration::Labels)
            }
            Some(LayoutTokenKind::Word("input")) => self.parse_input_decl().map(Declaration::Input),
            Some(LayoutTokenKind::Word("bound")) => self.parse_bound_decl().map(Declaration::Bound),
            Some(LayoutTokenKind::Word("record")) => {
                self.require_v1()?;
                self.parse_record_decl().map(Declaration::Record)
            }
            Some(LayoutTokenKind::Word("variant")) => {
                self.require_v1()?;
                self.parse_variant_decl().map(Declaration::Variant)
            }
            Some(LayoutTokenKind::Word("vectors")) => {
                self.require_v1()?;
                self.parse_vectors_decl().map(Declaration::Vectors)
            }
            // WX-G: `ensures` is reserved since VER04 section 2 but was only ever
            // valid as the last row of an `input` block, so at declaration level
            // it is new in v1 (VER04k section 1.2): W0101 in a v0 document.
            Some(LayoutTokenKind::Word("ensures")) => {
                self.require_v1()?;
                self.parse_ensures_decl().map(Declaration::Ensures)
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        };
        self.leave_depth();
        result
    }

    // ── Declarations ────────────────────────────────────────────────────────

    fn parse_model_decl(&mut self) -> Result<ModelDecl<'a>, ParserError> {
        let start_tok = self.expect_word("model")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;

        let mut params = Vec::new();
        let target_type;
        let mut body = None;

        if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("="))) {
            self.advance()?;
            target_type = self.parse_type()?;
            self.expect_newline()?;
        } else if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("("))) {
            self.advance()?;
            if !matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(")"))) {
                params = self.parse_parameters()?;
            }
            self.expect_symbol(")")?;
            self.expect_symbol("->")?;
            target_type = self.parse_type()?;
            self.expect_symbol("=")?;
            body = Some(self.parse_expression()?);
            self.expect_newline()?;
        } else {
            return Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            ));
        }

        let end_span = body
            .as_ref()
            .map(|b| self.expr_span(b))
            .unwrap_or(self.type_span(&target_type));

        Ok(ModelDecl {
            name,
            params,
            target_type,
            body,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_error_decl(&mut self) -> Result<ErrorDecl<'a>, ParserError> {
        let start_tok = self.expect_word("error")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol("=")?;

        let mut variants = Vec::new();
        let (first_var, mut end_span) = self.expect_name()?;
        variants.push(first_var);

        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("|"))) {
            self.advance()?;
            let (v, sp) = self.expect_name()?;
            variants.push(v);
            end_span = sp;
        }
        self.expect_newline()?;

        Ok(ErrorDecl {
            name,
            variants,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_op_decl(&mut self) -> Result<OpDecl<'a>, ParserError> {
        let start_tok = self.expect_word("op")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;

        let mut static_params = Vec::new();
        if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("<"))) {
            self.advance()?;
            loop {
                let (sp_name, sp_start) = self.expect_name()?;
                self.expect_symbol(":")?;
                let (sp_type, sp_end) = self.expect_name()?;
                static_params.push(StaticParam {
                    name: sp_name,
                    type_name: sp_type,
                    span: (sp_start.0, sp_end.1),
                });
                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                    self.advance()?;
                } else {
                    break;
                }
            }
            self.expect_symbol(">")?;
        }

        self.expect_symbol("(")?;
        let mut params = Vec::new();
        if !matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(")"))) {
            params = self.parse_op_parameters()?;
        }
        self.expect_symbol(")")?;
        self.expect_symbol("->")?;

        let return_type = self.parse_type()?;
        let mut is_fresh = false;
        let mut end_span = self.type_span(&return_type);

        if matches!(self.peek_kind(), Some(LayoutTokenKind::Word("fresh"))) {
            let tok = self.advance()?.unwrap();
            is_fresh = true;
            end_span = self.token_span_range(tok);
        }
        self.expect_newline()?;

        Ok(OpDecl {
            name,
            static_params,
            params,
            return_type,
            is_fresh,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_example_decl(&mut self) -> Result<ExampleDecl<'a>, ParserError> {
        let start_tok = self.expect_word("example")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;

        let body = self.parse_expression()?;
        self.expect_newline()?;
        self.expect_dedent()?;

        let end_span = self.expr_span(&body);
        Ok(ExampleDecl {
            name,
            body,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_claim_decl(&mut self) -> Result<ClaimDecl<'a>, ParserError> {
        let start_tok = self.expect_word("claim")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;

        let body = self.parse_expression()?;
        self.expect_newline()?;
        self.expect_dedent()?;

        let end_span = self.expr_span(&body);
        Ok(ClaimDecl {
            name,
            body,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_assumptions_decl(&mut self) -> Result<AssumptionsDecl<'a>, ParserError> {
        let start_tok = self.expect_word("assume")?;
        let start_span = self.token_span_range(start_tok);

        match self.peek_kind() {
            Some(LayoutTokenKind::Word("platform")) => {
                self.advance()?;
                self.expect_symbol(":")?;
                self.expect_newline()?;
                self.expect_indent()?;

                let mut rows = Vec::new();
                if matches!(self.peek_kind(), Some(LayoutTokenKind::Word("none"))) {
                    self.advance()?;
                    self.expect_newline()?;
                } else {
                    while let Some(tok) = self.peek() {
                        if matches!(tok.kind, LayoutTokenKind::Dedent) {
                            break;
                        }
                        rows.push(self.parse_qualified_name()?);
                        self.expect_newline()?;
                    }
                }
                self.expect_dedent()?;
                let end_pos = rows.last().map(|r| r.span.1).unwrap_or(start_span.1);
                Ok(AssumptionsDecl::Platform {
                    rows,
                    span: (start_span.0, end_pos),
                })
            }
            Some(LayoutTokenKind::Word("environment")) => {
                self.advance()?;
                self.expect_symbol(":")?;
                self.expect_newline()?;
                self.expect_indent()?;

                let mut rows = Vec::new();
                if matches!(self.peek_kind(), Some(LayoutTokenKind::Word("none"))) {
                    self.advance()?;
                    self.expect_newline()?;
                } else {
                    while let Some(tok) = self.peek() {
                        if matches!(tok.kind, LayoutTokenKind::Dedent) {
                            break;
                        }
                        let (name, sp_start) = self.expect_name()?;
                        self.expect_symbol(":")?;
                        let expr = self.parse_expression()?;
                        self.expect_newline()?;
                        let sp_end = self.expr_span(&expr);
                        rows.push(EnvironmentRow {
                            name,
                            expr,
                            span: (sp_start.0, sp_end.1),
                        });
                    }
                }
                self.expect_dedent()?;
                let end_pos = rows.last().map(|r| r.span.1).unwrap_or(start_span.1);
                Ok(AssumptionsDecl::Environment {
                    rows,
                    span: (start_span.0, end_pos),
                })
            }
            _ => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
        }
    }

    fn parse_labels_decl(&mut self) -> Result<LabelsDecl<'a>, ParserError> {
        let start_tok = self.expect_word("labels")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;

        let mut rows = Vec::new();
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, LayoutTokenKind::Dedent) {
                break;
            }
            let qname = self.parse_qualified_name()?;
            let is_secret = match self.peek_kind() {
                Some(LayoutTokenKind::Word("public")) => {
                    self.advance()?;
                    false
                }
                Some(LayoutTokenKind::Word("secret")) => {
                    self.advance()?;
                    true
                }
                _ => {
                    return Err(ParserError::new(
                        ParserErrorKind::UnexpectedToken,
                        self.current_raw_span(),
                    ))
                }
            };
            self.expect_newline()?;
            let end_pos = qname.span.1;
            rows.push(LabelRow {
                name: qname,
                is_secret,
                span: (start_span.0, end_pos),
            });
        }
        self.expect_dedent()?;

        let end_pos = rows.last().map(|r| r.span.1).unwrap_or(start_span.1);
        Ok(LabelsDecl {
            name,
            rows,
            span: (start_span.0, end_pos),
        })
    }

    fn parse_input_decl(&mut self) -> Result<InputDecl<'a>, ParserError> {
        let start_tok = self.expect_word("input")?;
        let start_span = self.token_span_range(start_tok);
        let target = self.parse_qualified_name()?;
        if target.segments.len() < 2 {
            // Must be QNAME
            return Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                Some(target.span),
            ));
        }
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;

        self.expect_word("hostile")?;
        self.expect_newline()?;

        let mut rejections = Vec::new();
        while matches!(self.peek_kind(), Some(LayoutTokenKind::Word("rejects"))) {
            let rej_start = self.advance()?.unwrap();
            let rej_span_start = self.token_span_range(rej_start);
            let condition = self.parse_expression()?;
            self.expect_word("with")?;
            let err_qname = self.parse_qualified_name()?;
            if err_qname.segments.len() < 2 {
                return Err(ParserError::new(
                    ParserErrorKind::UnexpectedToken,
                    Some(err_qname.span),
                ));
            }
            self.expect_newline()?;
            rejections.push(Rejection {
                condition,
                error_variant: err_qname.clone(),
                span: (rej_span_start.0, err_qname.span.1),
            });
        }

        self.expect_word("ensures")?;
        let mut ensures = Vec::new();
        let (first_ens, first_ens_span) = self.expect_name()?;
        ensures.push(first_ens);
        let mut last_span = first_ens_span;
        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
            self.advance()?;
            let (ens, sp) = self.expect_name()?;
            ensures.push(ens);
            last_span = sp;
        }
        self.expect_newline()?;
        self.expect_dedent()?;

        Ok(InputDecl {
            target,
            is_hostile: true,
            rejections,
            ensures,
            span: (start_span.0, last_span.1),
        })
    }

    fn parse_bound_decl(&mut self) -> Result<BoundDecl<'a>, ParserError> {
        let start_tok = self.expect_word("bound")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;

        let mut rows = Vec::new();
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, LayoutTokenKind::Dedent) {
                break;
            }
            let (var_name, sp_start) = self.expect_name()?;
            // WX-H: `output` is a contextual metric, legal in a v1 document
            // only (VER04j sections 1.2 and 6.6).
            if var_name == "output" {
                self.require_v1()?;
            }
            self.expect_symbol("<=")?;
            let bound_expr = self.parse_expression()?;
            self.expect_newline()?;
            let sp_end = self.expr_span(&bound_expr);
            rows.push(BoundRow {
                name: var_name,
                bound: bound_expr,
                span: (sp_start.0, sp_end.1),
            });
        }
        self.expect_dedent()?;

        let end_pos = rows.last().map(|r| r.span.1).unwrap_or(start_span.1);
        Ok(BoundDecl {
            name,
            rows,
            span: (start_span.0, end_pos),
        })
    }

    // ── WX-A declarations (VER04j section 4.2) ─────────────────────────────

    /// A WX-A construct was found: legal only in a v1 document (W0101).
    fn require_v1(&self) -> Result<(), ParserError> {
        if self.version == Version::V1 {
            Ok(())
        } else {
            Err(ParserError::new(
                ParserErrorKind::V1ConstructInV0,
                self.current_raw_span(),
            ))
        }
    }

    /// `field { "," field }` up to (not including) the closing parenthesis,
    /// `1 <= n <= 32`, names unique.
    ///
    /// ```text
    /// tag: U8, constructed: Bool, number: U32, value: Bytes
    /// ```
    fn parse_fields(&mut self) -> Result<Vec<FieldDecl<'a>>, ParserError> {
        let mut fields: Vec<FieldDecl<'a>> = Vec::new();
        loop {
            self.count_node()?;
            let (name, start) = self.expect_name()?;
            self.expect_symbol(":")?;
            let ty = self.parse_type()?;
            let span = (start.0, self.type_span(&ty).1);
            if fields.iter().any(|f| f.name == name) {
                return Err(ParserError::new(ParserErrorKind::DuplicateName, Some(span)));
            }
            if fields.len() == MAX_FIELDS {
                return Err(ParserError::new(ParserErrorKind::UnexpectedToken, Some(span)));
            }
            fields.push(FieldDecl { name, ty, span });
            if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                self.advance()?;
            } else {
                return Ok(fields);
            }
        }
    }

    /// `record Name(f1: T1, ..., fn: Tn)`
    fn parse_record_decl(&mut self) -> Result<RecordDecl<'a>, ParserError> {
        let start_tok = self.expect_word("record")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol("(")?;
        let fields = self.parse_fields()?;
        let close = self.expect_symbol(")")?;
        let end = self.token_span_range(close);
        self.expect_newline()?;
        Ok(RecordDecl {
            name,
            fields,
            span: (start_span.0, end.1),
        })
    }

    /// `variant Name = C1(fields) | C2 | ...` (at least two constructors).
    fn parse_variant_decl(&mut self) -> Result<VariantDecl<'a>, ParserError> {
        let start_tok = self.expect_word("variant")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol("=")?;
        let mut ctors: Vec<CtorDecl<'a>> = Vec::new();
        loop {
            self.count_node()?;
            let (cname, cstart) = self.expect_name()?;
            let mut fields = Vec::new();
            let mut end = cstart;
            if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("("))) {
                self.advance()?;
                fields = self.parse_fields()?;
                let close = self.expect_symbol(")")?;
                end = self.token_span_range(close);
            }
            if ctors.iter().any(|c| c.name == cname) {
                return Err(ParserError::new(
                    ParserErrorKind::DuplicateName,
                    Some((cstart.0, end.1)),
                ));
            }
            if ctors.len() == MAX_CTORS {
                return Err(ParserError::new(
                    ParserErrorKind::UnexpectedToken,
                    Some((cstart.0, end.1)),
                ));
            }
            ctors.push(CtorDecl {
                name: cname,
                fields,
                span: (cstart.0, end.1),
            });
            if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("|"))) {
                self.advance()?;
            } else {
                break;
            }
        }
        // A sum type needs a choice: a single constructor is a record.
        if ctors.len() < 2 {
            return Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            ));
        }
        self.expect_newline()?;
        let end = ctors.last().map_or(start_span, |c| c.span);
        Ok(VariantDecl {
            name,
            ctors,
            span: (start_span.0, end.1),
        })
    }

    // ── WX-E declarations (VER04j section 5.2) ─────────────────────────────

    /// `source "<path>"`'s operand: a quoted text token that is terminated and
    /// obeys [`valid_table_path`] (W0708; an unterminated quote is W0001).
    fn expect_table_path(&mut self) -> Result<&'a str, ParserError> {
        match self.peek() {
            Some(tok) => match (tok.kind, tok.span) {
                (LayoutTokenKind::Text(text), TokenSpan::Real { start, end }) => {
                    let span = self.token_span_range(tok);
                    self.advance()?;
                    // The lexer keeps an unterminated quote as a Text token whose
                    // range lacks the closing quote.
                    if end - start != text.len() + 2 {
                        return Err(ParserError::new(ParserErrorKind::UnexpectedToken, Some(span)));
                    }
                    if !valid_table_path(text) {
                        return Err(ParserError::new(ParserErrorKind::TableIntegrity, Some(span)));
                    }
                    Ok(text)
                }
                _ => Err(ParserError::new(
                    ParserErrorKind::UnexpectedToken,
                    self.current_raw_span(),
                )),
            },
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    /// `sha256 b"<hex>"`'s operand: exactly 64 lowercase hex digits (32 bytes).
    fn expect_table_digest(&mut self) -> Result<&'a str, ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Bytes(raw)) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);
                if raw.len() < 3 || !raw.starts_with("b\"") || !raw.ends_with('"') {
                    return Err(ParserError::new(ParserErrorKind::UnexpectedToken, Some(span)));
                }
                let hex = &raw[2..raw.len() - 1];
                if hex.len() != 64 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                    return Err(ParserError::new(ParserErrorKind::TableIntegrity, Some(span)));
                }
                Ok(hex)
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    /// `rows N`'s operand: a plain decimal literal (no suffix, no sign, no
    /// leading zero) that fits a `u64`.
    fn expect_row_count(&mut self) -> Result<u64, ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Number(n)) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);
                let plain = !n.is_empty()
                    && n.bytes().all(|b| b.is_ascii_digit())
                    && (n == "0" || !n.starts_with('0'));
                match (plain, n.parse::<u64>()) {
                    (true, Ok(v)) => Ok(v),
                    _ => Err(ParserError::new(ParserErrorKind::UnexpectedToken, Some(span))),
                }
            }
            Some(_) => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
            None => Err(ParserError::new(ParserErrorKind::UnexpectedEof, None)),
        }
    }

    /// `vectors NAME:` block (see [`VectorsDecl`]); the five row words are
    /// contextual and appear once each, in this order.
    fn parse_vectors_decl(&mut self) -> Result<VectorsDecl<'a>, ParserError> {
        let start_tok = self.expect_word("vectors")?;
        let start_span = self.token_span_range(start_tok);
        let (name, _) = self.expect_name()?;
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;

        self.expect_word("source")?;
        let source = self.expect_table_path()?;
        self.expect_newline()?;
        self.expect_word("sha256")?;
        let sha256 = self.expect_table_digest()?;
        self.expect_newline()?;
        self.expect_word("rows")?;
        let rows = self.expect_row_count()?;
        self.expect_newline()?;

        // At least one column; none may repeat a name.
        let mut columns: Vec<ColumnDecl<'a>> = Vec::new();
        loop {
            self.count_node()?;
            let kw = self.expect_word("column")?;
            let kw_span = self.token_span_range(kw);
            let (cname, _) = self.expect_name()?;
            self.expect_symbol(":")?;
            let ty = self.parse_type()?;
            self.expect_newline()?;
            let span = (kw_span.0, self.type_span(&ty).1);
            if columns.iter().any(|c| c.name == cname) {
                return Err(ParserError::new(ParserErrorKind::DuplicateName, Some(span)));
            }
            if columns.len() == MAX_FIELDS {
                return Err(ParserError::new(ParserErrorKind::UnexpectedToken, Some(span)));
            }
            columns.push(ColumnDecl {
                name: cname,
                ty,
                span,
            });
            if !matches!(self.peek_kind(), Some(LayoutTokenKind::Word("column"))) {
                break;
            }
        }

        self.expect_word("holds")?;
        let holds = self.parse_expression()?;
        self.expect_newline()?;
        let end = self.expr_span(&holds);
        self.expect_dedent()?;
        Ok(VectorsDecl {
            name,
            source,
            sha256,
            rows,
            columns,
            holds,
            span: (start_span.0, end.1),
        })
    }

    // ── Parameters & Types ──────────────────────────────────────────────────

    fn parse_parameters(&mut self) -> Result<Vec<Parameter<'a>>, ParserError> {
        self.parse_parameter_list(false)
    }

    /// The parameters of an `op` line: the only place the `inout` mode exists.
    fn parse_op_parameters(&mut self) -> Result<Vec<Parameter<'a>>, ParserError> {
        self.parse_parameter_list(true)
    }

    fn parse_parameter_list(&mut self, allow_inout: bool) -> Result<Vec<Parameter<'a>>, ParserError> {
        let mut params = Vec::new();
        params.push(self.parse_parameter(allow_inout)?);
        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
            self.advance()?;
            params.push(self.parse_parameter(allow_inout)?);
        }
        Ok(params)
    }

    /// Is the word `inout`, at the cursor, the WX-G parameter mode?
    ///
    /// `inout` is contextual (VER04k section 4.4): it is a mode only when it is
    /// immediately followed by the start of a type, a NAME or `(`.  `x: inout`
    /// followed by `,`, `)` or `|` is a type alias that happens to be called
    /// `inout`, exactly as before WX-G.
    fn at_inout_mode(&self) -> bool {
        matches!(self.peek_kind(), Some(LayoutTokenKind::Word("inout")))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| t.kind),
                Some(LayoutTokenKind::Word(_)) | Some(LayoutTokenKind::Symbol("("))
            )
    }

    fn parse_parameter(&mut self, allow_inout: bool) -> Result<Parameter<'a>, ParserError> {
        self.count_node()?;
        let (name, start_span) = self.expect_name()?;
        self.expect_symbol(":")?;
        let mut inout = false;
        if allow_inout && self.at_inout_mode() {
            // A v0 document has no such mode (no valid v0 text has two NAMEs
            // after the colon): W0101, never a silent acceptance.
            self.require_v1()?;
            self.advance()?;
            inout = true;
        }
        let param_type = self.parse_type()?;
        let mut refinement = None;
        let mut end_span = self.type_span(&param_type);

        if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("|"))) {
            self.advance()?;
            let ref_expr = self.parse_expression()?;
            end_span = self.expr_span(&ref_expr);
            refinement = Some(ref_expr);
        }

        Ok(Parameter {
            name,
            param_type,
            refinement,
            inout,
            span: (start_span.0, end_span.1),
        })
    }

    /// `ensures NAME: NEWLINE INDENT row { row } DEDENT` (VER04k section 4.4).
    fn parse_ensures_decl(&mut self) -> Result<EnsuresDecl<'a>, ParserError> {
        let start_tok = self.expect_word("ensures")?;
        let start_span = self.token_span_range(start_tok);
        let (op, _) = self.expect_name()?;
        self.expect_symbol(":")?;
        self.expect_newline()?;
        self.expect_indent()?;
        let mut rows = Vec::new();
        let mut end;
        loop {
            let row = self.parse_expression()?;
            end = self.expr_span(&row);
            self.expect_newline()?;
            rows.push(row);
            // The cap of 32 rows is the checker's (W0602); the parser only
            // bounds what it is willing to hold by the node budget.
            if matches!(self.peek_kind(), Some(LayoutTokenKind::Dedent)) {
                break;
            }
        }
        self.expect_dedent()?;
        Ok(EnsuresDecl {
            op,
            rows,
            span: (start_span.0, end.1),
        })
    }

    fn parse_type(&mut self) -> Result<TypeExpr<'a>, ParserError> {
        self.count_node()?;
        self.enter_depth()?;
        let result = match self.peek_kind() {
            Some(LayoutTokenKind::Word("Bytes")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                let mut width = None;
                let mut end_span = start_span;

                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("<"))) {
                    self.advance()?;
                    if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(">"))) {
                        // Empty width <> is forbidden!
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            self.current_raw_span(),
                        ));
                    }
                    let w = self.parse_width()?;
                    let gt = self.expect_symbol(">")?;
                    end_span = self.token_span_range(gt);
                    width = Some(w);
                }
                Ok(TypeExpr::Bytes {
                    width,
                    span: (start_span.0, end_span.1),
                })
            }
            Some(LayoutTokenKind::Word("List")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("<")?;
                let inner = Box::new(self.parse_type()?);
                let gt = self.expect_symbol(">")?;
                let end_span = self.token_span_range(gt);
                Ok(TypeExpr::List {
                    inner,
                    span: (start_span.0, end_span.1),
                })
            }
            Some(LayoutTokenKind::Word("Option")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("<")?;
                let inner = Box::new(self.parse_type()?);
                let gt = self.expect_symbol(">")?;
                let end_span = self.token_span_range(gt);
                Ok(TypeExpr::Option {
                    inner,
                    span: (start_span.0, end_span.1),
                })
            }
            Some(LayoutTokenKind::Word("Result")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("<")?;
                let ok_type = Box::new(self.parse_type()?);
                self.expect_symbol(",")?;
                let (err_name, _) = self.expect_name()?;
                let gt = self.expect_symbol(">")?;
                let end_span = self.token_span_range(gt);
                Ok(TypeExpr::Result {
                    ok_type,
                    err_name,
                    span: (start_span.0, end_span.1),
                })
            }
            Some(LayoutTokenKind::Word(name)) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);
                Ok(TypeExpr::Named { name, span })
            }
            // WX-A tuple type `(T1, ..., Tn)`, `2 <= n <= 8`.
            Some(LayoutTokenKind::Symbol("(")) => {
                self.require_v1()?;
                let open = self.advance()?.unwrap();
                let start_span = self.token_span_range(open);
                let mut elems = vec![self.parse_type()?];
                while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                    self.advance()?;
                    elems.push(self.parse_type()?);
                }
                let close = self.expect_symbol(")")?;
                let end_span = self.token_span_range(close);
                if elems.len() < 2 || elems.len() > MAX_TUPLE {
                    return Err(ParserError::new(
                        ParserErrorKind::UnexpectedToken,
                        Some((start_span.0, end_span.1)),
                    ));
                }
                Ok(TypeExpr::Tuple {
                    elems,
                    span: (start_span.0, end_span.1),
                })
            }
            _ => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
        };
        self.leave_depth();
        result
    }

    fn parse_width(&mut self) -> Result<Width<'a>, ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Number(num_str)) => {
                let val: u64 = num_str.parse().map_err(|_| {
                    ParserError::new(ParserErrorKind::UnexpectedToken, self.current_raw_span())
                })?;
                self.advance()?;
                Ok(Width::Literal(val))
            }
            Some(LayoutTokenKind::Word(name)) => {
                self.advance()?;
                Ok(Width::Named(name))
            }
            _ => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
        }
    }

    fn parse_qualified_name(&mut self) -> Result<QualifiedName<'a>, ParserError> {
        let (first, start_span) = self.expect_name()?;
        let mut segments = vec![first];
        let mut end_span = start_span;

        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("."))) {
            let dot_tok = self.peek().unwrap();
            let dot_span = self.token_span_range(dot_tok);
            if dot_span.0 != end_span.1 {
                break;
            }
            if let Some(next_tok) = self.tokens.get(self.pos + 1) {
                let next_span = self.token_span_range(next_tok);
                if next_span.0 != dot_span.1 || !matches!(next_tok.kind, LayoutTokenKind::Word(_)) {
                    break;
                }
            } else {
                break;
            }

            self.advance()?;
            let (next_seg, next_span) = self.expect_name()?;
            segments.push(next_seg);
            end_span = next_span;
        }

        Ok(QualifiedName {
            segments,
            span: (start_span.0, end_span.1),
        })
    }

    // ── Expressions ─────────────────────────────────────────────────────────

    fn parse_expression(&mut self) -> Result<Expression<'a>, ParserError> {
        self.count_node()?;
        self.enter_depth()?;
        let result = match self.peek_kind() {
            Some(LayoutTokenKind::Word("forall")) | Some(LayoutTokenKind::Word("exists")) => {
                self.parse_quantified()
            }
            Some(LayoutTokenKind::Word("if")) => self.parse_conditional(),
            Some(LayoutTokenKind::Word("match")) => self.parse_match(),
            _ => self.parse_implication(),
        };
        self.leave_depth();
        result
    }

    fn parse_quantified(&mut self) -> Result<Expression<'a>, ParserError> {
        let kw_tok = self.advance()?.unwrap();
        let start_span = self.token_span_range(kw_tok);
        let is_forall = match kw_tok.kind {
            LayoutTokenKind::Word("forall") => true,
            LayoutTokenKind::Word("exists") => false,
            _ => unreachable!(),
        };

        let binders = self.parse_parameters()?;

        // Expect dot token
        let dot_tok = self.expect_symbol(".")?;
        let dot_span = match dot_tok.span {
            TokenSpan::Real { start, end } => (start, end),
            TokenSpan::Synthetic { at } => (at, at),
        };

        // Grammar contract: @space_after_next DOT
        // There must be at least one byte gap between the dot token and next token.
        if let Some(next_tok) = self.peek() {
            let next_start = match next_tok.span {
                TokenSpan::Real { start, .. } => start,
                TokenSpan::Synthetic { at } => at,
            };
            if next_start <= dot_span.1 {
                return Err(ParserError::new(
                    ParserErrorKind::UnexpectedToken,
                    self.current_raw_span(),
                ));
            }
        } else {
            return Err(ParserError::new(ParserErrorKind::UnexpectedEof, None));
        }

        let body = Box::new(self.parse_expression()?);
        let end_span = self.expr_span(&body);

        Ok(Expression::Quantified {
            is_forall,
            binders,
            body,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_conditional(&mut self) -> Result<Expression<'a>, ParserError> {
        let if_tok = self.expect_word("if")?;
        let start_span = self.token_span_range(if_tok);

        let condition = Box::new(self.parse_expression()?);
        self.expect_word("then")?;
        let then_branch = Box::new(self.parse_expression()?);
        self.expect_word("else")?;
        let else_branch = Box::new(self.parse_expression()?);
        let end_span = self.expr_span(&else_branch);

        Ok(Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_match(&mut self) -> Result<Expression<'a>, ParserError> {
        let match_tok = self.expect_word("match")?;
        let start_span = self.token_span_range(match_tok);

        let target = Box::new(self.parse_expression()?);
        self.expect_word("with")?;

        let mut arms = Vec::new();
        arms.push(self.parse_match_arm()?);

        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("|"))) {
            self.advance()?;
            arms.push(self.parse_match_arm()?);
        }

        let end_span = arms
            .last()
            .map(|a| a.span)
            .unwrap_or((start_span.0, start_span.1));

        Ok(Expression::Match {
            target,
            arms,
            span: (start_span.0, end_span.1),
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm<'a>, ParserError> {
        self.count_node()?;
        let (pattern, pattern_span) = self.parse_pattern()?;
        self.expect_symbol("->")?;
        let body = self.parse_expression()?;
        let end_span = self.expr_span(&body);

        Ok(MatchArm {
            pattern,
            body,
            span: (pattern_span.0, end_span.1),
        })
    }

    /// `pattern` of VER04j section 4.3.  A v0 document accepts exactly
    /// `None | Some(NAME) | Ok(NAME) | Err(NAME)`; every richer form is W0101
    /// there and a syntax error everywhere else it was one before.
    fn parse_pattern(&mut self) -> Result<(Pattern<'a>, (usize, usize)), ParserError> {
        self.parse_pattern_at(1)
    }

    fn parse_pattern_at(
        &mut self,
        depth: usize,
    ) -> Result<(Pattern<'a>, (usize, usize)), ParserError> {
        self.count_node()?;
        if depth > MAX_PATTERN_DEPTH {
            return Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            ));
        }
        match self.peek_kind() {
            Some(LayoutTokenKind::Word("None")) => {
                let tok = self.advance()?.unwrap();
                let sp = self.token_span_range(tok);
                Ok((Pattern::None, sp))
            }
            Some(LayoutTokenKind::Word(w @ ("Some" | "Ok" | "Err"))) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("(")?;
                let inner = if self.version == Version::V0 {
                    // v0: a binder NAME only (it may spell `_`, `Some`, ...).
                    match self.peek_kind() {
                        Some(LayoutTokenKind::Symbol("("))
                        | Some(LayoutTokenKind::Word("true"))
                        | Some(LayoutTokenKind::Word("false")) => {
                            return Err(ParserError::new(
                                ParserErrorKind::V1ConstructInV0,
                                self.current_raw_span(),
                            ))
                        }
                        _ => {}
                    }
                    let (name, sp) = self.expect_name()?;
                    if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("(" | "."))) {
                        return Err(ParserError::new(
                            ParserErrorKind::V1ConstructInV0,
                            self.current_raw_span(),
                        ));
                    }
                    Pattern::Ctor {
                        name: QualifiedName {
                            segments: vec![name],
                            span: sp,
                        },
                        args: Vec::new(),
                    }
                } else {
                    self.parse_pattern_at(depth + 1)?.0
                };
                let close = self.expect_symbol(")")?;
                let end_span = self.token_span_range(close);
                let inner = Box::new(inner);
                let pat = match w {
                    "Some" => Pattern::Some(inner),
                    "Ok" => Pattern::Ok(inner),
                    _ => Pattern::Err(inner),
                };
                Ok((pat, (start_span.0, end_span.1)))
            }
            Some(LayoutTokenKind::Word("true" | "false")) => {
                self.require_v1()?;
                let tok = self.advance()?.unwrap();
                let sp = self.token_span_range(tok);
                let is_true = matches!(tok.kind, LayoutTokenKind::Word("true"));
                Ok((Pattern::Bool(is_true), sp))
            }
            Some(LayoutTokenKind::Word("_")) if self.version == Version::V1 => {
                let tok = self.advance()?.unwrap();
                let sp = self.token_span_range(tok);
                Ok((Pattern::Wild, sp))
            }
            Some(LayoutTokenKind::Word(_)) => {
                self.require_v1()?;
                let name = self.parse_qualified_name()?;
                let start = name.span.0;
                let mut end = name.span.1;
                let mut args = Vec::new();
                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("("))) {
                    self.advance()?;
                    loop {
                        args.push(self.parse_pattern_at(depth + 1)?.0);
                        if args.len() > MAX_FIELDS {
                            return Err(ParserError::new(
                                ParserErrorKind::UnexpectedToken,
                                self.current_raw_span(),
                            ));
                        }
                        if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                            self.advance()?;
                        } else {
                            break;
                        }
                    }
                    let close = self.expect_symbol(")")?;
                    end = self.token_span_range(close).1;
                }
                Ok((Pattern::Ctor { name, args }, (start, end)))
            }
            Some(LayoutTokenKind::Symbol("(")) => {
                self.require_v1()?;
                let open = self.advance()?.unwrap();
                let start = self.token_span_range(open).0;
                let mut items = vec![self.parse_pattern_at(depth + 1)?.0];
                while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                    self.advance()?;
                    items.push(self.parse_pattern_at(depth + 1)?.0);
                }
                let close = self.expect_symbol(")")?;
                let end = self.token_span_range(close).1;
                if items.len() < 2 || items.len() > MAX_TUPLE {
                    return Err(ParserError::new(
                        ParserErrorKind::UnexpectedToken,
                        Some((start, end)),
                    ));
                }
                Ok((Pattern::Tuple(items), (start, end)))
            }
            _ => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
        }
    }

    // Implication: right associative =>
    fn parse_implication(&mut self) -> Result<Expression<'a>, ParserError> {
        let left = self.parse_disjunction()?;

        if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("=>"))) {
            self.advance()?;
            let right = self.parse_expression()?; // Right associative
            let sp_start = self.expr_span(&left).0;
            let sp_end = self.expr_span(&right).1;
            self.count_node()?;
            Ok(Expression::Binary {
                op: BinaryOp::Implies,
                left: Box::new(left),
                right: Box::new(right),
                span: (sp_start, sp_end),
            })
        } else {
            Ok(left)
        }
    }

    // Disjunction: left associative 'or'
    fn parse_disjunction(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_conjunction()?;

        while matches!(self.peek_kind(), Some(LayoutTokenKind::Word("or"))) {
            self.advance()?;
            let right = self.parse_conjunction()?;
            let sp_start = self.expr_span(&left).0;
            let sp_end = self.expr_span(&right).1;
            self.count_node()?;
            left = Expression::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span: (sp_start, sp_end),
            };
        }

        Ok(left)
    }

    // Conjunction: left associative 'and'
    fn parse_conjunction(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_comparison()?;

        while matches!(self.peek_kind(), Some(LayoutTokenKind::Word("and"))) {
            self.advance()?;
            let right = self.parse_comparison()?;
            let sp_start = self.expr_span(&left).0;
            let sp_end = self.expr_span(&right).1;
            self.count_node()?;
            left = Expression::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span: (sp_start, sp_end),
            };
        }

        Ok(left)
    }

    // Comparison: NON-associative (single comparison only)
    fn parse_comparison(&mut self) -> Result<Expression<'a>, ParserError> {
        let left = self.parse_sum()?;

        let op = match self.peek_kind() {
            Some(LayoutTokenKind::Symbol("==")) => Some(BinaryOp::Eq),
            Some(LayoutTokenKind::Symbol("!=")) => Some(BinaryOp::NotEq),
            Some(LayoutTokenKind::Symbol("<")) => Some(BinaryOp::Lt),
            Some(LayoutTokenKind::Symbol(">")) => Some(BinaryOp::Gt),
            Some(LayoutTokenKind::Symbol("<=")) => Some(BinaryOp::Le),
            Some(LayoutTokenKind::Symbol(">=")) => Some(BinaryOp::Ge),
            _ => None,
        };

        if let Some(bin_op) = op {
            self.advance()?;
            let right = self.parse_sum()?;
            let sp_start = self.expr_span(&left).0;
            let sp_end = self.expr_span(&right).1;

            // Reject chained comparisons (e.g. 1 < 2 < 3)
            if matches!(
                self.peek_kind(),
                Some(LayoutTokenKind::Symbol("=="))
                    | Some(LayoutTokenKind::Symbol("!="))
                    | Some(LayoutTokenKind::Symbol("<"))
                    | Some(LayoutTokenKind::Symbol(">"))
                    | Some(LayoutTokenKind::Symbol("<="))
                    | Some(LayoutTokenKind::Symbol(">="))
            ) {
                return Err(ParserError::new(
                    ParserErrorKind::UnexpectedToken,
                    self.current_raw_span(),
                ));
            }

            self.count_node()?;
            Ok(Expression::Binary {
                op: bin_op,
                left: Box::new(left),
                right: Box::new(right),
                span: (sp_start, sp_end),
            })
        } else {
            Ok(left)
        }
    }

    // Sum: left associative + -
    fn parse_sum(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_product()?;

        while let Some(op) = match self.peek_kind() {
            Some(LayoutTokenKind::Symbol("+")) => Some(BinaryOp::Add),
            Some(LayoutTokenKind::Symbol("-")) => Some(BinaryOp::Sub),
            _ => None,
        } {
            self.advance()?;
            let right = self.parse_product()?;
            let sp_start = self.expr_span(&left).0;
            let sp_end = self.expr_span(&right).1;
            self.count_node()?;
            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span: (sp_start, sp_end),
            };
        }

        Ok(left)
    }

    // Product: left associative *
    fn parse_product(&mut self) -> Result<Expression<'a>, ParserError> {
        let mut left = self.parse_unary()?;

        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("*"))) {
            self.advance()?;
            let right = self.parse_unary()?;
            let sp_start = self.expr_span(&left).0;
            let sp_end = self.expr_span(&right).1;
            self.count_node()?;
            left = Expression::Binary {
                op: BinaryOp::Mul,
                left: Box::new(left),
                right: Box::new(right),
                span: (sp_start, sp_end),
            };
        }

        Ok(left)
    }

    // Unary: not, -
    fn parse_unary(&mut self) -> Result<Expression<'a>, ParserError> {
        self.count_node()?;
        self.enter_depth()?;
        let result = match self.peek_kind() {
            Some(LayoutTokenKind::Word("not")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                let operand = Box::new(self.parse_unary()?);
                let end_span = self.expr_span(&operand);
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    operand,
                    span: (start_span.0, end_span.1),
                })
            }
            Some(LayoutTokenKind::Symbol("-")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                // VER04j section 2.2: `-128i8` is legal although `128i8` is not.
                // The minimum of a signed type is the one literal whose
                // magnitude is out of range, accepted only as the *direct*
                // operand of unary minus; it is folded into one literal here.
                if let Some(LayoutTokenKind::Number(text)) = self.peek_kind() {
                    let num_tok = self.peek().unwrap();
                    let num_span = self.token_span_range(num_tok);
                    if let Ok(Literal::Fixed(ty, v)) = self.parse_number(text, num_span, true) {
                        if ty.signed() && v == ty.max() + 1 {
                            self.advance()?;
                            self.leave_depth();
                            return Ok(Expression::Literal {
                                lit: Literal::Fixed(ty, -v),
                                span: (start_span.0, num_span.1),
                            });
                        }
                    }
                }
                let operand = Box::new(self.parse_unary()?);
                let end_span = self.expr_span(&operand);
                Ok(Expression::Unary {
                    op: UnaryOp::Neg,
                    operand,
                    span: (start_span.0, end_span.1),
                })
            }
            _ => self.parse_primary(),
        };
        self.leave_depth();
        result
    }

    // Primary: literal, list_literal, reference, ( expression )
    fn parse_primary(&mut self) -> Result<Expression<'a>, ParserError> {
        self.count_node()?;
        self.enter_depth()?;
        let result = match self.peek_kind() {
            // Parenthesized expression
            Some(LayoutTokenKind::Symbol("(")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                // Check if empty ()
                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(")"))) {
                    return Err(ParserError::new(
                        ParserErrorKind::UnexpectedToken,
                        self.current_raw_span(),
                    ));
                }
                let first = self.parse_expression()?;
                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                    // WX-A tuple literal `(e1, ..., en)`, `2 <= n <= 8`.
                    self.require_v1()?;
                    let mut elements = vec![first];
                    while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                        self.advance()?;
                        elements.push(self.parse_expression()?);
                    }
                    let close = self.expect_symbol(")")?;
                    let end_span = self.token_span_range(close);
                    if elements.len() > MAX_TUPLE {
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            Some((start_span.0, end_span.1)),
                        ));
                    }
                    Ok(Expression::Tuple {
                        elements,
                        span: (start_span.0, end_span.1),
                    })
                } else {
                    let inner = Box::new(first);
                    let close = self.expect_symbol(")")?;
                    let end_span = self.token_span_range(close);
                    Ok(Expression::Grouped {
                        inner,
                        span: (start_span.0, end_span.1),
                    })
                }
            }
            // List literal
            Some(LayoutTokenKind::Symbol("[")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                let mut elements = Vec::new();
                if !matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("]"))) {
                    elements.push(self.parse_expression()?);
                    while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                        self.advance()?;
                        elements.push(self.parse_expression()?);
                    }
                }
                let close = self.expect_symbol("]")?;
                let end_span = self.token_span_range(close);
                Ok(Expression::List {
                    elements,
                    span: (start_span.0, end_span.1),
                })
            }
            // Literals: true / false
            Some(LayoutTokenKind::Word("true")) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);
                Ok(Expression::Literal {
                    lit: Literal::Bool(true),
                    span,
                })
            }
            Some(LayoutTokenKind::Word("false")) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);
                Ok(Expression::Literal {
                    lit: Literal::Bool(false),
                    span,
                })
            }
            // Bytes literal: b"..."
            Some(LayoutTokenKind::Bytes(raw)) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);

                // Validate: must start with b" and end with ", length >= 3 (b""), and inner content must be valid hex pairs
                if raw.len() < 3 || !raw.starts_with("b\"") || !raw.ends_with('"') {
                    return Err(ParserError::new(
                        ParserErrorKind::UnexpectedToken,
                        Some(span),
                    ));
                }
                let hex_content = &raw[2..raw.len() - 1];
                if hex_content.len() % 2 != 0 {
                    return Err(ParserError::new(
                        ParserErrorKind::UnexpectedToken,
                        Some(span),
                    ));
                }
                for ch in hex_content.chars() {
                    if !matches!(ch, '0'..='9' | 'a'..='f') {
                        // Uppercase hex or invalid character is rejected
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            Some(span),
                        ));
                    }
                }
                Ok(Expression::Literal {
                    lit: Literal::Bytes(hex_content),
                    span,
                })
            }
            // Number literal: INT, the suffixed forms and HEX_LITERAL.
            Some(LayoutTokenKind::Number(num_str)) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);
                let lit = self.parse_number(num_str, span, false)?;
                Ok(Expression::Literal { lit, span })
            }
            // Reference: qualified_name [ < widths > ] [ ( [ args ] ) ]
            Some(LayoutTokenKind::Word(_)) => {
                let qname = self.parse_qualified_name()?;
                let mut widths = Vec::new();
                let mut args = None;
                let mut end_span = qname.span;

                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("<")))
                    && self.is_widths_ahead()?
                {
                    self.advance()?;
                    if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(">"))) {
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            self.current_raw_span(),
                        ));
                    }
                    widths.push(self.parse_width()?);
                    while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                        self.advance()?;
                        widths.push(self.parse_width()?);
                    }
                    let gt = self.expect_symbol(">")?;
                    end_span = self.token_span_range(gt);
                }

                if matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol("("))) {
                    self.advance()?;
                    let mut arg_list = Vec::new();
                    if !matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(")"))) {
                        arg_list.push(self.parse_expression()?);
                        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
                            self.advance()?;
                            arg_list.push(self.parse_expression()?);
                        }
                    }
                    let close = self.expect_symbol(")")?;
                    end_span = self.token_span_range(close);
                    args = Some(arg_list);
                }

                let start_pos = qname.span.0;
                Ok(Expression::Reference {
                    name: qname,
                    widths,
                    args,
                    span: (start_pos, end_span.1),
                })
            }
            _ => Err(ParserError::new(
                ParserErrorKind::UnexpectedToken,
                self.current_raw_span(),
            )),
        };
        self.leave_depth();
        self.parse_projections(result?)
    }

    /// `primary = atom { PROJ }` (VER04j section 4.3).  A `PROJ` is a `.` with
    /// no whitespace on either side followed by a decimal INT (no leading zero,
    /// no suffix) or a NAME.  A `.` followed by whitespace is the quantifier
    /// separator and never a projection.
    ///
    /// ```text
    /// f(x).0      Project { base: f(x), key: Index(0) }
    /// (a, b).1.v  Project { Project { .., Index(1) }, Field("v") }
    /// t. 0        not a projection (a syntax error further up)
    /// t.00 t.0u8  syntax errors
    /// ```
    fn parse_projections(&mut self, mut e: Expression<'a>) -> Result<Expression<'a>, ParserError> {
        loop {
            let Some(dot_tok) = self.peek() else {
                return Ok(e);
            };
            if !matches!(dot_tok.kind, LayoutTokenKind::Symbol(".")) {
                return Ok(e);
            }
            let dot = self.token_span_range(dot_tok);
            let Some(next_tok) = self.tokens.get(self.pos + 1) else {
                return Ok(e);
            };
            let next = self.token_span_range(next_tok);
            if dot.0 != self.expr_span(&e).1 || next.0 != dot.1 {
                return Ok(e);
            }
            let key = match next_tok.kind {
                LayoutTokenKind::Number(text) => {
                    self.require_v1()?;
                    let plain = !text.is_empty()
                        && text.bytes().all(|b| b.is_ascii_digit())
                        && (text == "0" || !text.starts_with('0'));
                    match text.parse::<usize>() {
                        Ok(i) if plain => ProjKey::Index(i),
                        _ => {
                            return Err(ParserError::new(
                                ParserErrorKind::UnexpectedToken,
                                Some(next),
                            ))
                        }
                    }
                }
                LayoutTokenKind::Word(name) => {
                    self.require_v1()?;
                    ProjKey::Field(name)
                }
                _ => return Ok(e),
            };
            self.count_node()?;
            self.advance()?;
            self.advance()?;
            let start = self.expr_span(&e).0;
            e = Expression::Project {
                base: Box::new(e),
                key,
                span: (start, next.1),
            };
        }
    }

    /// Parse the text of a `Number` token (VER04j section 2.2).
    ///
    /// ```text
    /// 5          INT                      -> Literal::Int
    /// 5u8 5u64   v0 suffixes              -> Literal::U8 / Literal::U64
    /// 5u16 5i8   v1 suffixes              -> Literal::Fixed (v0 document: W0101)
    /// 0x1d       hex, no suffix           -> Literal::Int (v1 only)
    /// 0x1du8     hex with suffix          -> as the decimal literal (v1 only)
    /// 256u8      out of range             -> W0101 in v1
    /// 128i8      magnitude 2^(w-1)        -> accepted only when `allow_min`
    ///                                        (the operand of unary minus)
    /// ```
    ///
    /// Leading zeros are a syntax error in decimal (`01u16`), exactly as the
    /// `INT` token has always forbidden them; hex may have leading zeros.
    fn parse_number(
        &self,
        text: &str,
        span: (usize, usize),
        allow_min: bool,
    ) -> Result<Literal<'a>, ParserError> {
        let syntax = || ParserError::new(ParserErrorKind::UnexpectedToken, Some(span));
        let out_of_range = || ParserError::new(ParserErrorKind::LiteralOutOfRange, Some(span));
        let v1_only = || {
            if self.version == Version::V1 {
                Ok(())
            } else {
                Err(ParserError::new(
                    ParserErrorKind::V1ConstructInV0,
                    Some(span),
                ))
            }
        };
        // Hex: `0x` hexdigits+ [suffix].  The lexer hands over every
        // alphanumeric after `0x`, so an uppercase digit or a bad suffix is
        // diagnosed here rather than silently split into two tokens.
        let (digits, suffix, radix) = if let Some(rest) = text.strip_prefix("0x") {
            v1_only()?;
            let n = rest
                .bytes()
                .take_while(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
                .count();
            if n == 0 {
                return Err(syntax());
            }
            (&rest[..n], &rest[n..], 16)
        } else {
            let n = text.bytes().take_while(u8::is_ascii_digit).count();
            let digits = &text[..n];
            if digits.len() > 1 && digits.starts_with('0') {
                return Err(syntax());
            }
            (digits, &text[n..], 10)
        };
        let ty = if suffix.is_empty() {
            None
        } else {
            Some(FixedTy::from_suffix(suffix).ok_or_else(syntax)?)
        };
        let magnitude = u128::from_str_radix(digits, radix);
        match ty {
            None => {
                let v: u64 = magnitude
                    .ok()
                    .and_then(|m| u64::try_from(m).ok())
                    .ok_or_else(|| {
                        if radix == 16 {
                            out_of_range()
                        } else {
                            syntax()
                        }
                    })?;
                Ok(Literal::Int(v))
            }
            Some(legacy @ (FixedTy::U8 | FixedTy::U64)) if self.version == Version::V0 => {
                // Legacy v0 behaviour, byte for byte: a failure is W0001.
                let m = magnitude.map_err(|_| syntax())?;
                if m > legacy.max() as u128 {
                    return Err(syntax());
                }
                Ok(if legacy == FixedTy::U8 {
                    Literal::U8(m as u8)
                } else {
                    Literal::U64(m as u64)
                })
            }
            Some(ty) => {
                v1_only()?;
                let m = magnitude.map_err(|_| out_of_range())?;
                let max = ty.max() as u128;
                let is_min = ty.signed() && m == max + 1;
                if m > max && !(allow_min && is_min) {
                    return Err(out_of_range());
                }
                let v = m as i128;
                Ok(match ty {
                    FixedTy::U8 => Literal::U8(v as u8),
                    FixedTy::U64 => Literal::U64(v as u64),
                    _ => Literal::Fixed(ty, v),
                })
            }
        }
    }

    fn expr_span(&self, expr: &Expression<'a>) -> (usize, usize) {
        match expr {
            Expression::Quantified { span, .. } => *span,
            Expression::Conditional { span, .. } => *span,
            Expression::Match { span, .. } => *span,
            Expression::Binary { span, .. } => *span,
            Expression::Unary { span, .. } => *span,
            Expression::Literal { span, .. } => *span,
            Expression::List { span, .. } => *span,
            Expression::Reference { span, .. } => *span,
            Expression::Grouped { span, .. } => *span,
            Expression::Tuple { span, .. } => *span,
            Expression::Project { span, .. } => *span,
        }
    }

    fn type_span(&self, ty: &TypeExpr<'a>) -> (usize, usize) {
        match ty {
            TypeExpr::Bytes { span, .. } => *span,
            TypeExpr::List { span, .. } => *span,
            TypeExpr::Option { span, .. } => *span,
            TypeExpr::Result { span, .. } => *span,
            TypeExpr::Named { span, .. } => *span,
            TypeExpr::Tuple { span, .. } => *span,
        }
    }
}

/// Parses an admitted source document and layout token stream into a Warrant AST.
pub fn parse_document<'a>(
    tokens: &'a [LayoutToken<'a>],
    doc: &'a SourceDocument<'a>,
    limits: ParserLimits,
) -> Result<WarrantFile<'a>, ParserError> {
    let mut parser = Parser::new(tokens, doc, limits)?;
    parser.parse_file()
}
