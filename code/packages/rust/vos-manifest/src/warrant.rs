//! # What the Warrant tools say about a document
//!
//! Warrant text is parsed and type-checked only by `warrant-grammar`.  This module
//! calls it and reads the answer; it parses nothing itself.
//!
//! ## `claim_sha256` (VOS02 section 10.1, D-GD7)
//!
//! VER29 section 5.3 defines the identity of a claim: the SHA-256 of "the comment-free
//! token stream of the declaration, tokens joined by single spaces".  VER29 slice 2 is
//! not implemented, so the definition lives here until it is (backlog VOS-D056):
//!
//! ```text
//!   claim insert_safe:                         tokens:  claim insert_safe : forall b0 : Bool , ...
//!     forall b0: Bool, ... (                   joined:  "claim insert_safe : forall b0 : Bool , ..."
//!       a == b and                             digest:  SHA-256 of that string
//!       c == d)
//! ```
//!
//! * Comments never reach the token stream (the Warrant lexer drops them), so editing a
//!   comment changes no digest.
//! * `Indent`, `Dedent` and `Newline` are *layout*, not text, and are left out: wrapping
//!   a claim differently changes nothing the claim says.  Any change to a name, a
//!   literal, an operator or a bracket changes the digest.
//! * The tokens used are those inside the claim declaration's source span.
//!
//! Two claims with the same digest are the same text, so "this implementation carries
//! the contract's obligation" is decidable by comparing digests (R4), without anyone
//! reading Warrant.
//!
//! ## The facts a document gives the R6 checks
//!
//! [`analyze_document`] parses and type-checks the whole document and returns an
//! [`Analysis`]: every claim with its digest, the `assume` rows and where they start,
//! whether there is a `bound` row, and the *shape of `build`*: the VOS02 R6 structural
//! rule needs to know which fields of `Impl` the `build` model sets from the abstract
//! state `m` (its cone).

use std::collections::HashMap;

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::layout::{
    capture_layout, LayoutLimits, LayoutToken, LayoutTokenKind, TokenSpan,
};
use coding_adventures_warrant_grammar::parser::{
    parse_document, AssumptionsDecl, BinaryOp, Declaration, Expression, Literal, ModelDecl,
    ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use crate::digest::sha256_hex;

/// A claim and its identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimFact {
    /// The claim's name.
    pub name: String,
    /// `claim_sha256`: 64 lowercase hex characters.
    pub sha256: String,
}

/// Which `assume` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssumeKind {
    /// `assume platform:`
    Platform,
    /// `assume environment:`
    Environment,
}

/// An `assume` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssumeFact {
    /// Which one.
    pub kind: AssumeKind,
    /// True for `none` (no rows).
    pub is_none: bool,
    /// Byte offset of its start in the document.
    pub start: usize,
}

/// What the `build` model looks like, for the structural cone rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildShape {
    /// No `model build` (or no body: it is a bound operation).
    Missing,
    /// A body that is not an application of the `Impl` constructor.
    NotConstructor,
    /// `Impl(e0, ..., en)`: for each field argument, whether it mentions the model's
    /// parameter (the abstract state), i.e. whether the field is in the cone of `build`.
    Constructor {
        /// One entry per field, in order.
        in_cone: Vec<bool>,
    },
}

/// What the Warrant tools found in a document that type-checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analysis {
    /// Every claim, in document order.
    pub claims: Vec<ClaimFact>,
    /// Every `assume` row.
    pub assumptions: Vec<AssumeFact>,
    /// The number of `bound` declarations (a reference implementation has none).
    pub bound_declarations: usize,
    /// The field count of `record Impl`, if there is one.
    pub impl_fields: Option<usize>,
    /// The shape of `model build`.
    pub build: BuildShape,
}

/// Where the Warrant tools refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarrantStage {
    /// Source admission, layout or parsing.
    Parse,
    /// The type checker.
    Check,
    /// A part that is not claims only.
    NotClaims,
}

/// A refusal by the Warrant tools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarrantFailure {
    /// Which stage.
    pub stage: WarrantStage,
    /// The tool's own words (its closed error code and kind; never source text).
    pub message: String,
    /// 1-based line when the error carries a source span.
    pub line: Option<u32>,
}

