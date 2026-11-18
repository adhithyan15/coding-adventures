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
//! ## `ir.fixed` (slice S10b): lowered, typed, costed, never evaluated
//!
//! The 35 built-ins of VER04r Appendix A.1 are **lowered** (one [`Ir::Fixed`] node per call,
//! the operand types inferred as for `add`, a static type argument written as the canonical
//! type name in position 1) and each lowered unit records its `requires` (R9: the sorted union
//! of the levels of its tags and of its callees' own `requires`, `ir.core` always present).  A
//! unit whose `requires` holds `ir.fixed` is **refused** by [`lower_document`] and by the census
//! with the level `ir.fixed`, exactly as before S10b, so `generate` stays a fail-closed stub
//! for such documents; [`Partial::into_lowered_with_fixed`] hands the IR to the parity dump
//! (`tests/fixed_parity.rs`), which is what S10b has to reproduce byte for byte.
//!
//! ## `ir.tuple` (slice S10t-2): tuples, `project` and `match` on a `Result`
//!
//! The tuple type `(T1,..,Tn)`, the literal `["tuple", [..]]`, `["project", e, i]` (also `t.i`,
//! `t.0.1`, `f(x).1`) and `match` on a `Result` with the six patterns of Appendix A.1 are lowered
//! like the core forms, written from the spec and compared byte for byte with the oracle's
//! committed dumps (`tests/tuple_parity.rs`).  What is different from a core form:
//!
//! ```text
//!   patterns    lowered and checked BEFORE any arm body (so a corpus error cannot hide behind
//!               a body refused at a level): duplicate bind, shadowing bind, then, per match,
//!               unreachable arms in arm order, then fall-through (R12); all of them `ir`
//!   Err(e)      lowers to ["err_any"]: an error value has no IR form, so an arm that *uses* `e`
//!               is refused at `ir.data`; `Err(_)` is the same node
//!   arm typing  an arm that needs a context takes the first self-typed arm's type, as `if`
//!               does; the arms must unify, else a `type` error (not a corpus error)
//!   scrutinee   `match` is `ir.tuple` exactly when the scrutinee's static type is a `Result`
//!               and `ir.data` for any other type
//!   requires    R9, the type half: a binder, parameter, result or sub-expression whose type is or
//!               contains a tuple adds `ir.tuple` even where no tuple tag appears
//! ```
//!
//! The grammar crate's checker enforces the same match rules (`W0301`, `W0203`), so the corpus
//! errors usually reach this generator as a checker refusal; [`check_or_refuse`] maps those three
//! kinds, and only those, to the generator code `ir`, naming the precise corpus error by
//! re-lowering the document without the checker.
//!
//! ## Static widths flow to `Bytes<N>` everywhere (VER04r A.1, slice S10t)
//!
//! A `concat` of two statically sized operands, a bytes literal and an `if` of equal widths are
//! `Bytes<N>` to the checker.  This typer forgets the width in the *type* (it stays `Bytes`) and
//! records it as the node is lowered; [`Lowerer::flows`] reads it back wherever a value flows to
//! a `Bytes<N>`: a model body to its declared result, an `Ok` payload, an argument to its
//! parameter, and the operand of `from_be`/`from_le`.  (Before S10t only the last did.)
//!
//! `Option<T>` is an IR-only intermediate (D23, R8): nine built-ins produce one and
//! `unwrap_or` consumes it, as its first operand and directly; anywhere else it is refused with
//! the level `ir.data` (it is [`Ty::Option`], and it never reaches a signature).
//!
//! A refused unit is named by the **first construct, in source order, that cannot be lowered**;
//! it is named `ir.fixed` only when nothing else blocks it (a built-in lowers, so a later tuple
//! or constructor in the same unit gives the unit that level, the oracle's rule).
//!
//! The refusals, by level:
//!
//! ```text
//!   ir.fixed   a unit that lowers and uses a built-in of Appendix A.1 (see above)
//!   ir.tuple   tuple types, the tuple literal, projection, `match` on a `Result`
//!   ir.data    list literals, `match` on anything but a `Result`, constructors, `Option` and
//!              `List` types, records, variants; Some None fixed slice to_list to_bytes
//!              replicate range, and an `Option` that is not the first operand of `unwrap_or`
//!   ir.seq     fold map all any count (reserved)
//!   inout / ensures / vectors / assume   WX-G, WX-E and assumption rows
//! ```
//!
//! `exists`, a named static width at a call site, a subject call inside a model body, a
//! subject call under an impure guard (VER04r 4.4) and, in a **v0** document, a
//! fixed-width `+ - *` or unary `-` with a subject call beneath it (VER04r 3.3 rule 3) are
//! `W0701` refusals: the document is well typed but the corpus cannot express it honestly.

use std::collections::{BTreeSet, HashMap, HashSet};

use coding_adventures_warrant_grammar::checker::{check_warrant, CheckerError, CheckerErrorKind};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::parser::{
    AssumptionsDecl, BinaryOp, ClaimDecl, Declaration, ExampleDecl, Expression, Literal, MatchArm,
    ModelDecl, OpDecl, Parameter, Pattern, ProjKey, TypeExpr, UnaryOp, Version, WarrantFile,
    Width as AstWidth,
};

use crate::analysis::{check_pure_guards, check_v0_static_arith};
use crate::error::{code, Beyond, GenError};
use crate::ir::{
    self, Arith, Cmp, Ir, Logic, Pat, MAX_IR_DEPTH, OPTION_PRODUCERS, STATIC_TYPE_TAGS,
};
use crate::json::Json;
use crate::patterns::{self, MAX_PATTERN_DEPTH, MAX_TUPLE_ARITY};
use crate::types::{Ty, Width};

/// Expression nesting limit while lowering (VER04 section 2: "64 nested expression nodes
/// on any path"; VER04r 14: IR depth 64).  Parentheses do not count: a group vanishes.
pub const MAX_EXPR_DEPTH: usize = MAX_IR_DEPTH;

/// Type nesting limit (VER04r 14: type depth 16).
const MAX_TYPE_DEPTH: usize = 16;

/// The longest `Bytes<N>` (VER04h section 3, `MAX_FIXED_WIDTH`).
const MAX_FIXED_WIDTH: u64 = 4096;

/// Constructors and sequence built-ins of `ir.data` (`unwrap_or` is `ir.fixed` since S10b).
const LEVEL_DATA: [&str; 8] = [
    "Some",
    "None",
    "fixed",
    "slice",
    "to_list",
    "to_bytes",
    "replicate",
    "range",
];

