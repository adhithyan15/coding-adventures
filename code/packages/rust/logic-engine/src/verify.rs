//! # `verify` — re-execute a proof and check that it still holds.
//!
//! This module is the machine described by `ADJ-REASON-MATH.md` §E.5, the
//! **checkability invariant**:
//!
//! > A standalone re-checker, given the `ReasoningTrace` and the KB, can
//! > re-verify each step independently.
//!
//! ## Why this exists at all
//!
//! Everything before this module *produced* an audit trail. A produced trail is
//! **testimony**: the engine says it did these steps, and you believe it because
//! the engine wrote it down. Testimony is exactly what a hallucinating system
//! also emits — fluent, well-formatted, and unfalsifiable.
//!
//! This module turns testimony into **evidence**. It never reads the trail's
//! claims as authority. For every step it goes back to the knowledge base and
//! *does the work again*:
//!
//! | Step kind | What re-checking actually means |
//! |---|---|
//! | `FromFact` | the cited fact still exists, and still unifies with the goal |
//! | `FromRule` | the cited rule still exists, and its head still unifies |
//! | `FromNegation` | re-run the subgoal — the proof set must **still be empty** |
//! | `FromPrior` | the prior clause exists and its log-odds match |
//! | `FromContribution` | the evidence is still observable, and log(LR) × confidence still equals the recorded delta |
//! | `FromJointContribution` | *every* evidence term is still observable, and the joint delta still reproduces |
//! | `FromPredicateContribution` | re-read the slot and re-evaluate the comparison on CPU |
//!
//! plus, for every step that quotes a source, an **anchored** span check
//! against the pinned snapshot.
//!
//! ## The two independent verdicts per step
//!
//! A step has *two* ways to be wrong and they are not the same failure, so they
//! are reported separately:
//!
//! - **logic** — did the inference actually go through? A `Failed` logic verdict
//!   means the answer is unsound *right now*.
//! - **quote** — do the bytes the step rests on actually say what it claims? A
//!   `QuoteMissing` verdict means the trail is *fabricated* — the reasoning may
//!   be internally valid while resting on a sentence nobody ever wrote.
//!
//! Collapsing these into one boolean would lose the most interesting
//! distinction in the whole system: a *valid derivation from an invented fact*.
//!
//! ## Anchored, never "somewhere on the page"
//!
//! The quote check requires a recorded byte offset and compares the exact byte
//! range. §E.5 is explicit that this is "an *anchored* check, not an unanchored
//! substring search anywhere in the document," and the reason is worth stating
//! plainly: on a long document, a short common phrase appears *somewhere* with
//! near-certainty. An unanchored search would report `Verified` for spans the
//! citation never pointed at, which is the same manufactured confidence the
//! whole effort exists to prevent. A span with no offset is [`Unverified`], not
//! verified-by-searching.
//!
//! [`Unverified`]: QuoteStatus::Unverified
//!
//! ## Offline by default
//!
//! Nothing here touches the network. Verification reads the **pinned snapshot**
//! that was captured when the fact entered the KB, supplied by the caller
//! through a [`SnapshotStore`]. `locator`s are spider-authored strings from
//! untrusted web pages; treating one as a fetchable URL would make the verifier
//! an SSRF primitive aimed by whoever can land a single KB entry. Live
//! re-fetch, when it exists, routes through the ADJ39 `CitationVerifier`
//! adapter registry — never a generic HTTP GET from this module. The
//! [`SourceDrifted`](QuoteStatus::SourceDrifted) and
//! [`SourceUnreachable`](QuoteStatus::SourceUnreachable) statuses exist so that
//! layer has somewhere honest to report into; this offline pass never produces
//! them.

use std::collections::{BTreeSet, HashMap, HashSet};

use logic_core::{unify, LogicVar, Substitution, Term};

use crate::compute::{compute, ComputeError, DerivationNode, Derived};
use crate::lr_aggregate::CmpOp;
use crate::proof_dag::{DerivationOrigin, Proof, ProofStep};
use crate::provenance::{ContentHash, Provenance, Quote};
use crate::BodyLiteral;
use crate::{
    enumerate_all, ContributionClauseId, FactId, JointContributionClauseId, KnowledgeBase,
    PredicateContributionClauseId, PriorClauseId, RuleId,
};

/// How far two log-odds values may differ and still count as "the same
/// arithmetic".
///
/// Re-multiplying `log(LR) × confidence` in a different order than the original
/// run can land a unit or two off in the last mantissa bit. That is float
/// noise, not a discrepancy, and flagging it would bury real findings under
/// false ones. `1e-9` is many orders of magnitude below any log-odds difference
/// that changes a decision, and many orders above accumulated rounding.
pub const LOGIT_TOLERANCE: f64 = 1e-9;

/// The outcome of independently evaluating the original computation expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ComputationStatus {
    ReChecked,
    Unverifiable(&'static str),
    Failed(ComputationFailure),
}

/// A localized reason a computed artifact did not reproduce.
#[derive(Debug, Clone, PartialEq)]
pub enum ComputationFailure {
    PlanUnavailable,
    ArtifactDoesNotMatchPlan,
    ScopeUnavailable,
    EvaluationFailed(ComputeError),
    ValueDiffers { recorded: f64, recomputed: f64 },
    ExactValueDiffers,
    DimensionDiffers,
    TreeDiffers,
    ReferencedDerivedDiffers(String),
}

/// One input fact's byte-verification result in a computed answer.
#[derive(Debug, Clone, PartialEq)]
pub struct InputQuoteVerification {
    pub fact_id: FactId,
    pub quote: QuoteStatus,
}

/// Independent math and byte verification for one derived result.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedVerification {
    pub name: String,
    pub computation: ComputationStatus,
    /// Exact source identities checked by `formula_quotes`, in matching order.
    pub formula_sources: Vec<Provenance>,
    pub formula_quotes: Vec<QuoteStatus>,
    pub input_quotes: Vec<InputQuoteVerification>,
    pub is_query_answer: bool,
}

