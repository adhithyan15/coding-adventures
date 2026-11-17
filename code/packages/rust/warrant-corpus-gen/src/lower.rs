//! # Lowering a checked document to `ir.core`
//!
//! VER04r section 10.3 (check 1) says two independent implementations can only agree byte
//! for byte if lowering is **syntax-directed and unoptimised**:
//!
//! * one IR node per source construct, operands in source order;
//! * no constant folding, no reassociation, no sharing, no inlining of models;
//! * the desugarings of VER04r section 4 and Appendix A, and no others.
//!
//! This module is the Rust reading of those rules.  It was written from the spec and then
//! compared with `python/warrant-oracle` (`lower.py`), a second reader that shares no code
//! with it; the parity tests of this crate pin the two together.
//!
//! ```text
//!   source construct                     IR
//!   -----------------------------------  ---------------------------------------
//!   ( e )                                e              (a group vanishes)
//!   a != b      x => y                   ["ne",a,b]     ["implies",x,y]
//!   -e                                   ["neg",e]
//!   forall a: T | r1, b: U | r2. body    admit = ["and", r1, r2]   (folded LEFT, binary)
//!   forall a: T. body                    admit = ["lit", true]
//!   op(args)    model(args)              subject_op / model_call (no inlining)
//!   op<4>(a, b)                          ["subject_op","op",[a,b],[4]]
//!   5  (a bare decimal literal)          Nat in a Nat context, Int otherwise
//!   5u8                                  ["lit",{"type":"U8","value":"5"}]
//!   -128i8                               ["neg", lit -128 I8]   (VER04j 2.2: both nodes kept)
//! ```
//!
//! ## What "the small annotating typer" is for
//!
//! The S2 checker accepts or rejects a document but does not publish the type of every
//! sub-expression, and three decisions depend on those types: the type of a bare literal,
//! the static widths of a `Bytes<N>` operation, and whether an arithmetic node wraps
//! (VER04r 3.2).  So every `expr` call returns `(IR, type)`.  This duplicates part of the
//! checker on purpose; a disagreement between the two readers shows up as a lowering
//! mismatch against the oracle, which is the point of having a second reader.  The
//! duplication can be deleted when a later slice (S13) lets `warrant-grammar` publish
//! expression types.
//!
//! ## Fail closed (VER04r section 13, "Slice S3 in detail")
//!
//! The generator computes a digest for every suite it emits, so it cannot emit a suite it
//! cannot evaluate.  A construct beyond `ir.core` therefore refuses the **whole document**
//! with `code == "level"` and the level named; nothing is written and nothing is skipped.
//! Every `match` over the grammar's [`Expression`], [`TypeExpr`] and [`Declaration`] ends
//! in an explicit refusal arm for the same reason.  [`census`] reports, per item, the
//! refusal that would block it, without failing the document.
//!
//! The refusals, by level:
//!
//! ```text
//!   ir.fixed   a V1 built-in of integer shape: widen wrap try to_int to_nat max_value
//!              min_value checked_* rem div div_trunc rem_trunc bit_* shl shr sar rotl
//!              rotr popcount clz ctz bit be_bytes le_bytes from_be from_le imin imax at
//!   ir.data    tuples, projections, list literals, `match`, constructors, `Option`,
//!              `List`, records, variants; Some None unwrap_or fixed slice to_list
//!              to_bytes replicate range
//!   ir.seq     fold map all any count (reserved)
//!   inout / ensures / vectors / assume   WX-G, WX-E and assumption rows
//! ```
//!
//! `exists`, a named static width at a call site, a subject call inside a model body, a
//! subject call under an impure guard (VER04r 4.4) and, in a **v0** document, a
//! fixed-width `+ - *` or unary `-` with a subject call beneath it (VER04r 3.3 rule 3) are
//! `W0701` refusals: the document is well typed but the corpus cannot express it honestly.

use std::collections::{HashMap, HashSet};

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::parser::{
    AssumptionsDecl, BinaryOp, ClaimDecl, Declaration, ExampleDecl, Expression, Literal, ModelDecl,
    OpDecl, Parameter, TypeExpr, UnaryOp, Version, WarrantFile, Width as AstWidth,
};

use crate::analysis::{check_pure_guards, check_v0_static_arith};
use crate::error::{code, Beyond, GenError};
use crate::ir::{Arith, Cmp, Ir, Logic, MAX_IR_DEPTH};
use crate::json::Json;
use crate::types::{Ty, Width};

/// Expression nesting limit while lowering (VER04 section 2: "64 nested expression nodes
/// on any path"; VER04r 14: IR depth 64).  Parentheses do not count: a group vanishes.
pub const MAX_EXPR_DEPTH: usize = MAX_IR_DEPTH;

/// Type nesting limit (VER04r 14: type depth 16).
const MAX_TYPE_DEPTH: usize = 16;

/// The longest `Bytes<N>` (VER04h section 3, `MAX_FIXED_WIDTH`).
const MAX_FIXED_WIDTH: u64 = 4096;

/// V1 built-ins of integer shape (VER04r 4.1 `ir.fixed`), by the names the grammar uses.
const LEVEL_FIXED: [&str; 34] = [
    "widen",
    "wrap",
    "try",
    "to_int",
    "to_nat",
    "max_value",
    "min_value",
    "checked_add",
    "checked_sub",
    "checked_mul",
    "div_trunc",
    "rem_trunc",
    "rem",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "shl",
    "shr",
    "sar",
    "rotl",
    "rotr",
    "popcount",
    "clz",
    "ctz",
    "bit",
    "be_bytes",
    "le_bytes",
    "from_be",
    "from_le",
    "at",
    "div",
    "imin",
    "imax",
];

/// Constructors and sequence built-ins of `ir.data`.
const LEVEL_DATA: [&str; 9] = [
    "Some",
    "None",
    "unwrap_or",
    "fixed",
    "slice",
    "to_list",
    "to_bytes",
    "replicate",
    "range",
];

/// The reserved WX-D sequence level.
const LEVEL_SEQ: [&str; 5] = ["fold", "map", "all", "any", "count"];

// ---------------------------------------------------------------------------
// The lowered document
// ---------------------------------------------------------------------------

/// A declared error type: `error SelectError = A | B`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorType {
    /// The type's name.
    pub name: String,
    /// Its cases, in declaration order (the corpus compares them by name).
    pub cases: Vec<String>,
}

/// A subject operation's signature (VER04r section 6, without `inout`/`ensures`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredOp {
    /// The Warrant name.
    pub name: String,
    /// Static width parameters (`N` in `op eq_fixed<N: Nat>`).
    pub statics: Vec<String>,
    /// Parameters in order; a `Bytes<N>` parameter keeps its static name.
    pub params: Vec<(String, Ty)>,
    /// The declared result type.
    pub ret: Ty,
    /// The result must not share storage with an argument.
    pub fresh: bool,
    /// The parameters' refinements as one Bool tree (`and`-folded left), if any.
    pub admit: Option<Ir>,
}

impl LoweredOp {
    /// The row of `corpus.json` `ops` (VER04r section 6): `params`, `ret`, `width_params`
    /// (only for a static-width op), `fresh` (only when true), `admit` (only when the
    /// parameters have refinements) and `requires`.
    pub fn to_json(&self) -> Json {
        let mut row = vec![
            (
                "params".to_string(),
                Json::arr(
                    self.params
                        .iter()
                        .map(|(n, t)| Json::arr([Json::str(n.as_str()), Json::str(t.name())])),
                ),
            ),
            ("ret".to_string(), Json::str(self.ret.name())),
            ("requires".to_string(), Json::strs(["ir.core"])),
        ];
        if !self.statics.is_empty() {
            row.push(("width_params".into(), Json::strs(self.statics.iter())));
        }
        if self.fresh {
            row.push(("fresh".into(), Json::Bool(true)));
        }
        if let Some(admit) = &self.admit {
            row.push(("admit".into(), admit.to_json()));
        }
        Json::Obj(row)
    }
}

/// A pure model, lowered.  Its body contains no `subject_op`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredModel {
    /// The model's name.
    pub name: String,
    /// Parameters in order.
    pub params: Vec<(String, Ty)>,
    /// The declared result type.
    pub ret: Ty,
    /// The body.
    pub body: Ir,
}

/// How a claim relates its subject to the model (VER04r 4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuiteKind {
    /// `body` is `subject == pure` (either order) with no `subject_op` in `expected`.
    Reference {
        /// The side that calls the subject.
        subject: Ir,
        /// The pure side: what the generator evaluates for `expected_sha256`.
        expected: Ir,
        /// The type of the subject side (what the stream encodes).
        result: Ty,
    },
    /// Any other Bool body: a relational claim, with no expected values.
    Predicate,
}

