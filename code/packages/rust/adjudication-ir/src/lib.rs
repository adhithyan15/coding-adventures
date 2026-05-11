//! # adjudication-ir — the typed IR for rule-based adjudication.
//!
//! Reference implementation of [`ADJ01`](../../../specs/ADJ01-adjudication-ir-grammar.md).
//! Defines every IR node shape, the polarity / modality lattices, the
//! lowering DAG, and a total `validate` function that enforces every
//! well-formedness rule before any downstream component touches the
//! document.
//!
//! ## Why a separate IR crate
//!
//! The IR sits above the term layer (`logic-core`) and below the
//! checker passes (ADJ02..ADJ05), the dialogue (ADJ06), the audit
//! trail (ADJ07), and the rule-compilation pipeline (ADJ09). All of
//! those consume IR documents; none should reconstruct the grammar
//! locally.
//!
//! ## Layer Position
//!
//! ```text
//!    logic-core (LP00)            ← Term, LogicVar, Substitution, unify
//!         │
//!         ▼
//!    adjudication-ir              ← this crate (ADJ01)
//!         │
//!         ├── ADJ02 coverage checker
//!         ├── ADJ03 polarity/modality checker
//!         ├── ADJ04 round-trip checker
//!         ├── ADJ05 adversarial verifier
//!         ├── ADJ06 clarification dialogue
//!         └── ADJ09 rule compilation
//! ```

use std::collections::{HashMap, HashSet};

use logic_core::Term;

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Stable identifier for a document. Opaque to this crate; deployments
/// typically use UUIDv4 strings, but any unique identifier is acceptable
/// provided it is stable across clarification turns (per ADJ01).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Stable identifier for an IRNode within a document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

// ---------------------------------------------------------------------------
// Spans
// ---------------------------------------------------------------------------

/// A byte-offset range into a document's normalized text.
///
/// `start` and `end` are **byte** offsets, not character indices, to
/// avoid Unicode normalization disagreements between implementations.
/// Half-open: the range covers `[start, end)`. `end > start` is required
/// for non-degenerate spans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub document_id: DocumentId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(document_id: DocumentId, start: usize, end: usize) -> Self {
        Self {
            document_id,
            start,
            end,
        }
    }

    /// `true` iff `self.start < self.end` and they refer to the same document.
    pub fn is_valid(&self) -> bool {
        self.start < self.end
    }

    /// `true` iff `other`'s range is fully contained within `self`'s
    /// range AND both spans cite the same document.
    pub fn contains(&self, other: &Span) -> bool {
        self.document_id == other.document_id
            && self.start <= other.start
            && other.end <= self.end
    }
}

// ---------------------------------------------------------------------------
// Lattices: Polarity, Modality
// ---------------------------------------------------------------------------

/// Whether the node asserts, denies, or records uncertainty about its
/// term. Per `ADJ01` the lattice is flat — no element subsumes another,
/// and there is no `Unknown` value. The absence of evidence is
/// represented by the absence of a node (coverage enforces this).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    Affirmed,
    Denied,
    Uncertain,
}

/// The temporal / hypothetical / ownership context of the term. Flat
/// lattice. Combining modalities requires multiple nodes, not a join.
///
/// `RuledOut` and `Denied` (a polarity value) are *not* synonyms. See
/// `ADJ01 §"Modality"` and `ADJ03 §"RuledOut vs. Denied"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modality {
    Present,
    Past,
    Future,
    Hypothetical,
    FamilyHistory,
    RuledOut,
    Conditional,
}

// ---------------------------------------------------------------------------
// Node kinds and discard reasons
// ---------------------------------------------------------------------------

/// The role a node plays in the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Fact,
    Query,
    Uncertainty,
    Rule,
    Exception,
    Discarded,
}

/// Controlled vocabulary for `Discarded` nodes' `discard_reason` field.
/// Coverage analysis (`ADJ02`) audits these to ensure dropped spans are
/// dropped *for a reason*, not silently.
///
/// `Unparseable` is always a coverage failure (see `ADJ02`): an extractor
/// that produces a `Discarded(Unparseable)` triggers clarification rather
/// than shipping the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscardReason {
    Pleasantry,
    DocumentMetadata,
    NonDomainContent,
    Restatement,
    Unparseable,
    AdministrativeOnly,
    ExplicitlyOutOfScope,
}

// ---------------------------------------------------------------------------
// IRNode and IRDocument
// ---------------------------------------------------------------------------