impl DerivedVerification {
    pub fn passed(&self) -> bool {
        matches!(self.computation, ComputationStatus::ReChecked)
            && self
                .formula_quotes
                .iter()
                .all(|quote| !matches!(quote, QuoteStatus::QuoteMissing(_)))
            && self
                .input_quotes
                .iter()
                .all(|input| !matches!(input.quote, QuoteStatus::QuoteMissing(_)))
    }

    pub fn fully_verified(&self) -> bool {
        self.passed()
            && !self.formula_quotes.is_empty()
            && self
                .formula_quotes
                .iter()
                .all(|quote| matches!(quote, QuoteStatus::Verified { .. }))
            && self
                .input_quotes
                .iter()
                .all(|input| matches!(input.quote, QuoteStatus::Verified { .. }))
    }
}

// ---------------------------------------------------------------------------
// Snapshot access
// ---------------------------------------------------------------------------

/// Where the verifier gets the **bytes** of a pinned snapshot.
///
/// `Provenance::snapshot` stores a [`ContentHash`], not a document — a hash is
/// small, stable, and safe to commit, but you cannot check a quote against a
/// hash. The bytes live somewhere else (a content-addressed store on disk, a
/// test fixture, a cache), and that "somewhere else" differs per embedding.
/// This trait is the seam.
///
/// A store that returns `None` is not an error: it means "I do not have that
/// snapshot," which yields
/// [`SnapshotUnavailable`](UnverifiedReason::SnapshotUnavailable) — honestly
/// unverified, never verified.
pub trait SnapshotStore {
    /// The bytes whose SHA-256 is `hash`, if this store has them.
    fn get(&self, hash: &ContentHash) -> Option<Vec<u8>>;
}

/// A store with nothing in it — the safe default.
///
/// Every quote check reports `Unverified(SnapshotUnavailable)`. Useful when you
/// want the *logic* re-checked and have no snapshot corpus at hand.
pub struct NoSnapshots;

impl SnapshotStore for NoSnapshots {
    fn get(&self, _hash: &ContentHash) -> Option<Vec<u8>> {
        None
    }
}

/// An in-memory snapshot store, keyed by content hash.
///
/// Insertion *derives* the key by hashing the bytes, so a document can never be
/// filed under a hash it does not have.
#[derive(Debug, Default, Clone)]
pub struct MemorySnapshots {
    docs: HashMap<String, Vec<u8>>,
}

impl MemorySnapshots {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `bytes` and return the [`ContentHash`] they were filed under.
    pub fn insert(&mut self, bytes: impl Into<Vec<u8>>) -> ContentHash {
        let bytes = bytes.into();
        let hash = ContentHash::of(&bytes);
        self.docs.insert(hash.as_hex().to_string(), bytes);
        hash
    }

    /// How many documents are held.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

impl SnapshotStore for MemorySnapshots {
    fn get(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        self.docs.get(hash.as_hex()).cloned()
    }
}

// ---------------------------------------------------------------------------
// Verdicts
// ---------------------------------------------------------------------------

/// The outcome of re-checking a step's **quotation** — the five-valued outcome
/// of `ADJ-REASON-MATH.md` §E.5.
///
/// The five values exist because collapsing them loses information that
/// changes what a reader should *do*. In particular, "the source changed" and
/// "the source is unreachable" must not become the same bucket as "the quote is
/// wrong": otherwise a third party's outage — or a deliberate network denial —
/// could invalidate a true audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuoteStatus {
    /// The quote matches the pinned snapshot at the recorded byte range.
    Verified {
        /// Where the span starts in the snapshot.
        byte_offset: usize,
        /// How many bytes it spans. Surfaced because span *length* is the
        /// cheapest signal a reviewer has for "is this quote actually
        /// load-bearing, or a two-word fragment that would match anything?"
        byte_len: usize,
    },
    /// Snapshot present, quote absent. **The trail is wrong** — the only status
    /// that fails a step.
    QuoteMissing(QuoteMiss),
    /// Nothing conclusive could be checked. Never reported as passing.
    Unverified(UnverifiedReason),
    /// A live re-fetch differed from the snapshot. Drift evidence — a
    /// fact-maintenance finding, not proof the reasoning was unsound when it
    /// ran. Never produced by the offline pass.
    SourceDrifted,
    /// A live re-fetch failed. Reported, but does **not** fail the step.
    /// Never produced by the offline pass.
    SourceUnreachable,
    /// This step quotes nothing because there is nothing to quote — an
    /// [`DerivationOrigin::FromNegation`] step rests on an *absence*, and an
    /// absence has no sentence in any document. Distinct from `Unverified`,
    /// which means "there should be a quote and there isn't."
    NotApplicable,
}

/// Precisely how a quote failed to appear where it claimed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuoteMiss {
    /// The recorded span has no visible characters.
    ///
    /// A blank or zero-width-only span is a substring of *every* document at
    /// *every* offset, so accepting one would hand out `Verified` for free.
    /// [`crate::VerbatimSpan::new`] already refuses to build one — but a span
    /// can also arrive by deserialization, which bypasses every constructor.
    /// The verifier therefore re-checks the invariant itself rather than
    /// trusting that the value was built the honest way.
    BlankSpan,
    /// The recorded range runs past the end of the snapshot.
    RangeOutOfBounds {
        byte_offset: usize,
        byte_len: usize,
        snapshot_len: usize,
    },
    /// The range's endpoints fall inside a UTF-8 character, so it does not name
    /// a slice of text at all.
    NotACharBoundary { byte_offset: usize, byte_len: usize },
    /// The range is valid and readable, and the bytes there are **not** the
    /// quoted text. This is the fabricated-citation case.
    TextDiffers { byte_offset: usize, byte_len: usize },
}

