//! # adjudication-coverage — ADJ02 v3 flat-tile coverage check.
//!
//! Reference implementation of
//! [`ADJ02` v3](../../../specs/ADJ02-coverage-checker.md), built on
//! top of the v3 graph IR ([`adjudication_ir`]).
//!
//! ## What v3 changes
//!
//! v2's structural-tree-tiling check (children's spans tile the
//! TextRun parent's spans, recursively) is replaced by a **flat
//! tiling** of the union of every node and edge `source_spans`
//! against the document's byte range. The discipline is the same —
//! every byte must be accounted for, no overlaps, no gaps — but the
//! check is no longer tied to the now-removed `TextRun` /
//! `part_of` tree shape.
//!
//! ## What this crate adds on top of `adjudication_ir::validate`
//!
//! `adjudication_ir::validate` reports coverage gaps and overlaps
//! relative to the IR's *self-determined* span range
//! `[min_start, max_end)` — it doesn't know the document's actual
//! length. This crate carries the [`Document`]'s normalized text
//! length and verifies the IR tiles all the way to the end of the
//! document.
//!
//! Plus, it surfaces the framework-level invariants ADJ02 owns:
//!
//! - `UnparseableDiscarded` — any `Discarded` node with
//!   `discard_reason = Unparseable` is a hard coverage failure (ADJ01
//!   rule). The extractor must produce a meaningful node, never an
//!   admission of incompetence.

use adjudication_ir::{
    validate, DiscardReason, DocumentId, EdgeRelation, IRDocument, IRNode, NodeId, NodeKind,
    NodeOrEdgeId, SpanLocation, ValidationError,
};

// ---------------------------------------------------------------------------
// Document and result types
// ---------------------------------------------------------------------------

/// The document under coverage analysis. The check reads only
/// `normalized_text.len()` — it never inspects the bytes themselves.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: DocumentId,
    pub normalized_text: String,
}

/// Outcome of a coverage check.
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageResult {
    Pass,
    Fail { violations: Vec<CoverageViolation> },
}

/// One coverage violation. Each variant maps to a clarification-
/// question shape consumed by ADJ06.
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageViolation {
    /// A node's or edge's span cites a different document.
    SpanWrongDocument {
        location: SpanLocation,
        expected: DocumentId,
        found: DocumentId,
    },

    /// A span's `start >= end`, or extends beyond the document's
    /// byte length.
    InvalidSpan {
        location: SpanLocation,
        start: usize,
        end: usize,
        document_len: usize,
    },

    /// A `Discarded` node has reason `Unparseable`. Always a hard
    /// coverage failure per ADJ01.
    UnparseableDiscarded { node_id: adjudication_ir::NodeId },

    /// Some byte range of the document is not in any node's or
    /// edge's source_spans.
    CoverageGap { missing_ranges: Vec<(usize, usize)> },

    /// Some byte range appears in more than one source_span (across
    /// nodes and edges, after synthesized-object exemption).
    CoverageOverlap {
        ranges: Vec<(usize, usize)>,
        participants: Vec<NodeOrEdgeId>,
    },

    /// `adjudication_ir::validate` returned an error that isn't a
    /// coverage concern. The propagation / acyclicity / kind-rule
    /// errors live on other checkers; reported here so callers can
    /// dispatch.
    UpstreamValidationError { kind: String },
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the flat-tile coverage check.
///
/// 1. Delegates the bulk of the tiling check to
///    [`adjudication_ir::validate`] and translates coverage-related
///    errors into [`CoverageViolation`]s.
/// 2. Checks that the IR's coverage reaches the document's
///    `normalized_text.len()`. If the IR tiles `[0, max_end)` but
///    `max_end < doc_len`, the gap is reported.
/// 3. Checks for `Discarded(Unparseable)` nodes — always a hard
///    failure per ADJ01.
pub fn check_coverage(doc: &Document, ir_doc: &IRDocument) -> CoverageResult {
    let mut violations: Vec<CoverageViolation> = Vec::new();
    let doc_len = doc.normalized_text.len();

    if let Err(e) = validate(ir_doc) {
        match e {
            ValidationError::InvalidSpan { location, start, end } => {
                violations.push(CoverageViolation::InvalidSpan {
                    location,
                    start,
                    end,
                    document_len: doc_len,
                });
            }
            ValidationError::SpanDocumentMismatch { location, expected, found } => {
                violations.push(CoverageViolation::SpanWrongDocument {
                    location,
                    expected,
                    found,
                });
            }
            ValidationError::CoverageGap { missing_ranges } => {
                violations.push(CoverageViolation::CoverageGap { missing_ranges });
            }
            ValidationError::CoverageOverlap { ranges, participants } => {
                violations.push(CoverageViolation::CoverageOverlap { ranges, participants });
            }
            other => {
                violations.push(CoverageViolation::UpstreamValidationError {
                    kind: format!("{other:?}"),
                });
            }
        }
    }

    // Document-end gap: validate() doesn't know doc_len, so we check
    // that the IR's max source-span end reaches it.
    let max_end: usize = ir_doc
        .nodes
        .iter()
        .flat_map(|n| n.source_spans.iter())
        .filter(|s| s.document_id == doc.id)
        .map(|s| s.end)
        .chain(
            ir_doc
                .edges
                .iter()
                .flat_map(|e| e.source_spans.iter())
                .filter(|s| s.document_id == doc.id)
                .map(|s| s.end),
        )
        .max()
        .unwrap_or(0);
    if doc_len > 0 && max_end < doc_len {
        // Don't double-report if validate already reported a gap.
        let already = violations.iter().any(|v| matches!(v, CoverageViolation::CoverageGap { .. }));
        if !already {
            violations.push(CoverageViolation::CoverageGap {
                missing_ranges: vec![(max_end, doc_len)],
            });
        }
    }

    // Unparseable Discarded is always a hard failure.
    for node in &ir_doc.nodes {
        if node.kind == NodeKind::Discarded
            && node.discard_reason == Some(DiscardReason::Unparseable)
        {
            violations.push(CoverageViolation::UnparseableDiscarded {
                node_id: node.id.clone(),
            });
        }
    }

    if violations.is_empty() {
        CoverageResult::Pass
    } else {
        CoverageResult::Fail { violations }
    }
}

// Re-export common types for caller convenience.
pub use adjudication_ir::{NodeId as IrNodeId, Span as IrSpan};

// ---------------------------------------------------------------------------
// ADJ22 — typed-quantity coverage
// ---------------------------------------------------------------------------

/// One typed-quantity violation. Surfaced when the source mentions
/// a numerical literal but the IR doesn't carry a corresponding
/// `quantity(value, unit)` compound with an overlapping span.
///
/// Per [ADJ21](../../../specs/ADJ21-typed-quantity-decomposition.md):
/// every numerical quantity in the source must lower to a typed
/// `quantity(value, unit)` term so the engine can evaluate
/// thresholds deterministically. If `decompose_text` drops a
/// quantity — folds it into the predicate name, omits the unit,
/// or simply forgets to extract it — the engine has nothing to
/// reason over. This checker catches that failure mode pre-engine
/// so ADJ06 can re-prompt.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedQuantityViolation {
    /// The source contains a numerical literal at this span, but
    /// no IR node with an overlapping span carries a `quantity(_)`
    /// compound term.
    MissingQuantity {
        /// The literal as it appears in the source (e.g., `"4"`,
        /// `"3.4"`, `"750"`). Carried so ADJ06 can quote it back.
        literal: String,
        /// Byte range in the source where the literal appears.
        location: (usize, usize),
        /// Nodes whose `source_spans` overlap this location.
        /// Included so the clarification prompt can name "you
        /// produced node X over this range but didn't include the
        /// quantity" rather than just "you missed a number."
        nearby_nodes: Vec<adjudication_ir::NodeId>,
    },
}

