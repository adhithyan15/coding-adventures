//! # adjudication-polarity-modality — ADJ03 v2 propagation check.
//!
//! Reference implementation of
//! [`ADJ03` v2](../../../specs/ADJ03-polarity-modality-checker.md).
//! The check is a **structural propagation consistency check** over
//! the hierarchical IR from ADJ01 v2. The LLM declares polarity /
//! modality at each level of the decomposition tree; the framework
//! verifies the declarations are self-consistent.
//!
//! No trigger taxonomy. No NegEx. No scope detector. No English
//! assumption. Language-agnostic by construction.
//!
//! ## The invariant
//!
//! A leaf's **effective** polarity / modality is the value declared
//! on the nearest ancestor (or, if every ancestor is `Inherit`, the
//! leaf's own declaration or the framework default). A leaf cannot
//! silently contradict its ancestor — any declared override surfaces
//! in the audit trail and may trigger clarification via ADJ06.
//!
//! ## Violations vs. warnings
//!
//! - **Violation** (gates adjudication): `InheritChainUnresolved`,
//!   `RuledOutMustBeAffirmed`.
//! - **Warning** (recorded, does not gate by default):
//!   `LeafOverridesAncestorPolarity`, `LeafOverridesAncestorModality`.
//!
//! The default policy is *warn-do-not-block* because legitimate
//! overrides are common in real documents ("Denies chest pain,
//! fever, palpitations; admits shortness of breath."). A deployment
//! can configure warnings-as-errors for strict semantics.

use std::collections::HashMap;

use adjudication_ir::{IRDocument, IRNode, Modality, NodeId, NodeKind, Polarity};

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

/// Gating failures. The propagation check fails if any of these
/// is non-empty.
#[derive(Debug, Clone, PartialEq)]
pub enum PropagationViolation {
    /// Every ancestor up to the root declared `Inherit` AND the node
    /// itself declared `Inherit`. With no concrete value anywhere in
    /// the chain, the effective value cannot be resolved.
    InheritChainUnresolved { node_id: NodeId },

    /// A leaf with `modality = RuledOut` must have `polarity =
    /// Affirmed` (per ADJ01 hard rule). RuledOut is the clinician's
    /// adjudication, not a polarity flip.
    RuledOutMustBeAffirmed {
        node_id: NodeId,
        actual_polarity: Polarity,
    },
}

