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

use std::cell::{Cell, RefCell};
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
    /// The taint walk ran out of its step budget (`TAINT_BUDGET`): no answer, which
    /// `vos-link` treats as a failure for production and `any` (fail closed).
    BudgetExhausted,
    /// A model the walk had to summarise has more than 63 parameters, more than a
    /// 64-bit mask can tell apart: no answer, so a failure for production and `any`.
    WideModel,
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
/// `None` when a token's span is not a slice of the source: the digest cannot be trusted,
/// so the caller fails closed instead of hashing a claim with a token missing.
fn claim_digest(tokens: &[LayoutToken<'_>], norm: &str, span: (usize, usize)) -> Option<String> {
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
        let text = norm.get(start..end)?;
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(text);
    }
    Some(sha256_hex(joined.as_bytes()))
}

/// The refusal for a claim whose token spans are not slices of the source.
fn digest_failure(text: &str, span: (usize, usize)) -> WarrantFailure {
    refuse(
        WarrantStage::Parse,
        text,
        "a token of the claim is not a slice of the source: no claim_sha256".to_string(),
        Some(span),
    )
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
                    sha256: claim_digest(tokens, norm, c.span)
                        .ok_or_else(|| digest_failure(norm, c.span))?,
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
    analyze_document_with_budget(text, TAINT_BUDGET)
}

