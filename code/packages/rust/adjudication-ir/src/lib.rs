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
/// term. The lattice is flat — no element subsumes another, and there
/// is no `Unknown` value. The absence of evidence is represented by
/// the absence of a node (coverage enforces this).
///
/// **v2**: `Inherit` lets a node defer to its structural ancestor's
/// polarity (per `ADJ01` v2 propagation). A `Polarity::Inherit` value
/// on a node means "use the nearest non-`Inherit` ancestor's value";
/// only the root (or a leaf with no overriding ancestor) needs a
/// concrete value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    Affirmed,
    Denied,
    Uncertain,
    /// Take the parent's effective polarity (v2 propagation).
    Inherit,
}

/// The temporal / hypothetical / ownership context of the term. Flat
/// lattice. Combining modalities requires multiple nodes, not a join.
///
/// `RuledOut` and `Denied` (a polarity value) are *not* synonyms. See
/// `ADJ01 §"Modality"` and `ADJ03 §"RuledOut vs. Denied"`.
///
/// **v2**: `Inherit` mirrors `Polarity::Inherit` — defer to the
/// structural ancestor's modality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modality {
    Present,
    Past,
    Future,
    Hypothetical,
    FamilyHistory,
    RuledOut,
    Conditional,
    /// Take the parent's effective modality (v2 propagation).
    Inherit,
}

// ---------------------------------------------------------------------------
// Node kinds and discard reasons
// ---------------------------------------------------------------------------

/// The role a node plays in the IR.
///
/// **v2**: adds `TextRun` — a non-leaf node that exists only to
/// carry the structural decomposition of the document (per `ADJ01`
/// v2). `TextRun` nodes group children but do not themselves
/// represent a domain claim; their `term` is conventionally the
/// zero-arity compound `text_run/0` and is not consumed by
/// validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// Non-leaf decomposition node (v2).
    TextRun,
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