/// One claim, lowered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredSuite {
    /// `warrant/<module>/exhaustive/<claim>`.
    pub id: String,
    /// The claim's name.
    pub claim: String,
    /// The binders in order.
    pub binders: Vec<(String, Ty)>,
    /// The refinements, `and`-folded left; `["lit", true]` when there are none.
    pub admit: Ir,
    /// The whole claim body (Bool).
    pub body: Ir,
    /// Reference or predicate.
    pub kind: SuiteKind,
    /// The operations the body calls, in order of first occurrence.
    pub ops: Vec<String>,
}

/// The reference decomposition of an example, when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleRef {
    /// The side that calls the subject.
    pub subject: Ir,
    /// The pure side.
    pub expected: Ir,
    /// The subject side's type.
    pub result: Ty,
}

/// One `example`, lowered (S3c turns it into a `cases.tfv` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredExample {
    /// The example's name.
    pub name: String,
    /// The whole Bool body.
    pub body: Ir,
    /// Its decomposition: exactly one subject call compared with a pure value (anything
    /// else is refused at lowering, so this is always present).
    pub reference: ExampleRef,
}

/// A whole document lowered to `ir.core`.  Every item is within `ir.core`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lowered {
    /// The module name (`ct_compare`).
    pub module: String,
    /// `v0` or `v1`.
    pub version: Version,
    /// Declared error types, in declaration order.
    pub errors: Vec<ErrorType>,
    /// Operations, in declaration order.
    pub ops: Vec<LoweredOp>,
    /// Models in declaration order, which is also `model_order` (VER04r 4.2 rule 1).
    pub models: Vec<LoweredModel>,
    /// Claims in declaration order.
    pub suites: Vec<LoweredSuite>,
    /// Examples in declaration order.
    pub examples: Vec<LoweredExample>,
}

impl Lowered {
    /// `"v0"` or `"v1"`, the corpus `document` field.
    pub fn document(&self) -> &'static str {
        match self.version {
            Version::V0 => "v0",
            Version::V1 => "v1",
        }
    }

    /// The model named `name`.
    pub fn model(&self, name: &str) -> Option<&LoweredModel> {
        self.models.iter().find(|m| m.name == name)
    }

    /// The operation named `name`.
    pub fn op(&self, name: &str) -> Option<&LoweredOp> {
        self.ops.iter().find(|o| o.name == name)
    }
}

// ---------------------------------------------------------------------------
// The partial result (what census reads)
// ---------------------------------------------------------------------------

/// One declared item and, if it is blocked, why.
type Slot<T> = Result<T, GenError>;

/// A document lowered item by item: each operation, model, claim and example either
/// lowered or blocked by a refusal.  [`lower_document`] turns the first refusal into an
/// error; [`census`] reports them all.
#[derive(Debug, Clone)]
pub struct Partial {
    module: String,
    version: Version,
    errors: Vec<ErrorType>,
    /// Document-level blockers (`ensures`, `vectors`, records, variants, assumptions).
    declarations: Vec<GenError>,
    ops: Vec<(String, Slot<LoweredOp>)>,
    models: Vec<(String, Slot<LoweredModel>)>,
    suites: Vec<(String, Slot<LoweredSuite>)>,
    examples: Vec<(String, Slot<LoweredExample>)>,
}

/// What kind of item a [`CensusRow`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A document-level declaration (`ensures`, `vectors`, a record, an `assume` row).
    Declaration,
    /// An `op`.
    Op,
    /// A `model`.
    Model,
    /// A `claim`.
    Claim,
    /// An `example`.
    Example,
}

/// One line of [`census`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CensusRow {
    /// What the item is.
    pub kind: ItemKind,
    /// Its name (empty for an anonymous declaration).
    pub name: String,
    /// `None` if the item is within `ir.core`, else the refusal that blocks it
    /// (`blocked.level` names the level for `code == "level"`).
    pub blocked: Option<GenError>,
}

impl Partial {
    /// The module name.
    pub fn module(&self) -> &str {
        &self.module
    }

    /// `v0` or `v1`.
    pub fn document(&self) -> &'static str {
        match self.version {
            Version::V0 => "v0",
            Version::V1 => "v1",
        }
    }

    /// The first refusal, in the order: declarations, operations, models, claims,
    /// examples.
    pub fn first_block(&self) -> Option<GenError> {
        self.declarations
            .iter()
            .cloned()
            .chain(
                self.ops
                    .iter()
                    .filter_map(|(_, s)| s.as_ref().err().cloned()),
            )
            .chain(
                self.models
                    .iter()
                    .filter_map(|(_, s)| s.as_ref().err().cloned()),
            )
            .chain(
                self.suites
                    .iter()
                    .filter_map(|(_, s)| s.as_ref().err().cloned()),
            )
            .chain(
                self.examples
                    .iter()
                    .filter_map(|(_, s)| s.as_ref().err().cloned()),
            )
            .next()
    }

    /// Every item with its refusal, in the same order as [`Partial::first_block`].
    pub fn census(&self) -> Vec<CensusRow> {
        let mut rows = Vec::new();
        for d in &self.declarations {
            rows.push(CensusRow {
                kind: ItemKind::Declaration,
                name: d.at.clone(),
                blocked: Some(d.clone()),
            });
        }
        let mut push = |kind, name: &String, blocked: Option<GenError>| {
            rows.push(CensusRow {
                kind,
                name: name.clone(),
                blocked,
            })
        };
        for (n, s) in &self.ops {
            push(ItemKind::Op, n, s.as_ref().err().cloned());
        }
        for (n, s) in &self.models {
            push(ItemKind::Model, n, s.as_ref().err().cloned());
        }
        for (n, s) in &self.suites {
            push(ItemKind::Claim, n, s.as_ref().err().cloned());
        }
        for (n, s) in &self.examples {
            push(ItemKind::Example, n, s.as_ref().err().cloned());
        }
        rows
    }

    /// The models that lowered, in declaration order (for the census's cost check).
    pub fn lowered_models(&self) -> Vec<LoweredModel> {
        self.models
            .iter()
            .filter_map(|(_, s)| s.as_ref().ok().cloned())
            .collect()
    }

    /// The claims that lowered, in declaration order.
    pub fn lowered_suites(&self) -> Vec<&LoweredSuite> {
        self.suites
            .iter()
            .filter_map(|(_, s)| s.as_ref().ok())
            .collect()
    }

    /// The fully lowered document, or the first refusal.
    pub fn into_lowered(self) -> Result<Lowered, GenError> {
        if let Some(block) = self.first_block() {
            return Err(block);
        }
        // `first_block` found nothing, so every slot is `Ok`.
        Ok(Lowered {
            module: self.module,
            version: self.version,
            errors: self.errors,
            ops: oks(self.ops),
            models: oks(self.models),
            suites: oks(self.suites),
            examples: oks(self.examples),
        })
    }
}

/// The lowered values of a list of slots that are all `Ok`.
fn oks<T>(slots: Vec<(String, Slot<T>)>) -> Vec<T> {
    slots.into_iter().filter_map(|(_, s)| s.ok()).collect()
}

/// Lower a document to `ir.core`, refusing the whole document at the first construct
/// beyond it.
///
/// The document is first run through the S2 checker; a checker error is returned as
/// `code == "check"` (the checker's own diagnostic code in the message).
///
/// # Errors
///
/// See the module documentation.  The first refusal in the order declarations,
/// operations, models, claims, examples is returned.
pub fn lower_document(file: &WarrantFile<'_>) -> Result<Lowered, GenError> {
    lower_partial(file)?.into_lowered()
}

/// Per-item census: what blocks each operation, model, claim and example (VER04r 16.2 and
/// the S3b census build on this).  A malformed document (checker or type error) is an
/// `Err`; a document with refusals is an `Ok` whose rows carry them.
pub fn census(file: &WarrantFile<'_>) -> Result<Vec<CensusRow>, GenError> {
    Ok(lower_partial(file)?.census())
}

/// Lower every item independently, keeping refusals per item.  Only a malformed document
/// (checker error, type error, resource limit) is an `Err`.
pub fn lower_partial(file: &WarrantFile<'_>) -> Result<Partial, GenError> {
    if let Err(e) = check_warrant(file) {
        return Err(GenError::new(
            code::CHECK,
            "",
            format!("the document does not pass the checker ({})", e.code),
        ));
    }
    lower_partial_unchecked(file)
}