/// A single node in the IR. Every field corresponds 1:1 to a field in
/// `ADJ01 §"IR Nodes"`.
///
/// Three properties are non-negotiable (enforced by [`validate`]):
///
/// 1. Every IR node is span-grounded (`source_spans` is non-empty for
///    every kind except `Rule` which cites rulebook spans).
/// 2. `polarity` and `modality` are always set — no defaults.
/// 3. `Discarded` is an explicit node citing both the span being
///    discarded and the reason (`DiscardReason`); silently omitting a
///    span is not a valid representation of "irrelevant".
#[derive(Debug, Clone, PartialEq)]
pub struct IRNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub term: Term,
    pub polarity: Polarity,
    pub modality: Modality,
    pub source_spans: Vec<Span>,
    /// Extractor's self-reported confidence. Informational only; not
    /// used by the type check.
    pub confidence: f64,
    /// If present, points to the parent node in the lowering DAG.
    pub lowered_from: Option<NodeId>,
    /// Required iff `kind == Discarded`.
    pub discard_reason: Option<DiscardReason>,
    /// Free-form extension for downstream consumers. The framework
    /// reserves any key beginning with `adj.` for future use.
    pub metadata: HashMap<String, String>,
}

/// An IR document is a container of nodes belonging to one input
/// document. The `document_id` matches the input's identifier; nodes'
/// `source_spans` reference offsets into that document's normalized
/// text.
#[derive(Debug, Clone, PartialEq)]
pub struct IRDocument {
    pub document_id: DocumentId,
    pub nodes: Vec<IRNode>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Every reason an IR document can fail well-formedness.
///
/// Each variant corresponds to a numbered rule from `ADJ01 §"Well-
/// Formedness Summary"`. Returning a specific variant rather than a
/// generic error lets callers surface precise feedback to the extractor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A node's `source_spans` was empty when the kind requires at
    /// least one span.
    MissingSourceSpans { node_id: NodeId, kind: NodeKind },

    /// A span had `start >= end` (degenerate range).
    InvalidSpan {
        node_id: NodeId,
        start: usize,
        end: usize,
    },

    /// A node's source span cites a document other than the IRDocument's
    /// `document_id`. (Cross-document references are out of scope here.)
    SpanDocumentMismatch {
        node_id: NodeId,
        expected: DocumentId,
        found: DocumentId,
    },

    /// `kind = Fact`, but `polarity` is `Uncertain`. Facts cannot be
    /// uncertain by construction.
    FactWithUncertainPolarity { node_id: NodeId },

    /// `kind = Uncertainty`, but `polarity` is not `Uncertain`.
    UncertaintyWithDefinitePolarity { node_id: NodeId, polarity: Polarity },

    /// `kind = Query`, but `polarity` is not `Affirmed`. Querying `¬p`
    /// is itself a question; we represent the question as an Affirmed
    /// query and require the polarity field to be Affirmed.
    QueryWithNonAffirmedPolarity { node_id: NodeId, polarity: Polarity },

    /// `kind = Discarded`, but `discard_reason` is absent. Every
    /// Discarded node must declare why.
    DiscardedWithoutReason { node_id: NodeId },

    /// `kind != Discarded`, but `discard_reason` is set. Only Discarded
    /// nodes carry a discard reason.
    NonDiscardedWithReason { node_id: NodeId, kind: NodeKind },

    /// A node's `lowered_from` points to an id that doesn't exist in
    /// the document.
    DanglingLoweredFrom {
        node_id: NodeId,
        missing_parent: NodeId,
    },

    /// The lowering DAG has a cycle. Detected by topological-sort
    /// failure; the cycle's participating ids are reported.
    LoweringCycle { participants: Vec<NodeId> },

    /// A lowered node's spans are not a subset of its parent's spans —
    /// lowering should only narrow provenance, not invent it.
    LoweringExpandsSpans {
        child_id: NodeId,
        parent_id: NodeId,
    },

    /// A lowered node's kind is not compatible with its parent's kind
    /// under the `kind ≺ kind` relation from ADJ01.
    IncompatibleLowering {
        child_id: NodeId,
        parent_id: NodeId,
        child_kind: NodeKind,
        parent_kind: NodeKind,
    },

    /// Two nodes share the same `NodeId`.
    DuplicateNodeId { id: NodeId },
}