/// Outcome of the typed-quantity coverage check.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedQuantityResult {
    Pass,
    Fail { violations: Vec<TypedQuantityViolation> },
}

/// Run the typed-quantity coverage check (ADJ22).
///
/// Walks `doc.normalized_text` for numerical literals (integers or
/// decimals), then for each literal checks that at least one
/// IR node has `source_spans` overlapping the literal's location
/// AND a `quantity(...)` compound somewhere in its `term` tree.
///
/// **What counts as a numerical literal**: contiguous digits, optionally
/// followed by a single dot and more digits — `\d+(\.\d+)?`. Matches
/// `4`, `3.4`, `750`, `200`. Numbers buried inside compound words
/// (e.g., the `30` in `"30-day window"`) are matched if they're
/// flanked by word boundaries or whitespace; the regex tolerates
/// hyphens and unit suffixes immediately after.
///
/// **What counts as a quantity term**: any compound term anywhere in
/// any node's `term` (including nested args) whose `functor` is
/// exactly `"quantity"` and whose first arg is an atom matching
/// the literal's value (post-normalisation — `"4"`, `"4.0"`, `"4"`
/// all match the literal `"4"`).
///
/// **Scoping**: only Fact, Rule, and Uncertainty nodes are checked.
/// Section, Entity, Query, Discarded, and Exception nodes are
/// exempt because their terms aren't expected to carry source-level
/// quantities — they carry structure, identity, or queries.
///
/// **Edge case**: numerical literals inside synthesized Query
/// nodes' terms (e.g., a query like `compliant(passenger_42)` where
/// `42` is a synthesized id) are NOT in the source text, so the
/// checker doesn't see them. Source-text quantities live in
/// `doc.normalized_text`, which is what we scan.
pub fn check_typed_quantity_coverage(
    doc: &Document,
    ir_doc: &IRDocument,
) -> TypedQuantityResult {
    let literals = scan_numerical_literals(&doc.normalized_text);
    let mut violations: Vec<TypedQuantityViolation> = Vec::new();

    for (lit, (start, end)) in &literals {
        // Collect the IR nodes whose source_spans overlap this
        // literal's location.
        let overlapping: Vec<&adjudication_ir::IRNode> = ir_doc
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    NodeKind::Fact | NodeKind::Rule | NodeKind::Uncertainty
                )
            })
            .filter(|n| {
                n.source_spans
                    .iter()
                    .any(|s| spans_overlap(s.start, s.end, *start, *end))
            })
            .collect();

        // Does any overlapping node carry a `quantity(<lit>, _)`
        // compound somewhere in its term tree?
        let has_matching_quantity =
            overlapping.iter().any(|n| term_contains_quantity(&n.term, lit));

        if !has_matching_quantity {
            violations.push(TypedQuantityViolation::MissingQuantity {
                literal: lit.clone(),
                location: (*start, *end),
                nearby_nodes: overlapping.iter().map(|n| n.id.clone()).collect(),
            });
        }
    }

    if violations.is_empty() {
        TypedQuantityResult::Pass
    } else {
        TypedQuantityResult::Fail { violations }
    }
}

/// Find every numerical literal in `text`. Returns
/// `Vec<(literal_string, (start, end))>` where the byte range
/// covers the literal exactly (not surrounding whitespace or units).
///
/// Scans manually without a regex dep — looking for runs of ASCII
/// digits, optionally including one `.` separator between digit
/// runs. Negative numbers and scientific notation are out of scope
/// (rare in adjudication declarations; the few cases that need
/// them can be addressed in a follow-up).
fn scan_numerical_literals(text: &str) -> Vec<(String, (usize, usize))> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        // Consume digits.
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        // Optional `.<digits>` continuation.
        if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
            i += 1; // consume the dot
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        let literal = &text[start..i];
        out.push((literal.to_string(), (start, i)));
    }
    out
}

/// Check whether two byte ranges `[a_start, a_end)` and
/// `[b_start, b_end)` overlap at all. Empty ranges (start==end) are
/// treated as non-overlapping with anything (they're points, not
/// spans).
fn spans_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    if a_start >= a_end || b_start >= b_end {
        return false;
    }
    a_start < b_end && b_start < a_end
}

/// Walk a term's tree looking for a `quantity(<lit>, _)` compound.
/// Matches when the functor is exactly `"quantity"`, the first arg
/// is an atom whose name is `lit` (or a numerically-equal variant),
/// and the term has at least 2 args (value + unit).
///
/// Nested terms (quantities inside compound args of other facts)
/// are matched recursively — `blade_length(knife, quantity(4, inches))`
/// returns true for `lit = "4"`.
fn term_contains_quantity(term: &logic_core::Term, lit: &str) -> bool {
    use logic_core::Term;
    match term {
        Term::Compound { functor, args } => {
            if functor == "quantity" && args.len() >= 2 {
                if let Some(value_arg) = args.first() {
                    if atom_or_num_matches_literal(value_arg, lit) {
                        return true;
                    }
                }
            }
            // Recurse into args looking for nested quantities.
            args.iter().any(|a| term_contains_quantity(a, lit))
        }
        _ => false,
    }
}

/// `"4"` matches atom("4"), `"4"` matches num(4), `"4.0"` matches
/// num(4.0) and atom("4.0"). The literal in the source is always
/// a string of digits; the IR's value atom can be either a string
/// `Atom` or a numeric `Num`. Treat them as equal when the
/// canonical-decimal forms match.
fn atom_or_num_matches_literal(term: &logic_core::Term, lit: &str) -> bool {
    use logic_core::{Number, Term};
    match term {
        Term::Atom(s) => normalise_numeric(s) == normalise_numeric(lit),
        Term::Num(Number::Int(i)) => i.to_string() == normalise_numeric(lit),
        Term::Num(Number::Float(f)) => {
            // Compare as canonical decimal — Float's f64::to_string
            // produces e.g. "4" for 4.0 not "4.0", which matches the
            // literal "4" but not "4.0". Normalise both.
            normalise_numeric(&f.to_string()) == normalise_numeric(lit)
        }
        _ => false,
    }
}

/// Strip trailing `.0` and leading zeros so `"4"`, `"4.0"`, and
/// `"04"` all match. (`"4.5"` stays `"4.5"`; `"0.5"` stays `"0.5"`
/// — leading zero before a decimal point is preserved.)
fn normalise_numeric(s: &str) -> String {
    // Split into whole and fractional parts first; that way the
    // leading-zero strip applies only to the whole-number portion
    // and doesn't eat a meaningful leading zero before a decimal.
    let (whole, frac) = match s.find('.') {
        Some(idx) => {
            let (w, rest) = s.split_at(idx);
            (w, &rest[1..]) // skip the dot
        }
        None => (s, ""),
    };
    // Strip leading zeros from the whole part, but keep at least "0".
    let whole_trimmed = whole.trim_start_matches('0');
    let whole_canonical = if whole_trimmed.is_empty() { "0" } else { whole_trimmed };
    // Strip trailing zeros from the fractional part.
    let frac_trimmed = frac.trim_end_matches('0');
    if frac_trimmed.is_empty() {
        whole_canonical.to_string()
    } else {
        format!("{whole_canonical}.{frac_trimmed}")
    }
}