/// [`lower_partial`] without the checker, so the unit tests can reach the defensive
/// refusals that the checker normally rules out first.
pub(crate) fn lower_partial_unchecked(file: &WarrantFile<'_>) -> Result<Partial, GenError> {
    Lowerer::new(file).run()
}

/// A refusal of a specific construct (as opposed to a malformed document) is recorded
/// per item; everything else aborts the document.
fn is_refusal(e: &GenError) -> bool {
    e.code == code::LEVEL || e.code == code::W0701
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// An operation signature while claims are being lowered.
#[derive(Clone)]
struct OpSig {
    statics: Vec<String>,
    params: Vec<(String, Ty)>,
    ret: Ty,
}

/// A model as claims see it.
struct ModelEntry {
    params: Option<Vec<(String, Ty)>>,
    ret: Option<Ty>,
    outcome: Result<(), GenError>,
    /// Does the model's call closure (its body and every model it calls) contain a
    /// fixed-width `add`/`sub`/`mul`/`neg`?  Needed by the v0 guard (VER04r 3.3 rule 3).
    wraps: bool,
}

struct Lowerer<'f, 'a> {
    file: &'f WarrantFile<'a>,
    module: String,
    errors: HashMap<String, Vec<String>>,
    unsupported_types: HashSet<String>,
    /// `model Word = Bytes<4>`: a type alias (a model with no parameters and no body).
    aliases: HashMap<String, TypeExpr<'a>>,
    op_sigs: HashMap<String, Slot<OpSig>>,
    model_names: HashSet<String>,
    models: HashMap<String, ModelEntry>,
    scope: HashMap<String, Ty>,
    nest: usize,
    current_model: Option<String>,
    /// The operand types the first typing pass gave the top-level `==`/`!=` of the claim or
    /// example body being lowered, so `classify` never re-types a side without its context
    /// (S3a review, known gap L6).
    top_eq_types: Option<(Ty, Ty)>,
}

/// Strip any number of parentheses: a group vanishes in the IR.
fn strip<'e, 'a>(mut e: &'e Expression<'a>) -> &'e Expression<'a> {
    while let Expression::Grouped { inner, .. } = e {
        e = inner;
    }
    e
}

/// Does `e` have no type of its own, so that its sibling must be typed first?
fn needs_context(e: &Expression<'_>) -> bool {
    match strip(e) {
        Expression::Literal {
            lit: Literal::Int(_),
            ..
        } => true,
        Expression::Reference {
            name,
            args: Some(_),
            ..
        } if name.segments.len() == 1 && matches!(name.segments[0], "Ok" | "Err") => true,
        Expression::Unary {
            op: UnaryOp::Neg,
            operand,
            ..
        } => needs_context(operand),
        Expression::Conditional {
            then_branch,
            else_branch,
            ..
        } => needs_context(then_branch) && needs_context(else_branch),
        Expression::Binary {
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul,
            left,
            right,
            ..
        } => needs_context(left) && needs_context(right),
        _ => false,
    }
}

impl<'f, 'a> Lowerer<'f, 'a> {
    fn new(file: &'f WarrantFile<'a>) -> Self {
        Lowerer {
            file,
            module: file.name.to_string(),
            errors: HashMap::new(),
            unsupported_types: HashSet::new(),
            aliases: HashMap::new(),
            op_sigs: HashMap::new(),
            model_names: HashSet::new(),
            models: HashMap::new(),
            scope: HashMap::new(),
            nest: 0,
            current_model: None,
            top_eq_types: None,
        }
    }

    // -- the document ------------------------------------------------------------

    fn run(&mut self) -> Result<Partial, GenError> {
        let mut errors = Vec::new();
        let mut declarations = Vec::new();
        // Pass 1: names (error types, record and variant names, model names).
        for d in &self.file.declarations {
            match d {
                Declaration::Error(e) => {
                    let cases: Vec<String> = e.variants.iter().map(|v| v.to_string()).collect();
                    self.errors.insert(e.name.to_string(), cases.clone());
                    errors.push(ErrorType {
                        name: e.name.to_string(),
                        cases,
                    });
                }
                Declaration::Record(r) => {
                    self.unsupported_types.insert(r.name.to_string());
                    declarations.push(GenError::level(
                        Beyond::IrData,
                        r.name,
                        "a record declaration needs ir.data",
                    ));
                }
                Declaration::Variant(v) => {
                    self.unsupported_types.insert(v.name.to_string());
                    declarations.push(GenError::level(
                        Beyond::IrData,
                        v.name,
                        "a variant declaration needs ir.data",
                    ));
                }
                Declaration::Vectors(v) => declarations.push(GenError::level(
                    Beyond::Vectors,
                    v.name,
                    "a `vectors` table has no corpus representation yet",
                )),
                Declaration::Ensures(e) => declarations.push(GenError::level(
                    Beyond::Ensures,
                    e.op,
                    "`ensures` rows need slice S12",
                )),
                Declaration::Assumptions(AssumptionsDecl::Platform { rows, .. })
                    if !rows.is_empty() =>
                {
                    declarations.push(GenError::level(
                        Beyond::Assume,
                        "platform",
                        "an `assume platform` row is not representable yet",
                    ))
                }
                Declaration::Assumptions(AssumptionsDecl::Environment { rows, .. })
                    if !rows.is_empty() =>
                {
                    declarations.push(GenError::level(
                        Beyond::Assume,
                        "environment",
                        "an `assume environment` row is not representable yet",
                    ))
                }
                Declaration::Model(m) if m.body.is_none() && m.params.is_empty() => {
                    // A type alias, resolved away: it never appears in a corpus.
                    self.aliases
                        .insert(m.name.to_string(), m.target_type.clone());
                }
                Declaration::Model(m) => {
                    self.model_names.insert(m.name.to_string());
                }
                // Nothing in these reaches a corpus: labels, input contracts, resource
                // bounds, empty assumption blocks, and the declarations lowered below.
                Declaration::Assumptions(_)
                | Declaration::Labels(_)
                | Declaration::Input(_)
                | Declaration::Bound(_)
                | Declaration::Op(_)
                | Declaration::Example(_)
                | Declaration::Claim(_) => {}
            }
        }
        // Pass 2: operation signatures.
        let mut ops: Vec<(String, Slot<LoweredOp>)> = Vec::new();
        let mut op_decls: Vec<&OpDecl<'a>> = Vec::new();
        for d in &self.file.declarations {
            if let Declaration::Op(op) = d {
                let slot = self.lower_op_signature(op);
                let slot = match slot {
                    Err(e) if is_refusal(&e) => Err(e.at(op.name)),
                    Err(e) => return Err(e.at(op.name)),
                    Ok(v) => Ok(v),
                };
                match &slot {
                    Ok(lop) => {
                        self.op_sigs.insert(
                            op.name.to_string(),
                            Ok(OpSig {
                                statics: lop.statics.clone(),
                                params: lop.params.clone(),
                                ret: lop.ret.clone(),
                            }),
                        );
                    }
                    Err(e) => {
                        self.op_sigs.insert(op.name.to_string(), Err(e.clone()));
                    }
                }
                ops.push((op.name.to_string(), slot));
                op_decls.push(op);
            }
        }
        // Pass 3: models, in declaration order (a model sees only earlier models).
        let mut models: Vec<(String, Slot<LoweredModel>)> = Vec::new();
        for d in &self.file.declarations {
            if let Declaration::Model(m) = d {
                if self.aliases.contains_key(m.name) {
                    continue;
                }
                let slot = self.lower_model(m)?;
                models.push((m.name.to_string(), slot));
            }
        }
        // Pass 4: operation refinements (they may call models).
        for (i, decl) in op_decls.iter().enumerate() {
            if let Ok(lop) = &ops[i].1 {
                match self.lower_op_admit(decl, lop) {
                    Ok(admit) => {
                        if let Ok(l) = &mut ops[i].1 {
                            l.admit = admit;
                        }
                    }
                    Err(e) if is_refusal(&e) => {
                        let e = e.at(decl.name);
                        self.op_sigs.insert(decl.name.to_string(), Err(e.clone()));
                        ops[i].1 = Err(e);
                    }
                    Err(e) => return Err(e.at(decl.name)),
                }
            }
        }
        // Pass 5: claims and examples.
        let mut suites = Vec::new();
        let mut examples = Vec::new();
        for d in &self.file.declarations {
            match d {
                Declaration::Claim(c) => {
                    let slot = match self.lower_claim(c) {
                        Ok(s) => Ok(s),
                        Err(e) if is_refusal(&e) => Err(e.at(c.name)),
                        Err(e) => return Err(e.at(c.name)),
                    };
                    suites.push((c.name.to_string(), slot));
                }
                Declaration::Example(x) => {
                    let slot = match self.lower_example(x) {
                        Ok(s) => Ok(s),
                        Err(e) if is_refusal(&e) => Err(e.at(x.name)),
                        Err(e) => return Err(e.at(x.name)),
                    };
                    examples.push((x.name.to_string(), slot));
                }
                _ => {}
            }
        }
        Ok(Partial {
            module: self.module.clone(),
            version: self.file.version,
            errors,
            declarations,
            ops,
            models,
            suites,
            examples,
        })
    }

    fn type_err(&self, message: &str) -> GenError {
        GenError::new(code::TYPE, "", message)
    }

    fn level(&self, beyond: Beyond, message: &str) -> GenError {
        GenError::level(beyond, "", message)
    }

    // -- types -------------------------------------------------------------------

    fn resolve(&self, t: &TypeExpr<'_>, statics: &[String], depth: usize) -> Result<Ty, GenError> {
        if depth > MAX_TYPE_DEPTH {
            return Err(GenError::new(
                code::LIMIT,
                "",
                "type nesting exceeds 16 levels",
            ));
        }
        match t {
            TypeExpr::Bytes { width: None, .. } => Ok(Ty::BYTES),
            TypeExpr::Bytes {
                width: Some(AstWidth::Literal(n)),
                ..
            } => {
                if *n > MAX_FIXED_WIDTH {
                    return Err(self.type_err("Bytes width exceeds 4096"));
                }
                Ok(Ty::bytes_n(*n))
            }
            TypeExpr::Bytes {
                width: Some(AstWidth::Named(p)),
                ..
            } => {
                if statics.iter().any(|s| s == p) {
                    Ok(Ty::Bytes(Some(Width::Param((*p).to_string()))))
                } else {
                    Err(self.type_err("an unknown static width name"))
                }
            }
            TypeExpr::List { .. } | TypeExpr::Option { .. } | TypeExpr::Tuple { .. } => {
                Err(self.level(Beyond::IrData, "List, Option and tuple types need ir.data"))
            }
            TypeExpr::Result {
                ok_type, err_name, ..
            } => {
                if !self.errors.contains_key(*err_name) {
                    return Err(self.type_err("Result takes a declared error type"));
                }
                let ok = self.resolve(ok_type, statics, depth + 1)?;
                Ok(Ty::Result(Box::new(ok), (*err_name).to_string()))
            }
            TypeExpr::Named { name, .. } => match *name {
                "Bool" => Ok(Ty::Bool),
                "Int" => Ok(Ty::Int),
                "Nat" => Ok(Ty::Nat),
                other => {
                    if let Some(target) = self.aliases.get(other) {
                        self.resolve(target, statics, depth + 1)
                    } else if let Some(f) = FixedTy::from_name(other) {
                        Ok(Ty::Fixed(f))
                    } else if self.unsupported_types.contains(other) {
                        Err(self.level(Beyond::IrData, "record and variant types need ir.data"))
                    } else if self.errors.contains_key(other) {
                        Err(self.type_err("an error type is not a value type"))
                    } else {
                        Err(self.type_err("an unknown type name"))
                    }
                }
            },
        }
    }

    // -- operations --------------------------------------------------------------

    fn lower_op_signature(&mut self, op: &OpDecl<'a>) -> Result<LoweredOp, GenError> {
        let statics: Vec<String> = op
            .static_params
            .iter()
            .map(|s| s.name.to_string())
            .collect();
        let mut params = Vec::new();
        for p in &op.params {
            if p.inout {
                return Err(self.level(Beyond::Inout, "an `inout` parameter needs slice S12"));
            }
            params.push((
                p.name.to_string(),
                self.resolve(&p.param_type, &statics, 0)?,
            ));
        }
        let ret = self.resolve(&op.return_type, &statics, 0)?;
        Ok(LoweredOp {
            name: op.name.to_string(),
            statics,
            params,
            ret,
            fresh: op.is_fresh,
            admit: None,
        })
    }

    /// The parameters' refinements as one `and`-folded Bool tree.  The scope is the
    /// operation's parameters; a refinement is typed Bool and has no `subject_op`.
    fn lower_op_admit(&mut self, op: &OpDecl<'a>, lop: &LoweredOp) -> Result<Option<Ir>, GenError> {
        self.scope = lop.params.iter().cloned().collect();
        let mut refinements = Vec::new();
        for p in &op.params {
            if let Some(r) = &p.refinement {
                let (ir, ty) = self.expr(r, Some(&Ty::Bool))?;
                if ty != Ty::Bool {
                    return Err(self.type_err("a refinement must be Bool"));
                }
                refinements.push(ir);
            }
        }
        let admit = fold_and(refinements);
        if let Some(a) = &admit {
            if a.has_subject() {
                return Err(GenError::new(
                    code::W0701,
                    "",
                    "an operation refinement may not call the subject",
                ));
            }
        }
        Ok(admit)
    }

    // -- models ------------------------------------------------------------------

    /// Lower a model.  A refusal is recorded in the slot (and in `self.models`, so a
    /// caller inherits it); a malformed model aborts.
    fn lower_model(&mut self, decl: &ModelDecl<'a>) -> Result<Slot<LoweredModel>, GenError> {
        self.current_model = Some(decl.name.to_string());
        let result = self.lower_model_inner(decl);
        self.current_model = None;
        match result {
            Ok(model) => {
                self.models.insert(
                    decl.name.to_string(),
                    ModelEntry {
                        params: Some(model.params.clone()),
                        ret: Some(model.ret.clone()),
                        outcome: Ok(()),
                        wraps: self.closure_wraps(&model.body),
                    },
                );
                Ok(Ok(model))
            }
            Err(e) if is_refusal(&e) => {
                let e = e.at(decl.name);
                self.models.insert(
                    decl.name.to_string(),
                    ModelEntry {
                        params: None,
                        ret: None,
                        outcome: Err(e.clone()),
                        wraps: false,
                    },
                );
                Ok(Err(e))
            }
            Err(e) => Err(e.at(decl.name)),
        }
    }

    /// Does `body` contain a fixed-width arithmetic node, directly or through a model
    /// it calls (callees are earlier models, already flagged)?
    fn closure_wraps(&self, body: &Ir) -> bool {
        let mut found = false;
        body.walk(&mut |n| match n {
            Ir::Arith(_, ty, ..) | Ir::Neg(ty, _) if ty.is_fixed() => found = true,
            Ir::Model { name, .. } if self.models.get(name).is_some_and(|e| e.wraps) => {
                found = true
            }
            _ => {}
        });
        found
    }

    /// The v0 guard, rule 3, for a lowered claim or example body.
    fn v0_guard(&self, body: &Ir) -> Result<(), GenError> {
        if self.file.version == Version::V0 {
            check_v0_static_arith(body, &|n| self.models.get(n).is_some_and(|e| e.wraps))?;
        }
        Ok(())
    }

    fn lower_model_inner(&mut self, decl: &ModelDecl<'a>) -> Result<LoweredModel, GenError> {
        let mut params = Vec::new();
        for p in &decl.params {
            if p.inout {
                return Err(self.level(Beyond::Inout, "an `inout` parameter needs slice S12"));
            }
            if p.refinement.is_some() {
                return Err(self.type_err("a model parameter has no refinement"));
            }
            params.push((p.name.to_string(), self.resolve(&p.param_type, &[], 0)?));
        }
        let ret = self.resolve(&decl.target_type, &[], 0)?;
        let body = decl
            .body
            .as_ref()
            .ok_or_else(|| self.type_err("a model has no body"))?;
        self.scope = params.iter().cloned().collect();
        let (ir, ty) = self.expr(body, Some(&ret))?;
        if !Ty::assignable(&ty, &ret) {
            return Err(self.type_err("a model body does not have its declared type"));
        }
        if ir.has_subject() {
            return Err(GenError::new(
                code::W0701,
                "",
                "a subject call inside a model body (VER04r 4.2 rule 3)",
            ));
        }
        Ok(LoweredModel {
            name: decl.name.to_string(),
            params,
            ret,
            body: ir,
        })
    }

    // -- claims and examples -------------------------------------------------------

    fn lower_claim(&mut self, decl: &ClaimDecl<'a>) -> Result<LoweredSuite, GenError> {
        let id = format!("warrant/{}/exhaustive/{}", self.module, decl.name);
        self.scope = HashMap::new();
        let mut binders: Vec<(String, Ty)> = Vec::new();
        let mut refinements: Vec<Ir> = Vec::new();
        let mut body_ast = strip(&decl.body);
        if let Expression::Quantified {
            is_forall,
            binders: bs,
            body,
            ..
        } = body_ast
        {
            if !*is_forall {
                return Err(GenError::new(
                    code::W0701,
                    "",
                    "`exists` has no model-IR form; a claim quantifier must be `forall`",
                ));
            }
            for b in bs {
                self.declare_binder(b, &mut binders, &mut refinements)?;
            }
            body_ast = strip(body);
        }
        let admit = fold_and(refinements).unwrap_or_else(Ir::t);
        if admit.has_subject() {
            return Err(GenError::new(
                code::W0701,
                "",
                "a refinement (`admit`) may not call the subject (VER04r 4.5)",
            ));
        }
        self.top_eq_types = None;
        let (body, ty) = self.expr(body_ast, Some(&Ty::Bool))?;
        if ty != Ty::Bool {
            return Err(self.type_err("a claim body must be Bool"));
        }
        // VER04r 4.4: a subject call may sit under a guard only if the guard is pure.
        check_pure_guards(&body)?;
        // VER04r 3.3 rule 3: a v0 document refuses fixed-width arithmetic over a subject
        // result, statically.
        self.v0_guard(&body)?;
        let ops = body.subject_ops();
        // A claim that calls no operation has nothing to test: its suite would pass for any
        // implementation, and `expected_calls` would be zero (S3a review, known gap L5).
        if ops.is_empty() {
            return Err(GenError::new(
                code::W0701,
                "",
                "a claim must call at least one operation of the subject (VER04r 4.3)",
            ));
        }
        let kind = self.classify(&body)?;
        Ok(LoweredSuite {
            id,
            claim: decl.name.to_string(),
            binders,
            admit,
            body,
            kind,
            ops,
        })
    }

    fn declare_binder(
        &mut self,
        b: &Parameter<'a>,
        binders: &mut Vec<(String, Ty)>,
        refinements: &mut Vec<Ir>,
    ) -> Result<(), GenError> {
        if self.scope.contains_key(b.name) {
            return Err(self.type_err("duplicate or shadowing binder name"));
        }
        if b.inout {
            return Err(self.type_err("a binder cannot be inout"));
        }
        let ty = self.resolve(&b.param_type, &[], 0)?;
        self.scope.insert(b.name.to_string(), ty.clone());
        binders.push((b.name.to_string(), ty));
        if let Some(r) = &b.refinement {
            let (ir, rty) = self.expr(r, Some(&Ty::Bool))?;
            if rty != Ty::Bool {
                return Err(self.type_err("a refinement must be Bool"));
            }
            refinements.push(ir);
        }
        Ok(())
    }

    fn lower_example(&mut self, decl: &ExampleDecl<'a>) -> Result<LoweredExample, GenError> {
        self.scope = HashMap::new();
        let body_ast = strip(&decl.body);
        self.top_eq_types = None;
        let (body, ty) = self.expr(body_ast, Some(&Ty::Bool))?;
        if ty != Ty::Bool {
            return Err(self.type_err("an example body must be Bool"));
        }
        self.v0_guard(&body)?;
        check_pure_guards(&body)?;
        // The `examples` row of VER04r 1.4 can only say `op(literals) == pure`: exactly one
        // subject call, its arguments free of subject calls, on one side of an equality.
        let reference =
            match self.classify(&body)? {
                SuiteKind::Reference {
                    subject: subject @ Ir::Subject { .. },
                    expected,
                    result,
                } if subject.children().iter().all(|a| !a.has_subject()) => ExampleRef {
                    subject,
                    expected,
                    result,
                },
                _ => return Err(GenError::new(
                    code::W0701,
                    "",
                    "an example must be one subject call compared with a pure value (VER04r 1.4)",
                )),
            };
        Ok(LoweredExample {
            name: decl.name.to_string(),
            body,
            reference,
        })
    }

    /// Decide `reference` or `predicate` (VER04r 4.3): `body` is `subject == pure`, in
    /// either order, with the subject on exactly one side.  The result type is the type
    /// the first typing pass gave the subject side (kept in `top_eq_types`, never re-derived
    /// without its context).
    fn classify(&mut self, body: &Ir) -> Result<SuiteKind, GenError> {
        let Ir::Cmp(Cmp::Eq, lhs, rhs) = body else {
            return Ok(SuiteKind::Predicate);
        };
        let (lh, rh) = (lhs.has_subject(), rhs.has_subject());
        if lh == rh {
            return Ok(SuiteKind::Predicate);
        }
        let (subject, expected) = if lh { (lhs, rhs) } else { (rhs, lhs) };
        // The body is a top-level `==`, so the first typing pass recorded its operand types.
        let Some((l_ty, r_ty)) = self.top_eq_types.take() else {
            return Err(self.type_err("a reference body is a comparison"));
        };
        Ok(SuiteKind::Reference {
            subject: (**subject).clone(),
            expected: (**expected).clone(),
            result: if lh { l_ty } else { r_ty },
        })
    }

    // -- expressions -----------------------------------------------------------------

    /// Lower `e`; `expected` types bare literals and `Ok`/`Err`.
    fn expr(&mut self, e: &Expression<'a>, expected: Option<&Ty>) -> Result<(Ir, Ty), GenError> {
        let e = strip(e);
        self.nest += 1;
        let result = if self.nest > MAX_EXPR_DEPTH {
            Err(GenError::new(
                code::LIMIT,
                "",
                "an expression nests deeper than 64 nodes",
            ))
        } else {
            self.expr_inner(e, expected)
        };
        self.nest -= 1;
        result
    }

    fn expr_inner(
        &mut self,
        e: &Expression<'a>,
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        match e {
            Expression::Literal { lit, .. } => self.literal(lit, expected),
            Expression::Reference {
                name, widths, args, ..
            } => {
                if args.is_some() || !widths.is_empty() {
                    self.call(&name.segments, widths, args.as_deref(), expected)
                } else {
                    self.name(&name.segments)
                }
            }
            Expression::Unary { op, operand, .. } => self.unary(*op, operand, expected),
            Expression::Binary {
                op, left, right, ..
            } => self.binary(*op, left, right, expected),
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let (c_ir, c_ty) = self.expr(condition, Some(&Ty::Bool))?;
                if c_ty != Ty::Bool {
                    return Err(self.type_err("an `if` condition must be Bool"));
                }
                let ((t_ir, t_ty), (o_ir, o_ty)) =
                    if needs_context(then_branch) && !needs_context(else_branch) {
                        let o = self.expr(else_branch, expected)?;
                        let hint = if expected.is_none() {
                            Some(&o.1)
                        } else {
                            expected
                        };
                        let t = self.expr(then_branch, hint)?;
                        (t, o)
                    } else {
                        let t = self.expr(then_branch, expected)?;
                        let hint = if expected.is_some() {
                            expected
                        } else {
                            Some(&t.1)
                        };
                        let o = self.expr(else_branch, hint)?;
                        (t, o)
                    };
                let common = Ty::unify(&t_ty, &o_ty)
                    .ok_or_else(|| self.type_err("the branches of an `if` have no common type"))?;
                Ok((
                    Ir::If(Box::new(c_ir), Box::new(t_ir), Box::new(o_ir)),
                    common,
                ))
            }
            Expression::List { .. } | Expression::Tuple { .. } | Expression::Project { .. } => {
                Err(self.level(Beyond::IrData, "tuples, lists and projections need ir.data"))
            }
            Expression::Match { .. } => Err(self.level(Beyond::IrData, "match needs ir.data")),
            Expression::Quantified { .. } => {
                Err(self.type_err("a quantifier is only allowed at the head of a claim"))
            }
            // Stripped before the match; kept as an explicit arm so adding a variant to
            // the grammar's `Expression` is a compile error here, never a silent skip.
            Expression::Grouped { .. } => Err(self.type_err("unexpected group")),
        }
    }

    fn literal(&self, lit: &Literal<'_>, expected: Option<&Ty>) -> Result<(Ir, Ty), GenError> {
        match lit {
            Literal::Bool(b) => Ok((Ir::Bool(*b), Ty::Bool)),
            Literal::Int(v) => {
                let ty = if matches!(expected, Some(Ty::Nat)) {
                    Ty::Nat
                } else {
                    Ty::Int
                };
                Ok((
                    Ir::Int {
                        ty: ty.clone(),
                        value: i128::from(*v),
                    },
                    ty,
                ))
            }
            Literal::U8(v) => self.fixed_literal(FixedTy::U8, i128::from(*v)),
            Literal::U64(v) => self.fixed_literal(FixedTy::U64, i128::from(*v)),
            Literal::Fixed(t, v) => self.fixed_literal(*t, *v),
            Literal::Bytes(hex) => {
                let ty = match expected {
                    Some(Ty::Bytes(Some(Width::Lit(n)))) if hex.len() as u64 / 2 == *n => {
                        expected.cloned().unwrap_or(Ty::BYTES)
                    }
                    _ => Ty::BYTES,
                };
                Ok((Ir::Bytes((*hex).to_string()), ty))
            }
        }
    }

    /// A suffixed literal.  The parser folds `-128i8` into one literal holding `-128`;
    /// VER04j 2.2 and the oracle keep both nodes, `["neg", lit -128]`, and `neg` of the
    /// minimum wraps to itself (VER04r 3.2).
    fn fixed_literal(&self, t: FixedTy, v: i128) -> Result<(Ir, Ty), GenError> {
        if !t.contains(v) {
            return Err(self.type_err("a suffixed literal is out of range"));
        }
        let ty = Ty::Fixed(t);
        let lit = Ir::Int {
            ty: ty.clone(),
            value: v,
        };
        if v < 0 {
            // Only the signed minimum is folded by the parser (VER04j 2.2); any other
            // negative literal is not a literal.
            if v != t.min() {
                return Err(self.type_err("a negative suffixed literal is only the signed minimum"));
            }
            Ok((Ir::Neg(ty.clone(), Box::new(lit)), ty))
        } else {
            Ok((lit, ty))
        }
    }

    fn name(&self, segments: &[&str]) -> Result<(Ir, Ty), GenError> {
        if segments.len() == 1 {
            let n = segments[0];
            if let Some(ty) = self.scope.get(n) {
                return Ok((Ir::Var(n.to_string()), ty.clone()));
            }
            if n == "None" || self.unsupported_types.contains(n) || self.is_ctor(n) {
                return Err(self.level(Beyond::IrData, "constructors need ir.data"));
            }
            return Err(self.type_err("an unbound name"));
        }
        if self.scope.contains_key(segments[0]) {
            return Err(self.level(Beyond::IrData, "projection needs ir.data"));
        }
        Err(self.type_err("a qualified name is only valid inside Err(...)"))
    }

    fn is_ctor(&self, name: &str) -> bool {
        self.file.declarations.iter().any(|d| match d {
            Declaration::Variant(v) => v.ctors.iter().any(|c| c.name == name),
            _ => false,
        })
    }

    fn unary(
        &mut self,
        op: UnaryOp,
        operand: &Expression<'a>,
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        if op == UnaryOp::Not {
            let (ir, ty) = self.expr(operand, Some(&Ty::Bool))?;
            if ty != Ty::Bool {
                return Err(self.type_err("`not` needs a Bool"));
            }
            return Ok((Ir::Not(Box::new(ir)), Ty::Bool));
        }
        let hint = expected.filter(|t| t.is_numeric());
        let (ir, ty) = self.expr(operand, hint)?;
        if !ty.is_numeric() {
            return Err(self.type_err("unary minus needs a number"));
        }
        if let Ty::Fixed(f) = &ty {
            if !f.signed() {
                return Err(self.type_err("unary minus on an unsigned type; write 0uN - a"));
            }
        }
        let result = if ty.is_fixed() { ty } else { Ty::Int };
        Ok((Ir::Neg(result.clone(), Box::new(ir)), result))
    }

    fn binary(
        &mut self,
        op: BinaryOp,
        left: &Expression<'a>,
        right: &Expression<'a>,
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        if let BinaryOp::And | BinaryOp::Or | BinaryOp::Implies = op {
            let (l, lt) = self.expr(left, Some(&Ty::Bool))?;
            let (r, rt) = self.expr(right, Some(&Ty::Bool))?;
            if lt != Ty::Bool || rt != Ty::Bool {
                return Err(self.type_err("boolean operators need Bool operands"));
            }
            let kind = match op {
                BinaryOp::And => Logic::And,
                BinaryOp::Or => Logic::Or,
                _ => Logic::Implies,
            };
            return Ok((Ir::Logic(kind, Box::new(l), Box::new(r)), Ty::Bool));
        }
        let arithmetic = matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul);
        // Type the self-typed side first so a bare literal on the other side picks up
        // its type.
        let hint = if arithmetic { expected } else { None };
        let ((l_ir, l_ty), (r_ir, r_ty)) = if needs_context(left) && !needs_context(right) {
            let r = self.expr(right, hint)?;
            let l = self.expr(left, Some(&r.1))?;
            (l, r)
        } else {
            let l = self.expr(left, hint)?;
            let r = self.expr(right, Some(&l.1))?;
            (l, r)
        };
        let common =
            Ty::unify(&l_ty, &r_ty).ok_or_else(|| self.type_err("operands have no common type"))?;
        let (l, r) = (Box::new(l_ir), Box::new(r_ir));
        if let BinaryOp::Eq | BinaryOp::NotEq = op {
            if self.nest == 1 {
                self.top_eq_types = Some((l_ty.clone(), r_ty.clone()));
            }
            let c = if op == BinaryOp::Eq { Cmp::Eq } else { Cmp::Ne };
            return Ok((Ir::Cmp(c, l, r), Ty::Bool));
        }
        if !common.is_numeric() {
            return Err(self.type_err("an ordering or arithmetic operator needs numbers"));
        }
        if l_ty.is_fixed() != r_ty.is_fixed() {
            return Err(self.type_err("a fixed-width integer never mixes with Int or Nat"));
        }
        if !arithmetic {
            let c = match op {
                BinaryOp::Lt => Cmp::Lt,
                BinaryOp::Le => Cmp::Le,
                BinaryOp::Gt => Cmp::Gt,
                _ => Cmp::Ge,
            };
            return Ok((Ir::Cmp(c, l, r), Ty::Bool));
        }
        // Arithmetic: fixed-width stays fixed (and wraps); Nat/Int become Int.
        let result = if common.is_fixed() { common } else { Ty::Int };
        let kind = match op {
            BinaryOp::Add => Arith::Add,
            BinaryOp::Sub => Arith::Sub,
            _ => Arith::Mul,
        };
        Ok((Ir::Arith(kind, result.clone(), l, r), result))
    }

    // -- calls ---------------------------------------------------------------------------

    fn call(
        &mut self,
        segments: &[&str],
        widths: &[AstWidth<'_>],
        args: Option<&[Expression<'a>]>,
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        if segments.len() != 1 {
            return Err(self.type_err("a call head is a single name"));
        }
        let name = segments[0];
        if let Some(slot) = self.op_sigs.get(name) {
            let slot = slot.clone();
            return self.subject_call(name, slot, widths, args);
        }
        if self.model_names.contains(name) {
            return self.model_call(name, widths, args);
        }
        let Some(args) = args else {
            return Err(self.type_err("a callable needs an argument list"));
        };
        let arity = |n: usize| -> Result<(), GenError> {
            if args.len() != n || !widths.is_empty() {
                Err(GenError::new(code::TYPE, "", "wrong number of arguments"))
            } else {
                Ok(())
            }
        };
        match name {
            "len" => {
                arity(1)?;
                let (ir, ty) = self.expr(&args[0], None)?;
                if !ty.is_bytes() {
                    return Err(self.type_err("len needs a byte string"));
                }
                Ok((Ir::Len(Box::new(ir)), Ty::Nat))
            }
            "concat" => {
                arity(2)?;
                let (a, a_ty) = self.expr(&args[0], None)?;
                let (b, b_ty) = self.expr(&args[1], None)?;
                if !a_ty.is_bytes() || !b_ty.is_bytes() {
                    return Err(self.type_err("concat needs byte strings"));
                }
                // Widths add: concat(Bytes<1>, Bytes<30>) is Bytes<31>.
                let ty = match (&a_ty, &b_ty) {
                    (Ty::Bytes(Some(Width::Lit(x))), Ty::Bytes(Some(Width::Lit(y)))) => {
                        Ty::bytes_n(x.saturating_add(*y))
                    }
                    _ => Ty::BYTES,
                };
                Ok((Ir::Concat(Box::new(a), Box::new(b)), ty))
            }
            "take" | "drop" if args.len() == 2 && widths.is_empty() => {
                let (a, a_ty) = self.expr(&args[0], None)?;
                if !a_ty.is_bytes() {
                    return Err(self.type_err("take and drop need a byte string"));
                }
                let (n, n_ty) = self.expr(&args[1], Some(&Ty::Nat))?;
                if !matches!(n_ty, Ty::Nat | Ty::Int) {
                    return Err(self.type_err("a take/drop count is a Nat"));
                }
                let node = if name == "take" {
                    Ir::Take(Box::new(a), Box::new(n))
                } else {
                    Ir::Drop(Box::new(a), Box::new(n))
                };
                Ok((node, Ty::BYTES))
            }
            "Ok" => {
                arity(1)?;
                let Some(Ty::Result(ok_ty, err)) = expected else {
                    return Err(self.type_err("Ok needs a Result context"));
                };
                let (ir, ty) = self.expr(&args[0], Some(ok_ty))?;
                if !Ty::assignable(&ty, ok_ty) {
                    return Err(
                        self.type_err("an Ok payload does not have the Result's value type")
                    );
                }
                Ok((Ir::Ok(Box::new(ir)), Ty::Result(Box::new(ty), err.clone())))
            }
            "Err" => {
                arity(1)?;
                let Expression::Reference {
                    name: q,
                    widths: w,
                    args: None,
                    ..
                } = strip(&args[0])
                else {
                    return Err(self.type_err("Err takes a qualified error case"));
                };
                if q.segments.len() != 2 || !w.is_empty() {
                    return Err(self.type_err("Err takes a qualified error case"));
                }
                let (err_name, case) = (q.segments[0], q.segments[1]);
                match self.errors.get(err_name) {
                    Some(cases) if cases.iter().any(|c| c == case) => {}
                    _ => return Err(self.type_err("an unknown error case")),
                }
                let Some(Ty::Result(_, want)) = expected else {
                    return Err(self.type_err("Err needs a Result context"));
                };
                if want != err_name {
                    return Err(self.type_err("an error case of the wrong error type"));
                }
                Ok((
                    Ir::Err(case.to_string()),
                    expected.cloned().unwrap_or(Ty::Bool),
                ))
            }
            n if LEVEL_FIXED.contains(&n) => {
                Err(self.level(Beyond::IrFixed, "fixed-width built-ins need ir.fixed"))
            }
            n if LEVEL_DATA.contains(&n) => {
                Err(self.level(Beyond::IrData, "Option and list built-ins need ir.data"))
            }
            n if LEVEL_SEQ.contains(&n) => Err(self.level(
                Beyond::IrSeq,
                "sequence built-ins need the reserved ir.seq level",
            )),
            n if self.unsupported_types.contains(n) || self.is_ctor(n) => {
                Err(self.level(Beyond::IrData, "constructors need ir.data"))
            }
            _ => Err(self.type_err("an unknown callable")),
        }
    }

    fn args_against(
        &mut self,
        params: &[(String, Ty)],
        args: Option<&[Expression<'a>]>,
        statics: &[(String, u64)],
    ) -> Result<Vec<Ir>, GenError> {
        let Some(args) = args.filter(|a| a.len() == params.len()) else {
            return Err(self.type_err("wrong number of arguments"));
        };
        let mut out = Vec::with_capacity(args.len());
        for (arg, (_, pty)) in args.iter().zip(params) {
            let want = pty.substitute(statics);
            let (ir, ty) = self.expr(arg, Some(&want))?;
            if !Ty::assignable(&ty, &want) {
                return Err(self.type_err("an argument does not match its parameter type"));
            }
            out.push(ir);
        }
        Ok(out)
    }

    fn model_call(
        &mut self,
        name: &str,
        widths: &[AstWidth<'_>],
        args: Option<&[Expression<'a>]>,
    ) -> Result<(Ir, Ty), GenError> {
        let Some(entry) = self.models.get(name) else {
            let why = if self.current_model.as_deref() == Some(name) {
                "a model calls itself (VER04r 4.2 rule 1)"
            } else {
                "a model calls a model declared later (VER04r 4.2 rule 1)"
            };
            return Err(GenError::new(code::W0701, "", why));
        };
        if !widths.is_empty() {
            return Err(self.type_err("a model takes no static arguments"));
        }
        let (params, ret, outcome) = (
            entry.params.clone(),
            entry.ret.clone(),
            entry.outcome.clone(),
        );
        let (Some(params), Some(ret)) = (params, ret) else {
            // The callee's signature was refused: its refusal is this expression's.
            return Err(outcome
                .err()
                .unwrap_or_else(|| self.type_err("a model has no signature")));
        };
        let lowered = self.args_against(&params, args, &[])?;
        outcome?;
        Ok((
            Ir::Model {
                name: name.to_string(),
                args: lowered,
            },
            ret,
        ))
    }

    fn subject_call(
        &mut self,
        name: &str,
        slot: Slot<OpSig>,
        widths: &[AstWidth<'_>],
        args: Option<&[Expression<'a>]>,
    ) -> Result<(Ir, Ty), GenError> {
        let sig = slot?;
        if widths.len() != sig.statics.len() {
            return Err(self.type_err("wrong number of static arguments"));
        }
        let mut statics = Vec::new();
        let mut static_values = Vec::new();
        for (sname, w) in sig.statics.iter().zip(widths) {
            match w {
                AstWidth::Literal(n) => {
                    statics.push((sname.clone(), *n));
                    static_values.push(*n);
                }
                AstWidth::Named(_) => {
                    return Err(GenError::new(
                        code::W0701,
                        "",
                        "a named static width cannot enter corpus IR",
                    ))
                }
            }
        }
        let lowered = self.args_against(&sig.params, args, &statics)?;
        Ok((
            Ir::Subject {
                op: name.to_string(),
                args: lowered,
                widths: static_values,
            },
            sig.ret.substitute(&statics),
        ))
    }
}

/// `r0 and r1 and r2` folded **left** into binary nodes (`and(and(r0,r1),r2)`); `None`
/// when there are no refinements.  (The v1 corpus wrote an n-ary `and`; v2 does not.)
fn fold_and(mut refinements: Vec<Ir>) -> Option<Ir> {
    if refinements.is_empty() {
        return None;
    }
    let mut acc = refinements.remove(0);
    for next in refinements {
        acc = Ir::Logic(Logic::And, Box::new(acc), Box::new(next));
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    //! The defensive refusals the checker normally rules out first, reached by skipping
    //! the checker (the parser alone admits these documents).

    use super::*;
    use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
    use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
    use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

    fn unchecked(src: &str) -> Result<Lowered, GenError> {
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        lower_partial_unchecked(&file)?.into_lowered()
    }

    const OP: &str =
        "warrant u\n  op f(a: Bytes, b: Bytes) -> Bool\n  op g<N: Nat>(a: Bytes<N>) -> Bool\n";

    #[test]
    fn a_subject_call_in_a_model_body_is_w0701() {
        let e = unchecked(&format!("{OP}  model bad(a: Bytes) -> Bool = f(a, a)\n")).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.at, "bad");
        assert!(e.message.contains("model body"));
    }

    #[test]
    fn a_model_that_calls_itself_is_w0701() {
        let e = unchecked(&format!("{OP}  model m(a: U8) -> U8 = m(a)\n")).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("itself"));
    }

    #[test]
    fn the_reserved_sequence_built_ins_name_ir_seq() {
        for name in ["fold", "map", "all", "any", "count"] {
            let src = format!("{OP}  model m(a: Bytes) -> Bool = {name}(a, a)\n");
            let e = unchecked(&src).unwrap_err();
            assert_eq!(e.level, Some("ir.seq"), "{name}");
        }
    }

    #[test]
    fn a_named_static_width_cannot_enter_corpus_ir() {
        let src = format!("{OP}  claim c:\n    forall a: Bytes<4>. g<N>(a) == true\n");
        let e = unchecked(&src).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.at, "c");
        assert!(e.message.contains("named static width"));
        // the literal form is fine
        let ok = format!("{OP}  claim c:\n    forall a: Bytes<4>. g<4>(a) == true\n");
        assert!(unchecked(&ok).is_ok());
    }

    #[test]
    fn more_malformed_documents_are_refused_as_type_errors() {
        let cases = [
            (
                "a non-Bool op refinement",
                "  op h(a: U8 | a, b: U8) -> Bool\n",
            ),
            (
                "a subject call in an op refinement",
                "  op h(a: Bytes | f(a, a), b: Bytes) -> Bool\n",
            ),
            ("an example that is not Bool", "  example x:\n    1\n"),
            (
                "an if condition that is not Bool",
                "  model m(a: U8) -> U8 = if a then a else a\n",
            ),
            ("negating a Bool", "  model m(a: Bool) -> Bool = -a\n"),
            ("ordering Bools", "  model m(a: Bool) -> Bool = a < a\n"),
            ("a dotted call head", "  model m(a: U8) -> U8 = a.b(a)\n"),
            (
                "a callable with no argument list",
                "  model m(a: U8) -> U8 = zork<4>\n",
            ),
            (
                "take of a number",
                "  model m(a: U8) -> Bytes = take(a, 1)\n",
            ),
            (
                "take with a Bool count",
                "  model m(a: Bytes) -> Bytes = take(a, true)\n",
            ),
            (
                "an Ok payload of the wrong type",
                "  error E = X\n  model m(a: U8) -> Result<U8, E> = Ok(true)\n",
            ),
            (
                "Err of a non-name",
                "  error E = X\n  model m(a: U8) -> Result<U8, E> = Err(a)\n",
            ),
            (
                "Err of a three-part name",
                "  error E = X\n  model m(a: U8) -> Result<U8, E> = Err(E.X.Y)\n",
            ),
            (
                "Err with no Result context",
                "  error E = X\n  model m(a: U8) -> U8 = Err(E.X)\n",
            ),
            (
                "a model with static arguments",
                "  model m(a: U8) -> U8 = a\n  model n(a: U8) -> U8 = m<4>(a)\n",
            ),
            (
                "an op called without its static width",
                "  claim c:\n    forall a: Bytes<4>. g(a) == true\n",
            ),
            (
                "a self-referential alias chain",
                "  model A = B\n  model B = A\n  model m(a: A) -> Bool = true\n",
            ),
        ];
        for (what, extra) in cases {
            let e = unchecked(&format!("{OP}{extra}")).expect_err(what);
            assert!(
                matches!(e.code, code::TYPE | code::W0701 | code::LIMIT),
                "{what}: expected a type or limit refusal, got {e}"
            );
        }
    }

    #[test]
    fn an_if_without_a_context_types_its_literal_branch_from_the_other_branch() {
        let src = format!(
            "{OP}  claim c:\n    forall a: Nat. f(b\"00\", b\"00\") and (if true then 1 else a) == a and (if true then a else 1) == a\n"
        );
        let l = unchecked(&src).unwrap();
        // both literals became Nat from the typed sibling branch
        let text = format!("{:?}", l.suites[0].body);
        assert!(!text.contains("ty: Int, value: 1"), "{text}");
        assert!(text.contains("ty: Nat, value: 1"), "{text}");
    }

    #[test]
    fn a_claim_that_calls_no_operation_is_w0701() {
        // L5: nothing to test, and `expected_calls` would be zero.
        let src = format!("{OP}  claim c:\n    forall a: Nat. a == a\n");
        let e = unchecked(&src).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.at, "c");
        assert!(e.message.contains("at least one operation"));
        // a ground claim with a call is fine, and so is one that calls through a model's
        // argument position only
        let ok = format!("{OP}  claim c:\n    f(b\"00\", b\"00\") == true\n");
        assert!(unchecked(&ok).is_ok());
    }

    #[test]
    fn the_result_type_comes_from_the_first_typing_pass_not_a_retype_without_context() {
        // L6: the subject side `pick(..)` is `Result<Bytes, E>` and the pure side is a bare
        // `Ok(..)`; both orders must give the declared type of the subject side.
        let src = "warrant u\n  op pick(a: Bytes, b: Bytes, c: Bool) -> Result<Bytes, E> fresh\n  error E = BAD\n  claim left:\n    forall a: Bytes, b: Bytes, c: Bool. pick(a, b, c) == Ok(a)\n  claim right:\n    forall a: Bytes, b: Bytes, c: Bool. Ok(a) == pick(a, b, c)\n";
        let l = unchecked(src).unwrap();
        for s in &l.suites {
            let SuiteKind::Reference { result, .. } = &s.kind else {
                panic!("{} is a reference suite", s.claim)
            };
            assert_eq!(result.name(), "Result<Bytes,E>", "{}", s.claim);
        }
    }

    #[test]
    fn expression_nesting_past_64_is_a_limit_error() {
        let deep = format!("{}true", "not ".repeat(70));
        let src = format!("{OP}  model m(a: U8) -> Bool = {deep}\n");
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let limits = ParserLimits {
            max_depth: 128,
            ..ParserLimits::default()
        };
        let file = parse_document(&layout, &doc, limits).unwrap();
        let e = lower_partial_unchecked(&file).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        // 60 levels is fine
        let ok = format!("{}true", "not ".repeat(30));
        let src = format!("{OP}  model m(a: U8) -> Bool = {ok}\n");
        assert!(unchecked(&src).is_ok());
    }

    #[test]
    fn a_suffixed_literal_out_of_range_is_a_type_error() {
        let doc = SourceDocument::admit(OP.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        let lw = Lowerer::new(&file);
        assert_eq!(
            lw.fixed_literal(FixedTy::U8, 300).unwrap_err().code,
            code::TYPE
        );
        assert_eq!(
            lw.fixed_literal(FixedTy::U8, -1).unwrap_err().code,
            code::TYPE
        );
        let (ir, ty) = lw.fixed_literal(FixedTy::I8, -128).unwrap();
        assert_eq!(ty, Ty::Fixed(FixedTy::I8));
        assert!(matches!(ir, Ir::Neg(..)), "the minimum keeps both nodes");
        let (ir, _) = lw.fixed_literal(FixedTy::U8, 255).unwrap();
        assert!(matches!(ir, Ir::Int { value: 255, .. }));
    }

    #[test]
    fn a_projection_on_a_binder_needs_ir_data() {
        let e = unchecked(&format!("{OP}  model m(a: U8) -> U8 = a.zz\n")).unwrap_err();
        assert_eq!(e.level, Some("ir.data"));
    }

    #[test]
    fn type_errors_the_checker_would_also_catch_are_type_errors_here() {
        let cases = [
            ("a model body of the wrong type", "  model m(a: U8) -> Bool = a\n"),
            ("an unbound name", "  model m(a: U8) -> U8 = zz\n"),
            ("an if with unrelated branches", "  model m(a: U8) -> U8 = if true then a else true\n"),
            ("an unsigned negation", "  model m(a: U8) -> U8 = -a\n"),
            ("mixed fixed and Nat", "  model m(a: U8, n: Nat) -> Bool = a < n\n"),
            ("not of a number", "  model m(a: U8) -> Bool = not a\n"),
            ("and of numbers", "  model m(a: U8) -> Bool = a and a\n"),
            ("an unknown callable", "  model m(a: U8) -> U8 = zork(a)\n"),
            ("len of a number", "  model m(a: U8) -> Nat = len(a)\n"),
            ("concat of numbers", "  model m(a: U8) -> Bytes = concat(a, a)\n"),
            ("a bare Ok", "  model m(a: U8) -> U8 = Ok(a)\n"),
            ("a bare Err", "  model m(a: U8) -> U8 = Err(E.X)\n"),
            ("a qualified name", "  model m(a: U8) -> U8 = zz.yy\n"),
            ("a model with a refinement", "  model m(a: U8 | a > 1u8) -> U8 = a\n"),
            ("an arity error", "  model m(a: U8) -> Nat = len(a, a)\n"),
            ("an op with the wrong arity", "  model m(a: Bytes) -> Bool = a == a\n  claim c:\n    forall a: Bytes. f(a) == true\n"),
            ("a model with a bad arity call", "  model m(a: U8) -> U8 = a\n  model n(a: U8) -> U8 = m(a, a)\n"),
            ("an op with a wrong argument type", "  claim c:\n    forall a: U8. f(a, a) == true\n"),
            ("a non-Bool claim", "  claim c:\n    forall a: Bytes. len(a)\n"),
            ("a duplicate binder", "  claim c:\n    forall a: Bytes, a: Bytes. f(a, a) == true\n"),
            ("a nested quantifier", "  claim c:\n    forall a: Bytes. forall b: Bytes. f(a, b) == true\n"),
            ("an unknown error case", "  error E = X\n  model m(a: U8) -> Result<U8, E> = Err(E.Y)\n"),
            ("an Err of the wrong error type", "  error E = X\n  error F = Y\n  model m(a: U8) -> Result<U8, E> = Err(F.Y)\n"),
            ("a non-Bool refinement", "  claim c:\n    forall a: Bytes | len(a). f(a, a) == true\n"),
            ("an unknown static width", "  op h(a: Bytes<M>) -> Bool\n"),
            ("a width past 4096", "  model m(a: Bytes<4097>) -> Bool = true\n"),
            ("an unknown type", "  model m(a: Zork) -> Bool = true\n"),
            ("an error used as a value type", "  error E = X\n  model m(a: E) -> Bool = true\n"),
            ("a Result of an undeclared error", "  model m(a: U8) -> Result<U8, Q> = a\n"),
        ];
        for (what, extra) in cases {
            let e = unchecked(&format!("{OP}{extra}")).expect_err(what);
            assert!(
                matches!(e.code, code::TYPE | code::W0701),
                "{what}: expected a type refusal, got {e}"
            );
        }
    }
}