/// Why a quote could not be checked either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnverifiedReason {
    /// `Quote::Unmigrated` — this clause predates the quote/source split.
    Unmigrated,
    /// A quote was recorded, but no snapshot was pinned to check it against.
    NoSnapshotPinned,
    /// A snapshot hash was pinned, but the store does not have those bytes.
    SnapshotUnavailable,
    /// A quote and snapshot exist, but no byte offset was recorded, and an
    /// unanchored search is not a verification (see the module docs).
    NoByteOffset,
    /// The step names no clause that carries provenance at all.
    NoProvenance,
}

/// The outcome of re-running a step's **inference**.
#[derive(Debug, Clone, PartialEq)]
pub enum LogicStatus {
    /// The step was re-executed and still holds.
    ReChecked,
    /// The step was re-executed and does **not** hold.
    Failed(LogicFailure),
}

/// Precisely how re-execution of a step's inference failed.
#[derive(Debug, Clone, PartialEq)]
pub enum LogicFailure {
    /// The cited fact is not in the knowledge base.
    UnknownFact(FactId),
    /// The cited rule is not in the knowledge base.
    UnknownRule(RuleId),
    /// The cited clause exists, but its head no longer unifies with the goal
    /// the step claims it proved.
    GoalDoesNotUnify,
    /// The step's goal is a bare variable, which names no predicate and would
    /// unify with every clause in the knowledge base.
    GoalIsBareVariable,
    /// A rule fired, but its body literals are not accounted for by the step's
    /// immediate children — so its premises were never established.
    RuleBodyNotDischarged { expected: usize, found: usize },
    /// A negation-as-failure step claimed `goal` had no proof, and it now has
    /// one. The absence that licensed the conclusion is gone.
    NegatedGoalProvable,
    /// Re-running the negated subgoal hit the resolver's depth limit, so the
    /// search was **truncated**.
    ///
    /// This is a failure, not a pass. Negation-as-failure succeeds exactly when
    /// the proof set is empty, and a truncated search produces an empty set for
    /// a completely different reason: nobody finished looking. Reading "I
    /// stopped" as "there is none" is the accounting failure the audit trail
    /// exists to prevent.
    NegationSearchTruncated,
    /// The cited prior clause is not in the knowledge base.
    UnknownPrior(PriorClauseId),
    /// The cited contribution clause is not in the knowledge base.
    UnknownContribution(ContributionClauseId),
    /// The cited joint-contribution clause is not in the knowledge base.
    UnknownJointContribution(JointContributionClauseId),
    /// The cited predicate-contribution clause is not in the knowledge base.
    UnknownPredicateContribution(PredicateContributionClauseId),
    /// The evidence that licensed a contribution is no longer observable.
    EvidenceNotObservable,
    /// The step's inline log-odds number does not reproduce from the clause.
    LogitDiffers { recorded: f64, recomputed: f64 },
    /// A predicate-gated step's slot has no observed numeric value now.
    SlotNotObserved(String),
    /// The right-hand side of a predicate-gated clause could not be evaluated.
    ThresholdNotEvaluable,
    /// The comparison that fired no longer holds on the current observation.
    PredicateDoesNotHold {
        slot: String,
        op: CmpOp,
        threshold: f64,
        observed: f64,
    },
}

/// The verdict on one step.
#[derive(Debug, Clone, PartialEq)]
pub struct StepVerification {
    /// Index of this step within `proof.steps`.
    pub index: usize,
    /// The step's nesting depth, copied so a report can be rendered as a tree
    /// without re-walking the proof.
    pub depth: usize,
    /// Stable name of the step kind — `"FromFact"`, `"FromNegation"`, ….
    pub kind: &'static str,
    /// The goal the step claimed to prove.
    pub goal: Term,
    /// Did the inference re-execute?
    pub logic: LogicStatus,
    /// Do the bytes it rests on say what it claims?
    pub quote: QuoteStatus,
}

impl StepVerification {
    /// Whether this step passes.
    ///
    /// A step passes when its logic re-executed **and** its quote is not
    /// affirmatively missing. `Unverified` does not fail a step — it is honest
    /// about not knowing — but it never counts as `Verified` either, and
    /// [`TraceVerification::fully_verified`] is the predicate that cares.
    pub fn passed(&self) -> bool {
        matches!(self.logic, LogicStatus::ReChecked)
            && !matches!(self.quote, QuoteStatus::QuoteMissing(_))
    }

    /// Whether this step re-executed *and* its quote was affirmatively
    /// confirmed against a pinned snapshot.
    pub fn fully_verified(&self) -> bool {
        matches!(self.logic, LogicStatus::ReChecked)
            && matches!(
                self.quote,
                QuoteStatus::Verified { .. } | QuoteStatus::NotApplicable
            )
    }
}

/// The verdict on a whole proof.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceVerification {
    /// One entry per step, in the proof's own preorder.
    pub steps: Vec<StepVerification>,
}

impl TraceVerification {
    /// The **first** failing step, which is what localizes an error.
    ///
    /// §E.5: "The first step whose re-check fails localizes the error to a
    /// single clause + citation." Later failures are usually consequences of
    /// the first, so reporting the earliest is reporting the cause.
    pub fn first_failure(&self) -> Option<&StepVerification> {
        self.steps.iter().find(|s| !s.passed())
    }

    /// Whether every step passed.
    pub fn passed(&self) -> bool {
        self.first_failure().is_none()
    }

    /// Whether every step both re-executed and had its quote confirmed.
    ///
    /// Strictly stronger than [`passed`](Self::passed): a trail full of
    /// unmigrated quotes passes but is not fully verified, and a report should
    /// not let the two read the same.
    ///
    /// **An empty trace is not fully verified**, and neither is one in which no
    /// span was ever confirmed. `all()` over nothing is `true`, and that vacuous
    /// truth is precisely the manufactured-confidence pattern this whole module
    /// exists to prevent — at two levels. A proof with no steps would report the
    /// strongest verdict for having checked nothing; and so would a proof made
    /// entirely of `FromNegation` steps, whose quotes are legitimately
    /// `NotApplicable`. The second is subtler and just as wrong: an absence is
    /// re-checkable, but it grounds nothing in any document, so a trail built
    /// only from absences has confirmed zero bytes.
    pub fn fully_verified(&self) -> bool {
        !self.steps.is_empty()
            && self.steps.iter().all(|s| s.fully_verified())
            && self
                .steps
                .iter()
                .any(|s| matches!(s.quote, QuoteStatus::Verified { .. }))
    }
}