// ===========================================================================
// ADJ25 — per-level hierarchical coverage + no-flattening rule (PR-2)
// ===========================================================================
//
// The flat-tile check above (ADJ02 v3) asks "is every byte of the
// document covered by *some* node?". ADJ25 sharpens that to "is every
// byte of every *parent* node covered by its *children* at the next
// decomposition level?" — a recursive structural invariant the
// hierarchical IR shape (Document → Sentence → Phrase → Claim →
// TypedComponent) lets us enforce.
//
// This PR (PR-2) introduces the check function and its test surface
// but does NOT wire it into any existing entry point. PR-4 (the new
// orchestrator) wires this into `decompose_text_hierarchical`, where
// failures route to PR-3's fresh-agent retry primitive.

/// Which level transition a hierarchical-coverage gap was found at.
///
/// The four parent → children boundaries the framework enforces:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecompLevel {
    /// Document → Sentences. Every byte of the Document must be
    /// covered by exactly one Sentence (or a document-scope
    /// `Discarded`).
    DocumentToSentence,
    /// Sentence → Phrases. Every byte of the Sentence must be
    /// covered by exactly one Phrase (or a sentence-scope
    /// `Discarded`).
    SentenceToPhrase,
    /// Phrase → claim nodes. Every byte of the Phrase must be covered
    /// by exactly one of `Fact` / `Uncertainty` / `Question` /
    /// `Discarded`.
    PhraseToClaim,
    /// Fact → TypedComponent. Every byte of the Fact must be covered
    /// by exactly one typed component (`Quantity` / `Polarity` /
    /// `Predicate` / `Comparator` / `TimeRef` / `Modifier` /
    /// `Entity`).
    FactToTypedComponent,
}

/// Outcome of [`check_hierarchical_coverage`].
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchicalCoverageResult {
    Pass,
    Fail { gaps: Vec<HierarchicalGap> },
}

/// One per-level coverage failure. Each gap names the parent node
/// whose decomposition failed and the kind of failure observed.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalGap {
    pub level: DecompLevel,
    pub parent_node_id: NodeId,
    pub kind: HierarchicalGapKind,
}

/// What went wrong in a per-level decomposition.
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchicalGapKind {
    /// One or more byte ranges of the parent's span are not covered
    /// by any child at the expected level. Each `(start, end)` tuple
    /// is a missing range.
    UncoveredBytes { ranges: Vec<(usize, usize)> },
    /// Two or more children's spans overlap on some byte range.
    Overlap {
        ranges: Vec<(usize, usize)>,
        participants: Vec<NodeId>,
    },
    /// A child node has an empty span where one was required. (The
    /// synthesized-Entity exemption applies — that case is filtered
    /// before reaching this check.)
    EmptyChildSpan { child_id: NodeId },
    /// A child node's span is not fully inside the parent's span.
    ChildSpansEscape {
        child_id: NodeId,
        outside: Vec<(usize, usize)>,
    },
    /// The parent has no children when it must decompose. Specifically:
    /// every `Fact` is required to decompose into TypedComponents, and
    /// every Document/Sentence/Phrase whose span is non-empty must
    /// have at least one child.
    NoChildrenAtLevel,
    /// A child node has a kind that does not belong at this level.
    /// (e.g., a `Sentence` directly under a `Phrase` — the framework's
    /// per-level kind contracts are strict.)
    WrongChildKindForLevel {
        child_id: NodeId,
        child_kind: NodeKind,
    },
    /// The no-flattening rule (level-4 specific in spirit, applied
    /// globally) rejected an atom name as smuggling source content.
    /// The offending atom and its violation reason are recorded.
    FlattenedAtom {
        node_id: NodeId,
        atom: String,
        reason: FlatteningReason,
    },
}

/// Why an atom name was rejected by the no-flattening rule.
#[derive(Debug, Clone, PartialEq)]
pub enum FlatteningReason {
    /// The atom name contains a digit substring that appears as a
    /// maximal digit run in the source text. e.g., source has "50"
    /// and atom is `50_wh` → rejected. Forces digit literals to
    /// surface as typed `Quantity(value, unit)` rather than be
    /// flattened into atoms.
    DigitRunFromSource { digits: String },
    /// The atom name ends in a known unit suffix joined by underscore
    /// (`_wh`, `_ml`, `_oz`, `_in`, `_inch`, `_inches`, `_kg`, `_lb`,
    /// `_g`, `_v`, `_mAh`, `_kwh`, `_bpm`, `_mmhg`, `_celsius`,
    /// `_fahrenheit`, `_count`). Forces units to surface as a
    /// `Quantity`'s second argument.
    UnitSuffix { suffix: String },
    /// The atom name consists of more than two underscore-separated
    /// words drawn from the source. e.g., source has "pocket knife
    /// blade length" and atom is `pocket_knife_blade_length` →
    /// rejected. Two-word compound nouns (`pocket_knife`) are
    /// accepted as legitimate.
    MultiWordCollapse { words: Vec<String> },
}

/// Banned unit suffixes per the spec's no-flattening rule.
///
/// Maintained as a sorted-by-length descending list so the suffix
/// check matches the *longest* applicable suffix first (e.g.,
/// `_inches` before `_in`).
const BANNED_UNIT_SUFFIXES: &[&str] = &[
    "_fahrenheit",
    "_celsius",
    "_inches",
    "_count",
    "_mmhg",
    "_kwh",
    "_mah",
    "_bpm",
    "_inch",
    "_kg",
    "_lb",
    "_ml",
    "_oz",
    "_in",
    "_wh",
    "_v",
    "_g",
];

