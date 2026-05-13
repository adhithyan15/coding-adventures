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
    validate, DiscardReason, DocumentId, IRDocument, NodeOrEdgeId, NodeKind, SpanLocation,
    ValidationError,
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
}