// ---------------------------------------------------------------------------
// The quote check
// ---------------------------------------------------------------------------

/// Does this character carry no visible mark?
///
/// Deliberately duplicated from `provenance.rs` rather than shared. The
/// verifier's whole job is to not take the producer's word for anything, and a
/// value that arrived by deserialization never ran the producer's check. An
/// independent implementation is the point, not an oversight.
fn is_invisible(c: char) -> bool {
    c.is_whitespace()
        || matches!(c,
            '\u{00AD}'                 // SOFT HYPHEN
            | '\u{061C}'               // ARABIC LETTER MARK
            | '\u{180E}'               // MONGOLIAN VOWEL SEPARATOR
            | '\u{200B}'..='\u{200F}'  // zero-width space/joiners, LRM, RLM
            | '\u{202A}'..='\u{202E}'  // bidi embedding / override
            | '\u{2060}'..='\u{2064}'  // word joiner, invisible math operators
            | '\u{2066}'..='\u{2069}'  // bidi isolates
            | '\u{FEFF}'               // ZERO WIDTH NO-BREAK SPACE / BOM
        )
}

fn has_visible_content(text: &str) -> bool {
    text.chars().any(|c| !is_invisible(c))
}

/// Re-check one clause's quotation against the pinned snapshot.
///
/// Every failure path is explicit and every slice is bounds- and
/// boundary-checked before it is taken, so a malformed or hostile trail
/// produces a verdict rather than a panic.
pub fn verify_quote(prov: &Provenance, snapshots: &dyn SnapshotStore) -> QuoteStatus {
    let Quote::Verbatim(span) = &prov.quote else {
        return QuoteStatus::Unverified(UnverifiedReason::Unmigrated);
    };

    // Independent blank-span rejection — see `is_invisible` above.
    if !has_visible_content(span.text()) {
        return QuoteStatus::QuoteMissing(QuoteMiss::BlankSpan);
    }

    let Some(hash) = &prov.snapshot else {
        return QuoteStatus::Unverified(UnverifiedReason::NoSnapshotPinned);
    };
    let Some(bytes) = snapshots.get(hash) else {
        return QuoteStatus::Unverified(UnverifiedReason::SnapshotUnavailable);
    };
    let Some(byte_offset) = span.byte_offset() else {
        return QuoteStatus::Unverified(UnverifiedReason::NoByteOffset);
    };

    let text = span.text();
    let byte_len = text.len();

    // A snapshot that is not valid UTF-8 cannot be sliced as text at all. Treat
    // it as unavailable rather than guessing at an encoding.
    let Ok(doc) = std::str::from_utf8(&bytes) else {
        return QuoteStatus::Unverified(UnverifiedReason::SnapshotUnavailable);
    };

    let Some(end) = byte_offset.checked_add(byte_len) else {
        return QuoteStatus::QuoteMissing(QuoteMiss::RangeOutOfBounds {
            byte_offset,
            byte_len,
            snapshot_len: doc.len(),
        });
    };
    if end > doc.len() {
        return QuoteStatus::QuoteMissing(QuoteMiss::RangeOutOfBounds {
            byte_offset,
            byte_len,
            snapshot_len: doc.len(),
        });
    }
    if !doc.is_char_boundary(byte_offset) || !doc.is_char_boundary(end) {
        return QuoteStatus::QuoteMissing(QuoteMiss::NotACharBoundary {
            byte_offset,
            byte_len,
        });
    }
    if &doc[byte_offset..end] == text {
        QuoteStatus::Verified {
            byte_offset,
            byte_len,
        }
    } else {
        QuoteStatus::QuoteMissing(QuoteMiss::TextDiffers {
            byte_offset,
            byte_len,
        })
    }
}

// ---------------------------------------------------------------------------
// Computed-answer verification
// ---------------------------------------------------------------------------

fn same_number(left: f64, right: f64) -> bool {
    left.to_bits() == right.to_bits()
}

fn collect_computation_dependencies(
    node: &DerivationNode,
    facts: &mut BTreeSet<FactId>,
    derived_names: &mut Vec<String>,
) {
    match node {
        DerivationNode::Leaf { fact_id, .. } => {
            facts.insert(*fact_id);
        }
        DerivationNode::DerivedRef { name, .. } => derived_names.push(name.clone()),
        DerivationNode::Op { operands, .. } => {
            for operand in operands {
                collect_computation_dependencies(operand, facts, derived_names);
            }
        }
        DerivationNode::Round { operand, .. }
        | DerivationNode::ToScientific { operand, .. }
        | DerivationNode::ToPercent { operand, .. }
        | DerivationNode::ToCurrency { operand, .. } => {
            collect_computation_dependencies(operand, facts, derived_names);
        }
        DerivationNode::Lit { .. } => {}
    }
}

fn has_inexact_narrowing(node: &DerivationNode) -> bool {
    match node {
        DerivationNode::Round {
            operand,
            operand_exact,
            ..
        }
        | DerivationNode::ToScientific {
            operand,
            operand_exact,
            ..
        }
        | DerivationNode::ToPercent {
            operand,
            operand_exact,
            ..
        }
        | DerivationNode::ToCurrency {
            operand,
            operand_exact,
            ..
        } => operand_exact.is_none() || has_inexact_narrowing(operand),
        DerivationNode::Op { operands, .. } => operands.iter().any(has_inexact_narrowing),
        DerivationNode::Leaf { .. }
        | DerivationNode::DerivedRef { .. }
        | DerivationNode::Lit { .. } => false,
    }
}