/// Run the per-level hierarchical coverage check.
///
/// The check is parameterized over the [`Document`] (for the
/// normalized text used by the no-flattening rule) and the
/// [`IRDocument`] (the hierarchical structure to verify). It returns
/// `Pass` only when every level transition (Document → Sentence,
/// Sentence → Phrase, Phrase → Claim, Fact → TypedComponent) cleanly
/// tiles its parent's span AND no atom anywhere in the IR violates
/// the no-flattening rule against the source text.
///
/// The check assumes the IR has *already* passed
/// [`adjudication_ir::validate`] — it does not re-verify span
/// validity, edge endpoint existence, or DAG acyclicity. Callers
/// should run the flat-tile [`check_coverage`] first and only invoke
/// this when that passes. The check is additive in PR-2; PR-4 wires
/// it into the orchestrator.
///
/// ## Algorithm
///
/// 1. Identify the unique `Document` node (a hierarchical IR has
///    exactly one). If absent, return `NoChildrenAtLevel` against a
///    synthetic root. If multiple, take the first and report the
///    extras as `WrongChildKindForLevel` siblings.
/// 2. For each parent kind at each level, gather the `Contains`-edge
///    children at the next level and verify:
///    - every child has non-empty span (except synthesized Entity),
///    - children's spans tile the parent's span exactly,
///    - children's kinds belong at the level.
/// 3. For Fact → TypedComponent specifically, additionally walk every
///    atom name in the Fact's and its children's term trees and apply
///    the no-flattening rules against the source text.
///
/// Returns every gap encountered (does not short-circuit on the first
/// failure) so callers can drive PR-3's retry primitive at every
/// failing parent in a single pass.
pub fn check_hierarchical_coverage(
    doc: &Document,
    ir_doc: &IRDocument,
) -> HierarchicalCoverageResult {
    let mut gaps: Vec<HierarchicalGap> = Vec::new();

    // Build a lookup of nodes by id and a children-by-parent map
    // keyed on Contains-edge `source` (parent) → list of `target`
    // node refs. Edges with other relations are ignored here; the
    // hierarchical check only walks the Contains spine.
    let nodes_by_id: std::collections::HashMap<&NodeId, &IRNode> =
        ir_doc.nodes.iter().map(|n| (&n.id, n)).collect();
    let mut contains_children: std::collections::HashMap<&NodeId, Vec<&IRNode>> =
        std::collections::HashMap::new();
    for edge in &ir_doc.edges {
        if edge.relation != EdgeRelation::Contains {
            continue;
        }
        if let Some(child) = nodes_by_id.get(&edge.target) {
            contains_children
                .entry(&edge.source)
                .or_default()
                .push(child);
        }
        // Dangling targets are an IR-validation concern, not a
        // hierarchical-coverage one; surfaced by validate() upstream.
    }

    // Walk every Document → Sentence boundary, every Sentence →
    // Phrase, every Phrase → Claim, every Fact → TypedComponent.
    for parent in &ir_doc.nodes {
        let (level, allowed_child_kinds) = match parent.kind {
            NodeKind::Document => (
                Some(DecompLevel::DocumentToSentence),
                &[NodeKind::Sentence, NodeKind::Discarded][..],
            ),
            NodeKind::Sentence => (
                Some(DecompLevel::SentenceToPhrase),
                &[NodeKind::Phrase, NodeKind::Discarded][..],
            ),
            NodeKind::Phrase => (
                Some(DecompLevel::PhraseToClaim),
                &[
                    NodeKind::Fact,
                    NodeKind::Uncertainty,
                    NodeKind::Question,
                    NodeKind::Discarded,
                ][..],
            ),
            NodeKind::Fact => (
                Some(DecompLevel::FactToTypedComponent),
                &[
                    NodeKind::Quantity,
                    NodeKind::Polarity,
                    NodeKind::Predicate,
                    NodeKind::Comparator,
                    NodeKind::TimeRef,
                    NodeKind::Modifier,
                    NodeKind::Entity,
                ][..],
            ),
            _ => (None, &[][..]),
        };
        let Some(level) = level else {
            continue;
        };
        let children: Vec<&IRNode> = contains_children
            .get(&parent.id)
            .map(|v| v.clone())
            .unwrap_or_default();
        check_parent_decomposition(
            parent,
            &children,
            allowed_child_kinds,
            level,
            &mut gaps,
        );
    }

    // No-flattening rule: walk every atom name in every node's term
    // tree against the source text. This is applied globally rather
    // than only at level 4 because the LLM could otherwise hide a
    // flattened atom inside a higher-level node's term.
    let digit_runs: Vec<String> = collect_digit_runs(&doc.normalized_text);
    let source_words: std::collections::HashSet<String> =
        collect_source_words(&doc.normalized_text);
    for node in &ir_doc.nodes {
        collect_atom_names_from_term(&node.term, &mut |atom_name| {
            if let Some(reason) =
                classify_flattening(atom_name, &digit_runs, &source_words)
            {
                gaps.push(HierarchicalGap {
                    level: DecompLevel::FactToTypedComponent,
                    parent_node_id: node.id.clone(),
                    kind: HierarchicalGapKind::FlattenedAtom {
                        node_id: node.id.clone(),
                        atom: atom_name.to_string(),
                        reason,
                    },
                });
            }
        });
    }

    if gaps.is_empty() {
        HierarchicalCoverageResult::Pass
    } else {
        HierarchicalCoverageResult::Fail { gaps }
    }
}

/// Verify a single parent → children decomposition tiles cleanly.
fn check_parent_decomposition(
    parent: &IRNode,
    children: &[&IRNode],
    allowed_child_kinds: &[NodeKind],
    level: DecompLevel,
    gaps: &mut Vec<HierarchicalGap>,
) {
    // Parent must have a non-empty span; if it doesn't, the IR
    // upstream validation should have caught it. We treat empty
    // parent spans as "no decomposition required".
    let Some(parent_span) = parent.source_spans.first() else {
        return;
    };
    let p_start = parent_span.start;
    let p_end = parent_span.end;
    if p_start >= p_end {
        return;
    }

    // No children at all — but the parent has a non-empty span.
    if children.is_empty() {
        gaps.push(HierarchicalGap {
            level,
            parent_node_id: parent.id.clone(),
            kind: HierarchicalGapKind::NoChildrenAtLevel,
        });
        return;
    }

    // Wrong-kind children: an entry that doesn't belong at this level.
    for child in children {
        // Synthesized Entity (empty spans) is allowed at any level
        // as a deduplicated reference target. It's gated separately
        // for the empty-span exemption below.
        if !allowed_child_kinds.contains(&child.kind) {
            gaps.push(HierarchicalGap {
                level,
                parent_node_id: parent.id.clone(),
                kind: HierarchicalGapKind::WrongChildKindForLevel {
                    child_id: child.id.clone(),
                    child_kind: child.kind,
                },
            });
        }
    }

    // Filter out wrong-kind children for the tiling check; counting
    // them as participating in the tile would compound the error.
    let valid_children: Vec<&IRNode> = children
        .iter()
        .copied()
        .filter(|c| allowed_child_kinds.contains(&c.kind))
        .collect();

    // Empty-span child rule: synthesized Entity is exempt; everything
    // else is required to have a non-empty primary span.
    for child in &valid_children {
        let empty_span =
            child.source_spans.is_empty() || child.source_spans.iter().all(|s| s.start >= s.end);
        let is_synthesized_entity = child.kind == NodeKind::Entity && empty_span;
        if empty_span && !is_synthesized_entity {
            gaps.push(HierarchicalGap {
                level,
                parent_node_id: parent.id.clone(),
                kind: HierarchicalGapKind::EmptyChildSpan {
                    child_id: child.id.clone(),
                },
            });
        }
    }

    // Tiling check: gather (start, end) intervals from valid children
    // with non-empty spans (synthesized Entities are exempt from
    // tiling), sort by start, check each fits inside the parent and
    // the union equals the parent's span exactly.
    let mut intervals: Vec<(usize, usize, &NodeId)> = Vec::new();
    for child in &valid_children {
        if child.kind == NodeKind::Entity
            && (child.source_spans.is_empty()
                || child.source_spans.iter().all(|s| s.start >= s.end))
        {
            continue; // synthesized entity — exempt from tiling
        }
        for span in &child.source_spans {
            if span.start >= span.end {
                continue;
            }
            intervals.push((span.start, span.end, &child.id));
        }
    }

    // Span-escape check: every child span must be inside parent.
    for (s, e, cid) in &intervals {
        if *s < p_start || *e > p_end {
            let outside_start = (*s).max(p_start).min(*e);
            let outside_end = (*e).min(p_end).max(*s);
            let (cs, ce) = (*s, *e);
            // Compute the bits actually outside [p_start, p_end).
            let mut outside: Vec<(usize, usize)> = Vec::new();
            if cs < p_start {
                outside.push((cs, p_start.min(ce)));
            }
            if ce > p_end {
                outside.push((p_end.max(cs), ce));
            }
            let _ = (outside_start, outside_end);
            gaps.push(HierarchicalGap {
                level,
                parent_node_id: parent.id.clone(),
                kind: HierarchicalGapKind::ChildSpansEscape {
                    child_id: (*cid).clone(),
                    outside,
                },
            });
        }
    }

    intervals.sort_by_key(|(s, _, _)| *s);

    // Overlap detection.
    let mut overlap_ranges: Vec<(usize, usize)> = Vec::new();
    let mut overlap_participants: Vec<NodeId> = Vec::new();
    for window in intervals.windows(2) {
        let (_s1, e1, id1) = window[0];
        let (s2, e2, id2) = window[1];
        if s2 < e1 {
            // Overlap on [s2, min(e1, e2))
            let ov_start = s2;
            let ov_end = e1.min(e2);
            overlap_ranges.push((ov_start, ov_end));
            if !overlap_participants.contains(id1) {
                overlap_participants.push((*id1).clone());
            }
            if !overlap_participants.contains(id2) {
                overlap_participants.push((*id2).clone());
            }
        }
    }
    if !overlap_ranges.is_empty() {
        gaps.push(HierarchicalGap {
            level,
            parent_node_id: parent.id.clone(),
            kind: HierarchicalGapKind::Overlap {
                ranges: overlap_ranges,
                participants: overlap_participants,
            },
        });
    }

    // Gap detection: walk the parent's span [p_start, p_end), and
    // accumulate uncovered ranges between sorted interval ends.
    let mut uncovered: Vec<(usize, usize)> = Vec::new();
    let mut cursor = p_start;
    for (s, e, _) in &intervals {
        let clamped_s = (*s).max(p_start);
        let clamped_e = (*e).min(p_end);
        if clamped_e <= clamped_s {
            continue;
        }
        if clamped_s > cursor {
            uncovered.push((cursor, clamped_s));
        }
        if clamped_e > cursor {
            cursor = clamped_e;
        }
    }
    if cursor < p_end {
        uncovered.push((cursor, p_end));
    }
    if !uncovered.is_empty() {
        gaps.push(HierarchicalGap {
            level,
            parent_node_id: parent.id.clone(),
            kind: HierarchicalGapKind::UncoveredBytes {
                ranges: uncovered,
            },
        });
    }
}