/// A single node in the IR. Every field corresponds 1:1 to a field
/// in `ADJ01 §"IR Nodes"` (v2 schema).
///
/// Three properties are non-negotiable (enforced by [`validate`]):
///
/// 1. Every IR node is span-grounded (`source_spans` is non-empty for
///    every kind except `Rule` which cites rulebook spans).
/// 2. `polarity` and `modality` are always set (may be `Inherit`).
/// 3. `Discarded` is an explicit node citing both the span being
///    discarded and the reason (`DiscardReason`); silently omitting a
///    span is not a valid representation of "irrelevant".
///
/// **v2 additions**:
///
/// - `part_of: Option<NodeId>` — the structural-tree parent. A node
///   with `part_of = None` is at a document root.
/// - `polarity` and `modality` may carry `Inherit` to defer to the
///   ancestor's effective value (propagation).
/// - `TextRun` node kind is the non-leaf decomposition node.
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
    /// Structural parent in the decomposition tree (v2). A node with
    /// `part_of = None` is at a document root.
    pub part_of: Option<NodeId>,
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

    // ----- v2 structural-decomposition violations -----

    /// A node's `part_of` points to an id that doesn't exist.
    DanglingPartOf {
        node_id: NodeId,
        missing_parent: NodeId,
    },

    /// `part_of` edges form a cycle.
    PartOfCycle { participants: Vec<NodeId> },

    /// A non-TextRun node has children that point to it via `part_of`.
    /// Only TextRun nodes can be structural parents in v2.
    NonTextRunHasChildren {
        parent_id: NodeId,
        parent_kind: NodeKind,
        children: Vec<NodeId>,
    },

    /// A child node's source spans are not contained within its
    /// structural parent's source spans.
    ChildSpansExceedParent {
        child_id: NodeId,
        parent_id: NodeId,
    },

    /// A TextRun's children's source spans, taken together, do not
    /// tile its own source spans. `missing_ranges` lists the
    /// uncovered byte ranges.
    ChildrenDoNotTileParent {
        parent_id: NodeId,
        missing_ranges: Vec<(usize, usize)>,
    },
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

    // 3. Structural decomposition tree (v2): part_of edges form a
    //    forest; only TextRun nodes have children; children's spans
    //    fit inside parent's; TextRun children's spans tile the
    //    parent's spans.
    validate_structural_tree(doc)?;

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

    // Kind-specific rules. v2 allows Polarity::Inherit on any kind
    // that takes a declared polarity (the actual value is resolved
    // via ancestor lookup in propagation_check). The pre-Inherit
    // value-specific rules only fire on a *declared* (non-Inherit)
    // value.
    match n.kind {
        NodeKind::TextRun => {
            // TextRun nodes have no domain claim; their term is a
            // placeholder. They may carry polarity/modality (for
            // propagation) but no other constraints.
            if n.discard_reason.is_some() {
                return Err(ValidationError::NonDiscardedWithReason {
                    node_id: n.id.clone(),
                    kind: n.kind,
                });
            }
        }
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
            if n.polarity != Polarity::Affirmed && n.polarity != Polarity::Inherit {
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
            if n.polarity != Polarity::Uncertain && n.polarity != Polarity::Inherit {
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
            if n.polarity != Polarity::Affirmed && n.polarity != Polarity::Inherit {
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
// Structural decomposition tree (v2)
// ---------------------------------------------------------------------------

fn validate_structural_tree(doc: &IRDocument) -> Result<(), ValidationError> {
    let by_id: HashMap<&NodeId, &IRNode> = doc.nodes.iter().map(|n| (&n.id, n)).collect();

    // 1. Parent existence + parent-must-be-TextRun + parent-child
    //    span containment.
    let mut children_of: HashMap<NodeId, Vec<&IRNode>> = HashMap::new();
    for child in &doc.nodes {
        let Some(parent_id) = &child.part_of else {
            continue;
        };
        let Some(parent) = by_id.get(parent_id) else {
            return Err(ValidationError::DanglingPartOf {
                node_id: child.id.clone(),
                missing_parent: parent_id.clone(),
            });
        };
        // Spans subset (the per-edge check from ADJ02 v2 / ADJ01 v2).
        for child_span in &child.source_spans {
            let contained = parent.source_spans.iter().any(|ps| ps.contains(child_span));
            if !contained {
                return Err(ValidationError::ChildSpansExceedParent {
                    child_id: child.id.clone(),
                    parent_id: parent.id.clone(),
                });
            }
        }
        children_of
            .entry(parent_id.clone())
            .or_default()
            .push(child);
    }

    // 2. Only TextRun nodes may have children.
    for parent in &doc.nodes {
        if parent.kind != NodeKind::TextRun {
            if let Some(kids) = children_of.get(&parent.id) {
                if !kids.is_empty() {
                    return Err(ValidationError::NonTextRunHasChildren {
                        parent_id: parent.id.clone(),
                        parent_kind: parent.kind,
                        children: kids.iter().map(|c| c.id.clone()).collect(),
                    });
                }
            }
        }
    }

    // 3. Cycle detection in part_of edges (Kahn-style: every node has
    //    at most one parent, so we can detect a cycle by counting
    //    nodes reachable from the roots).
    let roots: Vec<&IRNode> = doc.nodes.iter().filter(|n| n.part_of.is_none()).collect();
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack: Vec<&IRNode> = roots.clone();
    while let Some(node) = stack.pop() {
        if !visited.insert(node.id.clone()) {
            // Already visited via a different parent — that's a cycle
            // (single-parent tree expected).
            continue;
        }
        if let Some(kids) = children_of.get(&node.id) {
            for k in kids {
                stack.push(*k);
            }
        }
    }
    if visited.len() != doc.nodes.len() {
        let participants: Vec<NodeId> = doc
            .nodes
            .iter()
            .filter(|n| !visited.contains(&n.id))
            .map(|n| n.id.clone())
            .collect();
        return Err(ValidationError::PartOfCycle { participants });
    }

    // 4. TextRun tiling: children's spans union to parent's spans.
    //    Empty TextRun (no children) is flagged as "doesn't tile";
    //    that's the right diagnostic.
    for parent in &doc.nodes {
        if parent.kind != NodeKind::TextRun {
            continue;
        }
        let kids = children_of.get(&parent.id).cloned().unwrap_or_default();
        let child_spans: Vec<(usize, usize)> = kids
            .iter()
            .flat_map(|k| k.source_spans.iter())
            .filter(|s| s.document_id == doc.document_id)
            .map(|s| (s.start, s.end))
            .collect();
        let parent_spans: Vec<(usize, usize)> = parent
            .source_spans
            .iter()
            .filter(|s| s.document_id == doc.document_id)
            .map(|s| (s.start, s.end))
            .collect();
        let missing = subtract_intervals(parent_spans, child_spans);
        if !missing.is_empty() {
            return Err(ValidationError::ChildrenDoNotTileParent {
                parent_id: parent.id.clone(),
                missing_ranges: missing,
            });
        }
    }

    Ok(())
}

/// Merge a list of (start, end) byte ranges into sorted, non-
/// overlapping intervals.
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

/// Return the byte ranges in `parent` that are not covered by any
/// range in `children`. Used to compute `missing_ranges` for
/// `ChildrenDoNotTileParent`.
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
            part_of: None,
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    /// v2 helper: build a TextRun parent node.
    fn ok_text_run(id: &str, doc: &DocumentId, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::TextRun,
            term: compound("text_run", vec![]),
            polarity: Polarity::Inherit,
            modality: Modality::Inherit,
            source_spans: vec![span(doc, start, end)],
            confidence: 1.0,
            part_of: None,
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

    // ----- v2 hierarchical-decomposition tests -----

    /// A TextRun parent containing one Fact child that tiles its span.
    #[test]
    fn textrun_with_one_child_tiling_full_span_passes() {
        let doc_id = DocumentId::new("doc1");
        let mut parent = ok_text_run("T0", &doc_id, 0, 30);
        let mut child = ok_fact("F1", &doc_id, 0, 30);
        child.part_of = Some(NodeId::new("T0"));
        // parent's polarity stays Inherit; that's fine for structural check
        // (propagation is ADJ03's concern, not validate's).
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent.clone(), child.clone()],
        };
        // Avoid unused-mut warnings.
        let _ = parent;
        let _ = child;
        assert_eq!(validate(&doc), Ok(()));
    }

    /// A TextRun with two adjacent children that together tile the span.
    #[test]
    fn textrun_with_children_that_jointly_tile_parent_passes() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_text_run("T0", &doc_id, 0, 50);
        let mut c1 = ok_fact("F1", &doc_id, 0, 20);
        let mut c2 = ok_fact("F2", &doc_id, 20, 50);
        c1.part_of = Some(NodeId::new("T0"));
        c2.part_of = Some(NodeId::new("T0"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, c1, c2],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    /// Children leaving a byte-gap fail with ChildrenDoNotTileParent and
    /// the missing range is reported.
    #[test]
    fn textrun_with_gap_in_children_fails_with_missing_range() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_text_run("T0", &doc_id, 0, 50);
        let mut c1 = ok_fact("F1", &doc_id, 0, 20);
        let mut c2 = ok_fact("F2", &doc_id, 30, 50);
        c1.part_of = Some(NodeId::new("T0"));
        c2.part_of = Some(NodeId::new("T0"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, c1, c2],
        };
        match validate(&doc) {
            Err(ValidationError::ChildrenDoNotTileParent { parent_id, missing_ranges }) => {
                assert_eq!(parent_id, NodeId::new("T0"));
                assert_eq!(missing_ranges, vec![(20, 30)]);
            }
            other => panic!("expected ChildrenDoNotTileParent, got {:?}", other),
        }
    }

    /// A TextRun with no children fails the tile check — its entire
    /// span is reported missing.
    #[test]
    fn empty_textrun_fails_with_full_missing_range() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_text_run("T0", &doc_id, 0, 50);
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent],
        };
        match validate(&doc) {
            Err(ValidationError::ChildrenDoNotTileParent {
                missing_ranges, ..
            }) => {
                assert_eq!(missing_ranges, vec![(0, 50)]);
            }
            other => panic!("expected ChildrenDoNotTileParent, got {:?}", other),
        }
    }

    /// A non-TextRun cannot have children; if it does, NonTextRunHasChildren
    /// fires.
    #[test]
    fn fact_with_child_via_part_of_rejected() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_fact("F0", &doc_id, 0, 30);
        let mut child = ok_fact("F1", &doc_id, 0, 30);
        child.part_of = Some(NodeId::new("F0"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, child],
        };
        match validate(&doc) {
            Err(ValidationError::NonTextRunHasChildren { parent_kind, .. }) => {
                assert_eq!(parent_kind, NodeKind::Fact);
            }
            other => panic!("expected NonTextRunHasChildren, got {:?}", other),
        }
    }

    /// `part_of` pointing to a nonexistent node fires DanglingPartOf.
    #[test]
    fn dangling_part_of_rejected() {
        let doc_id = DocumentId::new("doc1");
        let mut child = ok_fact("F1", &doc_id, 0, 30);
        child.part_of = Some(NodeId::new("DOES_NOT_EXIST"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![child],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DanglingPartOf { .. })
        ));
    }

    /// Child spans extending beyond parent fail with ChildSpansExceedParent.
    #[test]
    fn child_spans_exceeding_parent_rejected() {
        let doc_id = DocumentId::new("doc1");
        let parent = ok_text_run("T0", &doc_id, 10, 30);
        let mut child = ok_fact("F1", &doc_id, 0, 30); // 0..30 outside 10..30
        child.part_of = Some(NodeId::new("T0"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![parent, child],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::ChildSpansExceedParent { .. })
        ));
    }

    /// Polarity::Inherit on a Fact / Query / Uncertainty is accepted in v2.
    /// (The v1 strict checks gated on Inherit would have failed.)
    #[test]
    fn inherit_polarity_accepted_on_each_kind() {
        let doc_id = DocumentId::new("doc1");
        let mut fact = ok_fact("F1", &doc_id, 0, 10);
        fact.polarity = Polarity::Inherit;

        let mut q = ok_fact("Q1", &doc_id, 10, 20);
        q.kind = NodeKind::Query;
        q.polarity = Polarity::Inherit;

        let mut u = ok_fact("U1", &doc_id, 20, 30);
        u.kind = NodeKind::Uncertainty;
        u.polarity = Polarity::Inherit;

        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![fact, q, u],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    /// Nested TextRun decomposition (parent TextRun → child TextRun → leaf)
    /// validates if all tilings hold.
    #[test]
    fn nested_textrun_decomposition_validates() {
        let doc_id = DocumentId::new("doc1");
        let outer = ok_text_run("T0", &doc_id, 0, 50);
        let mut inner = ok_text_run("T1", &doc_id, 0, 50);
        inner.part_of = Some(NodeId::new("T0"));
        let mut leaf = ok_fact("F1", &doc_id, 0, 50);
        leaf.part_of = Some(NodeId::new("T1"));
        let doc = IRDocument {
            document_id: doc_id,
            nodes: vec![outer, inner, leaf],
        };
        assert_eq!(validate(&doc), Ok(()));
    }
}