/// Non-gating warnings. Recorded in the audit trail; may be promoted
/// to errors via configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum PropagationWarning {
    /// A leaf's declared polarity overrides its ancestor's. Worth a
    /// review (often legitimate, e.g. "denies X, Y; admits Z").
    LeafOverridesAncestorPolarity {
        node_id: NodeId,
        declared: Polarity,
        ancestor: Polarity,
    },

    /// A leaf's declared modality overrides its ancestor's.
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
/// Walks every non-TextRun, non-Discarded leaf, resolves its
/// effective polarity / modality from the ancestor chain, and:
///
/// - Emits `InheritChainUnresolved` if the chain has no concrete
///   value (a gating violation).
/// - Emits `RuledOutMustBeAffirmed` if `modality = RuledOut` but
///   polarity isn't Affirmed (a gating violation).
/// - Emits `LeafOverridesAncestor*` if the leaf's declared value is
///   non-Inherit and differs from the ancestor's effective value
///   (warnings).
pub fn check_propagation(ir_doc: &IRDocument) -> PropagationResult {
    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    let by_id: HashMap<NodeId, &IRNode> = ir_doc
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n))
        .collect();

    // Pre-compute effective polarity/modality for every node, with
    // memoization. The lookup uses ancestor traversal.
    let mut eff_polarity: HashMap<NodeId, Polarity> = HashMap::new();
    let mut eff_modality: HashMap<NodeId, Modality> = HashMap::new();
    for node in &ir_doc.nodes {
        let ep = resolve_effective_polarity(node, &by_id, &mut eff_polarity);
        let em = resolve_effective_modality(node, &by_id, &mut eff_modality);

        if ep == Polarity::Inherit {
            violations.push(PropagationViolation::InheritChainUnresolved {
                node_id: node.id.clone(),
            });
        }
        if em == Modality::Inherit {
            violations.push(PropagationViolation::InheritChainUnresolved {
                node_id: node.id.clone(),
            });
        }
    }

    // Walk every leaf. TextRun nodes don't participate. Discarded
    // nodes are skipped (their polarity and modality are formally
    // fixed by ADJ01).
    for node in &ir_doc.nodes {
        if node.kind == NodeKind::TextRun || node.kind == NodeKind::Discarded {
            continue;
        }

        // RuledOut + non-Affirmed = hard rule violation.
        if node.modality == Modality::RuledOut && node.polarity != Polarity::Affirmed {
            violations.push(PropagationViolation::RuledOutMustBeAffirmed {
                node_id: node.id.clone(),
                actual_polarity: node.polarity,
            });
        }

        // Check declared-vs-ancestor for non-Inherit leaf
        // declarations.
        if let Some(parent_id) = &node.part_of {
            let parent_eff_p = eff_polarity.get(parent_id).copied().unwrap_or(Polarity::Affirmed);
            let parent_eff_m = eff_modality.get(parent_id).copied().unwrap_or(Modality::Present);

            if node.polarity != Polarity::Inherit && node.polarity != parent_eff_p {
                warnings.push(PropagationWarning::LeafOverridesAncestorPolarity {
                    node_id: node.id.clone(),
                    declared: node.polarity,
                    ancestor: parent_eff_p,
                });
            }
            if node.modality != Modality::Inherit && node.modality != parent_eff_m {
                warnings.push(PropagationWarning::LeafOverridesAncestorModality {
                    node_id: node.id.clone(),
                    declared: node.modality,
                    ancestor: parent_eff_m,
                });
            }
        }
    }

    PropagationResult {
        violations,
        warnings,
    }
}

// ---------------------------------------------------------------------------
// Effective polarity / modality resolution (with memoisation)
// ---------------------------------------------------------------------------

fn resolve_effective_polarity(
    node: &IRNode,
    by_id: &HashMap<NodeId, &IRNode>,
    cache: &mut HashMap<NodeId, Polarity>,
) -> Polarity {
    if let Some(v) = cache.get(&node.id) {
        return *v;
    }
    let v = if node.polarity != Polarity::Inherit {
        node.polarity
    } else if let Some(parent_id) = &node.part_of {
        if let Some(parent) = by_id.get(parent_id) {
            resolve_effective_polarity(parent, by_id, cache)
        } else {
            // Dangling part_of — ADJ02 catches this; default here.
            Polarity::Affirmed
        }
    } else {
        // Root with Inherit declared and no ancestor — chain is
        // unresolved.
        Polarity::Inherit
    };
    cache.insert(node.id.clone(), v);
    v
}