fn recheck_derived_recursive(
    derived: &Derived,
    kb: &KnowledgeBase,
    visiting: &mut HashSet<usize>,
    checked: &mut HashSet<usize>,
    input_ids: &mut BTreeSet<FactId>,
    formula_sources: &mut Vec<Provenance>,
) -> ComputationStatus {
    let Some(id) = derived.computation_id else {
        return ComputationStatus::Failed(ComputationFailure::PlanUnavailable);
    };
    let Some(plan) = kb.computation_plan(id) else {
        return ComputationStatus::Failed(ComputationFailure::PlanUnavailable);
    };
    if kb.derived_bindings().get(id.0) != Some(derived) {
        return ComputationStatus::Failed(ComputationFailure::ArtifactDoesNotMatchPlan);
    }
    formula_sources.extend(plan.formula_sources.iter().cloned());
    let Some(view) = kb.at_computation_scope(plan.scope) else {
        return ComputationStatus::Failed(ComputationFailure::ScopeUnavailable);
    };
    let fresh = match compute(derived.name.clone(), &plan.expr, &view) {
        Ok(fresh) => fresh,
        Err(error) => {
            return ComputationStatus::Failed(ComputationFailure::EvaluationFailed(error))
        }
    };
    if !same_number(fresh.value, derived.value) {
        return ComputationStatus::Failed(ComputationFailure::ValueDiffers {
            recorded: derived.value,
            recomputed: fresh.value,
        });
    }
    if fresh.exact != derived.exact {
        return ComputationStatus::Failed(ComputationFailure::ExactValueDiffers);
    }
    if fresh.dim != derived.dim {
        return ComputationStatus::Failed(ComputationFailure::DimensionDiffers);
    }
    if fresh.tree != derived.tree {
        return ComputationStatus::Failed(ComputationFailure::TreeDiffers);
    }

    let mut names = Vec::new();
    collect_computation_dependencies(&fresh.tree, input_ids, &mut names);
    for name in names {
        let Some(index) = view
            .derived_bindings()
            .iter()
            .rposition(|candidate| candidate.name == name)
        else {
            return ComputationStatus::Failed(ComputationFailure::ReferencedDerivedDiffers(name));
        };
        if checked.contains(&index) {
            continue;
        }
        if !visiting.insert(index) {
            return ComputationStatus::Failed(ComputationFailure::ReferencedDerivedDiffers(name));
        }
        let dependency = &kb.derived_bindings()[index];
        let status = recheck_derived_recursive(
            dependency,
            kb,
            visiting,
            checked,
            input_ids,
            formula_sources,
        );
        visiting.remove(&index);
        if !matches!(status, ComputationStatus::ReChecked) {
            return match status {
                ComputationStatus::Unverifiable(_) => ComputationStatus::Unverifiable(
                    "referenced computation has an inexact narrowing source",
                ),
                _ => ComputationStatus::Failed(ComputationFailure::ReferencedDerivedDiffers(name)),
            };
        }
        checked.insert(index);
    }

    if has_inexact_narrowing(&fresh.tree) {
        ComputationStatus::Unverifiable("inexact narrowing source")
    } else {
        ComputationStatus::ReChecked
    }
}

/// Re-evaluate the original expression in its original binding scope, then
/// verify every transitive formula and observed-input byte span.
pub fn verify_derived(
    derived: &Derived,
    kb: &KnowledgeBase,
    snapshots: &dyn SnapshotStore,
) -> DerivedVerification {
    let mut input_ids = BTreeSet::new();
    let mut formula_sources = Vec::new();
    let computation = recheck_derived_recursive(
        derived,
        kb,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut input_ids,
        &mut formula_sources,
    );
    let formula_quotes = formula_sources
        .iter()
        .map(|provenance| verify_quote(provenance, snapshots))
        .collect();
    let input_quotes = input_ids
        .into_iter()
        .map(|fact_id| InputQuoteVerification {
            fact_id,
            quote: kb
                .fact(fact_id)
                .map(|fact| verify_quote(&fact.provenance, snapshots))
                .unwrap_or(QuoteStatus::Unverified(UnverifiedReason::NoProvenance)),
        })
        .collect();

    DerivedVerification {
        name: derived.name.clone(),
        computation,
        formula_sources,
        formula_quotes,
        input_quotes,
        is_query_answer: derived
            .computation_id
            .and_then(|id| kb.computation_plan(id))
            .is_some_and(|plan| plan.is_query_answer),
    }
}

// ---------------------------------------------------------------------------
// The step check
// ---------------------------------------------------------------------------

/// Rename every variable in `term` to a fresh one, so re-unifying a stored
/// clause head against a goal cannot accidentally succeed (or fail) because the
/// two happen to share variable identity.
fn rename_fresh(term: &Term, renames: &mut HashMap<u64, LogicVar>) -> Term {
    match term {
        Term::Var(v) => Term::Var(
            renames
                .entry(v.id)
                .or_insert_with(|| LogicVar::fresh(v.display_name.as_deref()))
                .clone(),
        ),
        Term::Compound { functor, args } => Term::Compound {
            functor: functor.clone(),
            args: args.iter().map(|a| rename_fresh(a, renames)).collect(),
        },
        other => other.clone(),
    }
}

fn unifies(goal: &Term, head: &Term) -> bool {
    let mut renames = HashMap::new();
    let fresh = rename_fresh(head, &mut renames);
    unify(goal, &fresh, &Substitution::empty()).is_some()
}

/// A goal that is a **bare variable** names no predicate at all.
///
/// It unifies with every fact and every rule head in the knowledge base, so a
/// forged step carrying `goal: Var(_)` and the id of any real, well-quoted
/// clause would re-check as sound and inherit that clause's verified citation —
/// a step that proved nothing in particular, wearing someone else's evidence.
/// The resolver never produces such a goal, so rejecting it costs nothing.
///
/// Note what is deliberately *not* required: that the goal be ground. Real
/// trails routinely carry partially-instantiated goals — a binding query's own
/// step is literally `length_to_metres(foot, Metres)` with `Metres` unbound —
/// so a groundness rule would reject the ordinary case. Mutual unification is
/// what SLD resolution actually does, and the check has to match it.
fn is_bare_variable(goal: &Term) -> bool {
    matches!(goal, Term::Var(_))
}

