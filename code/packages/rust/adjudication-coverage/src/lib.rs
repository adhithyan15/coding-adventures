//! # adjudication-coverage — ADJ02 v2 structural tree-coverage check.
//!
//! Reference implementation of
//! [`ADJ02` v2](../../../specs/ADJ02-coverage-checker.md). The check
//! is a **structural tree-tiling check** over the hierarchical IR
//! from ADJ01 v2 — language-agnostic by construction, deterministic,
//! linear in IR node count, no tagger, no stopword list, no NegEx
//! triggers.
//!
//! ## The invariant
//!
//! > Every byte of the document's normalized text is in the source
//! > spans of some leaf in the IR's decomposition tree.
//!
//! Equivalent to five structural conditions:
//!
//! 1. **Span validity** — every span has `start < end`, both within
//!    the document's bounds.
//! 2. **Root coverage** — the union of root nodes' source_spans
//!    equals the document's full byte range.
//! 3. **Parent-child containment** — child spans inside parent's.
//! 4. **TextRun tiling** — each TextRun's children's spans union to
//!    the parent's spans.
//! 5. **No `Unparseable`** — any `Discarded` node with reason
//!    `Unparseable` is a hard coverage failure.
//!
//! Conditions 1, 3, 4 are already enforced by
//! `adjudication_ir::validate`. This crate adds conditions 2 and 5
//! and packages all five violations into a `CoverageResult` shape
//! suitable for ADJ06 clarification.

use adjudication_ir::{
    validate, DiscardReason, DocumentId, IRDocument, NodeId, NodeKind, ValidationError,
};

// ---------------------------------------------------------------------------
// Document and result types
// ---------------------------------------------------------------------------

/// The document under coverage analysis. The check reads only
/// `normalized_text.len()` — it never inspects the bytes.
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