impl std::fmt::Display for WarrantFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for WarrantFailure {}

fn line_at(text: &str, offset: usize) -> u32 {
    let end = offset.min(text.len());
    let n = text.as_bytes()[..end]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1;
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// `claim_sha256` of the declaration whose source span is `span`.
fn claim_digest(tokens: &[LayoutToken<'_>], norm: &str, span: (usize, usize)) -> String {
    // Tokens are in source order, so a binary search finds the first one in the span
    // and the walk stops at the first one past it: linear in the claim, not in the
    // document.
    let first = tokens.partition_point(|t| match t.span {
        TokenSpan::Real { start, .. } => start < span.0,
        TokenSpan::Synthetic { at } => at < span.0,
    });
    let mut joined = String::new();
    for t in &tokens[first..] {
        let (start, end) = match t.span {
            TokenSpan::Real { start, end } => (start, end),
            // A synthetic layout token (Indent, Dedent, Newline): not text.
            TokenSpan::Synthetic { .. } => continue,
        };
        if start >= span.1 {
            break;
        }
        if end > span.1 {
            continue;
        }
        // The raw source slice of the token, not the lexer's `text`: the lexer drops the
        // quotes of a string literal, so `"x"` and `x` would otherwise hash alike.  For a
        // word, symbol or number the slice IS the token text, so VER29 5.3's definition
        // ("tokens joined by single spaces") is unchanged for them.
        match t.kind {
            LayoutTokenKind::Word(_)
            | LayoutTokenKind::Symbol(_)
            | LayoutTokenKind::Number(_)
            | LayoutTokenKind::Bytes(_)
            | LayoutTokenKind::Text(_) => {}
            LayoutTokenKind::Indent | LayoutTokenKind::Dedent | LayoutTokenKind::Newline => {
                continue
            }
        }
        let Some(text) = norm.get(start..end) else {
            continue;
        };
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(text);
    }
    sha256_hex(joined.as_bytes())
}

fn refuse(
    stage: WarrantStage,
    text: &str,
    message: String,
    span: Option<(usize, usize)>,
) -> WarrantFailure {
    WarrantFailure {
        stage,
        message,
        line: span.map(|s| line_at(text, s.0)),
    }
}

/// Run `f` over the parsed document (the AST borrows the source and the tokens).
fn with_parsed<R>(
    text: &str,
    f: impl FnOnce(
        &coding_adventures_warrant_grammar::parser::WarrantFile<'_>,
        &[LayoutToken<'_>],
        &str,
    ) -> Result<R, WarrantFailure>,
) -> Result<R, WarrantFailure> {
    let doc = SourceDocument::admit(text.as_bytes(), SourceLimits::default())
        .map_err(|e| refuse(WarrantStage::Parse, text, format!("{e}"), None))?;
    let tokens = capture_layout(&doc, LayoutLimits::default()).map_err(|e| {
        refuse(
            WarrantStage::Parse,
            text,
            format!("{e}"),
            e.byte_range().map(|r| (r.start, r.end)),
        )
    })?;
    let file = parse_document(&tokens, &doc, ParserLimits::default())
        .map_err(|e| refuse(WarrantStage::Parse, text, format!("{e}"), e.span()))?;
    f(&file, &tokens, doc.normalized())
}

/// The claims of an obligations (or any claim-only) block, with their digests.
///
/// The block is wrapped in a Warrant header and **parsed only**: an obligations block
/// refers to models and operations the contract and implementation blocks declare, so
/// it cannot type-check alone.  Every declaration must be a `claim`.
pub fn claims_of_block(block: &str) -> Result<Vec<ClaimFact>, WarrantFailure> {
    let text = format!("warrant vos_block v1\n{block}");
    with_parsed(&text, |file, tokens, norm| {
        let mut out = Vec::new();
        for d in &file.declarations {
            match d {
                Declaration::Claim(c) => out.push(ClaimFact {
                    name: c.name.to_string(),
                    sha256: claim_digest(tokens, norm, c.span),
                }),
                other => {
                    let span = declaration_span(other);
                    return Err(refuse(
                        WarrantStage::NotClaims,
                        &text,
                        "an obligations block holds `claim` declarations and comments, nothing else".to_string(),
                        Some(span),
                    ));
                }
            }
        }
        Ok(out)
    })
}

fn declaration_span(d: &Declaration<'_>) -> (usize, usize) {
    match d {
        Declaration::Model(x) => x.span,
        Declaration::Error(x) => x.span,
        Declaration::Op(x) => x.span,
        Declaration::Example(x) => x.span,
        Declaration::Claim(x) => x.span,
        Declaration::Assumptions(AssumptionsDecl::Platform { span, .. })
        | Declaration::Assumptions(AssumptionsDecl::Environment { span, .. }) => *span,
        Declaration::Labels(x) => x.span,
        Declaration::Input(x) => x.span,
        Declaration::Bound(x) => x.span,
        Declaration::Record(x) => x.span,
        Declaration::Variant(x) => x.span,
        Declaration::Vectors(x) => x.span,
        Declaration::Ensures(x) => x.span,
    }
}

/// Parse and type-check a whole document with `warrant-grammar` and report what the
/// R6 checks need.
pub fn analyze_document(text: &str) -> Result<Analysis, WarrantFailure> {
    with_parsed(text, |file, tokens, norm| {
        check_warrant(file)
            .map_err(|e| refuse(WarrantStage::Check, text, format!("{e:?}"), None))?;
        let models: HashMap<&str, &ModelDecl<'_>> = file
            .declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Model(m) => Some((m.name, m)),
                _ => None,
            })
            .collect();
        let taint = Taint { models, src: norm };
        let mut a = Analysis {
            claims: Vec::new(),
            assumptions: Vec::new(),
            bound_declarations: 0,
            impl_fields: None,
            build: BuildShape::Missing,
        };
        for d in &file.declarations {
            match d {
                Declaration::Claim(c) => a.claims.push(ClaimFact {
                    name: c.name.to_string(),
                    sha256: claim_digest(tokens, norm, c.span),
                }),
                Declaration::Assumptions(AssumptionsDecl::Platform { rows, span }) => {
                    a.assumptions.push(AssumeFact {
                        kind: AssumeKind::Platform,
                        is_none: rows.is_empty(),
                        start: span.0,
                    })
                }
                Declaration::Assumptions(AssumptionsDecl::Environment { rows, span }) => {
                    a.assumptions.push(AssumeFact {
                        kind: AssumeKind::Environment,
                        is_none: rows.is_empty(),
                        start: span.0,
                    })
                }
                Declaration::Bound(_) => a.bound_declarations += 1,
                Declaration::Record(r) if r.name == "Impl" => a.impl_fields = Some(r.fields.len()),
                Declaration::Model(m) if m.name == "build" => {
                    a.build =
                        build_shape(&taint, m.params.first().map(|p| p.name), m.body.as_ref());
                }
                _ => {}
            }
        }
        Ok(a)
    })
}

/// The facts the taint walk needs: the document's `model` declarations (to see through a
/// call) and its source (to compare two sub-expressions by text).
struct Taint<'a, 'b> {
    models: HashMap<&'b str, &'b ModelDecl<'a>>,
    src: &'b str,
}

/// How deep a chain of model calls the walk follows before it gives up.  Giving up means
/// "not tainted", which is the failing direction for a production crate.
const TAINT_DEPTH: u32 = 16;

fn span_of(e: &Expression<'_>) -> (usize, usize) {
    match e {
        Expression::Quantified { span, .. }
        | Expression::Conditional { span, .. }
        | Expression::Match { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Literal { span, .. }
        | Expression::List { span, .. }
        | Expression::Reference { span, .. }
        | Expression::Grouped { span, .. }
        | Expression::Tuple { span, .. }
        | Expression::Project { span, .. } => *span,
    }
}

fn is_zero(e: &Expression<'_>) -> bool {
    match e {
        Expression::Grouped { inner, .. } => is_zero(inner),
        Expression::Literal { lit, .. } => matches!(
            lit,
            Literal::U8(0) | Literal::U64(0) | Literal::Int(0) | Literal::Fixed(_, 0)
        ),
        _ => false,
    }
}

fn is_false(e: &Expression<'_>) -> bool {
    match e {
        Expression::Grouped { inner, .. } => is_false(inner),
        Expression::Literal { lit, .. } => matches!(lit, Literal::Bool(false)),
        _ => false,
    }
}

impl Taint<'_, '_> {
    /// The source text of an expression with its whitespace removed.
    fn text(&self, e: &Expression<'_>) -> String {
        let (a, b) = span_of(e);
        self.src
            .get(a..b)
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect()
    }

    fn same(&self, a: &Expression<'_>, b: &Expression<'_>) -> bool {
        let t = self.text(a);
        !t.is_empty() && t == self.text(b)
    }

    /// Does the VALUE of `e` depend on the parameter `param`?  A mention is not enough
    /// (VOS02 10.1, D-GD8): a mention does not count when it is
    ///
    /// * an operand of `and`, `bit_and` or `*` whose other operand is a literal zero or
    ///   `false` (the result is constant),
    /// * the two operands of `x - x` or `bit_xor(x, x)` (the text is identical),
    /// * an argument to a `model` whose body does not depend on that parameter.
    ///
    /// Still evaded, by design of a syntactic walk: a constant produced by arithmetic on
    /// a literal (`shl(1u8, 8)`), a constant condition (`if true then 0u8 else m.p0`),
    /// a dependence hidden by a bound operation, and `x - x` written two ways.
    fn depends(&self, e: &Expression<'_>, param: &str, depth: u32) -> bool {
        match e {
            Expression::Literal { .. } => false,
            Expression::Grouped { inner, .. } => self.depends(inner, param, depth),
            Expression::Unary { operand, .. } => self.depends(operand, param, depth),
            Expression::Project { base, .. } => self.depends(base, param, depth),
            Expression::Binary {
                op, left, right, ..
            } => {
                match op {
                    BinaryOp::And if is_false(left) || is_false(right) => return false,
                    BinaryOp::Mul if is_zero(left) || is_zero(right) => return false,
                    BinaryOp::Sub if self.same(left, right) => return false,
                    _ => {}
                }
                self.depends(left, param, depth) || self.depends(right, param, depth)
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.depends(condition, param, depth)
                    || self.depends(then_branch, param, depth)
                    || self.depends(else_branch, param, depth)
            }
            Expression::Match { target, arms, .. } => {
                self.depends(target, param, depth)
                    || arms.iter().any(|a| self.depends(&a.body, param, depth))
            }
            Expression::Tuple { elements, .. } | Expression::List { elements, .. } => {
                elements.iter().any(|x| self.depends(x, param, depth))
            }
            Expression::Quantified { body, .. } => self.depends(body, param, depth),
            Expression::Reference { name, args, .. } => {
                if name.segments.first() == Some(&param) {
                    return true;
                }
                let Some(args) = args else { return false };
                let [fname] = name.segments.as_slice() else {
                    return args.iter().any(|a| self.depends(a, param, depth));
                };
                match *fname {
                    "bit_and" if args.iter().any(is_zero) => return false,
                    "bit_xor" if args.len() == 2 && self.same(&args[0], &args[1]) => return false,
                    _ => {}
                }
                if let Some(m) = self.models.get(fname) {
                    if depth >= TAINT_DEPTH {
                        return false;
                    }
                    return args.iter().zip(&m.params).any(|(a, p)| {
                        self.depends(a, param, depth)
                            && m.body
                                .as_ref()
                                .is_some_and(|b| self.depends(b, p.name, depth + 1))
                    });
                }
                args.iter().any(|a| self.depends(a, param, depth))
            }
        }
    }
}

/// The cone of `build`: which constructor arguments depend on the abstract state.
fn build_shape(
    t: &Taint<'_, '_>,
    param: Option<&str>,
    body: Option<&Expression<'_>>,
) -> BuildShape {
    let (Some(param), Some(mut body)) = (param, body) else {
        return BuildShape::Missing;
    };
    // `build(m) = ( Impl(...) )`: the parentheses are a Grouped node.
    while let Expression::Grouped { inner, .. } = body {
        body = inner;
    }
    let Expression::Reference {
        name,
        args: Some(args),
        ..
    } = body
    else {
        return BuildShape::NotConstructor;
    };
    if name.segments.as_slice() != ["Impl"] {
        return BuildShape::NotConstructor;
    }
    let in_cone = args.iter().map(|arg| t.depends(arg, param, 0)).collect();
    BuildShape::Constructor { in_cone }
}