/// Do this rule step's immediate children account for **every literal in the
/// rule's body**?
///
/// # Why head unification is not enough
///
/// A rule step says "this rule fired." Checking only that its head still
/// unifies with the goal checks that the rule *could* apply — never that its
/// premises were established. A trail containing a single step, naming a real
/// rule, with no children at all, would otherwise re-check as sound and report
/// the strongest verdict in the system for a conclusion whose premises nobody
/// ever proved — including the `not …` guard the rule exists to enforce. That is
/// the manufactured confidence this module was written to prevent, appearing
/// inside the tool meant to prevent it.
///
/// # How the children are known
///
/// `steps` is a preorder walk and a rule's body steps sit exactly one level
/// deeper than the rule step itself (`enumerate::solve`), so the immediate
/// children are the maximal run of following steps at `depth + 1`, stopping at
/// the first step that returns to `depth` or shallower. `solve_body` consumes
/// the body left to right, so those children appear in **body order** — which is
/// what lets each one be matched against the literal it is supposed to discharge
/// rather than merely counted.
///
/// # Why one shared substitution, not per-literal unification
///
/// Checking each literal against its child *independently* — unify, throw the
/// bindings away, repeat — checks the wrong thing. `may_prescribe(D,P) :-
/// safe_for(D,P), not contraindicated(D,P)` would be "discharged" by a child
/// proving `safe_for(warfarin, child)` while the head unified with
/// `may_prescribe(aspirin, adult)`: each literal unifies with *some* child in
/// isolation, so the structural check passes, and a conclusion about aspirin is
/// certified by premises about warfarin. The variable `D` is shared between the
/// head and both body literals, and dropping the head's binding for it — and
/// not carrying one child's binding into the next — severs exactly the
/// constraint that makes a derivation a derivation.
///
/// So this threads **one** substitution. The rule head and its whole body are
/// renamed *together* (shared clause variables stay one variable); the head is
/// unified with the goal to seed it; then each literal, instantiated under the
/// running substitution, must unify with its child, and the resulting bindings
/// carry forward. `p(X) :- q(X), r(X)` can no longer be discharged by `q(a)`
/// and `r(b)`.
fn body_is_discharged(rule: &crate::Rule, goal: &Term, children: &[&ProofStep]) -> bool {
    if children.len() != rule.body.len() {
        return false;
    }
    // Rename head + body as ONE unit, so a clause variable that appears in the
    // head and in a body literal is the same fresh variable in both.
    let mut renames = HashMap::new();
    let head = rename_fresh(&rule.head, &mut renames);
    let body: Vec<BodyLiteral> = rule
        .body
        .iter()
        .map(|lit| match lit {
            BodyLiteral::Pos(t) => BodyLiteral::Pos(rename_fresh(t, &mut renames)),
            BodyLiteral::Neg(t) => BodyLiteral::Neg(rename_fresh(t, &mut renames)),
        })
        .collect();

    // Seed the running substitution with the head unification, so bindings the
    // goal forces on clause variables reach the body.
    let Some(mut subst) = unify(goal, &head, &Substitution::empty()) else {
        return false;
    };

    for (lit, child) in body.iter().zip(children) {
        match lit {
            BodyLiteral::Pos(t) => {
                // A positive premise is discharged only by an SLD step — a fact,
                // a rule, or an established absence. An LR step whose goal
                // happened to equal the literal only *evidenced* it; it did not
                // prove it, and must not stand in for a premise.
                if !matches!(
                    child.origin,
                    DerivationOrigin::FromFact(_)
                        | DerivationOrigin::FromRule(_)
                        | DerivationOrigin::FromNegation { .. }
                ) {
                    return false;
                }
                match unify(&child.goal, t, &subst) {
                    Some(next) => subst = next,
                    None => return false,
                }
            }
            // A negated literal is discharged only by an absence that was
            // actually established, on the SAME ground the running substitution
            // has fixed. Accepting any child here would let a positive step
            // stand in for the guard it was supposed to satisfy.
            BodyLiteral::Neg(t) => match &child.origin {
                DerivationOrigin::FromNegation { goal: neg_goal } => {
                    match unify(neg_goal, t, &subst) {
                        Some(next) => subst = next,
                        None => return false,
                    }
                }
                _ => return false,
            },
        }
    }
    true
}

/// The stable name of a step kind, used when a step is rejected before its
/// clause is ever looked up.
fn origin_kind(origin: &DerivationOrigin) -> &'static str {
    match origin {
        DerivationOrigin::FromFact(_) => "FromFact",
        DerivationOrigin::FromRule(_) => "FromRule",
        DerivationOrigin::FromNegation { .. } => "FromNegation",
        DerivationOrigin::FromPrior { .. } => "FromPrior",
        DerivationOrigin::FromContribution { .. } => "FromContribution",
        DerivationOrigin::FromJointContribution { .. } => "FromJointContribution",
        DerivationOrigin::FromPredicateContribution { .. } => "FromPredicateContribution",
    }
}

/// The immediate children of `steps[index]`: see [`body_is_discharged`].
fn immediate_children(steps: &[ProofStep], index: usize) -> Vec<&ProofStep> {
    let depth = steps[index].depth;
    let mut out = Vec::new();
    for step in &steps[index + 1..] {
        if step.depth <= depth {
            break;
        }
        if step.depth == depth + 1 {
            out.push(step);
        }
    }
    out
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= LOGIT_TOLERANCE
}