/// [`analyze_document`] with an explicit step budget for the taint walk (tests use a tiny
/// one to exercise the give-up path).
pub fn analyze_document_with_budget(text: &str, budget: u64) -> Result<Analysis, WarrantFailure> {
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
        let taint = Taint::new(models, norm, budget);
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
                    sha256: claim_digest(tokens, norm, c.span)
                        .ok_or_else(|| digest_failure(norm, c.span))?,
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
/// call), its source (to compare two sub-expressions by text), a memo of per-model
/// summaries and a step budget.
struct Taint<'a, 'b> {
    models: HashMap<&'b str, &'b ModelDecl<'a>>,
    src: &'b str,
    /// For each model already summarised: bit `j` is set when its value depends on its
    /// parameter `j`.  `None` while the model is being summarised (a recursion guard).
    summaries: RefCell<HashMap<String, Option<u64>>>,
    /// Steps left.  Exhaustion makes the walk give up, which the caller reports as a
    /// failure for production and `any` crates (fail closed).
    budget: Cell<u64>,
    exhausted: Cell<bool>,
    /// Set when a model with more than [`MAX_MODEL_PARAMS`] parameters is met.
    too_wide: Cell<bool>,
}

/// The most parameters of one model the walk can tell apart (one bit each, bit 63 spare).
/// A model with more is refused, not approximated: sharing a bit would let a dependence
/// on a later parameter hide behind an earlier one, the permissive direction.
pub const MAX_MODEL_PARAMS: usize = 63;

/// The default step budget of the taint walk.  With per-model summaries the work is
/// linear in the document, so a real document is far below it (the Warrant token budget
/// of 65,536 bounds the nodes); it exists so no input can cost more than this.
pub const TAINT_BUDGET: u64 = 1_000_000;

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

impl<'a, 'b> Taint<'a, 'b> {
    fn new(models: HashMap<&'b str, &'b ModelDecl<'a>>, src: &'b str, budget: u64) -> Self {
        Taint {
            models,
            src,
            summaries: RefCell::new(HashMap::new()),
            budget: Cell::new(budget),
            exhausted: Cell::new(false),
            too_wide: Cell::new(false),
        }
    }

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

    /// The bit of parameter `j`.  Callers have refused every model with more than
    /// [`MAX_MODEL_PARAMS`] parameters, so `j` is below 63 in every walk whose answer is used.
    fn bit(j: usize) -> u64 {
        // Past 63 only after `too_wide` is set, whose answer replaces this one.
        u32::try_from(j)
            .ok()
            .and_then(|j| 1u64.checked_shl(j))
            .unwrap_or(0)
    }

    /// Which parameters (a bit per index of `params`) does the VALUE of `e` depend on?
    /// A mention is not enough (VOS02 10.1, D-GD8): a mention does not count when it is
    ///
    /// * an operand of `and`, `bit_and` or `*` whose other operand is a literal zero or
    ///   `false` (the result is constant),
    /// * the two operands of `x - x` or `bit_xor(x, x)` (the text is identical),
    /// * an argument to a `model` whose value does not depend on that parameter.
    ///
    /// A model's dependence is computed ONCE (`summary`) and memoised, so a model called
    /// many times, or a chain of models each calling the next several times, costs one
    /// walk per model and not one per call.  Still evaded, by design of a syntactic walk:
    /// see VOS02 10.1 for the list.
    fn deps(&self, e: &Expression<'_>, params: &[&str]) -> u64 {
        let left = self.budget.get();
        if left == 0 {
            self.exhausted.set(true);
            return 0;
        }
        self.budget.set(left - 1);
        match e {
            Expression::Literal { .. } => 0,
            Expression::Grouped { inner, .. } => self.deps(inner, params),
            Expression::Unary { operand, .. } => self.deps(operand, params),
            Expression::Project { base, .. } => self.deps(base, params),
            Expression::Binary {
                op, left, right, ..
            } => {
                match op {
                    BinaryOp::And if is_false(left) || is_false(right) => return 0,
                    BinaryOp::Mul if is_zero(left) || is_zero(right) => return 0,
                    BinaryOp::Sub if self.same(left, right) => return 0,
                    _ => {}
                }
                self.deps(left, params) | self.deps(right, params)
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.deps(condition, params)
                    | self.deps(then_branch, params)
                    | self.deps(else_branch, params)
            }
            Expression::Match { target, arms, .. } => {
                arms.iter().fold(self.deps(target, params), |m, a| {
                    m | self.deps(&a.body, params)
                })
            }
            Expression::Tuple { elements, .. } | Expression::List { elements, .. } => {
                elements.iter().fold(0, |m, x| m | self.deps(x, params))
            }
            Expression::Quantified { body, .. } => self.deps(body, params),
            Expression::Reference { name, args, .. } => {
                if let Some(j) = params.iter().position(|p| name.segments.first() == Some(p)) {
                    return Self::bit(j);
                }
                let Some(args) = args else { return 0 };
                let all = |this: &Self| args.iter().fold(0, |m, a| m | this.deps(a, params));
                let [fname] = name.segments.as_slice() else {
                    return all(self);
                };
                match *fname {
                    "bit_and" if args.iter().any(is_zero) => return 0,
                    "bit_xor" if args.len() == 2 && self.same(&args[0], &args[1]) => return 0,
                    _ => {}
                }
                if self.models.contains_key(fname) {
                    let summary = self.summary(fname);
                    return args
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| summary & Self::bit(*i) != 0)
                        .fold(0, |m, (_, a)| m | self.deps(a, params));
                }
                all(self)
            }
        }
    }

    /// The parameters of model `name` its value depends on, computed once.  A model that
    /// is being summarised when it is asked for again (a cycle the checker should have
    /// refused) counts as depending on everything: conservative for the cone.
    fn summary(&self, name: &str) -> u64 {
        match self.summaries.borrow().get(name) {
            Some(Some(m)) => return *m,
            Some(None) => return u64::MAX,
            None => {}
        }
        let Some(model) = self.models.get(name).copied() else {
            return u64::MAX;
        };
        if model.params.len() > MAX_MODEL_PARAMS {
            self.too_wide.set(true);
            return u64::MAX;
        }
        self.summaries.borrow_mut().insert(name.to_string(), None);
        let params: Vec<&str> = model.params.iter().map(|p| p.name).collect();
        let m = model.body.as_ref().map_or(0, |b| self.deps(b, &params));
        self.summaries
            .borrow_mut()
            .insert(name.to_string(), Some(m));
        m
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
    let in_cone: Vec<bool> = args.iter().map(|arg| t.deps(arg, &[param]) != 0).collect();
    if t.too_wide.get() {
        return BuildShape::WideModel;
    }
    if t.exhausted.get() {
        return BuildShape::BudgetExhausted;
    }
    BuildShape::Constructor { in_cone }
}
