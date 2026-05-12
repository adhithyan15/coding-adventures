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
}