/// One coverage violation. Maps to an ADJ06 clarification question
/// shape.
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageViolation {
    /// A span cites a different document than the one under check.
    SpanWrongDocument {
        node_id: NodeId,
        expected: DocumentId,
        found: DocumentId,
    },

    /// A span's `start >= end` or extends beyond the document's
    /// byte length.
    InvalidSpan {
        node_id: NodeId,
        start: usize,
        end: usize,
        document_len: usize,
    },

    /// A `Discarded` node has reason `Unparseable`. Always a hard
    /// coverage failure per ADJ01.
    UnparseableDiscarded { node_id: NodeId },

    /// The union of root nodes' source_spans does not equal the
    /// document's full byte range. `missing_ranges` enumerates the
    /// uncovered byte ranges.
    RootsDoNotTileDocument { missing_ranges: Vec<(usize, usize)> },

    /// A node's `part_of` references an id that doesn't exist.
    DanglingPartOf { node_id: NodeId, missing_parent: NodeId },

    /// A child's source spans extend beyond its structural parent's.
    ChildSpansExceedParent { child_id: NodeId, parent_id: NodeId },

    /// A `TextRun`'s children's spans, taken together, do not tile
    /// its own spans. `missing_ranges` enumerates the gaps inside
    /// the parent.
    ChildrenDoNotTileParent {
        parent_id: NodeId,
        missing_ranges: Vec<(usize, usize)>,
    },

    /// A non-TextRun node has children. Only TextRun may be a
    /// structural parent.
    NonTextRunHasChildren {
        parent_id: NodeId,
        parent_kind: NodeKind,
        children: Vec<NodeId>,
    },

    /// A `part_of` cycle in the decomposition.
    PartOfCycle { participants: Vec<NodeId> },
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the structural coverage check. Returns `Pass` or
/// `Fail { violations }`.
///
/// The check does not call an LLM; it does not consult a tagger;
/// it does not classify tokens. The LLM-produced decomposition tree
/// already encoded what counts as content; this check verifies the
/// tree's structural completeness.
pub fn check_coverage(doc: &Document, ir_doc: &IRDocument) -> CoverageResult {
    let mut violations = Vec::new();

    // 1, 3, 4 — delegated to adjudication_ir::validate, which
    // enforces ADJ01's well-formedness rules including all the
    // tree-shape invariants.
    if let Err(e) = validate(ir_doc) {
        violations.extend(translate_validation_error(e, doc));
    }

    // 2. Root coverage: roots union to (0, doc_len).
    let doc_len = doc.normalized_text.len();
    let mut root_spans: Vec<(usize, usize)> = ir_doc
        .nodes
        .iter()
        .filter(|n| n.part_of.is_none())
        .flat_map(|n| n.source_spans.iter())
        .filter(|s| s.document_id == doc.id)
        .map(|s| (s.start, s.end))
        .collect();
    root_spans.sort_by_key(|(s, _)| *s);
    let root_merged = merge_ranges(root_spans);
    let document_range = if doc_len > 0 {
        vec![(0, doc_len)]
    } else {
        vec![]
    };
    let missing = subtract_intervals(document_range.clone(), root_merged.clone());
    if !missing.is_empty() && !document_range.is_empty() {
        violations.push(CoverageViolation::RootsDoNotTileDocument {
            missing_ranges: missing,
        });
    }

    // 5. Hard rule: Unparseable Discarded is a coverage failure.
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

fn translate_validation_error(e: ValidationError, doc: &Document) -> Vec<CoverageViolation> {
    let mut out = Vec::new();
    match e {
        ValidationError::InvalidSpan {
            node_id, start, end,
        } => out.push(CoverageViolation::InvalidSpan {
            node_id,
            start,
            end,
            document_len: doc.normalized_text.len(),
        }),
        ValidationError::SpanDocumentMismatch {
            node_id, expected, found,
        } => out.push(CoverageViolation::SpanWrongDocument {
            node_id,
            expected,
            found,
        }),
        ValidationError::DanglingPartOf {
            node_id, missing_parent,
        } => out.push(CoverageViolation::DanglingPartOf {
            node_id,
            missing_parent,
        }),
        ValidationError::ChildSpansExceedParent {
            child_id, parent_id,
        } => out.push(CoverageViolation::ChildSpansExceedParent {
            child_id,
            parent_id,
        }),
        ValidationError::ChildrenDoNotTileParent {
            parent_id, missing_ranges,
        } => out.push(CoverageViolation::ChildrenDoNotTileParent {
            parent_id,
            missing_ranges,
        }),
        ValidationError::NonTextRunHasChildren {
            parent_id, parent_kind, children,
        } => out.push(CoverageViolation::NonTextRunHasChildren {
            parent_id,
            parent_kind,
            children,
        }),
        ValidationError::PartOfCycle { participants } => {
            out.push(CoverageViolation::PartOfCycle { participants })
        }
        // Other adjudication-ir validation errors (DuplicateNodeId,
        // FactWithUncertainPolarity, etc.) are not coverage concerns
        // per ADJ02; they belong to ADJ01 well-formedness.
        _ => {}
    }
    out
}

/// Merge overlapping / adjacent byte-range intervals.
fn merge_ranges(mut rs: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if rs.is_empty() {
        return rs;
    }
    rs.sort_by_key(|(s, _)| *s);
    let mut out = Vec::with_capacity(rs.len());
    let mut cur = rs[0];
    for (s, e) in rs.into_iter().skip(1) {
        if s <= cur.1 {
            cur.1 = cur.1.max(e);
        } else {
            out.push(cur);
            cur = (s, e);
        }
    }
    out.push(cur);
    out
}

/// Bytes in `parent` not covered by any range in `children`.
fn subtract_intervals(
    parent: Vec<(usize, usize)>,
    children: Vec<(usize, usize)>,
) -> Vec<(usize, usize)> {
    let parent_merged = merge_ranges(parent);
    let child_merged = merge_ranges(children);
    let mut out = Vec::new();
    for (p_start, p_end) in parent_merged {
        let mut cursor = p_start;
        for &(c_start, c_end) in &child_merged {
            if c_end <= cursor || c_start >= p_end {
                continue;
            }
            if c_start > cursor {
                out.push((cursor, c_start.min(p_end)));
            }
            cursor = cursor.max(c_end);
            if cursor >= p_end {
                break;
            }
        }
        if cursor < p_end {
            out.push((cursor, p_end));
        }
    }
    out
}

// Re-export Span and NodeId for caller convenience.
pub use adjudication_ir::{NodeId as IrNodeId, Span as IrSpan};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{IRNode, Modality, Polarity, Span};
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

    fn text_run(id: &str, start: usize, end: usize, part_of: Option<&str>) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::TextRun,
            term: compound("text_run", vec![]),
            polarity: Polarity::Inherit,
            modality: Modality::Inherit,
            source_spans: vec![span_of(start, end)],
            confidence: 1.0,
            part_of: part_of.map(NodeId::new),
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn fact_leaf(id: &str, start: usize, end: usize, part_of: Option<&str>) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: atom("placeholder"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(start, end)],
            confidence: 0.9,
            part_of: part_of.map(NodeId::new),
            lowered_from: None,
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
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    #[test]
    fn nonempty_document_with_empty_ir_fails_with_full_range() {
        let doc = mk_doc("hello world");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has_root_miss = violations.iter().any(|v| matches!(
                    v,
                    CoverageViolation::RootsDoNotTileDocument { missing_ranges }
                        if missing_ranges == &vec![(0, 11)]
                ));
                assert!(has_root_miss, "expected RootsDoNotTileDocument: {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn single_root_textrun_with_one_child_tiling_full_doc_passes() {
        let doc = mk_doc("hello world");
        let parent = text_run("T0", 0, 11, None);
        let leaf = fact_leaf("F1", 0, 11, Some("T0"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, leaf],
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    #[test]
    fn root_tile_gap_reported_with_missing_range() {
        // doc 0..50 but only one root TextRun at 0..30
        let doc = mk_doc(&"x".repeat(50));
        let parent = text_run("T0", 0, 30, None);
        let leaf = fact_leaf("F1", 0, 30, Some("T0"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, leaf],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has_missing = violations.iter().any(|v| matches!(
                    v,
                    CoverageViolation::RootsDoNotTileDocument { missing_ranges }
                        if missing_ranges == &vec![(30, 50)]
                ));
                assert!(has_missing, "expected (30, 50) missing: {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn child_gap_within_textrun_reported_with_missing_range() {
        // Root TextRun 0..50, two children 0..20 and 30..50 — gap 20..30.
        let doc = mk_doc(&"x".repeat(50));
        let parent = text_run("T0", 0, 50, None);
        let leaf1 = fact_leaf("F1", 0, 20, Some("T0"));
        let leaf2 = fact_leaf("F2", 30, 50, Some("T0"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, leaf1, leaf2],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                let has_gap = violations.iter().any(|v| matches!(
                    v,
                    CoverageViolation::ChildrenDoNotTileParent { missing_ranges, .. }
                        if missing_ranges == &vec![(20, 30)]
                ));
                assert!(has_gap, "expected ChildrenDoNotTileParent(20..30): {:?}", violations);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn unparseable_discarded_always_fails() {
        let doc = mk_doc(&"x".repeat(20));
        let parent = text_run("T0", 0, 20, None);
        let discard = IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: atom("discarded"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(0, 20)],
            confidence: 1.0,
            part_of: Some(NodeId::new("T0")),
            lowered_from: None,
            discard_reason: Some(DiscardReason::Unparseable),
            metadata: HashMap::new(),
        };
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, discard],
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
    fn discarded_with_pleasantry_reason_is_ok() {
        let doc = mk_doc(&"x".repeat(20));
        let parent = text_run("T0", 0, 20, None);
        let discard = IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: atom("discarded"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(0, 20)],
            confidence: 1.0,
            part_of: Some(NodeId::new("T0")),
            lowered_from: None,
            discard_reason: Some(DiscardReason::Pleasantry),
            metadata: HashMap::new(),
        };
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, discard],
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    #[test]
    fn nested_textruns_tile_correctly_passes() {
        let doc = mk_doc(&"x".repeat(50));
        let outer = text_run("T0", 0, 50, None);
        let inner = text_run("T1", 0, 50, Some("T0"));
        let leaf = fact_leaf("F1", 0, 50, Some("T1"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![outer, inner, leaf],
        };
        assert_eq!(check_coverage(&doc, &ir), CoverageResult::Pass);
    }

    #[test]
    fn merge_ranges_combines_adjacent_intervals() {
        assert_eq!(
            merge_ranges(vec![(0, 5), (5, 10), (15, 20)]),
            vec![(0, 10), (15, 20)]
        );
    }

    #[test]
    fn subtract_intervals_finds_gaps() {
        assert_eq!(
            subtract_intervals(vec![(0, 50)], vec![(10, 20), (30, 40)]),
            vec![(0, 10), (20, 30), (40, 50)]
        );
    }

    #[test]
    fn dangling_part_of_surfaces_as_coverage_violation() {
        let doc = mk_doc(&"x".repeat(20));
        let leaf = fact_leaf("F1", 0, 20, Some("DOES_NOT_EXIST"));
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![leaf],
        };
        match check_coverage(&doc, &ir) {
            CoverageResult::Fail { violations } => {
                assert!(violations
                    .iter()
                    .any(|v| matches!(v, CoverageViolation::DanglingPartOf { .. })));
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }
}