/// Re-execute one step's inference and re-check its quotation.
pub fn verify_step(
    index: usize,
    step: &ProofStep,
    children: &[&ProofStep],
    kb: &KnowledgeBase,
    snapshots: &dyn SnapshotStore,
) -> StepVerification {
    // A goal that names no predicate would unify with everything; reject it
    // before any clause lookup can lend it a citation.
    if is_bare_variable(&step.goal) {
        return StepVerification {
            index,
            depth: step.depth,
            kind: origin_kind(&step.origin),
            goal: step.goal.clone(),
            logic: LogicStatus::Failed(LogicFailure::GoalIsBareVariable),
            quote: QuoteStatus::Unverified(UnverifiedReason::NoProvenance),
        };
    }
    let (kind, logic, prov) = match &step.origin {
        DerivationOrigin::FromFact(id) => match kb.find_fact_by_id(*id) {
            None => (
                "FromFact",
                LogicStatus::Failed(LogicFailure::UnknownFact(*id)),
                None,
            ),
            Some(fact) if !unifies(&step.goal, &fact.term) => (
                "FromFact",
                LogicStatus::Failed(LogicFailure::GoalDoesNotUnify),
                Some(fact.provenance.clone()),
            ),
            Some(fact) => (
                "FromFact",
                LogicStatus::ReChecked,
                Some(fact.provenance.clone()),
            ),
        },

        DerivationOrigin::FromRule(id) => match kb.find_rule_by_id(*id) {
            None => (
                "FromRule",
                LogicStatus::Failed(LogicFailure::UnknownRule(*id)),
                None,
            ),
            Some(rule) if !unifies(&step.goal, &rule.head) => (
                "FromRule",
                LogicStatus::Failed(LogicFailure::GoalDoesNotUnify),
                Some(rule.provenance.clone()),
            ),
            Some(rule) if !body_is_discharged(rule, &step.goal, children) => (
                "FromRule",
                LogicStatus::Failed(LogicFailure::RuleBodyNotDischarged {
                    expected: rule.body.len(),
                    found: children.len(),
                }),
                Some(rule.provenance.clone()),
            ),
            Some(rule) => (
                "FromRule",
                LogicStatus::ReChecked,
                Some(rule.provenance.clone()),
            ),
        },

        // The one step kind that is re-checked by *running a search and
        // demanding it come back empty*. Both ways it can go wrong are
        // failures: a proof now exists, or the search never finished.
        DerivationOrigin::FromNegation { goal } => {
            let dag = enumerate_all(goal, kb);
            let logic = if dag.truncated {
                LogicStatus::Failed(LogicFailure::NegationSearchTruncated)
            } else if dag.proofs.is_empty() {
                LogicStatus::ReChecked
            } else {
                LogicStatus::Failed(LogicFailure::NegatedGoalProvable)
            };
            ("FromNegation", logic, None)
        }

        DerivationOrigin::FromPrior {
            clause_id,
            prior_logit,
        } => match kb.prior_for(&step.goal).filter(|p| p.id == *clause_id) {
            None => (
                "FromPrior",
                LogicStatus::Failed(LogicFailure::UnknownPrior(*clause_id)),
                None,
            ),
            Some(prior) if !close(prior.prior_logit, *prior_logit) => (
                "FromPrior",
                LogicStatus::Failed(LogicFailure::LogitDiffers {
                    recorded: *prior_logit,
                    recomputed: prior.prior_logit,
                }),
                Some(prior.provenance.clone()),
            ),
            Some(prior) => (
                "FromPrior",
                LogicStatus::ReChecked,
                Some(prior.provenance.clone()),
            ),
        },

        DerivationOrigin::FromContribution {
            clause_id,
            logit_delta,
            ..
        } => {
            let clause = kb
                .contributions_for(&step.goal)
                .into_iter()
                .find(|c| c.id == *clause_id)
                .cloned();
            match clause {
                None => (
                    "FromContribution",
                    LogicStatus::Failed(LogicFailure::UnknownContribution(*clause_id)),
                    None,
                ),
                Some(clause) => {
                    let prov = Some(clause.provenance.clone());
                    match kb.observed_evidence(&clause.evidence_term) {
                        None => (
                            "FromContribution",
                            LogicStatus::Failed(LogicFailure::EvidenceNotObservable),
                            prov,
                        ),
                        Some(observed) => {
                            let recomputed = clause.logit_delta * observed.confidence;
                            if close(recomputed, *logit_delta) {
                                ("FromContribution", LogicStatus::ReChecked, prov)
                            } else {
                                (
                                    "FromContribution",
                                    LogicStatus::Failed(LogicFailure::LogitDiffers {
                                        recorded: *logit_delta,
                                        recomputed,
                                    }),
                                    prov,
                                )
                            }
                        }
                    }
                }
            }
        }

        DerivationOrigin::FromJointContribution {
            clause_id,
            joint_logit_delta,
            ..
        } => {
            let clause = kb
                .joint_contributions_for(&step.goal)
                .into_iter()
                .find(|c| c.id == *clause_id)
                .cloned();
            match clause {
                None => (
                    "FromJointContribution",
                    LogicStatus::Failed(LogicFailure::UnknownJointContribution(*clause_id)),
                    None,
                ),
                Some(clause) => {
                    let prov = Some(clause.provenance.clone());
                    // A joint term fires only when EVERY evidence term is still
                    // observable; one missing member retires the whole term.
                    let mut confidence = 1.0;
                    let mut all_observed = true;
                    for ev in &clause.evidence_set {
                        match kb.observed_evidence(ev) {
                            Some(observed) => confidence *= observed.confidence,
                            None => {
                                all_observed = false;
                                break;
                            }
                        }
                    }
                    if !all_observed {
                        (
                            "FromJointContribution",
                            LogicStatus::Failed(LogicFailure::EvidenceNotObservable),
                            prov,
                        )
                    } else {
                        let recomputed = clause.joint_logit_delta * confidence;
                        if close(recomputed, *joint_logit_delta) {
                            ("FromJointContribution", LogicStatus::ReChecked, prov)
                        } else {
                            (
                                "FromJointContribution",
                                LogicStatus::Failed(LogicFailure::LogitDiffers {
                                    recorded: *joint_logit_delta,
                                    recomputed,
                                }),
                                prov,
                            )
                        }
                    }
                }
            }
        }

        DerivationOrigin::FromPredicateContribution {
            clause_id,
            slot,
            logit_delta,
            ..
        } => {
            let clause = kb
                .predicate_contributions_for(&step.goal)
                .into_iter()
                .find(|c| c.id == *clause_id)
                .cloned();
            match clause {
                None => (
                    "FromPredicateContribution",
                    LogicStatus::Failed(LogicFailure::UnknownPredicateContribution(*clause_id)),
                    None,
                ),
                Some(clause) => {
                    let prov = Some(clause.provenance.clone());
                    // Re-read the observation and re-run the comparison on CPU.
                    // The trail's own `observed` / `threshold` numbers are
                    // deliberately NOT trusted as inputs here — they are the
                    // claim under test.
                    match kb.observed_numeric(slot) {
                        None => (
                            "FromPredicateContribution",
                            LogicStatus::Failed(LogicFailure::SlotNotObserved(slot.clone())),
                            prov,
                        ),
                        Some((observed, observed_exact)) => {
                            match compute("__verify_predicate_rhs", &clause.rhs, kb) {
                                Err(_) => (
                                    "FromPredicateContribution",
                                    LogicStatus::Failed(LogicFailure::ThresholdNotEvaluable),
                                    prov,
                                ),
                                Ok(rhs) => {
                                    if !clause.op.eval_values(
                                        observed,
                                        rhs.value,
                                        observed_exact,
                                        rhs.exact,
                                    ) {
                                        (
                                            "FromPredicateContribution",
                                            LogicStatus::Failed(
                                                LogicFailure::PredicateDoesNotHold {
                                                    slot: slot.clone(),
                                                    op: clause.op,
                                                    threshold: rhs.value,
                                                    observed,
                                                },
                                            ),
                                            prov,
                                        )
                                    } else if !close(clause.logit_delta, *logit_delta) {
                                        (
                                            "FromPredicateContribution",
                                            LogicStatus::Failed(LogicFailure::LogitDiffers {
                                                recorded: *logit_delta,
                                                recomputed: clause.logit_delta,
                                            }),
                                            prov,
                                        )
                                    } else {
                                        (
                                            "FromPredicateContribution",
                                            LogicStatus::ReChecked,
                                            prov,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    // A negation step rests on an absence, and an absence has no sentence in
    // any document — `NotApplicable`, not `Unverified`.
    let quote = match (&step.origin, &prov) {
        (DerivationOrigin::FromNegation { .. }, _) => QuoteStatus::NotApplicable,
        (_, Some(p)) => verify_quote(p, snapshots),
        (_, None) => QuoteStatus::Unverified(UnverifiedReason::NoProvenance),
    };

    StepVerification {
        index,
        depth: step.depth,
        kind,
        goal: step.goal.clone(),
        logic,
        quote,
    }
}

/// Re-execute every step of `proof` against `kb`.
///
/// Every step is checked, even after one fails: a report that stopped at the
/// first failure would hide whether the rest of the trail is sound, and the
/// caller can always ask for [`TraceVerification::first_failure`] when it wants
/// the localized cause.
pub fn verify_proof(
    proof: &Proof,
    kb: &KnowledgeBase,
    snapshots: &dyn SnapshotStore,
) -> TraceVerification {
    TraceVerification {
        steps: proof
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let children = immediate_children(&proof.steps, i);
                verify_step(i, step, &children, kb, snapshots)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::{TrustTier, VerbatimSpan};

    /// The blank-span door, opened the only way it can still be opened.
    ///
    /// `VerbatimSpan::new` refuses blank text, so no shipping path can build
    /// this value. Deserialization can — it writes fields directly and runs no
    /// constructor — which is exactly why the verifier re-checks rather than
    /// trusting provenance to have been built honestly. `from_parts_unchecked`
    /// stands in for that deserializer.
    fn blank_quote(text: &str) -> Provenance {
        let mut p = Provenance::new("Some Source".to_string(), None, TrustTier::Authoritative);
        p.quote = Quote::Verbatim(VerbatimSpan::from_parts_unchecked(text, Some(0)));
        p.snapshot = Some(ContentHash::of(b"any document at all"));
        p
    }

    #[test]
    fn a_whitespace_span_is_quote_missing_not_verified() {
        let mut snaps = MemorySnapshots::new();
        snaps.insert(b"any document at all".to_vec());
        assert_eq!(
            verify_quote(&blank_quote("   \t\n "), &snaps),
            QuoteStatus::QuoteMissing(QuoteMiss::BlankSpan),
            "a whitespace-only span matches everywhere; it must fail, not pass"
        );
    }

    #[test]
    fn a_zero_width_span_is_quote_missing_too() {
        let mut snaps = MemorySnapshots::new();
        snaps.insert(b"any document at all".to_vec());
        // U+200B ZERO WIDTH SPACE is *not* Unicode White_Space, so `trim` would
        // leave it and a trim-based check would call this a real quote.
        assert_eq!(
            verify_quote(&blank_quote("\u{200B}\u{FEFF}"), &snaps),
            QuoteStatus::QuoteMissing(QuoteMiss::BlankSpan),
            "invisible-but-not-whitespace is still invisible"
        );
    }

    #[test]
    fn the_blank_check_runs_before_the_snapshot_is_even_consulted() {
        // Ordering matters: if the store lookup ran first, a blank span with an
        // unavailable snapshot would report `Unverified` — a soft outcome — and
        // a reviewer scanning for hard failures would never see it.
        assert_eq!(
            verify_quote(&blank_quote(" "), &NoSnapshots),
            QuoteStatus::QuoteMissing(QuoteMiss::BlankSpan)
        );
    }
}