/// Validate an IR document. Returns `Ok(())` iff every rule in
/// `ADJ01 §"Well-Formedness Summary"` is satisfied; otherwise returns
/// the first violation found.
///
/// Validation is **total** — no partial well-formedness. A caller that
/// receives `Err(_)` must not pass the document to any downstream
/// component.
pub fn validate(doc: &IRDocument) -> Result<(), ValidationError> {
    // 0. Duplicate node ids fail immediately. Detection of subsequent
    //    rules can assume ids are unique.
    let mut seen_ids: HashSet<&NodeId> = HashSet::new();
    for n in &doc.nodes {
        if !seen_ids.insert(&n.id) {
            return Err(ValidationError::DuplicateNodeId { id: n.id.clone() });
        }
    }

    // 1. Per-node structural rules.
    for n in &doc.nodes {
        validate_per_node(n, doc)?;
    }

    // 2. Lowering DAG rules: no cycles, parent exists, kind compatible,
    //    span subset.
    validate_lowering_dag(doc)?;

    Ok(())
}

fn validate_per_node(n: &IRNode, doc: &IRDocument) -> Result<(), ValidationError> {
    // Span basics.
    if n.kind != NodeKind::Rule {
        // Rules cite rulebook spans; other kinds cite the input document.
        if n.source_spans.is_empty() {
            return Err(ValidationError::MissingSourceSpans {
                node_id: n.id.clone(),
                kind: n.kind,
            });
        }
        for span in &n.source_spans {
            if !span.is_valid() {
                return Err(ValidationError::InvalidSpan {
                    node_id: n.id.clone(),
                    start: span.start,
                    end: span.end,
                });
            }
            if span.document_id != doc.document_id {
                return Err(ValidationError::SpanDocumentMismatch {
                    node_id: n.id.clone(),
                    expected: doc.document_id.clone(),
                    found: span.document_id.clone(),
                });
            }
        }
    }

    // Kind-specific rules.
    match n.kind {
        NodeKind::Fact => {
            if n.polarity == Polarity::Uncertain {
                return Err(ValidationError::FactWithUncertainPolarity {
                    node_id: n.id.clone(),
                });
            }
            if n.discard_reason.is_some() {
                return Err(ValidationError::NonDiscardedWithReason {
                    node_id: n.id.clone(),
                    kind: n.kind,
                });
            }
        }
        NodeKind::Query => {
            if n.polarity != Polarity::Affirmed {
                return Err(ValidationError::QueryWithNonAffirmedPolarity {
                    node_id: n.id.clone(),
                    polarity: n.polarity,
                });
            }
            if n.discard_reason.is_some() {
                return Err(ValidationError::NonDiscardedWithReason {
                    node_id: n.id.clone(),
                    kind: n.kind,
                });
            }
        }
        NodeKind::Uncertainty => {
            if n.polarity != Polarity::Uncertain {
                return Err(ValidationError::UncertaintyWithDefinitePolarity {
                    node_id: n.id.clone(),
                    polarity: n.polarity,
                });
            }
            if n.discard_reason.is_some() {
                return Err(ValidationError::NonDiscardedWithReason {
                    node_id: n.id.clone(),
                    kind: n.kind,
                });
            }
        }
        NodeKind::Rule => {
            if n.polarity == Polarity::Uncertain {
                return Err(ValidationError::FactWithUncertainPolarity {
                    node_id: n.id.clone(),
                });
            }
            if n.discard_reason.is_some() {
                return Err(ValidationError::NonDiscardedWithReason {
                    node_id: n.id.clone(),
                    kind: n.kind,
                });
            }
        }
        NodeKind::Exception => {
            if n.polarity != Polarity::Affirmed {
                return Err(ValidationError::QueryWithNonAffirmedPolarity {
                    node_id: n.id.clone(),
                    polarity: n.polarity,
                });
            }
            if n.discard_reason.is_some() {
                return Err(ValidationError::NonDiscardedWithReason {
                    node_id: n.id.clone(),
                    kind: n.kind,
                });
            }
        }
        NodeKind::Discarded => {
            if n.discard_reason.is_none() {
                return Err(ValidationError::DiscardedWithoutReason {
                    node_id: n.id.clone(),
                });
            }
        }
    }

    Ok(())
}