/// Why an `Option` is refused anywhere but under `unwrap_or` (D23).
const OPTION_MESSAGE: &str = "Option is an IR-only intermediate (its only consumer is unwrap_or)";

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
    /// The sorted levels the `admit` needs (R9); `ir.core` is always present.
    pub requires: Vec<&'static str>,
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
            ("requires".to_string(), Json::strs(self.requires.iter())),
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
    /// The sorted levels of the body and of its callees (R9; a model has no `requires` of its
    /// own in a corpus, section 1.4, but a caller reads it instead of walking the closure).
    pub requires: Vec<&'static str>,
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
    /// The sorted levels of `admit`, `body` and their model closure (R9).
    pub requires: Vec<&'static str>,
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
    /// Its decomposition: exactly one subject call compared with a pure value.  Anything
    /// else is refused at lowering (`W0701`) **unless** the profile has an `omit_example` line
    /// for the example (VER04r 8.4, [D34]): then the shape decision is deferred and this is
    /// `None`, which the example stage reads as the reason `not-op-eq-pure`.  Only an
    /// omitted example can be `None`.
    pub reference: Option<ExampleRef>,
    /// The sorted levels of the body and its model closure (R9).
    pub requires: Vec<&'static str>,
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
    /// examples.  An item that lowered but needs `ir.fixed` is refused with that level.
    pub fn first_block(&self) -> Option<GenError> {
        self.blocks(true).into_iter().next()
    }

    /// The refusals in the order of [`Partial::first_block`]; `fixed_blocks` says whether an
    /// item that lowered but needs `ir.fixed` counts as one.
    fn blocks(&self, fixed_blocks: bool) -> Vec<GenError> {
        let mut out: Vec<GenError> = self.declarations.clone();
        fn push<T: Leveled>(out: &mut Vec<GenError>, slots: &[(String, Slot<T>)], fixed: bool) {
            out.extend(slots.iter().filter_map(|(n, s)| blocked(s, n, fixed)));
        }
        push(&mut out, &self.ops, fixed_blocks);
        push(&mut out, &self.models, fixed_blocks);
        push(&mut out, &self.suites, fixed_blocks);
        push(&mut out, &self.examples, fixed_blocks);
        out
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
            push(ItemKind::Op, n, blocked(s, n, true));
        }
        for (n, s) in &self.models {
            push(ItemKind::Model, n, blocked(s, n, true));
        }
        for (n, s) in &self.suites {
            push(ItemKind::Claim, n, blocked(s, n, true));
        }
        for (n, s) in &self.examples {
            push(ItemKind::Example, n, blocked(s, n, true));
        }
        rows
    }

    /// The models that lowered within `ir.core`, in declaration order (for the census's cost
    /// check).  A model that needs `ir.fixed` is refused by the census and left out.
    pub fn lowered_models(&self) -> Vec<LoweredModel> {
        self.models
            .iter()
            .filter_map(|(_, s)| s.as_ref().ok().filter(|m| !m.needs_fixed()).cloned())
            .collect()
    }

    /// The claims that lowered within `ir.core`, in declaration order.
    pub fn lowered_suites(&self) -> Vec<&LoweredSuite> {
        self.suites
            .iter()
            .filter_map(|(_, s)| s.as_ref().ok().filter(|s| !s.needs_fixed()))
            .collect()
    }

    /// The fully lowered document, or the first refusal (a unit that needs `ir.fixed` is one).
    pub fn into_lowered(self) -> Result<Lowered, GenError> {
        if let Some(block) = self.first_block() {
            return Err(block);
        }
        // `first_block` found nothing, so every slot is `Ok`.
        Ok(self.into_lowered_unchecked())
    }

    /// The fully lowered document **including its `ir.fixed` units**, or the first refusal of
    /// any other kind.  Nothing downstream may evaluate or emit what this returns: it exists
    /// for the lowering parity dump of slice S10b ([`crate::view::oracle_view_v1`]).
    pub fn into_lowered_with_fixed(self) -> Result<Lowered, GenError> {
        if let Some(block) = self.blocks(false).into_iter().next() {
            return Err(block);
        }
        Ok(self.into_lowered_unchecked())
    }

    fn into_lowered_unchecked(self) -> Lowered {
        Lowered {
            module: self.module,
            version: self.version,
            errors: self.errors,
            ops: oks(self.ops),
            models: oks(self.models),
            suites: oks(self.suites),
            examples: oks(self.examples),
        }
    }
}