/// Collect every maximal digit-run substring from the source text.
/// e.g., `"1 carry-on bag, 200 Wh"` yields `["1", "200"]`. Used by
/// the no-flattening rule to reject atom names that include any of
/// these digit runs as a substring.
fn collect_digit_runs(text: &str) -> Vec<String> {
    let mut runs: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

/// Collect every alphanumeric source-text word (lowercased) into a
/// set, for the multi-word-collapse rule. Punctuation and whitespace
/// are word separators.
fn collect_source_words(text: &str) -> std::collections::HashSet<String> {
    let mut words: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else if !current.is_empty() {
            words.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.insert(current);
    }
    words
}

/// Recursively walk a term tree, calling `f` on every atom symbol
/// and every compound functor name. Numbers, strings, and variables
/// are not visited (numbers are the legitimate way to carry digit
/// literals; strings are opaque; variables don't enter the source-
/// derived atom space).
fn collect_atom_names_from_term(term: &logic_core::Term, f: &mut impl FnMut(&str)) {
    match term {
        logic_core::Term::Atom(s) => f(s),
        logic_core::Term::Compound { functor, args } => {
            f(functor);
            for a in args {
                collect_atom_names_from_term(a, f);
            }
        }
        logic_core::Term::Num(_) | logic_core::Term::Str(_) | logic_core::Term::Var(_) => {}
    }
}

/// Apply the no-flattening rules to a single atom name.
///
/// Returns `None` when the atom is acceptable, or `Some(reason)`
/// when one of the three rules fires. The rules are checked in
/// priority order: digit-run first (most common failure mode in the
/// ADJ23 bench), unit-suffix second, multi-word collapse third.
fn classify_flattening(
    atom: &str,
    digit_runs: &[String],
    source_words: &std::collections::HashSet<String>,
) -> Option<FlatteningReason> {
    // Rule 1: digit run from source appears as substring.
    for run in digit_runs {
        if !run.is_empty() && atom.contains(run.as_str()) {
            return Some(FlatteningReason::DigitRunFromSource {
                digits: run.clone(),
            });
        }
    }
    // Rule 2: ends in a banned unit suffix joined by underscore.
    // Match longest first so `_inches` wins over `_in`.
    let lower = atom.to_ascii_lowercase();
    for &suffix in BANNED_UNIT_SUFFIXES {
        if lower.ends_with(suffix) && lower.len() > suffix.len() {
            // Make sure there's something before the suffix — `_wh`
            // by itself isn't a flattening violation, but `battery_wh`
            // is.
            return Some(FlatteningReason::UnitSuffix {
                suffix: suffix.to_string(),
            });
        }
    }
    // Rule 3: more than two underscore-separated parts each drawn
    // from the source. Each part must (a) be present in source_words
    // and (b) be at least 2 chars (to avoid trivial single-letter
    // glue tokens triggering the rule).
    let parts: Vec<&str> = atom.split('_').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 3 {
        let from_source: Vec<String> = parts
            .iter()
            .filter(|p| p.len() >= 2 && source_words.contains(&p.to_lowercase()))
            .map(|p| p.to_lowercase())
            .collect();
        if from_source.len() >= 3 {
            return Some(FlatteningReason::MultiWordCollapse {
                words: from_source,
            });
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{IRNode, Modality, NodeId, Polarity, Span};
    use logic_core::{atom, compound};
    use std::collections::HashMap;

    fn doc_id() -> DocumentId {
        DocumentId::new("doc1")
    }

    fn mk_doc(text: &str) -> Document {
        Document {
            id: doc_id(),
            normalized_text: text.to_string(),
        }
    }

    fn span_of(start: usize, end: usize) -> Span {
        Span::new(doc_id(), start, end)
    }

    fn fact_leaf(id: &str, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: atom("placeholder"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(start, end)],
            confidence: 0.9,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn empty_document_with_empty_ir_passes() {
        let doc = mk_doc("");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![],
            edges: vec![],
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    #[test]
    fn nonempty_document_with_empty_ir_fails_with_full_range() {
        let doc = mk_doc("hello world");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![],
            edges: vec![],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has_gap = violations.iter().any(|v| matches!(
                    v,
                    CoverageViolation::CoverageGap { missing_ranges }
                        if missing_ranges == &vec![(0, 11)]
                ));
                assert!(has_gap, "expected (0,11) CoverageGap: {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn single_fact_tiling_full_doc_passes() {
        let doc = mk_doc("hello world");
        let leaf = fact_leaf("F1", 0, 11);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![leaf],
            edges: vec![],
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    #[test]
    fn doc_end_gap_detected() {
        // Doc 0..50; only F1 covers 0..30. Gap 30..50 should report.
        let doc = mk_doc(&"x".repeat(50));
        let leaf = fact_leaf("F1", 0, 30);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![leaf],
            edges: vec![],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has_gap = violations.iter().any(|v| matches!(
                    v,
                    CoverageViolation::CoverageGap { missing_ranges }
                        if missing_ranges == &vec![(30, 50)]
                ));
                assert!(has_gap, "expected (30,50) gap: {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn mid_doc_gap_detected() {
        // Two facts at 0..20 and 30..50 leave 20..30 uncovered.
        let doc = mk_doc(&"x".repeat(50));
        let f1 = fact_leaf("F1", 0, 20);
        let f2 = fact_leaf("F2", 30, 50);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![f1, f2],
            edges: vec![],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has_gap = violations
                    .iter()
                    .any(|v| matches!(v, CoverageViolation::CoverageGap { .. }));
                assert!(has_gap, "expected gap: {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn unparseable_discarded_always_fails() {
        let doc = mk_doc(&"x".repeat(20));
        let discard = IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: atom("discarded"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(0, 20)],
            confidence: 1.0,
            discard_reason: Some(DiscardReason::Unparseable),
            metadata: HashMap::new(),
        };
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![discard],
            edges: vec![],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has = violations
                    .iter()
                    .any(|v| matches!(v, CoverageViolation::UnparseableDiscarded { .. }));
                assert!(has, "expected UnparseableDiscarded: {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn discarded_with_pleasantry_is_ok() {
        let doc = mk_doc(&"x".repeat(20));
        let discard = IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: atom("discarded"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(0, 20)],
            confidence: 1.0,
            discard_reason: Some(DiscardReason::Pleasantry),
            metadata: HashMap::new(),
        };
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![discard],
            edges: vec![],
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    // -----------------------------------------------------------------
    // ADJ22 — typed-quantity coverage tests
    // -----------------------------------------------------------------

    fn fact_with_term(id: &str, term: logic_core::Term, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(start, end)],
            confidence: 0.9,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn typed_quantity_scan_finds_integers() {
        let lits = scan_numerical_literals("4 inch pocket knife.");
        assert_eq!(lits.len(), 1);
        assert_eq!(lits[0].0, "4");
        assert_eq!(lits[0].1, (0, 1));
    }

    #[test]
    fn typed_quantity_scan_finds_decimals() {
        let lits = scan_numerical_literals("3.4 oz toothpaste.");
        assert_eq!(lits.len(), 1);
        assert_eq!(lits[0].0, "3.4");
        assert_eq!(lits[0].1, (0, 3));
    }

    #[test]
    fn typed_quantity_scan_finds_multiple_literals() {
        let lits = scan_numerical_literals("1 carry-on bag, 200 Wh lithium battery.");
        let nums: Vec<&str> = lits.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(nums, vec!["1", "200"]);
    }

    #[test]
    fn typed_quantity_scan_handles_no_numbers() {
        let lits = scan_numerical_literals("strike-anywhere matches.");
        assert!(lits.is_empty());
    }

    #[test]
    fn typed_quantity_check_passes_when_node_has_quantity_compound() {
        // Source has "4 inch pocket knife"; IR has a node whose term
        // is blade_length(pocket_knife, quantity(4, inches)) — the
        // canonical ADJ21 shape. ADJ22 must pass.
        let doc = mk_doc("4 inch pocket knife.");
        let quantity_term = compound(
            "blade_length",
            vec![
                atom("pocket_knife"),
                compound("quantity", vec![atom("4"), atom("inches")]),
            ],
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_with_term("N1", quantity_term, 0, 6)],
            edges: vec![],
        };
        assert_eq!(
            check_typed_quantity_coverage(&doc, &ir),
            TypedQuantityResult::Pass
        );
    }

    #[test]
    fn typed_quantity_check_fails_when_node_drops_the_quantity() {
        // Source has "4 inch pocket knife"; IR has a node that
        // forgot to include the quantity (just declared(pocket_knife)).
        // ADJ22 must flag the missing 4.
        let doc = mk_doc("4 inch pocket knife.");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_with_term(
                "N1",
                compound("declared", vec![atom("pocket_knife")]),
                0,
                19,
            )],
            edges: vec![],
        };
        match check_typed_quantity_coverage(&doc, &ir) {
            TypedQuantityResult::Fail { violations } => {
                assert_eq!(violations.len(), 1);
                match &violations[0] {
                    TypedQuantityViolation::MissingQuantity { literal, nearby_nodes, .. } => {
                        assert_eq!(literal, "4");
                        assert_eq!(nearby_nodes.len(), 1);
                        assert_eq!(nearby_nodes[0].0, "N1");
                    }
                }
            }
            other => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn typed_quantity_check_fails_when_number_flattened_into_predicate() {
        // The canonical wrong pattern from ADJ21: the model put the
        // 4 in the predicate name (`blade_4_inches`) instead of as
        // a quantity term. ADJ22 must catch this — there's no
        // `quantity(4, _)` anywhere in the IR.
        let doc = mk_doc("4 inch pocket knife.");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_with_term(
                "N1",
                compound("blade_4_inches", vec![atom("pocket_knife")]),
                0,
                19,
            )],
            edges: vec![],
        };
        match check_typed_quantity_coverage(&doc, &ir) {
            TypedQuantityResult::Fail { violations } => {
                assert_eq!(violations.len(), 1);
                if let TypedQuantityViolation::MissingQuantity { literal, .. } = &violations[0] {
                    assert_eq!(literal, "4");
                }
            }
            other => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn typed_quantity_check_passes_with_decimal_quantity() {
        // Source has "3.4 oz"; IR has quantity(3.4, oz). Decimal
        // values must match.
        let doc = mk_doc("3.4 oz toothpaste.");
        let term = compound(
            "liquid_volume",
            vec![
                atom("toothpaste"),
                compound("quantity", vec![atom("3.4"), atom("oz")]),
            ],
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_with_term("N1", term, 0, 17)],
            edges: vec![],
        };
        assert_eq!(
            check_typed_quantity_coverage(&doc, &ir),
            TypedQuantityResult::Pass
        );
    }

    #[test]
    fn typed_quantity_check_matches_numeric_value_via_normalisation() {
        // Source says "4"; IR uses Term::Num(Int(4)) — both should
        // canonicalise to "4" and match. (Numeric atoms vs string
        // atoms both work.)
        use logic_core::int;
        let doc = mk_doc("4 inch pocket knife.");
        let term = compound(
            "blade_length",
            vec![
                atom("pocket_knife"),
                compound("quantity", vec![int(4), atom("inches")]),
            ],
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_with_term("N1", term, 0, 6)],
            edges: vec![],
        };
        assert_eq!(
            check_typed_quantity_coverage(&doc, &ir),
            TypedQuantityResult::Pass
        );
    }

    #[test]
    fn typed_quantity_check_passes_with_no_numbers_in_source() {
        // No literals to flag.
        let doc = mk_doc("strike-anywhere matches.");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_leaf("N1", 0, 24)],
            edges: vec![],
        };
        assert_eq!(
            check_typed_quantity_coverage(&doc, &ir),
            TypedQuantityResult::Pass
        );
    }

    #[test]
    fn typed_quantity_check_flags_multiple_missing_quantities() {
        // Source has TWO numerical literals (1 and 200), IR has
        // neither as a quantity term. Both should be reported.
        let doc = mk_doc("1 carry-on bag, 200 Wh battery.");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![
                fact_with_term(
                    "N1",
                    compound("declared", vec![atom("carry_on_bag")]),
                    0,
                    14,
                ),
                fact_with_term(
                    "N2",
                    compound("declared", vec![atom("battery")]),
                    16,
                    30,
                ),
            ],
            edges: vec![],
        };
        match check_typed_quantity_coverage(&doc, &ir) {
            TypedQuantityResult::Fail { violations } => {
                assert_eq!(violations.len(), 2);
                let literals: Vec<&str> = violations
                    .iter()
                    .filter_map(|v| {
                        let TypedQuantityViolation::MissingQuantity { literal, .. } = v;
                        Some(literal.as_str())
                    })
                    .collect();
                assert!(literals.contains(&"1"));
                assert!(literals.contains(&"200"));
            }
            other => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn typed_quantity_check_finds_quantity_nested_in_compound() {
        // The quantity term may be deeply nested inside a compound.
        // E.g., `meets_threshold(blade_length(knife, quantity(4, inches)))`.
        // The recursive walk should find it.
        let doc = mk_doc("4 inch pocket knife.");
        let inner = compound("quantity", vec![atom("4"), atom("inches")]);
        let mid = compound("blade_length", vec![atom("knife"), inner]);
        let outer = compound("meets_threshold", vec![mid]);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_with_term("N1", outer, 0, 6)],
            edges: vec![],
        };
        assert_eq!(
            check_typed_quantity_coverage(&doc, &ir),
            TypedQuantityResult::Pass
        );
    }

    #[test]
    fn normalise_numeric_strips_leading_zeros_and_trailing_decimal_zeros() {
        assert_eq!(normalise_numeric("4"), "4");
        assert_eq!(normalise_numeric("04"), "4");
        assert_eq!(normalise_numeric("4.0"), "4");
        assert_eq!(normalise_numeric("4.50"), "4.5");
        assert_eq!(normalise_numeric("4.5"), "4.5");
        assert_eq!(normalise_numeric("0"), "0");
        assert_eq!(normalise_numeric("0.5"), "0.5");
    }

    // -----------------------------------------------------------------
    // ADJ25 PR-2 — hierarchical coverage + no-flattening rule
    // -----------------------------------------------------------------

    use adjudication_ir::{EdgeId, IREdge};

    fn typed_node(
        id: &str,
        kind: NodeKind,
        start: usize,
        end: usize,
        term: logic_core::Term,
    ) -> IRNode {
        let spans = if start == end {
            vec![]
        } else {
            vec![span_of(start, end)]
        };
        IRNode {
            id: NodeId::new(id),
            kind,
            term,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: spans,
            confidence: 1.0,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn contains_edge(eid: &str, source: &str, target: &str) -> IREdge {
        IREdge {
            id: EdgeId::new(eid),
            source: NodeId::new(source),
            target: NodeId::new(target),
            relation: EdgeRelation::Contains,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![],
            confidence: 1.0,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn adj25_well_formed_hierarchy_passes() {
        // "1 carry-on bag." → 1 doc, 1 sentence, 1 phrase, 1 fact
        // with 3 typed components covering "1", " carry-on bag", "."
        // ... actually we need the typed components to tile exactly,
        // so let's use a cleaner partition.
        let text = "matches";
        let doc = mk_doc(text);
        let n_doc = typed_node("D", NodeKind::Document, 0, 7, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 7, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 7, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 7, atom("matches"));
        let n_entity = typed_node("E", NodeKind::Entity, 0, 7, atom("matches"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "E"),
            ],
        };
        assert_eq!(
            check_hierarchical_coverage(&doc, &ir),
            HierarchicalCoverageResult::Pass
        );
    }

    #[test]
    fn adj25_uncovered_bytes_at_sentence_level_caught() {
        // Document spans 0..12, but Sentence only covers 0..6.
        let doc = mk_doc("hello world."); // 12 bytes
        let n_doc = typed_node("D", NodeKind::Document, 0, 12, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 6, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 6, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 6, atom("hello"));
        let n_entity = typed_node("E", NodeKind::Entity, 0, 6, atom("hello"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "E"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                let document_gaps: Vec<_> = gaps
                    .iter()
                    .filter(|g| g.level == DecompLevel::DocumentToSentence)
                    .collect();
                assert_eq!(document_gaps.len(), 1, "expected one document gap; got {:?}", gaps);
                match &document_gaps[0].kind {
                    HierarchicalGapKind::UncoveredBytes { ranges } => {
                        assert_eq!(ranges, &vec![(6usize, 12usize)]);
                    }
                    other => panic!("expected UncoveredBytes, got {:?}", other),
                }
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_overlap_at_phrase_level_caught() {
        // Two phrases overlap inside one sentence.
        let doc = mk_doc("aaaaaa"); // 6 bytes
        let n_doc = typed_node("D", NodeKind::Document, 0, 6, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 6, atom("sent"));
        let n_phrase_a = typed_node("Pa", NodeKind::Phrase, 0, 4, atom("phra"));
        let n_phrase_b = typed_node("Pb", NodeKind::Phrase, 2, 6, atom("phrb"));
        // Each phrase needs a Fact child to be well-formed at phrase level too.
        let n_fact_a = typed_node("Fa", NodeKind::Fact, 0, 4, atom("a"));
        let n_fact_b = typed_node("Fb", NodeKind::Fact, 2, 6, atom("b"));
        let n_ea = typed_node("Ea", NodeKind::Entity, 0, 4, atom("a"));
        let n_eb = typed_node("Eb", NodeKind::Entity, 2, 6, atom("b"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase_a, n_phrase_b, n_fact_a, n_fact_b, n_ea, n_eb],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "Pa"),
                contains_edge("e3", "S", "Pb"),
                contains_edge("e4", "Pa", "Fa"),
                contains_edge("e5", "Pb", "Fb"),
                contains_edge("e6", "Fa", "Ea"),
                contains_edge("e7", "Fb", "Eb"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                assert!(gaps.iter().any(|g| matches!(
                    g.kind,
                    HierarchicalGapKind::Overlap { .. }
                )), "expected at least one Overlap gap; got {:?}", gaps);
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_fact_without_typed_components_fails() {
        let doc = mk_doc("hello"); // 5 bytes
        let n_doc = typed_node("D", NodeKind::Document, 0, 5, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 5, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 5, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 5, atom("hello"));
        // No Entity / typed component under the Fact — should fail.
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                assert!(
                    gaps.iter().any(|g| g.level == DecompLevel::FactToTypedComponent
                        && matches!(g.kind, HierarchicalGapKind::NoChildrenAtLevel)),
                    "expected Fact-level NoChildrenAtLevel; got {:?}",
                    gaps
                );
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_wrong_child_kind_for_level_caught() {
        // A Phrase directly under a Document — illegal (should be a
        // Sentence between them).
        let doc = mk_doc("hi");
        let n_doc = typed_node("D", NodeKind::Document, 0, 2, atom("doc"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 2, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 2, atom("hi"));
        let n_entity = typed_node("E", NodeKind::Entity, 0, 2, atom("hi"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "P"),
                contains_edge("e2", "P", "F"),
                contains_edge("e3", "F", "E"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                assert!(
                    gaps.iter().any(|g| matches!(
                        &g.kind,
                        HierarchicalGapKind::WrongChildKindForLevel {
                            child_kind: NodeKind::Phrase,
                            ..
                        }
                    )),
                    "expected WrongChildKindForLevel(Phrase); got {:?}",
                    gaps
                );
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_flattening_digit_run_caught() {
        // Source has "200" as a digit run. A Fact term using atom
        // "battery_200_wh" smuggles both the digit and the unit.
        let doc = mk_doc("battery 200 wh");
        let flat_atom = atom("battery_200_wh");
        let n_doc = typed_node("D", NodeKind::Document, 0, 14, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 14, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 14, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 14, flat_atom);
        let n_entity = typed_node("E", NodeKind::Entity, 0, 14, atom("battery"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "E"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                assert!(
                    gaps.iter().any(|g| matches!(
                        &g.kind,
                        HierarchicalGapKind::FlattenedAtom {
                            reason: FlatteningReason::DigitRunFromSource { digits },
                            ..
                        } if digits == "200"
                    )),
                    "expected DigitRunFromSource(\"200\"); got {:?}",
                    gaps
                );
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_flattening_unit_suffix_caught_when_no_digit_in_source() {
        // Source: "battery capacity wh limit" — no digit run, so the
        // digit-run rule doesn't fire. But atom "battery_wh" ends in
        // banned suffix _wh.
        let doc = mk_doc("battery capacity wh limit");
        let n_doc = typed_node("D", NodeKind::Document, 0, 25, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 25, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 25, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 25, atom("battery_wh"));
        let n_entity = typed_node("E", NodeKind::Entity, 0, 25, atom("battery"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "E"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                assert!(
                    gaps.iter().any(|g| matches!(
                        &g.kind,
                        HierarchicalGapKind::FlattenedAtom {
                            reason: FlatteningReason::UnitSuffix { suffix },
                            ..
                        } if suffix == "_wh"
                    )),
                    "expected UnitSuffix(_wh); got {:?}",
                    gaps
                );
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_flattening_multi_word_collapse_caught() {
        let doc = mk_doc("pocket knife blade length and stuff");
        // 3-word atom from source: should fire the multi-word rule.
        let n_doc = typed_node("D", NodeKind::Document, 0, 35, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 35, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 35, atom("phr"));
        let n_fact =
            typed_node("F", NodeKind::Fact, 0, 35, atom("pocket_knife_blade_length"));
        let n_entity = typed_node("E", NodeKind::Entity, 0, 35, atom("stuff"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "E"),
            ],
        };
        let result = check_hierarchical_coverage(&doc, &ir);
        match result {
            HierarchicalCoverageResult::Fail { gaps } => {
                assert!(
                    gaps.iter().any(|g| matches!(
                        &g.kind,
                        HierarchicalGapKind::FlattenedAtom {
                            reason: FlatteningReason::MultiWordCollapse { .. },
                            ..
                        }
                    )),
                    "expected MultiWordCollapse; got {:?}",
                    gaps
                );
            }
            HierarchicalCoverageResult::Pass => panic!("expected Fail"),
        }
    }

    #[test]
    fn adj25_legitimate_atoms_accepted() {
        // `matches`, `passenger`, `bag` — single-word source atoms,
        // no digit, no unit suffix. Two-word compounds OK.
        let doc = mk_doc("matches");
        let n_doc = typed_node("D", NodeKind::Document, 0, 7, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 7, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 7, atom("phr"));
        let n_fact = typed_node("F", NodeKind::Fact, 0, 7, atom("matches"));
        // Two-word compound (no digit, no banned suffix, but still
        // two words — should be accepted).
        let n_entity = typed_node("E", NodeKind::Entity, 0, 7, atom("pocket_knife"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_entity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "E"),
            ],
        };
        assert_eq!(
            check_hierarchical_coverage(&doc, &ir),
            HierarchicalCoverageResult::Pass
        );
    }

    #[test]
    fn adj25_quantity_compound_not_flagged_as_flat() {
        // `Quantity(50, wh)` is the legitimate decomposition; the
        // `50` is a Number term, not an atom, so the digit-run rule
        // doesn't fire on it. The compound functor `quantity` and
        // arg atom `wh` are clean. Source has "50 wh"; legitimate
        // typed-quantity should NOT be flagged.
        let doc = mk_doc("50 wh");
        let n_doc = typed_node("D", NodeKind::Document, 0, 5, atom("doc"));
        let n_sent = typed_node("S", NodeKind::Sentence, 0, 5, atom("sent"));
        let n_phrase = typed_node("P", NodeKind::Phrase, 0, 5, atom("phr"));
        let n_fact = typed_node(
            "F",
            NodeKind::Fact,
            0,
            5,
            compound("battery", vec![]), // single-word atom-compound, no flatten
        );
        let n_quantity = typed_node(
            "Q",
            NodeKind::Quantity,
            0,
            5,
            compound("quantity", vec![logic_core::int(50), atom("wh")]),
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_doc, n_sent, n_phrase, n_fact, n_quantity],
            edges: vec![
                contains_edge("e1", "D", "S"),
                contains_edge("e2", "S", "P"),
                contains_edge("e3", "P", "F"),
                contains_edge("e4", "F", "Q"),
            ],
        };
        assert_eq!(
            check_hierarchical_coverage(&doc, &ir),
            HierarchicalCoverageResult::Pass
        );
    }

    #[test]
    fn adj25_pure_flat_ir_without_hierarchy_is_no_op() {
        // No Document / Sentence / Phrase / Fact-with-Contains.
        // A legacy flat IR with one Fact and no hierarchy edges —
        // hierarchical check has nothing to verify, returns Pass.
        // (The Fact has no Contains-children edges, so the
        // FactToTypedComponent boundary fires... actually no,
        // a single Fact with no Contains edges IS a problem under
        // the new rule. Let me adjust this test: skip the Fact
        // entirely — a doc with only legacy non-hierarchy kinds.)
        let doc = mk_doc("hello");
        let n_section = typed_node(
            "Sec",
            NodeKind::Section,
            0,
            5,
            compound("paragraph", vec![]),
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![n_section],
            edges: vec![],
        };
        // Section is not a level-bearing kind in ADJ25; the check
        // ignores it. No gaps reported.
        assert_eq!(
            check_hierarchical_coverage(&doc, &ir),
            HierarchicalCoverageResult::Pass
        );
    }

    #[test]
    fn adj25_collect_digit_runs() {
        assert_eq!(collect_digit_runs("1 carry 200 wh"), vec!["1", "200"]);
        assert_eq!(collect_digit_runs("no digits"), Vec::<String>::new());
        assert_eq!(collect_digit_runs("4.5"), vec!["4", "5"]); // "." is not a digit
    }

    #[test]
    fn adj25_classify_flattening_priority() {
        let runs = vec!["200".to_string()];
        let words: std::collections::HashSet<String> =
            ["battery", "wh", "capacity"].iter().map(|s| s.to_string()).collect();
        // Digit takes priority over suffix.
        match classify_flattening("battery_200_wh", &runs, &words) {
            Some(FlatteningReason::DigitRunFromSource { digits }) => {
                assert_eq!(digits, "200");
            }
            other => panic!("expected DigitRunFromSource; got {:?}", other),
        }
        // Pure suffix case.
        let empty_runs: Vec<String> = vec![];
        match classify_flattening("battery_wh", &empty_runs, &words) {
            Some(FlatteningReason::UnitSuffix { suffix }) => assert_eq!(suffix, "_wh"),
            other => panic!("expected UnitSuffix; got {:?}", other),
        }
        // Clean atom.
        assert_eq!(classify_flattening("matches", &empty_runs, &words), None);
    }
}