fn resolve_effective_modality(
    node: &IRNode,
    by_id: &HashMap<NodeId, &IRNode>,
    cache: &mut HashMap<NodeId, Modality>,
) -> Modality {
    if let Some(v) = cache.get(&node.id) {
        return *v;
    }
    let v = if node.modality != Modality::Inherit {
        node.modality
    } else if let Some(parent_id) = &node.part_of {
        if let Some(parent) = by_id.get(parent_id) {
            resolve_effective_modality(parent, by_id, cache)
        } else {
            Modality::Present
        }
    } else {
        Modality::Inherit
    };
    cache.insert(node.id.clone(), v);
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{DocumentId, Span};
    use logic_core::{atom, compound};
    use std::collections::HashMap;

    fn doc_id() -> DocumentId {
        DocumentId::new("doc1")
    }

    fn span_of(start: usize, end: usize) -> Span {
        Span::new(doc_id(), start, end)
    }

    fn text_run(
        id: &str,
        start: usize,
        end: usize,
        part_of: Option<&str>,
        polarity: Polarity,
        modality: Modality,
    ) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::TextRun,
            term: compound("text_run", vec![]),
            polarity,
            modality,
            source_spans: vec![span_of(start, end)],
            confidence: 1.0,
            part_of: part_of.map(NodeId::new),
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn fact_leaf(
        id: &str,
        start: usize,
        end: usize,
        part_of: Option<&str>,
        polarity: Polarity,
        modality: Modality,
    ) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: atom("placeholder"),
            polarity,
            modality,
            source_spans: vec![span_of(start, end)],
            confidence: 0.9,
            part_of: part_of.map(NodeId::new),
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    /// Single leaf with concrete polarity → Pass, no warnings.
    #[test]
    fn single_leaf_with_concrete_polarity_passes() {
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![fact_leaf(
                "F1",
                0,
                10,
                None,
                Polarity::Affirmed,
                Modality::Present,
            )],
        };
        let r = check_propagation(&ir);
        assert!(r.pass());
        assert!(r.warnings.is_empty());
    }

    /// Parent TextRun with Denied polarity, child Fact with Inherit
    /// → child inherits Denied. No warning.
    #[test]
    fn child_inherits_parent_denied_polarity() {
        let parent = text_run("T0", 0, 30, None, Polarity::Denied, Modality::Present);
        let child = fact_leaf("F1", 0, 30, Some("T0"), Polarity::Inherit, Modality::Inherit);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, child],
        };
        let r = check_propagation(&ir);
        assert!(r.pass(), "violations: {:?}", r.violations);
        assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    }

    /// Parent Denied, child explicitly Affirmed → override warning,
    /// but does not gate.
    #[test]
    fn child_overriding_parent_polarity_emits_warning() {
        let parent = text_run("T0", 0, 30, None, Polarity::Denied, Modality::Present);
        let child = fact_leaf("F1", 0, 30, Some("T0"), Polarity::Affirmed, Modality::Present);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, child],
        };
        let r = check_propagation(&ir);
        assert!(r.pass()); // warnings don't gate
        let has_override = r.warnings.iter().any(|w| matches!(
            w,
            PropagationWarning::LeafOverridesAncestorPolarity { declared: Polarity::Affirmed, ancestor: Polarity::Denied, .. }
        ));
        assert!(has_override, "expected polarity override warning: {:?}", r.warnings);
    }

    /// RuledOut + Affirmed = pass (the canonical RuledOut shape).
    #[test]
    fn ruled_out_with_affirmed_polarity_passes() {
        let leaf = fact_leaf(
            "F1",
            0,
            30,
            None,
            Polarity::Affirmed,
            Modality::RuledOut,
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![leaf],
        };
        let r = check_propagation(&ir);
        assert!(r.pass());
    }

    /// RuledOut + Denied = gating violation (the clinical/legal
    /// distinction enforced as a hard rule).
    #[test]
    fn ruled_out_with_denied_polarity_violates() {
        let leaf = fact_leaf(
            "F1",
            0,
            30,
            None,
            Polarity::Denied,
            Modality::RuledOut,
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![leaf],
        };
        let r = check_propagation(&ir);
        assert!(!r.pass());
        assert!(r.violations.iter().any(|v| matches!(
            v,
            PropagationViolation::RuledOutMustBeAffirmed { .. }
        )));
    }

    /// Parent Inherit + child Inherit + no ancestor → unresolved
    /// chain.
    #[test]
    fn unresolvable_inherit_chain_violates() {
        let parent = text_run("T0", 0, 30, None, Polarity::Inherit, Modality::Inherit);
        let child = fact_leaf("F1", 0, 30, Some("T0"), Polarity::Inherit, Modality::Inherit);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, child],
        };
        let r = check_propagation(&ir);
        assert!(!r.pass(), "expected violations, got {:?}", r);
        assert!(r.violations.iter().any(|v| matches!(
            v,
            PropagationViolation::InheritChainUnresolved { .. }
        )));
    }

    /// Multi-level inheritance: grandchild inherits from grandparent.
    #[test]
    fn multilevel_inheritance_through_textruns() {
        let outer = text_run("T0", 0, 50, None, Polarity::Denied, Modality::Past);
        let inner = text_run("T1", 0, 50, Some("T0"), Polarity::Inherit, Modality::Inherit);
        let leaf = fact_leaf(
            "F1",
            0,
            50,
            Some("T1"),
            Polarity::Inherit,
            Modality::Inherit,
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![outer, inner, leaf],
        };
        let r = check_propagation(&ir);
        assert!(r.pass());
        assert!(r.warnings.is_empty());
    }

    /// Legitimate override case: parent Denied with siblings inheriting
    /// Denied + one sibling explicitly Affirmed.
    #[test]
    fn list_of_denials_with_one_affirmation_emits_one_warning() {
        let parent = text_run("T0", 0, 100, None, Polarity::Denied, Modality::Present);
        let c1 = fact_leaf("F1", 0, 25, Some("T0"), Polarity::Inherit, Modality::Inherit);
        let c2 = fact_leaf("F2", 25, 50, Some("T0"), Polarity::Inherit, Modality::Inherit);
        let c3 = fact_leaf("F3", 50, 75, Some("T0"), Polarity::Inherit, Modality::Inherit);
        // The one explicit Affirmed amid the implicit Denieds.
        let c4 = fact_leaf("F4", 75, 100, Some("T0"), Polarity::Affirmed, Modality::Present);
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, c1, c2, c3, c4],
        };
        let r = check_propagation(&ir);
        assert!(r.pass());
        // Exactly one override warning (on F4); modality is Present
        // which matches the parent's effective Present, so no modality
        // warning. c1..c3 inherit cleanly.
        let polarity_overrides = r.warnings.iter().filter(|w| matches!(
            w,
            PropagationWarning::LeafOverridesAncestorPolarity { .. }
        )).count();
        assert_eq!(polarity_overrides, 1, "warnings: {:?}", r.warnings);
    }

    /// TextRun nodes themselves don't generate override warnings —
    /// only leaves do.
    #[test]
    fn textrun_does_not_emit_override_warnings() {
        let outer = text_run("T0", 0, 50, None, Polarity::Denied, Modality::Present);
        // Child TextRun that overrides; should not emit a warning
        // (only leaves do).
        let inner = text_run(
            "T1",
            0,
            50,
            Some("T0"),
            Polarity::Affirmed,
            Modality::Present,
        );
        let leaf = fact_leaf(
            "F1",
            0,
            50,
            Some("T1"),
            Polarity::Inherit,
            Modality::Inherit,
        );
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![outer, inner, leaf],
        };
        let r = check_propagation(&ir);
        assert!(r.pass());
        // Leaf F1's effective polarity is Affirmed (inherits from T1
        // which overrode T0). Since F1 declared Inherit, no warning
        // — the warning would only fire if F1 itself declared a
        // non-Inherit value disagreeing with T1.
        assert!(
            r.warnings.is_empty(),
            "intermediate TextRun overrides should not generate warnings: {:?}",
            r.warnings
        );
    }

    /// Discarded nodes are skipped from the check.
    #[test]
    fn discarded_nodes_are_skipped() {
        let parent = text_run("T0", 0, 30, None, Polarity::Denied, Modality::Present);
        // A Discarded node with Affirmed (which would override
        // ancestor's Denied if checked) is skipped per ADJ03.
        let discard = IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: atom("discarded"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span_of(0, 30)],
            confidence: 1.0,
            part_of: Some(NodeId::new("T0")),
            lowered_from: None,
            discard_reason: Some(adjudication_ir::DiscardReason::Pleasantry),
            metadata: HashMap::new(),
        };
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![parent, discard],
        };
        let r = check_propagation(&ir);
        assert!(r.pass());
        assert!(r.warnings.is_empty());
    }
}