/// An item with a computed `requires` (R9).
trait Leveled {
    fn levels(&self) -> &[&'static str];

    /// Does the item use a built-in of `ir.fixed` (directly or through a callee)?
    fn needs_fixed(&self) -> bool {
        self.levels().contains(&"ir.fixed")
    }
}

macro_rules! leveled {
    ($($t:ty),*) => {$(impl Leveled for $t {
        fn levels(&self) -> &[&'static str] { &self.requires }
    })*};
}
leveled!(LoweredOp, LoweredModel, LoweredSuite, LoweredExample);

/// The refusal that blocks one slot: its own, or (if `fixed`) `ir.fixed` for an item that
/// lowered but needs that level.
fn blocked<T: Leveled>(slot: &Slot<T>, name: &str, fixed: bool) -> Option<GenError> {
    match slot {
        Err(e) => Some(e.clone()),
        Ok(t) if fixed && t.needs_fixed() => Some(GenError::level(
            Beyond::IrFixed,
            name,
            "fixed-width built-ins need ir.fixed",
        )),
        Ok(_) => None,
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

/// [`lower_document`] for a document whose profile has `omit_example` lines: an example whose
/// full id (`warrant/<module>/example/<name>`) is in `omit` and whose shape is not
/// `op(args) == pure` is **classified, not refused**: its [`LoweredExample::reference`] is
/// `None` and the impure-guard and v0 rule-3 refusals of its body are not applied, as the
/// oracle's lowering never applies them to an example (VER04r 8.4).  Every other refusal
/// (a level, a type error, a limit) still refuses the document, and an example that is not
/// named in `omit` is refused exactly as in [`lower_document`].
///
/// # Errors
///
/// As [`lower_document`].
pub fn lower_document_omitting(
    file: &WarrantFile<'_>,
    omit: &[String],
) -> Result<Lowered, GenError> {
    check_or_refuse(file, || {
        let mut lowerer = Lowerer::new(file);
        lowerer.omit_examples = omit.to_vec();
        lowerer.run()
    })?;
    let mut lowerer = Lowerer::new(file);
    lowerer.omit_examples = omit.to_vec();
    lowerer.run()?.into_lowered()
}

/// [`lower_document`] that also accepts the `ir.fixed` units (S10b): the document lowered in
/// full, `ir.fixed` included, for the parity dump.  Everything else is refused as before.  The
/// result must never reach the evaluator or the emitter: its models and suites may hold
/// [`Ir::Fixed`] nodes, which only the lowering and the cost walk understand.
///
/// # Errors
///
/// As [`lower_document`], except that a unit that needs `ir.fixed` is not one.
#[doc(hidden)]
pub fn lower_document_with_fixed(file: &WarrantFile<'_>) -> Result<Lowered, GenError> {
    lower_partial(file)?.into_lowered_with_fixed()
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
    check_or_refuse(file, || lower_partial_unchecked(file))?;
    lower_partial_unchecked(file)
}
/// Run the S2 checker; a refusal is `code == "check"` (the checker's own diagnostic code in the
/// message), **except** the three that are corpus errors of Appendix A.1 and so the generator code
/// `ir` (VER04r 16.10 R2, R12): a `match` that falls through (`NonExhaustiveMatch`), an arm no
/// value can reach (`RedundantArm`), and a pattern name that repeats or shadows another
/// (`ShadowedBinding` raised at a match arm).  The checker enforces the same match rules as the
/// oracle's `patterns.py`, so a corpus-error document is refused here before the lowerer sees it;
/// to name *which* corpus error (and in the oracle's order: patterns before bodies), the document
/// is lowered once more **without** the checker (`unchecked`) and that refusal is used when it is
/// an `ir` one.
///
/// A name the checker refuses as a bind and the oracle accepts (a bind named like a model, an op,
/// an error type, a variant constructor or a reserved built-in) is also `ir` here: the generator
/// is stricter than the oracle there, and says so with the shadowing sentence.
fn check_or_refuse(
    file: &WarrantFile<'_>,
    unchecked: impl FnOnce() -> Result<Partial, GenError>,
) -> Result<(), GenError> {
    let Err(e) = check_warrant(file) else {
        return Ok(());
    };
    let generic = match &e.kind {
        CheckerErrorKind::NonExhaustiveMatch => Some(patterns::FALLS_THROUGH),
        CheckerErrorKind::RedundantArm => Some(patterns::UNREACHABLE),
        CheckerErrorKind::ShadowedBinding(_) if is_match_arm(file, &e) => {
            Some(patterns::SHADOWING_BIND)
        }
        _ => None,
    };
    let Some(generic) = generic else {
        return Err(GenError::new(
            code::CHECK,
            "",
            format!("the document does not pass the checker ({})", e.code),
        ));
    };
    match unchecked() {
        Err(precise) if precise.code == code::IR => Err(precise),
        _ => Err(GenError::new(code::IR, "", generic)),
    }
}

/// Is the checker error located at a `match` arm of the document (its span is an arm's)?
fn is_match_arm(file: &WarrantFile<'_>, e: &CheckerError) -> bool {
    fn arms_of<'e, 'a>(root: &'e Expression<'a>, out: &mut Vec<&'e MatchArm<'a>>) {
        let mut stack = vec![root];
        while let Some(x) = stack.pop() {
            match x {
                Expression::Quantified { binders, body, .. } => {
                    stack.extend(binders.iter().filter_map(|b| b.refinement.as_ref()));
                    stack.push(body);
                }
                Expression::Conditional {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => stack.extend([&**condition, &**then_branch, &**else_branch]),
                Expression::Match { target, arms, .. } => {
                    stack.push(target);
                    for a in arms {
                        out.push(a);
                        stack.push(&a.body);
                    }
                }
                Expression::Binary { left, right, .. } => stack.extend([&**left, &**right]),
                Expression::Unary { operand, .. } => stack.push(operand),
                Expression::Grouped { inner, .. } => stack.push(inner),
                Expression::List { elements, .. } | Expression::Tuple { elements, .. } => {
                    stack.extend(elements)
                }
                Expression::Reference { args, .. } => {
                    stack.extend(args.iter().flat_map(|a| a.iter()))
                }
                Expression::Project { base, .. } => stack.push(base),
                Expression::Literal { .. } => {}
            }
        }
    }
    let mut arms = Vec::new();
    for d in &file.declarations {
        match d {
            Declaration::Model(m) => {
                if let Some(b) = &m.body {
                    arms_of(b, &mut arms);
                }
            }
            Declaration::Claim(c) => arms_of(&c.body, &mut arms),
            Declaration::Example(x) => arms_of(&x.body, &mut arms),
            Declaration::Op(o) => {
                for p in &o.params {
                    if let Some(r) = &p.refinement {
                        arms_of(r, &mut arms);
                    }
                }
            }
            _ => {}
        }
    }
    arms.iter().any(|a| a.span == e.span)
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
    /// The levels of the model's body and callees (R9), read by its callers.
    requires: Vec<&'static str>,
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
    /// Set by `unwrap_or` for exactly one `expr` call, its first operand: the one place an
    /// `Option` may be the type of an expression (D23, R8).  Taken at the start of that call.
    option_ok: bool,
    /// Set by the branch that lowers a `Bytes` literal, a `concat` or an `if` of equal widths
    /// just before it returns, and taken by [`Lowerer::expr`] into `last_width`.
    width_out: Option<u64>,
    /// The static width of the `Bytes` expression the most recent `expr` call lowered, if it
    /// is one the checker types `Bytes<n>` although this typer forgets the width (a literal,
    /// a `concat` of sized operands, an `if` of equal widths).  Read by `from_be`/`from_le`
    /// immediately after the operand's `expr` call: one lowering pass, never a re-lowering.
    last_width: Option<u64>,
    /// The full ids of the examples an `omit_example` profile line names (VER04r 8.4).
    omit_examples: Vec<String>,
    /// The levels the *types* met by the unit being lowered bring (binder, parameter, result, the
    /// type of every sub-expression): R9's type half, `ir.tuple` for a tuple.  Reset at the start
    /// of every unit and added to its `requires` by [`Lowerer::levels_of`].
    type_levels: BTreeSet<&'static str>,
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
        // A tuple takes the type of its components; one whose component has no type of its own (a
        // bare literal, `Ok(..)`) is typed from its sibling or its context.
        Expression::Tuple { elements, .. } => elements.iter().any(needs_context),
        Expression::Match { arms, .. } => arms.iter().all(|a| needs_context(&a.body)),
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
            option_ok: false,
            width_out: None,
            last_width: None,
            omit_examples: Vec::new(),
            type_levels: BTreeSet::new(),
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
                        let requires = self.levels_of(admit.iter());
                        if let Ok(l) = &mut ops[i].1 {
                            l.admit = admit;
                            l.requires = requires;
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

    /// The `requires` of a lowered unit (VER04r 16.10 R9): [`ir::levels`] over `trees` (the tag
    /// half, reading each callee's own `requires` from the model table) united with the levels of
    /// the **types** the unit met ([`Lowerer::type_levels`], the type half: a tuple type adds
    /// `ir.tuple` even where no tuple tag appears).  Sorted and duplicate-free, `ir.core` always.
    fn levels_of<'t>(&self, trees: impl IntoIterator<Item = &'t Ir>) -> Vec<&'static str> {
        let mut found: BTreeSet<&'static str> = ir::levels(trees, &|name| {
            self.models
                .get(name)
                .map_or_else(|| vec!["ir.core"], |e| e.requires.clone())
        })
        .into_iter()
        .collect();
        found.extend(self.type_levels.iter().copied());
        found.into_iter().collect()
    }

    /// Record the levels a type brings (R9, the type half).
    fn note_type(&mut self, ty: &Ty) {
        if ty.contains_tuple() {
            self.type_levels.insert("ir.tuple");
        }
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
            TypeExpr::Tuple { elems, .. } => {
                if !(2..=MAX_TUPLE_ARITY).contains(&elems.len()) {
                    return Err(self.type_err("a tuple has 2 to 8 components"));
                }
                let parts = elems
                    .iter()
                    .map(|e| self.resolve(e, statics, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Ty::Tuple(parts))
            }
            TypeExpr::List { .. } => Err(self.level(Beyond::IrData, "List types need ir.data")),
            TypeExpr::Option { .. } => Err(self.level(Beyond::IrData, OPTION_MESSAGE)),
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
        // R9, the type half: the signature alone may need `ir.tuple` (the admit, if any, is
        // folded in by `lower_op_admit`).
        let tuple = params.iter().any(|(_, t)| t.contains_tuple()) || ret.contains_tuple();
        Ok(LoweredOp {
            name: op.name.to_string(),
            statics,
            params,
            ret,
            fresh: op.is_fresh,
            admit: None,
            requires: if tuple {
                vec!["ir.core", "ir.tuple"]
            } else {
                vec!["ir.core"]
            },
        })
    }

    /// The parameters' refinements as one `and`-folded Bool tree.  The scope is the
    /// operation's parameters; a refinement is typed Bool and has no `subject_op`.
    fn lower_op_admit(&mut self, op: &OpDecl<'a>, lop: &LoweredOp) -> Result<Option<Ir>, GenError> {
        self.scope = lop.params.iter().cloned().collect();
        self.type_levels.clear();
        for (_, t) in &lop.params {
            self.note_type(t);
        }
        self.note_type(&lop.ret);
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
                        requires: model.requires.clone(),
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
                        requires: vec!["ir.core"],
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
        self.type_levels.clear();
        let mut params = Vec::new();
        for p in &decl.params {
            if p.inout {
                return Err(self.level(Beyond::Inout, "an `inout` parameter needs slice S12"));
            }
            if p.refinement.is_some() {
                return Err(self.type_err("a model parameter has no refinement"));
            }
            let ty = self.resolve(&p.param_type, &[], 0)?;
            self.note_type(&ty);
            params.push((p.name.to_string(), ty));
        }
        let ret = self.resolve(&decl.target_type, &[], 0)?;
        self.note_type(&ret);
        let body = decl
            .body
            .as_ref()
            .ok_or_else(|| self.type_err("a model has no body"))?;
        self.scope = params.iter().cloned().collect();
        let (ir, ty) = self.expr(body, Some(&ret))?;
        if !self.flows(&ty, &ret, self.last_width) {
            return Err(self.type_err("a model body does not have its declared type"));
        }
        if ir.has_subject() {
            return Err(GenError::new(
                code::W0701,
                "",
                "a subject call inside a model body (VER04r 4.2 rule 3)",
            ));
        }
        let requires = self.levels_of([&ir]);
        Ok(LoweredModel {
            name: decl.name.to_string(),
            params,
            ret,
            body: ir,
            requires,
        })
    }

    // -- claims and examples -------------------------------------------------------

    fn lower_claim(&mut self, decl: &ClaimDecl<'a>) -> Result<LoweredSuite, GenError> {
        let id = format!("warrant/{}/exhaustive/{}", self.module, decl.name);
        self.scope = HashMap::new();
        self.type_levels.clear();
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
        let requires = self.levels_of([&admit, &body]);
        Ok(LoweredSuite {
            id,
            claim: decl.name.to_string(),
            binders,
            admit,
            body,
            kind,
            ops,
            requires,
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
        self.note_type(&ty);
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
        self.type_levels.clear();
        let body_ast = strip(&decl.body);
        self.top_eq_types = None;
        let (body, ty) = self.expr(body_ast, Some(&Ty::Bool))?;
        if ty != Ty::Bool {
            return Err(self.type_err("an example body must be Bool"));
        }
        // The `examples` row of VER04r 1.4 can only say `op(literals) == pure`: exactly one
        // subject call, its arguments free of subject calls, on one side of an equality.
        // The shape is decided first, so that an example the profile omits ([D34]) can be
        // classified `not-op-eq-pure` without the guard refusals below, which the oracle's
        // lowering never applies to an example.
        let reference = match self.classify(&body)? {
            SuiteKind::Reference {
                subject: subject @ Ir::Subject { .. },
                expected,
                result,
            } if subject.children().iter().all(|a| !a.has_subject()) => Some(ExampleRef {
                subject,
                expected,
                result,
            }),
            _ => None,
        };
        let id = format!("warrant/{}/example/{}", self.module, decl.name);
        let deferred = reference.is_none() && self.omit_examples.contains(&id);
        if !deferred {
            self.v0_guard(&body)?;
            check_pure_guards(&body)?;
            if reference.is_none() {
                return Err(GenError::new(
                    code::W0701,
                    "",
                    "an example must be one subject call compared with a pure value (VER04r 1.4)",
                ));
            }
        }
        let requires = self.levels_of([&body]);
        Ok(LoweredExample {
            name: decl.name.to_string(),
            body,
            reference,
            requires,
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
        // Taken before any operand is lowered: it is for this node alone.
        let option_ok = std::mem::take(&mut self.option_ok);
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
        // Every operand's `expr` call took its own `width_out`, so what is left is this node's.
        self.last_width = self.width_out.take();
        let (ir, ty) = result?;
        // D23, R8: an `Option` is the type of an expression only as the first operand of
        // `unwrap_or`; anywhere else it is `ir.data`.
        if matches!(ty, Ty::Option(_)) && !option_ok {
            return Err(self.level(Beyond::IrData, OPTION_MESSAGE));
        }
        self.note_type(&ty);
        Ok((ir, ty))
    }

    fn expr_inner(
        &mut self,
        e: &Expression<'a>,
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        match e {
            Expression::Literal { lit, .. } => {
                let lowered = self.literal(lit, expected)?;
                if let Literal::Bytes(hex) = lit {
                    self.width_out = Some(hex.len() as u64 / 2);
                }
                Ok(lowered)
            }
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
                // Each branch's static width is read right after its own `expr` call.
                let ((t_ir, t_ty, wt), (o_ir, o_ty, wo)) =
                    if needs_context(then_branch) && !needs_context(else_branch) {
                        let o = self.expr(else_branch, expected)?;
                        let wo = self.last_width;
                        let hint = if expected.is_none() {
                            Some(&o.1)
                        } else {
                            expected
                        };
                        let t = self.expr(then_branch, hint)?;
                        let wt = self.last_width;
                        ((t.0, t.1, wt), (o.0, o.1, wo))
                    } else {
                        let t = self.expr(then_branch, expected)?;
                        let wt = self.last_width;
                        let hint = if expected.is_some() {
                            expected
                        } else {
                            Some(&t.1)
                        };
                        let o = self.expr(else_branch, hint)?;
                        let wo = self.last_width;
                        ((t.0, t.1, wt), (o.0, o.1, wo))
                    };
                let common = Ty::unify(&t_ty, &o_ty)
                    .ok_or_else(|| self.type_err("the branches of an `if` have no common type"))?;
                // The checker's `common_type`: two branches of the same static width stay
                // `Bytes<n>`; anything else forgets it.
                let (wt, wo) = (static_width(&t_ty, wt), static_width(&o_ty, wo));
                if wt.is_some() && wt == wo {
                    self.width_out = wt;
                }
                Ok((
                    Ir::If(Box::new(c_ir), Box::new(t_ir), Box::new(o_ir)),
                    common,
                ))
            }
            Expression::List { .. } => Err(self.level(Beyond::IrData, "lists need ir.data")),
            Expression::Tuple { elements, .. } => self.tuple_literal(elements, expected),
            Expression::Project { base, key, .. } => {
                let (b_ir, b_ty) = self.expr(base, None)?;
                match (&b_ty, key) {
                    (Ty::Tuple(_), ProjKey::Index(i)) => {
                        let text = i.to_string();
                        self.project(b_ir, b_ty, &[text.as_str()])
                    }
                    (Ty::Tuple(_), ProjKey::Field(_)) => {
                        Err(self.type_err("a tuple projection is a component index of a tuple"))
                    }
                    _ => Err(self.type_err("a projection needs a tuple")),
                }
            }
            Expression::Match { target, arms, .. } => self.match_expr(target, arms, expected),
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

    fn name(&mut self, segments: &[&str]) -> Result<(Ir, Ty), GenError> {
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
        if let Some(head_ty) = self.scope.get(segments[0]).cloned() {
            if matches!(head_ty, Ty::Tuple(_)) {
                return self.project(Ir::Var(segments[0].to_string()), head_ty, &segments[1..]);
            }
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
                let wa = self.last_width;
                let (b, b_ty) = self.expr(&args[1], None)?;
                let wb = self.last_width;
                if !a_ty.is_bytes() || !b_ty.is_bytes() {
                    return Err(self.type_err("concat needs byte strings"));
                }
                // The checker types a `concat` of two sized operands `Bytes<sum>`.
                if let (Some(x), Some(y)) = (static_width(&a_ty, wa), static_width(&b_ty, wb)) {
                    self.width_out = Some(x.saturating_add(y));
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
                if !self.flows(&ty, ok_ty, self.last_width) {
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
            n if ir::fixed_tag(n).is_some() => self.fixed_call(n, widths, Some(args)),
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

    // -- ir.tuple: tuples, projection and match on a Result (slice S10t-2) ---------------------

    /// `(a, b, ..)` -> `["tuple", [a, b, ..]]` of type `(Ta,Tb,..)` (A.1, R2).
    ///
    /// A component is typed with the matching component of `expected` when that is a tuple of the
    /// same arity (so `(low7(b0), 1)` in a `Result<(U64,Nat),E>` context has a `Nat` literal), and
    /// on its own otherwise.  Two to eight components: the parser has already refused nine, and a
    /// one-component parenthesis is not a tuple.
    fn tuple_literal(
        &mut self,
        elements: &[Expression<'a>],
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        let n = elements.len();
        if !(2..=MAX_TUPLE_ARITY).contains(&n) {
            return Err(self.type_err("a tuple has 2 to 8 components"));
        }
        let hints: Vec<Option<Ty>> = match expected {
            Some(Ty::Tuple(parts)) if parts.len() == n => parts.iter().cloned().map(Some).collect(),
            _ => vec![None; n],
        };
        let mut irs = Vec::with_capacity(n);
        let mut tys = Vec::with_capacity(n);
        for (item, hint) in elements.iter().zip(&hints) {
            let (ir, ty) = self.expr(item, hint.as_ref())?;
            irs.push(ir);
            tys.push(ty);
        }
        Ok((Ir::Tuple(irs), Ty::Tuple(tys)))
    }

    /// Chain `["project", e, i]` for each index in `parts` (`t.0.1`); `i` is a decimal component
    /// index of the tuple type reached so far.
    fn project(&mut self, mut ir: Ir, mut ty: Ty, parts: &[&str]) -> Result<(Ir, Ty), GenError> {
        for part in parts {
            let (Ty::Tuple(components), Some(index)) = (&ty, tuple_index(part)) else {
                return Err(self.type_err("a tuple projection is a component index of a tuple"));
            };
            let Some(next) = components.get(index).cloned() else {
                return Err(self.type_err("a tuple projection is a component index of a tuple"));
            };
            ir = Ir::Project(Box::new(ir), index);
            ty = next;
            self.note_type(&ty);
        }
        Ok((ir, ty))
    }

    /// `match r with Ok(..) -> .. | Err(..) -> ..` on a `Result` (A.1; `ir.tuple`, D35).
    ///
    /// `match` is `ir.tuple` exactly when its scrutinee is a `Result` and `ir.data` for any other
    /// type (VER04r 4.1), so the scrutinee is typed first.  The patterns are lowered and the
    /// corpus-error rules (a duplicate or shadowing `bind`, an unreachable arm, fall-through) are
    /// decided *before* any arm body is lowered, so such a document is refused even if a body would
    /// have been recorded as a level refusal.  Arm bodies are typed with the pattern's binders in
    /// scope (an arm that needs a context takes the first self-typed arm's type, as `if` does);
    /// the arms must have one type.
    fn match_expr(
        &mut self,
        target: &Expression<'a>,
        arms: &[MatchArm<'a>],
        expected: Option<&Ty>,
    ) -> Result<(Ir, Ty), GenError> {
        let (s_ir, s_ty) = self.expr(target, None)?;
        if !matches!(s_ty, Ty::Result(..)) {
            return Err(self.level(Beyond::IrData, "match needs ir.data"));
        }
        let mut pats: Vec<Pat> = Vec::with_capacity(arms.len());
        let mut bound: Vec<Vec<(String, Option<Ty>)>> = Vec::with_capacity(arms.len());
        for arm in arms {
            let mut binds = Vec::new();
            let mut seen: Vec<String> = Vec::new();
            pats.push(self.pattern(&arm.pattern, &s_ty, &mut binds, &mut seen, 1)?);
            bound.push(binds);
        }
        let refs: Vec<&Pat> = pats.iter().collect();
        patterns::check_arms(&refs, &s_ty, &self.errors)?;
        let mut bodies: Vec<Option<Ir>> = vec![None; arms.len()];
        let mut types: Vec<Option<Ty>> = vec![None; arms.len()];
        let mut hint: Option<Ty> = expected.cloned();
        let order: Vec<usize> = (0..arms.len())
            .filter(|&i| !needs_context(&arms[i].body))
            .chain((0..arms.len()).filter(|&i| needs_context(&arms[i].body)))
            .collect();
        for i in order {
            let body = &arms[i].body;
            let saved = self.scope.clone();
            let mut refused = None;
            for (name, ty) in &bound[i] {
                match ty {
                    Some(t) => {
                        self.scope.insert(name.clone(), t.clone());
                    }
                    None if mentions(body, name) => {
                        refused = Some(self.level(
                            Beyond::IrData,
                            "an error value bound by a pattern has no IR form",
                        ));
                        break;
                    }
                    None => {}
                }
            }
            let lowered = match refused {
                Some(e) => Err(e),
                None => self.expr(body, hint.as_ref()),
            };
            self.scope = saved;
            let (ir, ty) = lowered?;
            if hint.is_none() {
                hint = Some(ty.clone());
            }
            bodies[i] = Some(ir);
            types[i] = Some(ty);
        }
        let mut common: Option<Ty> = types[0].clone();
        for ty in types.iter().skip(1) {
            common = match (common, ty) {
                (Some(c), Some(t)) => Ty::unify(&c, t),
                _ => None,
            };
        }
        let Some(common) = common else {
            return Err(self.type_err("the arms of a match have no common type"));
        };
        let arms_ir: Vec<(Pat, Ir)> = pats
            .into_iter()
            .zip(bodies)
            .map(|(p, b)| b.map(|b| (p, b)))
            .collect::<Option<_>>()
            .ok_or_else(|| GenError::internal("a match arm was not lowered"))?;
        Ok((
            Ir::Match {
                scrutinee: Box::new(s_ir),
                arms: arms_ir,
            },
            common,
        ))
    }

    /// Lower one source pattern against the static type `ty` to its IR pattern (A.1).
    ///
    /// `Ok(p)` is `["ok", p]`; `Err(E.CASE)` is `["err", "CASE"]`; `Err(_)` and `Err(e)` are
    /// `["err_any"]` (an error value has no IR form, so `e` may not be *used*; it is still
    /// declared, for the duplicate and shadowing rules); a tuple pattern is `["tuple", [..]]`; a
    /// name is `["bind", name]` and `_` is `["wild"]`.  A literal pattern (`true`) is not one of
    /// the six and is the level `ir.data`.
    fn pattern(
        &mut self,
        p: &Pattern<'a>,
        ty: &Ty,
        binds: &mut Vec<(String, Option<Ty>)>,
        seen: &mut Vec<String>,
        depth: usize,
    ) -> Result<Pat, GenError> {
        if depth > MAX_PATTERN_DEPTH {
            return Err(GenError::new(
                code::LIMIT,
                "",
                "pattern nesting exceeds 8 levels",
            ));
        }
        match p {
            Pattern::Wild => Ok(Pat::Wild),
            Pattern::Bool(_) => Err(self.level(Beyond::IrData, "a literal pattern needs ir.data")),
            Pattern::None | Pattern::Some(_) => Err(self.level(
                Beyond::IrData,
                "constructor patterns need ir.data",
            )),
            Pattern::Tuple(items) => {
                let Ty::Tuple(parts) = ty else {
                    return Err(self.type_err("a tuple pattern does not match the type of its value"));
                };
                if parts.len() != items.len() {
                    return Err(self.type_err("a tuple pattern does not match the type of its value"));
                }
                let mut out = Vec::with_capacity(items.len());
                for (item, t) in items.iter().zip(parts) {
                    out.push(self.pattern(item, t, binds, seen, depth + 1)?);
                }
                Ok(Pat::Tuple(out))
            }
            Pattern::Ok(inner) => {
                let Ty::Result(ok_ty, _) = ty else {
                    return Err(self.type_err("Ok takes one pattern and needs a Result"));
                };
                Ok(Pat::Ok(Box::new(self.pattern(inner, ok_ty, binds, seen, depth + 1)?)))
            }
            Pattern::Err(inner) => {
                let Ty::Result(_, err_name) = ty else {
                    return Err(self.type_err("Err takes one pattern and needs a Result"));
                };
                if matches!(**inner, Pattern::Wild) {
                    return Ok(Pat::ErrAny);
                }
                if let Some(name) = inner.as_binder() {
                    self.reject_constructor_name(name)?;
                    self.declare(name, None, binds, seen)?;
                    return Ok(Pat::ErrAny);
                }
                if let Pattern::Ctor { name, args } = &**inner {
                    if name.segments.len() == 2 && args.is_empty() {
                        let (en, case) = (name.segments[0], name.segments[1]);
                        let known = self
                            .errors
                            .get(en)
                            .is_some_and(|cases| cases.iter().any(|c| c == case));
                        if en != err_name || !known {
                            return Err(self.type_err("an error case of the wrong error type"));
                        }
                        return Ok(Pat::Err(case.to_string()));
                    }
                }
                Err(self.type_err("an Err pattern is an error case, a name or _"))
            }
            Pattern::Ctor { name, args } => {
                if let (Some(n), true) = (p.as_binder(), args.is_empty()) {
                    self.reject_constructor_name(n)?;
                    self.declare(n, Some(ty.clone()), binds, seen)?;
                    return Ok(Pat::Bind(n.to_string()));
                }
                if name.segments.len() == 1
                    && (matches!(name.segments[0], "Some" | "None") || self.is_ctor(name.segments[0]))
                {
                    return Err(self.level(Beyond::IrData, "constructor patterns need ir.data"));
                }
                Err(self.type_err("an unknown constructor in a pattern"))
            }
        }
    }

    fn reject_constructor_name(&self, name: &str) -> Result<(), GenError> {
        if matches!(name, "Some" | "None") || self.is_ctor(name) {
            return Err(self.level(Beyond::IrData, "constructor patterns need ir.data"));
        }
        if matches!(name, "Ok" | "Err") {
            return Err(self.type_err("Ok and Err are constructors, not names"));
        }
        Ok(())
    }

    /// Declare a pattern binder: a duplicate or a shadowing name is a corpus error (A.1, R2).
    fn declare(
        &self,
        name: &str,
        ty: Option<Ty>,
        binds: &mut Vec<(String, Option<Ty>)>,
        seen: &mut Vec<String>,
    ) -> Result<(), GenError> {
        if seen.iter().any(|n| n == name) {
            return Err(GenError::new(code::IR, "", patterns::DUPLICATE_BIND));
        }
        if self.scope.contains_key(name) || name == "result" {
            return Err(GenError::new(code::IR, "", patterns::SHADOWING_BIND));
        }
        seen.push(name.to_string());
        binds.push((name.to_string(), ty));
        Ok(())
    }

    /// Can the lowered expression of type `ty` flow to a place of type `want`?
    ///
    /// [`Ty::assignable`], plus the static-width rule of VER04r A.1: an expression whose *type*
    /// forgot its width (a bytes literal, a `concat` of sized operands, an `if` of equal widths)
    /// but whose recorded static width is `N` flows to `Bytes<N>`, as in the grammar checker.
    /// `recorded` is [`Lowerer::last_width`], read **immediately** after the operand's `expr`
    /// call (one lowering pass, never a re-lowering).
    fn flows(&self, ty: &Ty, want: &Ty, recorded: Option<u64>) -> bool {
        if Ty::assignable(ty, want) {
            return true;
        }
        match (ty, want) {
            (Ty::Bytes(_), Ty::Bytes(Some(Width::Lit(n)))) => static_width(ty, recorded) == Some(*n),
            _ => false,
        }
    }

    // -- the ir.fixed built-ins (Appendix A.1) ---------------------------------------------

    /// Type and lower one of the 35 `ir.fixed` built-ins to its Appendix A.1 shape: one
    /// [`Ir::Fixed`] node per call, same spelling, positional operands, the static type argument
    /// (`widen<U32>(x)`) in position 1.  The operand types are inferred as for `add`, and the
    /// rules are those of the table of VER04r A.1 (and of `checker.rs`, which has already
    /// accepted the document: a mismatch here is the defensive `type` refusal, never a silent
    /// conversion).  `div`/`rem` on two `Nat` operands give `Option<Int>` (R7).
    ///
    /// A built-in in a **v0** document is out of scope (VER04j 1.2: the names are version
    /// scoped) and keeps the refusal it had before S10b.
    fn fixed_call(
        &mut self,
        name: &str,
        widths: &[AstWidth<'_>],
        args: Option<&[Expression<'a>]>,
    ) -> Result<(Ir, Ty), GenError> {
        if self.file.version != Version::V1 {
            return Err(self.level(Beyond::IrFixed, "fixed-width built-ins need ir.fixed"));
        }
        let tag =
            ir::fixed_tag(name).ok_or_else(|| GenError::internal("a fixed built-in has no tag"))?;
        let static_ty: Option<FixedTy> = if STATIC_TYPE_TAGS.contains(&tag) {
            match widths {
                [AstWidth::Named(n)] => Some(FixedTy::from_name(n).ok_or_else(|| {
                    self.type_err("a static type argument is a fixed-width type")
                })?),
                _ => return Err(self.type_err("this built-in takes one fixed-width type argument")),
            }
        } else if widths.is_empty() {
            None
        } else {
            return Err(self.type_err("this built-in takes no static argument"));
        };
        let Some(args) = args.filter(|a| a.len() == ir::fixed_arity(tag)) else {
            return Err(self.type_err("wrong number of arguments"));
        };
        let fx = |ty: Option<FixedTy>, ops: Vec<Ir>| Ir::Fixed { tag, ty, args: ops };
        let opt = |t: Ty| Ty::Option(Box::new(t));
        // The operand shapes that recur.
        let fixed_operand = |t: &Ty| t.is_fixed();
        match tag {
            "max_value" | "min_value" => {
                let t = static_ty.ok_or_else(|| GenError::internal("no static type"))?;
                Ok((fx(Some(t), vec![]), Ty::Fixed(t)))
            }
            "widen" | "wrap" | "try" => {
                let t = static_ty.ok_or_else(|| GenError::internal("no static type"))?;
                let (x_ir, x_ty) = self.expr(&args[0], None)?;
                match x_ty.fixed() {
                    Some(src) => {
                        if tag == "widen" && !src.widens_to(t) {
                            return Err(self.type_err(
                                "widen needs a source whose range is inside the target",
                            ));
                        }
                    }
                    None => {
                        if tag == "widen" || !matches!(x_ty, Ty::Int | Ty::Nat) {
                            return Err(self
                                .type_err("a conversion needs a fixed-width, Int or Nat operand"));
                        }
                    }
                }
                let ret = if tag == "try" {
                    opt(Ty::Fixed(t))
                } else {
                    Ty::Fixed(t)
                };
                Ok((fx(Some(t), vec![x_ir]), ret))
            }
            "to_int" | "to_nat" => {
                let (x_ir, x_ty) = self.expr(&args[0], None)?;
                let signed = x_ty.fixed().is_some_and(FixedTy::signed);
                if !x_ty.is_fixed() || (tag == "to_nat" && signed) {
                    return Err(self.type_err(
                        "to_int needs a fixed-width operand and to_nat an unsigned one",
                    ));
                }
                Ok((
                    fx(None, vec![x_ir]),
                    if tag == "to_int" { Ty::Int } else { Ty::Nat },
                ))
            }
            "from_be" | "from_le" => {
                let t = static_ty.ok_or_else(|| GenError::internal("no static type"))?;
                let want = t.bytes();
                let (b_ir, b_ty) = self.expr(&args[0], Some(&Ty::bytes_n(want)))?;
                // The operand's static width, from the one pass that just lowered it.
                if static_width(&b_ty, self.last_width) != Some(want) {
                    return Err(
                        self.type_err("from_be/from_le need Bytes of exactly the target's width")
                    );
                }
                Ok((fx(Some(t), vec![b_ir]), Ty::Fixed(t)))
            }
            "bit_not" | "popcount" | "clz" | "ctz" | "be_bytes" | "le_bytes" => {
                let (x_ir, x_ty) = self.expr(&args[0], None)?;
                let Some(f) = x_ty.fixed() else {
                    return Err(self.type_err("a bit operation needs a fixed-width operand"));
                };
                let ret = match tag {
                    "bit_not" => x_ty,
                    "be_bytes" | "le_bytes" => Ty::bytes_n(f.bytes()),
                    _ => Ty::Nat,
                };
                Ok((fx(None, vec![x_ir]), ret))
            }
            "shl" | "shr" | "sar" | "rotl" | "rotr" | "bit" => {
                let (x_ir, x_ty) = self.expr(&args[0], None)?;
                let (n_ir, n_ty) = self.expr(&args[1], Some(&Ty::Nat))?;
                let Some(f) = x_ty.fixed() else {
                    return Err(self.type_err("a shift or bit test needs a fixed-width operand"));
                };
                if n_ty != Ty::Nat {
                    return Err(self.type_err("a shift count or bit index is a Nat"));
                }
                if tag == "sar" && !f.signed() {
                    return Err(
                        self.type_err("sar is for signed operands (a logical shift is shr)")
                    );
                }
                let ret = if tag == "bit" { Ty::Bool } else { x_ty };
                Ok((fx(None, vec![x_ir, n_ir]), ret))
            }
            "bit_and" | "bit_or" | "bit_xor" | "checked_add" | "checked_sub" | "checked_mul" => {
                let (a_ir, a_ty, b_ir, b_ty) = self.operand_pair(args)?;
                if !fixed_operand(&a_ty) || a_ty != b_ty {
                    return Err(self.type_err("operands must be fixed-width values of one type"));
                }
                let ret = if tag.starts_with("bit_") {
                    a_ty
                } else {
                    opt(a_ty)
                };
                Ok((fx(None, vec![a_ir, b_ir]), ret))
            }
            "div" | "rem" => {
                let (a_ir, a_ty, b_ir, b_ty) = self.operand_pair(args)?;
                let bad = || {
                    self.type_err("div and rem take two unsigned operands of one type, or Int/Nat")
                };
                let ret = if a_ty.is_fixed() || b_ty.is_fixed() {
                    // Unsigned pairs only: a signed pair must say div_trunc/rem_trunc (floor and
                    // truncation differ, VER04j 2.3) and a fixed width never mixes with Int/Nat.
                    if a_ty != b_ty || a_ty.fixed().is_some_and(FixedTy::signed) {
                        return Err(bad());
                    }
                    opt(a_ty)
                } else if matches!(a_ty, Ty::Int | Ty::Nat) && matches!(b_ty, Ty::Int | Ty::Nat) {
                    // R7: two Nat operands give Option<Int> (the grammar checker's typing).
                    opt(Ty::Int)
                } else {
                    return Err(bad());
                };
                Ok((fx(None, vec![a_ir, b_ir]), ret))
            }
            "div_trunc" | "rem_trunc" => {
                let (a_ir, a_ty, b_ir, b_ty) = self.operand_pair(args)?;
                if !a_ty.fixed().is_some_and(FixedTy::signed) || a_ty != b_ty {
                    return Err(self
                        .type_err("div_trunc and rem_trunc take two signed operands of one type"));
                }
                Ok((fx(None, vec![a_ir, b_ir]), opt(a_ty)))
            }
            "imin" | "imax" => {
                let (a_ir, a_ty) = self.expr(&args[0], Some(&Ty::Int))?;
                let (b_ir, b_ty) = self.expr(&args[1], Some(&Ty::Int))?;
                if !matches!(a_ty, Ty::Int | Ty::Nat) || !matches!(b_ty, Ty::Int | Ty::Nat) {
                    return Err(self.type_err("imin and imax take Int or Nat operands"));
                }
                Ok((fx(None, vec![a_ir, b_ir]), Ty::Int))
            }
            "at" => {
                let (b_ir, b_ty) = self.expr(&args[0], None)?;
                let (i_ir, i_ty) = self.expr(&args[1], Some(&Ty::Nat))?;
                if !b_ty.is_bytes() {
                    return Err(self.type_err("at needs Bytes"));
                }
                if !matches!(i_ty, Ty::Nat | Ty::Int) {
                    return Err(self.type_err("an index is a Nat or an Int"));
                }
                Ok((fx(None, vec![b_ir, i_ir]), opt(Ty::Fixed(FixedTy::U8))))
            }
            // `unwrap_or(o, d)`: the one consumer of an Option, as the first operand and directly.
            _ => {
                self.option_ok = true;
                let (o_ir, o_ty) = self.expr(&args[0], None)?;
                let Ty::Option(inner) = o_ty else {
                    return Err(self.type_err("unwrap_or needs an Option"));
                };
                let from_a_built_in =
                    matches!(&o_ir, Ir::Fixed { tag, .. } if OPTION_PRODUCERS.contains(tag));
                if !from_a_built_in {
                    return Err(self.level(
                        Beyond::IrData,
                        "an Option reaches unwrap_or only straight from a built-in",
                    ));
                }
                let (d_ir, d_ty) = self.expr(&args[1], Some(&inner))?;
                if !Ty::assignable(&d_ty, &inner) {
                    return Err(
                        self.type_err("the default of unwrap_or does not have the Option's type")
                    );
                }
                Ok((fx(None, vec![o_ir, d_ir]), *inner))
            }
        }
    }

    /// Two operands of one operator: a bare literal takes the other side's type.
    fn operand_pair(&mut self, args: &[Expression<'a>]) -> Result<(Ir, Ty, Ir, Ty), GenError> {
        let (left, right) = (&args[0], &args[1]);
        if needs_context(left) && !needs_context(right) {
            let (b_ir, b_ty) = self.expr(right, None)?;
            let (a_ir, a_ty) = self.expr(left, Some(&b_ty))?;
            Ok((a_ir, a_ty, b_ir, b_ty))
        } else {
            let (a_ir, a_ty) = self.expr(left, None)?;
            let (b_ir, b_ty) = self.expr(right, Some(&a_ty))?;
            Ok((a_ir, a_ty, b_ir, b_ty))
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
            if !self.flows(&ty, &want, self.last_width) {
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

/// A projection index: canonical decimal digits (no sign, no leading zero), else `None`.
fn tuple_index(text: &str) -> Option<usize> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if text != "0" && text.starts_with('0') {
        return None;
    }
    text.parse().ok()
}

/// Does a source expression name `name` (as a variable, the head of a qualified name, or a call
/// head)?  Used to refuse an arm that *uses* the error value an `Err(e)` pattern bound.
fn mentions(root: &Expression<'_>, name: &str) -> bool {
    let mut stack = vec![root];
    while let Some(x) = stack.pop() {
        match x {
            Expression::Reference {
                name: q, args, ..
            } => {
                if q.segments.first() == Some(&name) {
                    return true;
                }
                stack.extend(args.iter().flat_map(|a| a.iter()));
            }
            Expression::Quantified { binders, body, .. } => {
                stack.extend(binders.iter().filter_map(|b| b.refinement.as_ref()));
                stack.push(body);
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => stack.extend([&**condition, &**then_branch, &**else_branch]),
            Expression::Match { target, arms, .. } => {
                stack.push(target);
                stack.extend(arms.iter().map(|a| &a.body));
            }
            Expression::Binary { left, right, .. } => stack.extend([&**left, &**right]),
            Expression::Unary { operand, .. } => stack.push(operand),
            Expression::Grouped { inner, .. } => stack.push(inner),
            Expression::List { elements, .. } | Expression::Tuple { elements, .. } => {
                stack.extend(elements)
            }
            Expression::Project { base, .. } => stack.push(base),
            Expression::Literal { .. } => {}
        }
    }
    false
}

/// The static width of a lowered `Bytes` expression, or `None`: the width its type carries
/// (`Bytes<4>`), else the one recorded as the node was lowered (`recorded`: a bytes literal, a
/// `concat` of sized operands, an `if` of equal widths, the checker's `Literal::Bytes`, `concat`
/// and `common_type` rule, VER04r A.1).  A `Bytes<N>` of a static *name* has none.
fn static_width(ty: &Ty, recorded: Option<u64>) -> Option<u64> {
    match ty {
        Ty::Bytes(Some(Width::Lit(n))) => Some(*n),
        Ty::Bytes(_) => recorded,
        _ => None,
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

    /// The same document header as [`OP`] for a **v1** document (the built-ins are v1 only).
    const OP_V1: &str =
        "warrant u v1\n  op f(a: Bytes, b: Bytes) -> Bool\n  op g<N: Nat>(a: Bytes<N>) -> Bool\n";

    /// As `unchecked`, but keeps the units that need `ir.fixed` (the parity dump's path).
    fn unchecked_fixed(src: &str) -> Result<Lowered, GenError> {
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
        let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
        let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
        lower_partial_unchecked(&file)?.into_lowered_with_fixed()
    }

    #[test]
    fn a_misused_fixed_built_in_is_a_type_error_never_a_silent_conversion() {
        // The checker rules all of these out first; the lowerer states the same table on its own
        // (the second reader), so each is a `type` refusal and never lowers.
        let cases = [
            (
                "mixed widths",
                "model m(a: U8, b: U16) -> U8 = bit_and(a, b)",
            ),
            (
                "bit_and of Nat",
                "model m(a: Nat, b: Nat) -> Nat = bit_and(a, b)",
            ),
            ("sar of an unsigned", "model m(a: U8) -> U8 = sar(a, 1)"),
            (
                "a count that is not a Nat",
                "model m(a: U8, b: U8) -> U8 = shl(a, b)",
            ),
            ("a shift of a Nat", "model m(a: Nat) -> Nat = shl(a, 1)"),
            (
                "widen to a narrower type",
                "model m(a: U16) -> U8 = widen<U8>(a)",
            ),
            (
                "widen across signedness",
                "model m(a: I8) -> U32 = widen<U32>(a)",
            ),
            ("widen of a Nat", "model m(a: Nat) -> U8 = widen<U8>(a)"),
            ("wrap of a Bool", "model m(a: Bool) -> U8 = wrap<U8>(a)"),
            ("to_nat of a signed", "model m(a: I8) -> Nat = to_nat(a)"),
            ("to_int of a Nat", "model m(a: Nat) -> Int = to_int(a)"),
            (
                "a signed div",
                "model m(a: I8, b: I8) -> I8 = unwrap_or(div(a, b), 0i8)",
            ),
            (
                "div_trunc of unsigned",
                "model m(a: U8, b: U8) -> U8 = unwrap_or(div_trunc(a, b), 0u8)",
            ),
            (
                "div mixing U8 and Nat",
                "model m(a: U8, b: Nat) -> U8 = unwrap_or(div(a, b), 0u8)",
            ),
            (
                "div of Bool",
                "model m(a: Bool, b: Bool) -> Bool = unwrap_or(div(a, b), true)",
            ),
            (
                "imin of a fixed width",
                "model m(a: U8, b: U8) -> Int = imin(a, b)",
            ),
            (
                "from_be of a dynamic Bytes",
                "model m(a: Bytes) -> U16 = from_be<U16>(a)",
            ),
            (
                "from_be of the wrong width",
                "model m(a: Bytes<3>) -> U16 = from_le<U16>(a)",
            ),
            (
                "from_be of a concat of the wrong width",
                "model m(a: Bytes<4>) -> U32 = from_be<U32>(concat(a, b\"00\"))",
            ),
            (
                "unwrap_or of a non-Option",
                "model m(a: U8) -> U8 = unwrap_or(a, a)",
            ),
            (
                "an unwrap_or default of the wrong type",
                "model m(a: U8, b: U8) -> U8 = unwrap_or(div(a, b), true)",
            ),
            (
                "at of a fixed width",
                "model m(a: U8) -> U8 = unwrap_or(at(a, 0), 0u8)",
            ),
            (
                "at with a Bool index",
                "model m(a: Bytes) -> U8 = unwrap_or(at(a, true), 0u8)",
            ),
            (
                "a missing static argument",
                "model m(a: U8) -> U16 = widen(a)",
            ),
            (
                "a static argument that is not a fixed-width type",
                "model m(a: U8) -> U16 = widen<Nat>(a)",
            ),
            (
                "a numeric static argument",
                "model m(a: U8) -> U16 = widen<4>(a)",
            ),
            (
                "two static arguments",
                "model m(a: U8) -> U16 = widen<U16, U8>(a)",
            ),
            (
                "a static argument where there is none",
                "model m(a: U8) -> U8 = bit_not<U8>(a)",
            ),
            ("too few operands", "model m(a: U8) -> U8 = bit_and(a)"),
            (
                "an operand for max_value",
                "model m(a: U8) -> U8 = max_value<U8>(a)",
            ),
            ("no argument list at all", "model m(a: U8) -> U8 = bit_not"),
        ];
        for (what, model) in cases {
            let e = unchecked_fixed(&format!("{OP_V1}  {model}\n")).expect_err(what);
            assert_eq!(
                e.code,
                code::TYPE,
                "{what}: expected a type refusal, got {e}"
            );
        }
    }

    #[test]
    fn a_built_in_in_a_v0_document_keeps_the_refusal_it_had_before_s10b() {
        // VER04j 1.2: the names are version scoped; a v0 document does not have them.
        let e = unchecked_fixed("warrant u\n  model m(a: U8) -> U8 = bit_not(a)\n").unwrap_err();
        assert_eq!((e.code, e.level), (code::LEVEL, Some("ir.fixed")));
    }

    #[test]
    fn an_option_that_is_not_the_first_operand_of_unwrap_or_is_ir_data() {
        for (what, model) in [
            ("a model result", "model m(a: U8, b: U8) -> U8 = checked_add(a, b)"),
            ("an operand of eq", "model m(a: U8, b: U8) -> Bool = checked_add(a, b) == checked_add(b, a)"),
            ("the default", "model m(a: U8, b: U8) -> U8 = unwrap_or(div(a, b), checked_add(a, b))"),
            ("an if branch", "model m(a: U8, b: U8) -> U8 = unwrap_or(if true then checked_add(a, b) else checked_add(b, a), 0u8)"),
            ("an argument", "model k(x: U8) -> U8 = x\n  model m(a: U8, b: U8) -> U8 = k(checked_add(a, b))"),
            ("a try", "model m(a: Nat) -> U8 = to_nat(try<U8>(a))"),
        ] {
            let e = unchecked_fixed(&format!("{OP_V1}  {model}\n")).expect_err(what);
            assert_eq!((e.code, e.level), (code::LEVEL, Some("ir.data")), "{what}: {e}");
        }
        // and the legal use lowers
        let ok = "model m(a: U8, b: U8) -> U8 = unwrap_or(checked_add(a, b), 255u8)";
        assert!(unchecked_fixed(&format!("{OP_V1}  {ok}\n")).is_ok());
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
