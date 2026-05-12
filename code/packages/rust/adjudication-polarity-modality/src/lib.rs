//! # adjudication-polarity-modality — ADJ03 v3 propagation check.
//!
//! Reference implementation of
//! [`ADJ03` v3](../../../specs/ADJ03-polarity-modality-checker.md).
//! Built on top of the v3 graph IR ([`adjudication_ir`]):
//! propagation now runs along [`EdgeRelation::Contains`] edges (was
//! `part_of` in v2), and multi-parent disagreement is a structural
//! violation.
//!
//! ## What this crate adds on top of `adjudication_ir::validate`
//!
//! `adjudication_ir::validate` already enforces:
//!
//! - `Inherit` polarity / modality on a node *requires* at least one
//!   `Contains` parent (`InheritWithoutParent`).
//! - Multiple `Contains` parents must agree on the effective value
//!   (`PropagationConflict`).
//!
//! This crate runs that validation and additionally reports:
//!
//! - **`RuledOutMustBeAffirmed`** — a hard ADJ01 rule (gating
//!   violation): if `modality = RuledOut` the polarity must be
//!   `Affirmed`. `RuledOut` is a clinician's adjudication, not a
//!   polarity flip.
//! - **`LeafOverridesAncestorPolarity` / `LeafOverridesAncestorModality`**
//!   — non-gating warnings recorded in the audit trail. A node that
//!   carries a *concrete* polarity / modality (not `Inherit`) and
//!   whose value differs from its `Contains` parent's effective value
//!   is recorded for review. The default policy is *warn, do not
//!   block* because legitimate overrides are common in real text
//!   ("denies X, Y; admits Z"). Deployments can promote warnings to
//!   errors via configuration.

use std::collections::HashMap;

use adjudication_ir::{
    validate, EdgeRelation, IRDocument, IRNode, InheritField, Modality, NodeId, NodeKind,
    Polarity, ValidationError,
};

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// The outcome of running the propagation check.
#[derive(Debug, Clone, PartialEq)]
pub struct PropagationResult {
    pub violations: Vec<PropagationViolation>,
    pub warnings: Vec<PropagationWarning>,
}