fn validate_lowering_dag(doc: &IRDocument) -> Result<(), ValidationError> {
    let by_id: HashMap<&NodeId, &IRNode> = doc.nodes.iter().map(|n| (&n.id, n)).collect();

    // Parent existence + per-edge constraints.
    for child in &doc.nodes {
        let Some(parent_id) = &child.lowered_from else {
            continue;
        };
        let Some(parent) = by_id.get(parent_id) else {
            return Err(ValidationError::DanglingLoweredFrom {
                node_id: child.id.clone(),
                missing_parent: parent_id.clone(),
            });
        };

        // Kind compatibility per ADJ01 §"Lowering Rules":
        //   Fact            ≺ Fact
        //   Uncertainty     ≺ Fact          (clarification resolution)
        //   Uncertainty     ≺ Uncertainty
        //   Query           ≺ Query
        //   Rule            ≺ Rule
        //   Discarded cannot be lowered or lowered-to.
        let compat = matches!(
            (parent.kind, child.kind),
            (NodeKind::Fact, NodeKind::Fact)
                | (NodeKind::Uncertainty, NodeKind::Fact)
                | (NodeKind::Uncertainty, NodeKind::Uncertainty)
                | (NodeKind::Query, NodeKind::Query)
                | (NodeKind::Rule, NodeKind::Rule)
        );
        if !compat {
            return Err(ValidationError::IncompatibleLowering {
                child_id: child.id.clone(),
                parent_id: parent.id.clone(),
                child_kind: child.kind,
                parent_kind: parent.kind,
            });
        }

        // Span subset: every child span must be contained in some
        // parent span.
        for child_span in &child.source_spans {
            let contained = parent
                .source_spans
                .iter()
                .any(|ps| ps.contains(child_span));
            if !contained {
                return Err(ValidationError::LoweringExpandsSpans {
                    child_id: child.id.clone(),
                    parent_id: parent.id.clone(),
                });
            }
        }
    }

    // Cycle detection (Kahn's algorithm).
    let mut indegree: HashMap<&NodeId, usize> = doc.nodes.iter().map(|n| (&n.id, 0)).collect();
    for n in &doc.nodes {
        if let Some(p) = &n.lowered_from {
            *indegree.entry(p).or_insert(0) += 1;
        }
    }
    let mut queue: Vec<&NodeId> = indegree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(k, _)| *k)
        .collect();
    let mut visited = 0usize;
    while let Some(id) = queue.pop() {
        visited += 1;
        if let Some(node) = by_id.get(id) {
            if let Some(p) = &node.lowered_from {
                let d = indegree.entry(p).or_insert(0);
                *d = d.saturating_sub(1);
                if *d == 0 {
                    queue.push(p);
                }
            }
        }
    }
    if visited != doc.nodes.len() {
        // At least one cycle exists; collect the remaining indegree>0 ids.
        let participants: Vec<NodeId> = indegree
            .into_iter()
            .filter(|(_, d)| *d > 0)
            .map(|(k, _)| k.clone())
            .collect();
        return Err(ValidationError::LoweringCycle { participants });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Inline unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use logic_core::{atom, compound};

    fn span(doc: &DocumentId, start: usize, end: usize) -> Span {
        Span::new(doc.clone(), start, end)
    }

    fn ok_fact(id: &str, doc: &DocumentId, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: compound("p", vec![atom("a")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(doc, start, end)],
            confidence: 0.9,
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn empty_document_is_well_formed() {
        let doc = IRDocument {
            document_id: DocumentId::new("doc1"),
            nodes: vec![],
        };
        assert!(validate(&doc).is_ok());
    }

    #[test]
    fn single_well_formed_fact_passes() {
        let doc_id = DocumentId::new("doc1");
        let n = ok_fact("F1", &doc_id, 0, 10);
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn fact_with_uncertain_polarity_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("F1", &doc_id, 0, 10);
        n.polarity = Polarity::Uncertain;
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::FactWithUncertainPolarity { .. })
        ));
    }

    #[test]
    fn uncertainty_with_definite_polarity_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("U1", &doc_id, 0, 10);
        n.kind = NodeKind::Uncertainty;
        n.polarity = Polarity::Affirmed;
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::UncertaintyWithDefinitePolarity { .. })
        ));
    }

    #[test]
    fn query_with_denied_polarity_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("Q1", &doc_id, 0, 10);
        n.kind = NodeKind::Query;
        n.polarity = Polarity::Denied;
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::QueryWithNonAffirmedPolarity { .. })
        ));
    }

    #[test]
    fn discarded_without_reason_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("D1", &doc_id, 0, 10);
        n.kind = NodeKind::Discarded;
        // discard_reason left None
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DiscardedWithoutReason { .. })
        ));
    }

    #[test]
    fn discarded_with_reason_passes() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("D1", &doc_id, 0, 10);
        n.kind = NodeKind::Discarded;
        n.discard_reason = Some(DiscardReason::Pleasantry);
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn non_discarded_with_reason_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("F1", &doc_id, 0, 10);
        n.discard_reason = Some(DiscardReason::Pleasantry);
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::NonDiscardedWithReason { .. })
        ));
    }

    #[test]
    fn empty_source_spans_rejected_for_fact() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("F1", &doc_id, 0, 10);
        n.source_spans.clear();
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::MissingSourceSpans { .. })
        ));
    }

    #[test]
    fn invalid_span_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("F1", &doc_id, 10, 5); // start > end
        // ok_fact constructs with valid span; override here:
        n.source_spans = vec![Span::new(doc_id.clone(), 10, 5)];
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::InvalidSpan { .. })
        ));
    }

    #[test]
    fn span_referencing_different_document_rejected() {
        let doc_id = DocumentId::new("doc1");
        let other = DocumentId::new("doc2");
        let mut n = ok_fact("F1", &doc_id, 0, 10);
        n.source_spans = vec![Span::new(other, 0, 10)];
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::SpanDocumentMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_node_id_rejected() {
        let doc_id = DocumentId::new("doc1");
        let n1 = ok_fact("F1", &doc_id, 0, 10);
        let n2 = ok_fact("F1", &doc_id, 11, 20);
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n1, n2],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DuplicateNodeId { .. })
        ));
    }

    #[test]
    fn dangling_lowered_from_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("F1", &doc_id, 0, 10);
        n.lowered_from = Some(NodeId::new("DOES_NOT_EXIST"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DanglingLoweredFrom { .. })
        ));
    }

    #[test]
    fn lowering_fact_to_fact_with_span_subset_passes() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_fact("F1", &doc_id, 0, 100);
        let mut child = ok_fact("F1a", &doc_id, 10, 50);
        child.lowered_from = Some(NodeId::new("F1"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, child],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn lowering_to_wider_span_rejected() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_fact("F1", &doc_id, 10, 50);
        let mut child = ok_fact("F1a", &doc_id, 0, 100); // wider than parent
        child.lowered_from = Some(NodeId::new("F1"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, child],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::LoweringExpandsSpans { .. })
        ));
    }

    #[test]
    fn lowering_fact_to_uncertainty_rejected() {
        // Per ADJ01: Fact cannot lower to Uncertainty (would be a
        // coverage failure at the root).
        let doc_id = DocumentId::new("doc1");
        let parent = ok_fact("F1", &doc_id, 0, 100);
        let mut child = ok_fact("U1", &doc_id, 10, 50);
        child.kind = NodeKind::Uncertainty;
        child.polarity = Polarity::Uncertain;
        child.lowered_from = Some(NodeId::new("F1"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, child],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::IncompatibleLowering { .. })
        ));
    }

    #[test]
    fn lowering_uncertainty_to_fact_passes() {
        // Clarification resolves uncertainty to a fact.
        let doc_id = DocumentId::new("doc1");
        let mut parent = ok_fact("U1", &doc_id, 0, 100);
        parent.kind = NodeKind::Uncertainty;
        parent.polarity = Polarity::Uncertain;
        let mut child = ok_fact("F1a", &doc_id, 10, 50);
        child.lowered_from = Some(NodeId::new("U1"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, child],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn lowering_cycle_rejected() {
        // F1 -> F2 -> F1 forms a cycle.
        let doc_id = DocumentId::new("doc1");
        let mut n1 = ok_fact("F1", &doc_id, 0, 100);
        let mut n2 = ok_fact("F2", &doc_id, 0, 100);
        n1.lowered_from = Some(NodeId::new("F2"));
        n2.lowered_from = Some(NodeId::new("F1"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n1, n2],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::LoweringCycle { .. })
        ));
    }

    #[test]
    fn rule_does_not_require_source_spans() {
        // Rules cite rulebook spans which may be from a different
        // document or none at all in the structural check (the rule
        // pipeline enforces its own rules).
        let doc_id = DocumentId::new("doc1");
        let mut n = ok_fact("R1", &doc_id, 0, 0);
        n.kind = NodeKind::Rule;
        n.source_spans.clear();
        // Other Rule constraints still apply: polarity != Uncertain,
        // no discard_reason. Both already true via ok_fact defaults.
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![n],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn span_contains_self_is_true() {
        let doc = DocumentId::new("d");
        let s = Span::new(doc.clone(), 5, 10);
        assert!(s.contains(&s));
    }

    #[test]
    fn span_contains_handles_equality_at_boundaries() {
        let doc = DocumentId::new("d");
        let outer = Span::new(doc.clone(), 5, 10);
        let inner = Span::new(doc.clone(), 5, 8); // shares start
        let inner2 = Span::new(doc.clone(), 7, 10); // shares end
        assert!(outer.contains(&inner));
        assert!(outer.contains(&inner2));
    }

    #[test]
    fn span_contains_rejects_different_documents() {
        let outer = Span::new(DocumentId::new("d1"), 0, 100);
        let inner = Span::new(DocumentId::new("d2"), 10, 20);
        assert!(!outer.contains(&inner));
    }
}
