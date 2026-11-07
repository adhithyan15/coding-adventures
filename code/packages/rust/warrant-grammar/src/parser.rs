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
}

/// Parser error with stable diagnostic code W0001 and safe message.
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
        "W0001"
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

/// A parsed Warrant file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarrantFile<'a> {
    pub name: &'a str,
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
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter<'a> {
    pub name: &'a str,
    pub param_type: TypeExpr<'a>,
    pub refinement: Option<Expression<'a>>,
    pub span: (usize, usize),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm<'a> {
    pub pattern: Pattern<'a>,
    pub body: Expression<'a>,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern<'a> {
    None,
    Some(&'a str),
    Ok(&'a str),
    Err(&'a str),
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
            })
            .unwrap_or(start_span);

        Ok(WarrantFile {
            name: file_name,
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
            params = self.parse_parameters()?;
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

    // ── Parameters & Types ──────────────────────────────────────────────────

    fn parse_parameters(&mut self) -> Result<Vec<Parameter<'a>>, ParserError> {
        let mut params = Vec::new();
        params.push(self.parse_parameter()?);
        while matches!(self.peek_kind(), Some(LayoutTokenKind::Symbol(","))) {
            self.advance()?;
            params.push(self.parse_parameter()?);
        }
        Ok(params)
    }

    fn parse_parameter(&mut self) -> Result<Parameter<'a>, ParserError> {
        self.count_node()?;
        let (name, start_span) = self.expect_name()?;
        self.expect_symbol(":")?;
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
            span: (start_span.0, end_span.1),
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

    fn parse_pattern(&mut self) -> Result<(Pattern<'a>, (usize, usize)), ParserError> {
        match self.peek_kind() {
            Some(LayoutTokenKind::Word("None")) => {
                let tok = self.advance()?.unwrap();
                let sp = self.token_span_range(tok);
                Ok((Pattern::None, sp))
            }
            Some(LayoutTokenKind::Word("Some")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("(")?;
                let (name, _) = self.expect_name()?;
                let close = self.expect_symbol(")")?;
                let end_span = self.token_span_range(close);
                Ok((Pattern::Some(name), (start_span.0, end_span.1)))
            }
            Some(LayoutTokenKind::Word("Ok")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("(")?;
                let (name, _) = self.expect_name()?;
                let close = self.expect_symbol(")")?;
                let end_span = self.token_span_range(close);
                Ok((Pattern::Ok(name), (start_span.0, end_span.1)))
            }
            Some(LayoutTokenKind::Word("Err")) => {
                let tok = self.advance()?.unwrap();
                let start_span = self.token_span_range(tok);
                self.expect_symbol("(")?;
                let (name, _) = self.expect_name()?;
                let close = self.expect_symbol(")")?;
                let end_span = self.token_span_range(close);
                Ok((Pattern::Err(name), (start_span.0, end_span.1)))
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
                let inner = Box::new(self.parse_expression()?);
                let close = self.expect_symbol(")")?;
                let end_span = self.token_span_range(close);
                Ok(Expression::Grouped {
                    inner,
                    span: (start_span.0, end_span.1),
                })
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
            // Number literal: INT, U64_LITERAL, U8_LITERAL
            Some(LayoutTokenKind::Number(num_str)) => {
                let tok = self.advance()?.unwrap();
                let span = self.token_span_range(tok);

                if let Some(prefix) = num_str.strip_suffix("u64") {
                    if prefix.len() > 1 && prefix.starts_with('0') {
                        // Leading zero rejected
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            Some(span),
                        ));
                    }
                    let val: u64 = prefix.parse().map_err(|_| {
                        ParserError::new(ParserErrorKind::UnexpectedToken, Some(span))
                    })?;
                    Ok(Expression::Literal {
                        lit: Literal::U64(val),
                        span,
                    })
                } else if let Some(prefix) = num_str.strip_suffix("u8") {
                    if prefix.len() > 1 && prefix.starts_with('0') {
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            Some(span),
                        ));
                    }
                    let val: u8 = prefix.parse().map_err(|_| {
                        ParserError::new(ParserErrorKind::UnexpectedToken, Some(span))
                    })?;
                    Ok(Expression::Literal {
                        lit: Literal::U8(val),
                        span,
                    })
                } else {
                    if num_str.len() > 1 && num_str.starts_with('0') {
                        // Leading zero rejected (warrant.tokens INT = 0|[1-9][0-9]*)
                        return Err(ParserError::new(
                            ParserErrorKind::UnexpectedToken,
                            Some(span),
                        ));
                    }
                    let val: u64 = num_str.parse().map_err(|_| {
                        ParserError::new(ParserErrorKind::UnexpectedToken, Some(span))
                    })?;
                    Ok(Expression::Literal {
                        lit: Literal::Int(val),
                        span,
                    })
                }
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
        result
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
        }
    }

    fn type_span(&self, ty: &TypeExpr<'a>) -> (usize, usize) {
        match ty {
            TypeExpr::Bytes { span, .. } => *span,
            TypeExpr::List { span, .. } => *span,
            TypeExpr::Option { span, .. } => *span,
            TypeExpr::Result { span, .. } => *span,
            TypeExpr::Named { span, .. } => *span,
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