impl PropagationResult {
    pub fn pass(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Gating failures. The propagation check fails if any of these is
/// non-empty.
#[derive(Debug, Clone, PartialEq)]
pub enum PropagationViolation {
    /// A node has `polarity = Inherit` (or `modality = Inherit`) but
    /// no `Contains` parent. Surfaced from
    /// [`ValidationError::InheritWithoutParent`].
    InheritWithoutParent {
        node_id: NodeId,
        field: InheritField,
    },

    /// A node with `Inherit` has multiple `Contains` parents whose
    /// effective values disagree. Surfaced from
    /// [`ValidationError::PropagationConflict`].
    MultiParentConflict {
        node_id: NodeId,
        field: InheritField,
        candidates: Vec<(NodeId, String)>,
    },

    /// A node with `modality = RuledOut` must have `polarity =
    /// Affirmed` per ADJ01.
    RuledOutMustBeAffirmed {
        node_id: NodeId,
        actual_polarity: Polarity,
    },

    /// `adjudication_ir::validate` returned an error that isn't a
    /// propagation concern; the propagation check returns it
    /// faithfully so callers can route to the correct checker.
    UpstreamValidationError { kind: String },
}

/// Non-gating warnings. Recorded in the audit trail; may be promoted
/// to errors via configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum PropagationWarning {
    /// A node's declared polarity differs from its `Contains` parent's
    /// effective polarity.
    LeafOverridesAncestorPolarity {
        node_id: NodeId,
        declared: Polarity,
        ancestor: Polarity,
    },

    /// A node's declared modality differs from its `Contains` parent's
    /// effective modality.
    LeafOverridesAncestorModality {
        node_id: NodeId,
        declared: Modality,
        ancestor: Modality,
    },
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the propagation consistency check.
///
/// 1. Calls [`adjudication_ir::validate`] and translates any
///    `Inherit`-related errors into [`PropagationViolation`]s. Other
///    validation errors are surfaced as
///    `UpstreamValidationError { kind }`.
/// 2. Walks every non-`Section`, non-`Discarded` node and checks the
///    `RuledOut`/`Affirmed` constraint.
/// 3. For nodes with a *concrete* (non-`Inherit`) polarity / modality
///    that have at least one `Contains` parent, compares to each
///    parent's effective value and emits a `LeafOverridesAncestor*`
///    warning when they differ.
pub fn check_propagation(ir_doc: &IRDocument) -> PropagationResult {
    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    // Run validate first. If the IR has a structural integrity problem
    // (cycle, dangling edge endpoint, duplicate id, …), we can't
    // safely walk Contains edges to resolve effective polarity — the
    // walk could either recurse indefinitely on a cycle or index a
    // missing node. Return early with the upstream error and let the
    // caller route to the right checker (coverage / acyclicity /
    // edge-validity).
    if let Err(e) = validate(ir_doc) {
        match e {
            ValidationError::InheritWithoutParent { node_id, field } => {
                violations.push(PropagationViolation::InheritWithoutParent { node_id, field });
            }
            ValidationError::PropagationConflict { node_id, field, candidates } => {
                violations.push(PropagationViolation::MultiParentConflict {
                    node_id,
                    field,
                    candidates: candidates
                        .into_iter()
                        .map(|(id, s)| (id, s.to_string()))
                        .collect(),
                });
            }
            other => {
                // Structural integrity error: report and stop. The
                // Contains-edge walk below would be unsafe.
                violations.push(PropagationViolation::UpstreamValidationError {
                    kind: format!("{other:?}"),
                });
                return PropagationResult { violations, warnings };
            }
        }
    }

    // Build an adjacency view of `Contains` parents per node.
    let mut contains_parents: HashMap<&NodeId, Vec<&NodeId>> = HashMap::new();
    for e in &ir_doc.edges {
        if e.relation == EdgeRelation::Contains {
            contains_parents.entry(&e.target).or_default().push(&e.source);
        }
    }
    let by_id: HashMap<&NodeId, &IRNode> = ir_doc.nodes.iter().map(|n| (&n.id, n)).collect();

    // Memo for effective polarity/modality lookups. Pre-populate by
    // resolving every node once (the iterative resolvers only cache
    // nodes on the *walked* path, not the starting node — so we
    // capture the return value here so subsequent lookups by
    // parent-id always succeed).
    let mut eff_polarity: HashMap<NodeId, Polarity> = HashMap::new();
    let mut eff_modality: HashMap<NodeId, Modality> = HashMap::new();
    for node in &ir_doc.nodes {
        let p = resolve_effective_polarity(node, &by_id, &contains_parents, &mut eff_polarity);
        eff_polarity.insert(node.id.clone(), p);
        let m = resolve_effective_modality(node, &by_id, &contains_parents, &mut eff_modality);
        eff_modality.insert(node.id.clone(), m);
    }

    // RuledOut + override warnings.
    for node in &ir_doc.nodes {
        if matches!(node.kind, NodeKind::Section | NodeKind::Discarded | NodeKind::Entity) {
            continue;
        }

        // Hard rule: RuledOut requires Affirmed.
        if node.modality == Modality::RuledOut && node.polarity != Polarity::Affirmed {
            violations.push(PropagationViolation::RuledOutMustBeAffirmed {
                node_id: node.id.clone(),
                actual_polarity: node.polarity,
            });
        }

        // Override warnings — emitted only when the node carries a
        // concrete (non-Inherit) value AND has at least one Contains
        // parent.
        let Some(parents) = contains_parents.get(&node.id) else {
            continue;
        };
        let any_parent = parents.first().copied();
        if let Some(parent_id) = any_parent {
            if node.polarity != Polarity::Inherit {
                if let Some(parent_eff) = eff_polarity.get(parent_id) {
                    if *parent_eff != node.polarity && *parent_eff != Polarity::Inherit {
                        warnings.push(PropagationWarning::LeafOverridesAncestorPolarity {
                            node_id: node.id.clone(),
                            declared: node.polarity,
                            ancestor: *parent_eff,
                        });
                    }
                }
            }
            if node.modality != Modality::Inherit {
                if let Some(parent_eff) = eff_modality.get(parent_id) {
                    if *parent_eff != node.modality && *parent_eff != Modality::Inherit {
                        warnings.push(PropagationWarning::LeafOverridesAncestorModality {
                            node_id: node.id.clone(),
                            declared: node.modality,
                            ancestor: *parent_eff,
                        });
                    }
                }
            }
        }
    }

    PropagationResult { violations, warnings }
}

// ---------------------------------------------------------------------------
// Effective polarity / modality resolution (with memoisation)
// ---------------------------------------------------------------------------

/// Iterative `Contains`-chain walk. Iterative on purpose: cycles and
/// long linear chains are both safe — cycles are caught by
/// `validate()` upstream (we return early when one is reported), and
/// long chains are bounded by the heap-allocated `visited` Vec rather
/// than the OS thread stack.
///
/// If a parent id is missing from `by_id` (a dangling edge that
/// somehow survived validate), the walk treats the missing parent as
/// terminal and returns `Inherit` — graceful degradation rather than
/// a panic.
fn resolve_effective_polarity<'a>(
    start: &'a IRNode,
    by_id: &HashMap<&'a NodeId, &'a IRNode>,
    contains_parents: &HashMap<&'a NodeId, Vec<&'a NodeId>>,
    cache: &mut HashMap<NodeId, Polarity>,
) -> Polarity {
    if let Some(v) = cache.get(&start.id) {
        return *v;
    }
    let mut visited: Vec<&'a NodeId> = Vec::new();
    let mut cursor: &'a IRNode = start;
    let final_value = loop {
        if let Some(v) = cache.get(&cursor.id) {
            break *v;
        }
        if cursor.polarity != Polarity::Inherit {
            break cursor.polarity;
        }
        visited.push(&cursor.id);
        match contains_parents.get(&cursor.id).and_then(|ps| ps.first()) {
            Some(parent_id) => match by_id.get(parent_id) {
                Some(parent) => cursor = *parent,
                None => break Polarity::Inherit,
            },
            None => break Polarity::Inherit,
        }
    };
    for id in visited {
        cache.insert(id.clone(), final_value);
    }
    final_value
}

fn resolve_effective_modality<'a>(
    start: &'a IRNode,
    by_id: &HashMap<&'a NodeId, &'a IRNode>,
    contains_parents: &HashMap<&'a NodeId, Vec<&'a NodeId>>,
    cache: &mut HashMap<NodeId, Modality>,
) -> Modality {
    if let Some(v) = cache.get(&start.id) {
        return *v;
    }
    let mut visited: Vec<&'a NodeId> = Vec::new();
    let mut cursor: &'a IRNode = start;
    let final_value = loop {
        if let Some(v) = cache.get(&cursor.id) {
            break *v;
        }
        if cursor.modality != Modality::Inherit {
            break cursor.modality;
        }
        visited.push(&cursor.id);
        match contains_parents.get(&cursor.id).and_then(|ps| ps.first()) {
            Some(parent_id) => match by_id.get(parent_id) {
                Some(parent) => cursor = *parent,
                None => break Modality::Inherit,
            },
            None => break Modality::Inherit,
        }
    };
    for id in visited {
        cache.insert(id.clone(), final_value);
    }
    final_value
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{
        DocumentId, EdgeId, EdgeRelation, IREdge, Polarity, Span,
    };
    use logic_core::{atom, compound};
    use std::collections::HashMap as Map;

    fn doc_id() -> DocumentId {
        DocumentId::new("doc1")
    }

    fn span_of(start: usize, end: usize) -> Span {
        Span::new(doc_id(), start, end)
    }

    fn section(id: &str, start: usize, end: usize, polarity: Polarity, modality: Modality) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Section,
            term: compound("paragraph", vec![]),
            polarity,
            modality,
            source_spans: vec![span_of(start, end)],
            confidence: 1.0,
            discard_reason: None,
            metadata: Map::new(),
        }
    }

    fn fact(id: &str, start: usize, end: usize, polarity: Polarity, modality: Modality) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: atom("placeholder"),
            polarity,
            modality,
            source_spans: vec![span_of(start, end)],
            confidence: 0.9,
            discard_reason: None,
            metadata: Map::new(),
        }
    }

    fn contains(id: &str, source: &str, target: &str) -> IREdge {
        IREdge {
            id: EdgeId::new(id),
            source: NodeId::new(source),
            target: NodeId::new(target),
            relation: EdgeRelation::Contains,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![],
            confidence: 1.0,
            metadata: Map::new(),
        }
    }

    #[test]
    fn empty_doc_passes() {
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![],
            edges: vec![],
        };
        let result = check_propagation(&doc);
        assert!(result.violations.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn section_with_inherit_child_resolves_via_contains() {
        // Section (Denied) → Contains → Fact (Inherit polarity).
        let s = section("S1", 0, 5, Polarity::Denied, Modality::Present);
        let mut f = fact("F1", 5, 10, Polarity::Inherit, Modality::Present);
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![s, f],
            edges: vec![contains("E1", "S1", "F1")],
        };
        let r = check_propagation(&doc);
        assert!(r.pass(), "expected pass, got {:?}", r);
    }

    #[test]
    fn leaf_override_emits_warning() {
        // Section (Denied) → Contains → Fact (Affirmed). Concrete
        // override; should warn but not violate.
        let s = section("S1", 0, 5, Polarity::Denied, Modality::Present);
        let f = fact("F1", 5, 10, Polarity::Affirmed, Modality::Present);
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![s, f],
            edges: vec![contains("E1", "S1", "F1")],
        };
        let r = check_propagation(&doc);
        assert!(r.pass(), "expected violations empty");
        let has_warning = r.warnings.iter().any(|w| {
            matches!(
                w,
                PropagationWarning::LeafOverridesAncestorPolarity {
                    declared: Polarity::Affirmed,
                    ancestor: Polarity::Denied,
                    ..
                }
            )
        });
        assert!(has_warning, "expected polarity override warning: {:?}", r.warnings);
    }

    #[test]
    fn ruledout_with_non_affirmed_polarity_violates() {
        // A Fact with RuledOut modality and Denied polarity is a hard
        // ADJ01 violation.
        let f = fact("F1", 0, 10, Polarity::Denied, Modality::RuledOut);
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![f],
            edges: vec![],
        };
        let r = check_propagation(&doc);
        let has_v = r.violations.iter().any(|v| {
            matches!(v, PropagationViolation::RuledOutMustBeAffirmed { .. })
        });
        assert!(has_v, "expected RuledOutMustBeAffirmed: {:?}", r);
    }

    #[test]
    fn inherit_without_parent_violates() {
        // Fact with Inherit polarity, no Contains parent.
        let mut f = fact("F1", 0, 10, Polarity::Inherit, Modality::Present);
        // Suppress the Discarded-related coverage error by also
        // ensuring the doc isn't structurally broken otherwise; F1
        // alone tiles [0, 10).
        f.kind = NodeKind::Fact;
        let doc = IRDocument {
            document_id: doc_id(),
            nodes: vec![f],
            edges: vec![],
        };
        let r = check_propagation(&doc);
        // Either InheritWithoutParent or UpstreamValidationError —
        // both are gating.
        assert!(!r.pass());
    }
}
