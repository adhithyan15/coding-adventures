//! # ADJ25 PR-4 — Hierarchical decomposition orchestrator.
//!
//! Drives the four level-boundary LLM dispatches (`Document →
//! Sentence`, `Sentence → Phrase`, `Phrase → Claim`, `Fact →
//! TypedComponent`), assembles the resulting nodes into an
//! [`adjudication_ir::IRDocument`] with `Contains` edges, then runs
//! [`adjudication_coverage::check_hierarchical_coverage`] and
//! dispatches [`adjudication_clarification::retry_decompose_level`]
//! at every failing parent until the gate is clean or each parent's
//! retry budget is exhausted.
//!
//! ## Crate-placement deviation from the ADJ25 spec
//!
//! ADJ25's PR-4 entry suggests this orchestrator live in
//! `llm-primitives`. That cannot work without a cycle: the
//! orchestrator needs `retry_decompose_level` (in
//! `adjudication-clarification`) and `check_hierarchical_coverage`
//! (in `adjudication-coverage`), and both of those crates depend
//! transitively on `llm-primitives`. Placing the orchestrator in
//! `adjudication-pipeline` (which already depends on all of the
//! above) lets it use both without inverting the dependency graph.
//! The intent of the spec — one entry point that drives the
//! level-by-level flow — is unchanged.
//!
//! ## What this PR ships
//!
//! - [`HierarchicalDecomposeRequest`] / [`HierarchicalDecomposeOutcome`]
//!   / [`HierarchicalDecomposeError`] types.
//! - [`decompose_hierarchical`] orchestrator.
//! - JSON-to-`IRNode` parsing tolerant to LLM output variance.
//! - Span translation from parent-local offsets to document-absolute
//!   offsets.
//!
//! ## What it does NOT ship (deferred)
//!
//! - **Correlation vector propagation** — PR-5's job. The
//!   orchestrator assigns deterministic `NodeId`s; PR-5 adds a
//!   parallel `CorrelationId` space.
//! - **A new `decompose-text-vN` prompt** that teaches the LLM the
//!   hierarchy. The orchestrator currently relies on whatever prompt
//!   `decompose_text` is shipping with (currently `v5`, which teaches
//!   the flat IR shape). Real-LLM behaviour against the hierarchy
//!   prompt is the foundation bench (PR-6); the orchestrator is
//!   designed to work with scripted clients today and benefit from a
//!   richer prompt later without changing its own surface.

use std::collections::HashMap;

use adjudication_clarification::{
    retry_decompose_level, ClarificationError, DecompositionLevel,
    HierarchicalDecompRetryOutcome, HierarchicalDecompRetryRequest,
};
use adjudication_coverage::{
    check_hierarchical_coverage, DecompLevel, Document, HierarchicalCoverageResult,
    HierarchicalGap, HierarchicalGapKind,
};
use adjudication_ir::{
    set_edge_correlation_id, set_node_correlation_id, CorrelationId, DocumentId, EdgeId,
    EdgeRelation, IRDocument, IREdge, IRNode, Modality, NodeId, NodeKind, Polarity, Span,
};
use llm_primitives::GatewayConfig;
use logic_core::{atom, Term};

// ---------------------------------------------------------------------------
// Caps and defaults
// ---------------------------------------------------------------------------

/// Default per-parent retry budget. PR-2 doesn't gate on retry yet,
/// so this is a starting point informed by ADJ24 (where 3 attempts
/// caught roughly half of post-prompt failures on the small-model
/// bench). PR-6 will revisit.
pub const DEFAULT_MAX_RETRIES_PER_PARENT: usize = 3;

/// Hard cap on per-level dispatched LLM calls. Prevents runaway
/// fan-out from a malformed initial decomposition that produces
/// thousands of bogus children, each of which would trigger another
/// LLM call.
pub const PER_LEVEL_DISPATCH_CAP: usize = 1_024;

/// Hard cap on term-tree depth during JSON parsing. Bounds recursive
/// term walks so a deeply nested LLM response cannot stack-overflow
/// the parser.
const MAX_TERM_DEPTH: usize = 64;

/// Hard cap on per-compound-term argument count.
const MAX_TERM_ARGS: usize = 256;

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

/// What the caller hands the orchestrator.
#[derive(Debug, Clone)]
pub struct HierarchicalDecomposeRequest {
    /// Stable identifier for the document. Used as the
    /// `IRDocument::document_id`, the [`DocumentId`] in every
    /// span, and the `DecomposeTextRequest::document_id` in every
    /// LLM call dispatched.
    pub document_id: String,
    /// The text to decompose. The orchestrator never inspects the
    /// bytes; it only forwards them to the LLM at each level and
    /// uses `len()` for the Document span.
    pub source_text: String,
    /// Per-parent retry budget at every level. Default
    /// [`DEFAULT_MAX_RETRIES_PER_PARENT`].
    pub max_retries_per_parent: usize,
}

/// Successful outcome of [`decompose_hierarchical`].
#[derive(Debug, Clone)]
pub struct HierarchicalDecomposeOutcome {
    /// The assembled IR. Every level-bearing node has a span; every
    /// parent → children relationship is materialised as a `Contains`
    /// edge.
    pub ir_document: IRDocument,
    /// Total number of LLM calls dispatched (initial + retries
    /// across all levels and parents).
    pub total_llm_calls: usize,
    /// Number of retry calls (excluding the initial per-parent
    /// dispatch). Indicates how often coverage failed first-pass.
    pub retry_calls: usize,
}

/// Errors the orchestrator can return.
#[derive(Debug)]
pub enum HierarchicalDecomposeError {
    /// The LLM call (initial or retry) failed at the
    /// clarification-primitive level.
    Primitive {
        level: DecompLevel,
        parent_node_id: NodeId,
        cause: ClarificationError,
    },
    /// The response JSON could not be parsed into a list of
    /// `IRNode`s. The carrier is the raw JSON for diagnostics.
    UnparseableResponse {
        level: DecompLevel,
        parent_node_id: NodeId,
        raw_json: serde_json::Value,
    },
    /// After the per-parent retry budget was exhausted at some
    /// level, coverage still failed at one or more parents.
    CoverageUnresolved { gaps: Vec<HierarchicalGap> },
}

impl std::fmt::Display for HierarchicalDecomposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive {
                level,
                parent_node_id,
                cause,
            } => write!(
                f,
                "hierarchical-decompose primitive error at {:?} for parent {:?}: {}",
                level, parent_node_id.0, cause
            ),
            Self::UnparseableResponse {
                level,
                parent_node_id,
                ..
            } => write!(
                f,
                "hierarchical-decompose unparseable response at {:?} for parent {:?}",
                level, parent_node_id.0
            ),
            Self::CoverageUnresolved { gaps } => write!(
                f,
                "hierarchical-decompose coverage unresolved after retries ({} gap(s))",
                gaps.len()
            ),
        }
    }
}

impl std::error::Error for HierarchicalDecomposeError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Drive the level-by-level decomposition flow.
///
/// The algorithm:
///
/// 1. Build the synthetic `Document` root spanning the full source.
/// 2. For each level transition in order (`Document → Sentence`,
///    `Sentence → Phrase`, `Phrase → Claim`, `Fact → TypedComponent`):
///    iterate every parent at that level and dispatch one
///    [`retry_decompose_level`] call. Parse the response, splice
///    the children into the IR, and connect them with `Contains`
///    edges.
/// 3. Run [`check_hierarchical_coverage`] on the assembled IR. For
///    every reported gap, dispatch a retry against the failing
///    parent (with the gap description in the retry prompt).
/// 4. Re-validate. Repeat up to `max_retries_per_parent` per parent.
/// 5. Return either the clean IR or
///    [`HierarchicalDecomposeError::CoverageUnresolved`].
///
/// The orchestrator is designed to work end-to-end with both a
/// production gateway and a `ScriptedExtractor`-style test client.
/// PR-6 (the foundation bench) is where we measure real-LLM
/// behaviour against this scaffolding.
pub fn decompose_hierarchical(
    req: &HierarchicalDecomposeRequest,
    gateway: &GatewayConfig,
    now: impl Fn() -> String + Copy,
) -> Result<HierarchicalDecomposeOutcome, HierarchicalDecomposeError> {
    let doc_id = DocumentId::new(&req.document_id);
    let source_len = req.source_text.len();

    let mut ir = IRDocument {
        document_id: doc_id.clone(),
        nodes: vec![build_document_node(&doc_id, source_len)],
        edges: vec![],
    };

    let mut id_state = IdState::new();
    let mut total_calls: usize = 0;
    let mut retry_calls: usize = 0;

    // Initial top-down dispatch: walk levels in order, dispatching
    // one LLM call per parent at each level. This produces a fully-
    // populated IR (modulo gaps the coverage check will surface).
    for level in [
        DecompLevel::DocumentToSentence,
        DecompLevel::SentenceToPhrase,
        DecompLevel::PhraseToClaim,
        DecompLevel::FactToTypedComponent,
    ] {
        let parents: Vec<NodeId> = collect_parents_at_level(&ir, level);
        for parent_id in parents.into_iter().take(PER_LEVEL_DISPATCH_CAP) {
            let parent_node = match find_node(&ir, &parent_id) {
                Some(n) => n.clone(),
                None => continue,
            };
            let outcome = dispatch_level_call(
                &parent_node,
                &req.source_text,
                None,
                level,
                gateway,
                now,
                &doc_id,
            )?;
            total_calls += outcome.used_attempts;
            splice_children(
                &mut ir,
                &parent_node,
                outcome.corrected_children,
                &req.source_text,
                level,
                &mut id_state,
            )?;
        }
    }

    // Coverage-driven retry loop. Iterate until either clean or a
    // parent exhausts its budget. The dispatch order within an
    // iteration is gap-first (one retry per gap; same parent may
    // appear multiple times if its decomposition has multiple
    // gaps).
    let doc_for_coverage = Document {
        id: doc_id.clone(),
        normalized_text: req.source_text.clone(),
    };
    let mut budget: HashMap<NodeId, usize> = HashMap::new();
    // `max_retries_per_parent = 0` is honoured literally — no retries.
    // Cap at 64 so a runaway misconfiguration cannot blow LLM budgets.
    let max_retries = req.max_retries_per_parent.min(64);
    loop {
        let gaps = match check_hierarchical_coverage(&doc_for_coverage, &ir) {
            HierarchicalCoverageResult::Pass => break,
            HierarchicalCoverageResult::Fail { gaps } => gaps,
        };
        let mut made_progress = false;
        for gap in &gaps {
            // FlattenedAtom gaps are not parent-decomposition
            // failures the retry primitive can address (the atom is
            // already inside an existing node's term); leave them
            // for downstream tooling to surface.
            if matches!(gap.kind, HierarchicalGapKind::FlattenedAtom { .. }) {
                continue;
            }
            let used = budget.entry(gap.parent_node_id.clone()).or_insert(0);
            if *used >= max_retries {
                continue;
            }
            *used += 1;
            let parent_node = match find_node(&ir, &gap.parent_node_id) {
                Some(n) => n.clone(),
                None => continue,
            };
            let prior_children =
                snapshot_children_as_json(&ir, &gap.parent_node_id, &req.source_text);
            let gap_description =
                render_gap_description(gap, &parent_node, &req.source_text);
            let outcome = dispatch_level_call(
                &parent_node,
                &req.source_text,
                Some((prior_children, gap_description)),
                gap.level,
                gateway,
                now,
                &doc_id,
            )?;
            total_calls += outcome.used_attempts;
            retry_calls += outcome.used_attempts;
            // Replace the parent's children entirely with the
            // corrected set. This is conservative — a more
            // surgical update would splice only the new
            // children covering the gap's bytes — but the
            // wholesale replace is easier to reason about.
            evict_children(&mut ir, &gap.parent_node_id);
            splice_children(
                &mut ir,
                &parent_node,
                outcome.corrected_children,
                &req.source_text,
                gap.level,
                &mut id_state,
            )?;
            made_progress = true;
        }
        if !made_progress {
            // Every gap is either FlattenedAtom or every parent has
            // exhausted its budget; we cannot drive convergence further.
            return Err(HierarchicalDecomposeError::CoverageUnresolved { gaps });
        }
    }

    Ok(HierarchicalDecomposeOutcome {
        ir_document: ir,
        total_llm_calls: total_calls,
        retry_calls,
    })
}

// ---------------------------------------------------------------------------
// Per-level dispatch
// ---------------------------------------------------------------------------

fn dispatch_level_call(
    parent: &IRNode,
    full_source: &str,
    prior_and_gap: Option<(serde_json::Value, String)>,
    level: DecompLevel,
    gateway: &GatewayConfig,
    now: impl Fn() -> String,
    doc_id: &DocumentId,
) -> Result<HierarchicalDecompRetryOutcome, HierarchicalDecomposeError> {
    let parent_text = parent_text_for(parent, full_source);
    let (previous_children, gap_description) = prior_and_gap.unwrap_or_else(|| {
        (
            serde_json::json!({ "nodes": [] }),
            format!(
                "no decomposition has been produced yet — please decompose the entire \
                 parent into {children}",
                children = level_children_noun(level)
            ),
        )
    });
    let req = HierarchicalDecompRetryRequest {
        level: coverage_level_to_decomp_level(level),
        document_id: doc_id.0.clone(),
        parent_text,
        previous_children,
        gap_description,
        ancestor_context: None,
    };
    retry_decompose_level(&req, gateway, 1, now).map_err(|cause| {
        HierarchicalDecomposeError::Primitive {
            level,
            parent_node_id: parent.id.clone(),
            cause,
        }
    })
}

/// Map a coverage-side level enum to a clarification-side level
/// enum. The two are isomorphic by design; this exists so the two
/// crates need not depend on each other.
fn coverage_level_to_decomp_level(l: DecompLevel) -> DecompositionLevel {
    match l {
        DecompLevel::DocumentToSentence => DecompositionLevel::DocumentToSentence,
        DecompLevel::SentenceToPhrase => DecompositionLevel::SentenceToPhrase,
        DecompLevel::PhraseToClaim => DecompositionLevel::PhraseToClaim,
        DecompLevel::FactToTypedComponent => DecompositionLevel::FactToTypedComponent,
    }
}

fn level_children_noun(l: DecompLevel) -> &'static str {
    match l {
        DecompLevel::DocumentToSentence => "sentences",
        DecompLevel::SentenceToPhrase => "phrases",
        DecompLevel::PhraseToClaim => "claims (facts, uncertainties, or questions)",
        DecompLevel::FactToTypedComponent => {
            "typed components (quantities, entities, predicates, etc.)"
        }
    }
}

fn allowed_kinds_for_level(l: DecompLevel) -> &'static [NodeKind] {
    match l {
        DecompLevel::DocumentToSentence => &[NodeKind::Sentence, NodeKind::Discarded][..],
        DecompLevel::SentenceToPhrase => &[NodeKind::Phrase, NodeKind::Discarded][..],
        DecompLevel::PhraseToClaim => &[
            NodeKind::Fact,
            NodeKind::Uncertainty,
            NodeKind::Question,
            NodeKind::Discarded,
        ][..],
        DecompLevel::FactToTypedComponent => &[
            NodeKind::Quantity,
            NodeKind::Polarity,
            NodeKind::Predicate,
            NodeKind::Comparator,
            NodeKind::TimeRef,
            NodeKind::Modifier,
            NodeKind::Entity,
        ][..],
    }
}

// ---------------------------------------------------------------------------
// IR assembly
// ---------------------------------------------------------------------------

struct IdState {
    next_node: u64,
    next_edge: u64,
}

impl IdState {
    fn new() -> Self {
        Self {
            next_node: 1,
            next_edge: 1,
        }
    }
    fn next_node_id(&mut self, prefix: &str) -> NodeId {
        let id = NodeId::new(&format!("{}{}", prefix, self.next_node));
        self.next_node += 1;
        id
    }
    fn next_edge_id(&mut self) -> EdgeId {
        let id = EdgeId::new(&format!("ce{}", self.next_edge));
        self.next_edge += 1;
        id
    }
}

fn build_document_node(doc_id: &DocumentId, source_len: usize) -> IRNode {
    let mut n = IRNode {
        id: NodeId::new("Doc"),
        kind: NodeKind::Document,
        term: atom("doc"),
        polarity: Polarity::Affirmed,
        modality: Modality::Present,
        source_spans: vec![Span::new(doc_id.clone(), 0, source_len)],
        confidence: 1.0,
        discard_reason: None,
        metadata: HashMap::new(),
    };
    // ADJ25 PR-5: every node produced by the hierarchical orchestrator
    // carries a CorrelationId. The Document root's ID is derived from
    // its NodeId for stability across runs; downstream nodes use the
    // same NodeId-derived scheme so the correlation tree mirrors the
    // Contains-edge hierarchy 1:1.
    let corr = correlation_id_for_node(&n.id);
    set_node_correlation_id(&mut n, corr);
    n
}

/// Derive a `CorrelationId` from a `NodeId`. The orchestrator uses
/// deterministic IDs so a re-run with the same source produces the
/// same correlation tree, which lets the audit-trail replay match
/// IDs byte-for-byte.
fn correlation_id_for_node(node_id: &NodeId) -> CorrelationId {
    CorrelationId::new(format!("corr.{}", node_id.0))
}

/// Derive a `CorrelationId` from an `EdgeId`. Edges carry their own
/// IDs so a downstream consumer can trace a Contains-edge back to
/// the source-decomposition stage that produced it (alongside the
/// node IDs at either endpoint).
fn correlation_id_for_edge(edge_id: &EdgeId) -> CorrelationId {
    CorrelationId::new(format!("corr.e.{}", edge_id.0))
}

fn parent_text_for(parent: &IRNode, full_source: &str) -> String {
    let Some(span) = parent.source_spans.first() else {
        return String::new();
    };
    let start = span.start.min(full_source.len());
    let end = span.end.min(full_source.len());
    if start >= end {
        return String::new();
    }
    // Walk to the nearest UTF-8 char boundary if the LLM-supplied
    // span landed mid-codepoint. This shouldn't happen for
    // synthetic Document spans but it's a defensive belt-and-braces
    // measure for parent nodes whose spans came from an earlier
    // LLM response.
    let mut s = start;
    while s < end && !full_source.is_char_boundary(s) {
        s += 1;
    }
    let mut e = end;
    while e > s && !full_source.is_char_boundary(e) {
        e -= 1;
    }
    full_source[s..e].to_string()
}

fn collect_parents_at_level(ir: &IRDocument, level: DecompLevel) -> Vec<NodeId> {
    let parent_kind = match level {
        DecompLevel::DocumentToSentence => NodeKind::Document,
        DecompLevel::SentenceToPhrase => NodeKind::Sentence,
        DecompLevel::PhraseToClaim => NodeKind::Phrase,
        DecompLevel::FactToTypedComponent => NodeKind::Fact,
    };
    ir.nodes
        .iter()
        .filter(|n| n.kind == parent_kind)
        .map(|n| n.id.clone())
        .collect()
}

fn find_node<'a>(ir: &'a IRDocument, id: &NodeId) -> Option<&'a IRNode> {
    ir.nodes.iter().find(|n| n.id == *id)
}

fn evict_children(ir: &mut IRDocument, parent_id: &NodeId) {
    let to_remove: std::collections::HashSet<NodeId> = ir
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Contains && e.source == *parent_id)
        .map(|e| e.target.clone())
        .collect();
    ir.edges.retain(|e| {
        !(e.relation == EdgeRelation::Contains && e.source == *parent_id)
    });
    ir.nodes.retain(|n| !to_remove.contains(&n.id));
}

fn splice_children(
    ir: &mut IRDocument,
    parent: &IRNode,
    children_json: serde_json::Value,
    full_source: &str,
    level: DecompLevel,
    id_state: &mut IdState,
) -> Result<(), HierarchicalDecomposeError> {
    let nodes_raw = match children_json
        .get("nodes")
        .and_then(|v| v.as_array())
        .cloned()
    {
        Some(v) => v,
        None => {
            return Err(HierarchicalDecomposeError::UnparseableResponse {
                level,
                parent_node_id: parent.id.clone(),
                raw_json: children_json,
            });
        }
    };
    let allowed = allowed_kinds_for_level(level);
    let parent_start_in_doc = parent
        .source_spans
        .first()
        .map(|s| s.start)
        .unwrap_or(0);
    let parent_end_in_doc = parent
        .source_spans
        .first()
        .map(|s| s.end)
        .unwrap_or(full_source.len());
    let parent_bytes = if parent_start_in_doc < parent_end_in_doc
        && parent_end_in_doc <= full_source.len()
    {
        &full_source[parent_start_in_doc..parent_end_in_doc]
    } else {
        ""
    };
    let id_prefix = match level {
        DecompLevel::DocumentToSentence => "S",
        DecompLevel::SentenceToPhrase => "P",
        DecompLevel::PhraseToClaim => "C",
        DecompLevel::FactToTypedComponent => "T",
    };
    // ADJ27 content-matching: the LLM emits `text` for each child;
    // the framework matches each text left-to-right against the
    // parent's bytes to derive absolute spans. The cursor advances
    // past each match, so duplicate substrings in the parent are
    // distinguished by occurrence order, and out-of-order claims
    // are surfaced as content-not-found (which the coverage check
    // then renders as an uncovered-range gap).
    let mut cursor: usize = 0;
    let mut accepted_children: Vec<(NodeId, NodeKind)> = Vec::new();
    for raw in nodes_raw.into_iter().take(PER_LEVEL_DISPATCH_CAP) {
        let Some((mut node, claimed_text)) =
            parse_child_node(&raw, allowed, id_state, id_prefix)
        else {
            continue;
        };
        let needle = claimed_text.unwrap_or_default();
        if needle.is_empty() {
            // Synthesized Entity / synthesized object — accept
            // with empty spans. The coverage check handles the
            // exemption.
            if node.kind == NodeKind::Entity {
                accepted_children.push((node.id.clone(), node.kind));
                ir.nodes.push(node);
            }
            continue;
        }
        let search_in = if cursor <= parent_bytes.len() {
            &parent_bytes[cursor..]
        } else {
            ""
        };
        let Some(rel_match) = search_in.find(needle.as_str()) else {
            // Content not found in remaining parent text. Skip
            // this child — the coverage check will later surface
            // the bytes left uncovered.
            continue;
        };
        let abs_start_in_parent = cursor + rel_match;
        let abs_end_in_parent = abs_start_in_parent + needle.len();
        if abs_end_in_parent > parent_bytes.len() {
            continue;
        }
        let abs_start_in_doc = parent_start_in_doc + abs_start_in_parent;
        let abs_end_in_doc = parent_start_in_doc + abs_end_in_parent;
        node.source_spans = vec![Span::new(
            ir.document_id.clone(),
            abs_start_in_doc,
            abs_end_in_doc,
        )];
        cursor = abs_end_in_parent;
        accepted_children.push((node.id.clone(), node.kind));
        ir.nodes.push(node);
    }
    for (child_id, _kind) in accepted_children {
        let edge_id = id_state.next_edge_id();
        let mut edge = IREdge {
            id: edge_id.clone(),
            source: parent.id.clone(),
            target: child_id,
            relation: EdgeRelation::Contains,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![],
            confidence: 1.0,
            metadata: HashMap::new(),
        };
        // ADJ25 PR-5: Contains edges also carry correlation IDs so
        // the audit trail can trace which decomposition stage
        // emitted which structural link.
        set_edge_correlation_id(&mut edge, correlation_id_for_edge(&edge_id));
        ir.edges.push(edge);
    }
    Ok(())
}

fn snapshot_children_as_json(
    ir: &IRDocument,
    parent_id: &NodeId,
    full_source: &str,
) -> serde_json::Value {
    let mut nodes: Vec<serde_json::Value> = Vec::new();
    if find_node(ir, parent_id).is_none() {
        return serde_json::json!({ "nodes": nodes });
    }
    // ADJ27: snapshot uses the same content-shaped contract as the
    // primitive. Each child's `text` is the literal substring of
    // the document its spans cover. The model never sees byte
    // ranges in the snapshot — only the strings it claimed.
    for edge in &ir.edges {
        if edge.relation != EdgeRelation::Contains || edge.source != *parent_id {
            continue;
        }
        let Some(child) = find_node(ir, &edge.target) else {
            continue;
        };
        let text_for_child = child
            .source_spans
            .first()
            .and_then(|s| {
                let start = s.start.min(full_source.len());
                let end = s.end.min(full_source.len());
                if start < end {
                    Some(full_source[start..end].to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        nodes.push(serde_json::json!({
            "id": child.id.0,
            "kind": format!("{:?}", child.kind),
            "term": term_to_json(&child.term),
            "polarity": polarity_to_str(child.polarity),
            "modality": modality_to_str(child.modality),
            "text": text_for_child,
        }));
    }
    serde_json::json!({ "nodes": nodes })
}

/// Render a gap into a content-shaped description for the retry
/// prompt. ADJ27: instead of speaking in byte ranges (which the
/// model is bad at), the framework extracts the LITERAL missing
/// substrings from the source and shows them to the model. The
/// model sees text it forgot to account for, not arithmetic to do.
fn render_gap_description(
    gap: &HierarchicalGap,
    parent_node: &IRNode,
    full_source: &str,
) -> String {
    let parent_start = parent_node
        .source_spans
        .first()
        .map(|s| s.start)
        .unwrap_or(0);
    let parent_end = parent_node
        .source_spans
        .first()
        .map(|s| s.end)
        .unwrap_or(full_source.len());
    let parent_text = if parent_start < parent_end && parent_end <= full_source.len() {
        &full_source[parent_start..parent_end]
    } else {
        ""
    };
    match &gap.kind {
        HierarchicalGapKind::UncoveredBytes { ranges } => {
            // Translate document-absolute ranges into the literal
            // missing substrings of the parent text.
            let missing_strs: Vec<String> = ranges
                .iter()
                .filter_map(|(s, e)| {
                    let s = (*s).min(full_source.len());
                    let e = (*e).min(full_source.len());
                    if s < e {
                        Some(format!("\"{}\"", &full_source[s..e]))
                    } else {
                        None
                    }
                })
                .collect();
            format!(
                "Your previous attempt covered some of the parent text but not all of it. \
                 The parent text is exactly:\n  \"{parent_text}\"\n\n\
                 You missed the following piece(s): {}.\n\n\
                 Please redo the decomposition over the ENTIRE parent text. Every \
                 character — including the missed piece(s) above — must appear in \
                 exactly one child's `text` field. Do not skip any characters.",
                missing_strs.join(", "),
            )
        }
        HierarchicalGapKind::Overlap { ranges, .. } => {
            let overlap_strs: Vec<String> = ranges
                .iter()
                .filter_map(|(s, e)| {
                    let s = (*s).min(full_source.len());
                    let e = (*e).min(full_source.len());
                    if s < e {
                        Some(format!("\"{}\"", &full_source[s..e]))
                    } else {
                        None
                    }
                })
                .collect();
            format!(
                "Two or more of your children claimed the same piece(s) of the parent \
                 text: {}. Each character of the parent must appear in EXACTLY ONE \
                 child's `text` field. Redo the decomposition without overlapping \
                 claims.",
                overlap_strs.join(", "),
            )
        }
        HierarchicalGapKind::EmptyChildSpan { child_id } => format!(
            "Child {} had no `text`; every child must claim a non-empty substring \
             of the parent.",
            child_id.0
        ),
        HierarchicalGapKind::ChildSpansEscape { child_id, .. } => format!(
            "Child {}'s `text` did not match any substring of the parent. Make sure \
             every child's `text` is a LITERAL substring of the parent, character \
             for character.",
            child_id.0
        ),
        HierarchicalGapKind::NoChildrenAtLevel => format!(
            "The parent text is:\n  \"{parent_text}\"\n\n\
             You produced no children. Please decompose the parent text into one \
             or more children whose `text` fields together cover every character.",
        ),
        HierarchicalGapKind::WrongChildKindForLevel {
            child_id,
            child_kind,
        } => format!(
            "Child {} had kind {:?}, which is not allowed at this level. See the \
             system prompt for the allowed kinds.",
            child_id.0, child_kind,
        ),
        HierarchicalGapKind::FlattenedAtom { atom, reason, .. } => format!(
            "The atom \"{atom}\" smuggles source content into its name ({reason:?}). \
             Surface the underlying values as separate components (Quantity for \
             numbers, Entity for nouns) instead of flattening them into one atom."
        ),
    }
}

// ---------------------------------------------------------------------------
// JSON-to-IR parsing
// ---------------------------------------------------------------------------

/// Parse one LLM-emitted child node. Returns the IR node (with
/// EMPTY `source_spans` — those are computed by `splice_children`
/// via content-matching against the parent text) and the literal
/// `text` field the model emitted, if any.
///
/// ADJ27: the contract is content-based, not byte-offset-based.
/// The LLM emits the exact substring it is claiming for each child;
/// `splice_children` matches that substring left-to-right against
/// the parent's bytes to derive document-absolute spans. The model
/// never does byte arithmetic.
///
/// Legacy `source_spans` arrays are NOT read — older LLM responses
/// emitting byte offsets are not accepted; the orchestrator returns
/// an empty span list and the coverage check surfaces the gap.
/// Metadata key for the model's `discard_justification` field.
/// ADJ28: when the model marks a chunk Discarded, it must include a
/// sentence explaining WHY discarding loses no information. The
/// orchestrator stores that sentence in the node's metadata under
/// this reserved key so the audit trail can replay the model's
/// reasoning verbatim.
pub const DISCARD_JUSTIFICATION_METADATA_KEY: &str = "adj.discard_justification";

/// Read the kind from an LLM-emitted JSON node, supporting two
/// schemas (ADJ28):
///
/// 1. **Legacy single-`kind` string field** (levels 1 & 2 — Sentence
///    and Phrase prompts kept this since they only have 2 options
///    each). Maps directly via [`parse_kind`].
/// 2. **New per-kind `is_X` boolean schema** (levels 3 & 4 — Claim
///    with 4 options, TypedComponent with 7). The model emits a
///    boolean per allowed kind; the orchestrator requires exactly
///    one to be `true`. This decomposes multi-way picking into
///    sequential yes/no decisions, which small models handle
///    better than direct N-way classification.
///
/// Returns `None` if neither schema yields a valid kind (zero or
/// multiple `true` booleans count as invalid), letting the caller
/// skip the child and surface the gap via the coverage check.
fn extract_kind(obj: &serde_json::Map<String, serde_json::Value>) -> Option<NodeKind> {
    if let Some(kind_str) = obj.get("kind").and_then(|v| v.as_str()) {
        if let Some(k) = parse_kind(kind_str) {
            return Some(k);
        }
    }
    // ADJ28 boolean schema. Pairs map each `is_X` field to the
    // NodeKind it represents. Covers every kind that levels 3 and
    // 4 emit; levels 1 and 2 would also work via this path if the
    // model decides to switch.
    let pairs: &[(&str, NodeKind)] = &[
        ("is_fact", NodeKind::Fact),
        ("is_uncertainty", NodeKind::Uncertainty),
        ("is_question", NodeKind::Question),
        ("is_discarded", NodeKind::Discarded),
        ("is_sentence", NodeKind::Sentence),
        ("is_phrase", NodeKind::Phrase),
        ("is_quantity", NodeKind::Quantity),
        ("is_polarity", NodeKind::Polarity),
        ("is_entity", NodeKind::Entity),
        ("is_predicate", NodeKind::Predicate),
        ("is_comparator", NodeKind::Comparator),
        ("is_timeref", NodeKind::TimeRef),
        ("is_modifier", NodeKind::Modifier),
    ];
    let mut hits: Vec<NodeKind> = Vec::new();
    for (key, kind) in pairs {
        if obj.get(*key).and_then(|v| v.as_bool()) == Some(true) {
            hits.push(*kind);
        }
    }
    // Exactly-one-true contract. Zero hits or multiple hits both
    // indicate the model didn't commit; reject so the caller can
    // surface the gap.
    match hits.len() {
        1 => Some(hits[0]),
        _ => None,
    }
}

fn parse_child_node(
    v: &serde_json::Value,
    allowed_kinds: &[NodeKind],
    id_state: &mut IdState,
    id_prefix: &str,
) -> Option<(IRNode, Option<String>)> {
    let obj = v.as_object()?;
    let kind = extract_kind(obj)?;
    if !allowed_kinds.contains(&kind) {
        return None;
    }
    let id = match obj.get("id").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => NodeId::new(s),
        _ => id_state.next_node_id(id_prefix),
    };
    let term = obj
        .get("term")
        .and_then(|t| parse_term(t, 0))
        .unwrap_or_else(|| atom("unknown"));
    let polarity = obj
        .get("polarity")
        .and_then(|v| v.as_str())
        .map(parse_polarity)
        .unwrap_or(Polarity::Affirmed);
    let modality = obj
        .get("modality")
        .and_then(|v| v.as_str())
        .map(parse_modality)
        .unwrap_or(Modality::Present);
    // ADJ27: read the LLM-supplied `text` substring. The caller
    // (`splice_children`) matches this against the parent text to
    // compute spans. We don't do that here so this function stays
    // a pure parse step.
    let claimed_text = obj
        .get("text")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    // ADJ28: read `discard_reason` and `discard_justification` for
    // Discarded nodes. Default reason is `NonDomainContent` for
    // back-compat with the previous schema; the justification (if
    // any) lands in metadata so the audit trail keeps the model's
    // own rationale.
    let discard_reason = if kind == NodeKind::Discarded {
        Some(parse_discard_reason(
            obj.get("discard_reason").and_then(|v| v.as_str()),
        ))
    } else {
        None
    };
    let discard_justification = obj
        .get("discard_justification")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut node = IRNode {
        id,
        kind,
        term,
        polarity,
        modality,
        source_spans: vec![],
        confidence: 1.0,
        discard_reason,
        metadata: HashMap::new(),
    };
    if let Some(justification) = discard_justification {
        node.metadata
            .insert(DISCARD_JUSTIFICATION_METADATA_KEY.to_string(), justification);
    }
    // ADJ25 PR-5: assign a CorrelationId to every parsed child. The
    // ID derives from the (assigned) NodeId so the correlation tree
    // mirrors the Contains-edge hierarchy.
    let corr = correlation_id_for_node(&node.id);
    set_node_correlation_id(&mut node, corr);
    Some((node, claimed_text))
}

fn parse_discard_reason(s: Option<&str>) -> adjudication_ir::DiscardReason {
    use adjudication_ir::DiscardReason::*;
    match s.unwrap_or("") {
        "Pleasantry" => Pleasantry,
        "DocumentMetadata" => DocumentMetadata,
        "NonDomainContent" => NonDomainContent,
        "Restatement" => Restatement,
        "Unparseable" => Unparseable,
        "AdministrativeOnly" => AdministrativeOnly,
        "ExplicitlyOutOfScope" => ExplicitlyOutOfScope,
        _ => NonDomainContent,
    }
}

fn parse_term(v: &serde_json::Value, depth: usize) -> Option<Term> {
    if depth >= MAX_TERM_DEPTH {
        return None;
    }
    let obj = v.as_object()?;
    if let Some(name) = obj.get("atom").and_then(|x| x.as_str()) {
        return Some(atom(name));
    }
    if let Some(n) = obj.get("num").and_then(|x| x.as_i64()) {
        return Some(logic_core::int(n));
    }
    let functor = obj.get("functor")?.as_str()?;
    let args_slice: &[serde_json::Value] = obj
        .get("args")
        .and_then(|x| x.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    if args_slice.len() > MAX_TERM_ARGS {
        return None;
    }
    let parsed_args: Vec<Term> = args_slice
        .iter()
        .filter_map(|a| parse_term(a, depth + 1))
        .collect();
    Some(logic_core::compound(functor, parsed_args))
}

fn parse_kind(s: &str) -> Option<NodeKind> {
    Some(match s {
        "Fact" => NodeKind::Fact,
        "Query" => NodeKind::Query,
        "Uncertainty" => NodeKind::Uncertainty,
        "Rule" => NodeKind::Rule,
        "Exception" => NodeKind::Exception,
        "Discarded" => NodeKind::Discarded,
        "Section" => NodeKind::Section,
        "Entity" => NodeKind::Entity,
        "Document" => NodeKind::Document,
        "Sentence" => NodeKind::Sentence,
        "Phrase" => NodeKind::Phrase,
        "Question" => NodeKind::Question,
        "Quantity" => NodeKind::Quantity,
        "Polarity" => NodeKind::Polarity,
        "Predicate" => NodeKind::Predicate,
        "Comparator" => NodeKind::Comparator,
        "TimeRef" => NodeKind::TimeRef,
        "Modifier" => NodeKind::Modifier,
        _ => return None,
    })
}

fn parse_polarity(s: &str) -> Polarity {
    match s {
        "Affirmed" => Polarity::Affirmed,
        "Denied" => Polarity::Denied,
        "Uncertain" => Polarity::Uncertain,
        "Inherit" => Polarity::Inherit,
        _ => Polarity::Affirmed,
    }
}

fn parse_modality(s: &str) -> Modality {
    match s {
        "Present" => Modality::Present,
        "Past" => Modality::Past,
        "Future" => Modality::Future,
        "Hypothetical" => Modality::Hypothetical,
        "FamilyHistory" => Modality::FamilyHistory,
        "RuledOut" => Modality::RuledOut,
        "Conditional" => Modality::Conditional,
        "Inherit" => Modality::Inherit,
        _ => Modality::Present,
    }
}

// ---------------------------------------------------------------------------
// Term serialisation (for snapshot_children_as_json)
// ---------------------------------------------------------------------------

fn term_to_json(t: &Term) -> serde_json::Value {
    match t {
        Term::Atom(s) => serde_json::json!({ "atom": s }),
        Term::Num(n) => match n {
            logic_core::Number::Int(i) => serde_json::json!({ "num": *i }),
            logic_core::Number::Float(f) => serde_json::json!({ "num": *f }),
        },
        Term::Str(s) => serde_json::json!({ "str": s }),
        Term::Var(v) => serde_json::json!({ "var": v.to_string() }),
        Term::Compound { functor, args } => {
            let arg_json: Vec<serde_json::Value> = args.iter().map(term_to_json).collect();
            serde_json::json!({ "functor": functor, "args": arg_json })
        }
    }
}

fn polarity_to_str(p: Polarity) -> &'static str {
    match p {
        Polarity::Affirmed => "Affirmed",
        Polarity::Denied => "Denied",
        Polarity::Uncertain => "Uncertain",
        Polarity::Inherit => "Inherit",
    }
}

fn modality_to_str(m: Modality) -> &'static str {
    match m {
        Modality::Present => "Present",
        Modality::Past => "Past",
        Modality::Future => "Future",
        Modality::Hypothetical => "Hypothetical",
        Modality::FamilyHistory => "FamilyHistory",
        Modality::RuledOut => "RuledOut",
        Modality::Conditional => "Conditional",
        Modality::Inherit => "Inherit",
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use llm_gateway::{
        Capabilities, CompletionJsonResponse, CompletionRequest, CompletionResponse,
        JsonSchema, LlmClient, LlmError, ProviderIdentity, TokenUsage,
    };
    use llm_primitives::Role;
    use std::sync::Mutex;

    fn extractor_identity() -> ProviderIdentity {
        ProviderIdentity {
            vendor: "mock".into(),
            model_family: "scripted".into(),
            model_version: "1".into(),
            endpoint: None,
        }
    }

    /// Returns each scripted JSON value once, in queue order.
    struct ScriptedExtractor {
        queue: Mutex<Vec<serde_json::Value>>,
    }
    impl ScriptedExtractor {
        fn new(values: Vec<serde_json::Value>) -> Self {
            Self {
                queue: Mutex::new(values.into_iter().rev().collect()),
            }
        }
    }
    impl LlmClient for ScriptedExtractor {
        fn identity(&self) -> ProviderIdentity {
            extractor_identity()
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::modern_frontier()
        }
        fn complete(&self, _r: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            unreachable!("decompose_text uses complete_json")
        }
        fn complete_json(
            &self,
            _r: CompletionRequest,
            _s: &JsonSchema,
        ) -> Result<CompletionJsonResponse, LlmError> {
            let parsed = self.queue.lock().unwrap().pop().expect("ScriptedExtractor drained");
            let raw = parsed.to_string();
            Ok(CompletionJsonResponse {
                raw_text: raw,
                parsed,
                schema_valid: true,
                model: "scripted".into(),
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cached_tokens: 0,
                },
                provider_id: extractor_identity(),
                latency_ms: 12,
                polyfill_used: false,
            })
        }
    }

    fn gateway_with(values: Vec<serde_json::Value>) -> GatewayConfig {
        GatewayConfig::new()
            .with_client(Role::Extractor, Box::new(ScriptedExtractor::new(values)))
    }

    fn clock() -> impl Fn() -> String + Copy {
        || "2026-05-13T00:00:00Z".to_string()
    }

    /// Build a JSON child node in the ADJ27 content-shaped contract:
    /// the LLM emits the literal substring it claims for the child,
    /// and the framework computes spans by matching that text
    /// against the parent text. The `_start`/`_end` arguments are
    /// retained for test-call-site readability (so each test cell
    /// can document what byte range it expects to land at) but the
    /// orchestrator never reads them.
    fn child_node(
        id: &str,
        kind: &str,
        _start: usize,
        _end: usize,
        atom_name: &str,
        text: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "kind": kind,
            "term": { "atom": atom_name },
            "polarity": "Affirmed",
            "modality": "Present",
            "text": text,
        })
    }

    fn one_node_response(node: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "document_id": "doc1",
            "nodes": [node]
        })
    }

    #[test]
    fn adj25_orchestrator_assembles_clean_hierarchy_for_single_word_source() {
        // Source: "matches" (7 bytes).
        // Scripted responses for the 4 level boundaries:
        //   Doc → Sentence: 1 Sentence covering [0, 7)
        //   Sentence → Phrase: 1 Phrase covering [0, 7) (relative)
        //   Phrase → Claim: 1 Fact covering [0, 7) (relative)
        //   Fact → TypedComponent: 1 Entity covering [0, 7) (relative)
        let gateway = gateway_with(vec![
            one_node_response(child_node("Sx", "Sentence", 0, 7, "sent", "matches")),
            one_node_response(child_node("Px", "Phrase", 0, 7, "phr", "matches")),
            one_node_response(child_node("Fx", "Fact", 0, 7, "matches", "matches")),
            one_node_response(child_node("Ex", "Entity", 0, 7, "matches", "matches")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "matches".into(),
            max_retries_per_parent: DEFAULT_MAX_RETRIES_PER_PARENT,
        };
        let out = decompose_hierarchical(&req, &gateway, clock()).unwrap();
        // Document + Sentence + Phrase + Fact + Entity = 5 nodes.
        assert_eq!(out.ir_document.nodes.len(), 5);
        // Document→Sentence + Sentence→Phrase + Phrase→Fact + Fact→Entity = 4 Contains.
        let contains_count = out
            .ir_document
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Contains)
            .count();
        assert_eq!(contains_count, 4);
        assert_eq!(out.total_llm_calls, 4);
        assert_eq!(out.retry_calls, 0);
    }

    #[test]
    fn adj27_text_matching_derives_document_absolute_spans() {
        // ADJ27: the LLM emits `text` per child; the orchestrator
        // matches each text left-to-right against the parent's
        // bytes and derives absolute spans. End-to-end test through
        // the orchestrator: source "ABCDEFGHIJ" with a Sentence
        // covering "ABCDEFGHIJ", then a Phrase covering "ABC", a
        // Phrase covering "DEFGHIJ" — and the framework places
        // them at doc [0..3) and doc [3..10).
        let gateway = gateway_with(vec![
            // Doc → Sentence: one Sentence covering the full doc
            one_node_response(child_node(
                "Sa", "Sentence", 0, 10, "sent", "ABCDEFGHIJ",
            )),
            // Sentence → Phrase: two phrases tiling the sentence
            serde_json::json!({
                "document_id": "doc1",
                "nodes": [
                    child_node("Pa", "Phrase", 0, 3, "p1", "ABC"),
                    child_node("Pb", "Phrase", 3, 10, "p2", "DEFGHIJ"),
                ]
            }),
            // Phrase Pa → Claim
            one_node_response(child_node("Fa", "Fact", 0, 3, "fa", "ABC")),
            // Phrase Pb → Claim
            one_node_response(child_node("Fb", "Fact", 0, 7, "fb", "DEFGHIJ")),
            // Fact Fa → TypedComponent
            one_node_response(child_node("Ea", "Entity", 0, 3, "ea", "ABC")),
            // Fact Fb → TypedComponent
            one_node_response(child_node("Eb", "Entity", 0, 7, "eb", "DEFGHIJ")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "ABCDEFGHIJ".into(),
            max_retries_per_parent: DEFAULT_MAX_RETRIES_PER_PARENT,
        };
        let out = decompose_hierarchical(&req, &gateway, clock()).unwrap();
        // Phrase Pa should land at doc [0..3); Phrase Pb at doc [3..10).
        let phrases: Vec<_> = out
            .ir_document
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Phrase)
            .collect();
        assert_eq!(phrases.len(), 2);
        assert_eq!(phrases[0].source_spans[0].start, 0);
        assert_eq!(phrases[0].source_spans[0].end, 3);
        assert_eq!(phrases[1].source_spans[0].start, 3);
        assert_eq!(phrases[1].source_spans[0].end, 10);
    }

    #[test]
    fn adj27_text_not_in_parent_is_skipped() {
        // LLM emits text that doesn't appear in the parent at all —
        // the framework skips the child (no spans assigned). The
        // coverage check then surfaces the bytes as uncovered.
        let gateway = gateway_with(vec![
            one_node_response(child_node(
                "Sa", "Sentence", 0, 5, "sent", "ZZZZZZZ", // not in "hello"
            )),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "hello".into(),
            max_retries_per_parent: 0,
        };
        let err = decompose_hierarchical(&req, &gateway, clock()).unwrap_err();
        // Document has no children (the fabricated text was rejected);
        // coverage check surfaces NoChildrenAtLevel.
        match err {
            HierarchicalDecomposeError::CoverageUnresolved { gaps } => {
                assert!(gaps.iter().any(|g| matches!(
                    g.kind,
                    HierarchicalGapKind::NoChildrenAtLevel
                )));
            }
            other => panic!("expected CoverageUnresolved, got {other:?}"),
        }
    }

    #[test]
    fn adj25_orchestrator_rejects_unparseable_response() {
        // First call (Doc → Sentence) returns a JSON with no `nodes`
        // key. The orchestrator should report UnparseableResponse.
        let gateway = gateway_with(vec![serde_json::json!({ "junk": true })]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "x".into(),
            max_retries_per_parent: 1,
        };
        let err = decompose_hierarchical(&req, &gateway, clock()).unwrap_err();
        match err {
            HierarchicalDecomposeError::UnparseableResponse { level, .. } => {
                assert_eq!(level, DecompLevel::DocumentToSentence);
            }
            other => panic!("expected UnparseableResponse, got {other:?}"),
        }
    }

    #[test]
    fn adj25_orchestrator_filters_kinds_not_allowed_at_level() {
        // First response (Doc → Sentence) returns a Phrase, not a
        // Sentence. The Phrase is rejected by the allowed-kinds
        // filter, leaving the Document with no children — then
        // the coverage check will report no_children_at_level.
        let gateway = gateway_with(vec![
            one_node_response(child_node("Px", "Phrase", 0, 5, "phr", "hello")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "hello".into(),
            max_retries_per_parent: 0, // no retries — fail fast
        };
        let err = decompose_hierarchical(&req, &gateway, clock()).unwrap_err();
        match err {
            HierarchicalDecomposeError::CoverageUnresolved { gaps } => {
                assert!(gaps.iter().any(|g| matches!(
                    g.kind,
                    HierarchicalGapKind::NoChildrenAtLevel
                )));
            }
            other => panic!("expected CoverageUnresolved, got {other:?}"),
        }
    }

    #[test]
    fn adj25_orchestrator_terminates_when_retry_budget_exhausted() {
        // Initial Doc→Sentence dispatch returns a Sentence covering
        // only [0..3) of a 5-byte source. That leaves bytes 3..5
        // uncovered. The orchestrator dispatches a retry — also
        // covering only [0..3). With max_retries_per_parent=1, the
        // retry budget is exhausted on the second pass.
        let gateway = gateway_with(vec![
            one_node_response(child_node("S1", "Sentence", 0, 3, "s", "hel")),
            // Phrase / Claim / TypedComp for the 0..3 sub-tree
            // so the inner levels can complete cleanly.
            one_node_response(child_node("P1", "Phrase", 0, 3, "p", "hel")),
            one_node_response(child_node("F1", "Fact", 0, 3, "f", "hel")),
            one_node_response(child_node("E1", "Entity", 0, 3, "e", "hel")),
            // Retry on Document — still only [0..3).
            one_node_response(child_node("S2", "Sentence", 0, 3, "s", "hel")),
            // Inner levels for the replacement sub-tree.
            one_node_response(child_node("P2", "Phrase", 0, 3, "p", "hel")),
            one_node_response(child_node("F2", "Fact", 0, 3, "f", "hel")),
            one_node_response(child_node("E2", "Entity", 0, 3, "e", "hel")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "hello".into(),
            max_retries_per_parent: 1,
        };
        let err = decompose_hierarchical(&req, &gateway, clock()).unwrap_err();
        assert!(matches!(
            err,
            HierarchicalDecomposeError::CoverageUnresolved { .. }
        ));
    }

    #[test]
    fn adj25_parse_kind_round_trip_covers_all_kinds() {
        // Lock the parser against the v3 + ADJ25 enum surface. A
        // new variant added to NodeKind that isn't mapped here is a
        // bug to catch at PR time, not in production.
        for (s, k) in [
            ("Fact", NodeKind::Fact),
            ("Query", NodeKind::Query),
            ("Uncertainty", NodeKind::Uncertainty),
            ("Rule", NodeKind::Rule),
            ("Exception", NodeKind::Exception),
            ("Discarded", NodeKind::Discarded),
            ("Section", NodeKind::Section),
            ("Entity", NodeKind::Entity),
            ("Document", NodeKind::Document),
            ("Sentence", NodeKind::Sentence),
            ("Phrase", NodeKind::Phrase),
            ("Question", NodeKind::Question),
            ("Quantity", NodeKind::Quantity),
            ("Polarity", NodeKind::Polarity),
            ("Predicate", NodeKind::Predicate),
            ("Comparator", NodeKind::Comparator),
            ("TimeRef", NodeKind::TimeRef),
            ("Modifier", NodeKind::Modifier),
        ] {
            assert_eq!(parse_kind(s), Some(k), "kind round-trip failed for {}", s);
        }
        assert_eq!(parse_kind("Nonexistent"), None);
    }

    #[test]
    fn adj25_orchestrator_output_is_correlation_complete() {
        // The hierarchical orchestrator must assign a CorrelationId
        // to every node it produces. Run the orchestrator end-to-end
        // and assert `check_correlation_completeness` passes on the
        // result.
        let gateway = gateway_with(vec![
            one_node_response(child_node("Sx", "Sentence", 0, 7, "sent", "matches")),
            one_node_response(child_node("Px", "Phrase", 0, 7, "phr", "matches")),
            one_node_response(child_node("Fx", "Fact", 0, 7, "matches", "matches")),
            one_node_response(child_node("Ex", "Entity", 0, 7, "matches", "matches")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "matches".into(),
            max_retries_per_parent: DEFAULT_MAX_RETRIES_PER_PARENT,
        };
        let out = decompose_hierarchical(&req, &gateway, clock()).unwrap();
        let completeness =
            adjudication_ir::check_correlation_completeness(&out.ir_document);
        assert!(
            completeness.is_ok(),
            "orchestrator output missed a correlation id: {:?}",
            completeness
        );
        // Spot-check: the Document root should derive its
        // CorrelationId from its NodeId.
        let doc = out
            .ir_document
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Document)
            .expect("Document node present");
        let corr = adjudication_ir::node_correlation_id(doc).unwrap();
        assert_eq!(corr.0, "corr.Doc");
        // Spot-check: an LLM-supplied id "Sx" → correlation
        // "corr.Sx".
        let sent = out
            .ir_document
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Sentence)
            .expect("Sentence node present");
        let sent_corr = adjudication_ir::node_correlation_id(sent).unwrap();
        assert_eq!(sent_corr.0, "corr.Sx");
    }

    #[test]
    fn adj25_orchestrator_emits_correlation_ids_on_contains_edges() {
        let gateway = gateway_with(vec![
            one_node_response(child_node("Sx", "Sentence", 0, 7, "sent", "matches")),
            one_node_response(child_node("Px", "Phrase", 0, 7, "phr", "matches")),
            one_node_response(child_node("Fx", "Fact", 0, 7, "matches", "matches")),
            one_node_response(child_node("Ex", "Entity", 0, 7, "matches", "matches")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "matches".into(),
            max_retries_per_parent: DEFAULT_MAX_RETRIES_PER_PARENT,
        };
        let out = decompose_hierarchical(&req, &gateway, clock()).unwrap();
        for edge in &out.ir_document.edges {
            if edge.relation != EdgeRelation::Contains {
                continue;
            }
            let corr = adjudication_ir::edge_correlation_id(edge);
            assert!(
                corr.is_some() && !corr.as_ref().unwrap().is_empty(),
                "Contains edge {} missing correlation id",
                edge.id.0
            );
            assert!(corr.unwrap().0.starts_with("corr.e."));
        }
    }

    // -----------------------------------------------------------------
    // ADJ28 — boolean kind schema + discard_justification
    // -----------------------------------------------------------------

    #[test]
    fn adj28_boolean_kind_schema_derives_kind() {
        // The new Claim-level schema uses is_fact/is_uncertainty/
        // is_question/is_discarded booleans. Exactly one is true.
        let gateway = gateway_with(vec![
            one_node_response(child_node("S1", "Sentence", 0, 7, "sent", "matches")),
            one_node_response(child_node("P1", "Phrase", 0, 7, "phr", "matches")),
            // Claim level: emit via boolean schema instead of `kind`.
            serde_json::json!({
                "document_id": "doc1",
                "nodes": [{
                    "id": "F1",
                    "is_fact": true,
                    "is_uncertainty": false,
                    "is_question": false,
                    "is_discarded": false,
                    "term": {"atom": "matches"},
                    "polarity": "Affirmed",
                    "modality": "Present",
                    "text": "matches",
                }]
            }),
            one_node_response(child_node("E1", "Entity", 0, 7, "matches", "matches")),
        ]);
        let req = HierarchicalDecomposeRequest {
            document_id: "doc1".into(),
            source_text: "matches".into(),
            max_retries_per_parent: 0,
        };
        let out = decompose_hierarchical(&req, &gateway, clock()).unwrap();
        // The Claim-level child should be parsed as a Fact, deriving
        // its kind from the boolean schema.
        let fact = out
            .ir_document
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Fact)
            .expect("expected a Fact-kind node from boolean schema");
        assert_eq!(fact.id.0, "F1");
    }

    #[test]
    fn adj28_zero_or_multiple_true_booleans_rejected() {
        // Levels 3 and 4 use the boolean schema; if zero or multiple
        // is_X booleans are true, the orchestrator must reject the
        // child (and the coverage check surfaces the missing bytes).
        let mut id_state = IdState::new();
        // No booleans true → not a kind. Returns None.
        let raw = serde_json::json!({
            "id": "X1",
            "is_fact": false,
            "is_uncertainty": false,
            "is_question": false,
            "is_discarded": false,
            "term": {"atom": "x"},
            "text": "x",
        });
        let allowed = &[NodeKind::Fact, NodeKind::Discarded][..];
        assert!(parse_child_node(&raw, allowed, &mut id_state, "C").is_none());
        // Two booleans true → also rejected.
        let raw2 = serde_json::json!({
            "id": "X2",
            "is_fact": true,
            "is_uncertainty": true,
            "is_question": false,
            "is_discarded": false,
            "term": {"atom": "x"},
            "text": "x",
        });
        assert!(parse_child_node(&raw2, allowed, &mut id_state, "C").is_none());
    }

    #[test]
    fn adj28_discard_justification_lands_in_metadata() {
        // When the model marks a chunk Discarded with a
        // discard_justification, the orchestrator stores that
        // justification in the node's metadata under the
        // DISCARD_JUSTIFICATION_METADATA_KEY.
        let mut id_state = IdState::new();
        let raw = serde_json::json!({
            "id": "D1",
            "kind": "Discarded",
            "discard_reason": "Pleasantry",
            "discard_justification":
                "The string `please` is a politeness marker that adds no claim content.",
            "term": {"atom": "x"},
            "polarity": "Affirmed",
            "modality": "Present",
            "text": "please",
        });
        let allowed = &[NodeKind::Fact, NodeKind::Discarded][..];
        let (node, _text) =
            parse_child_node(&raw, allowed, &mut id_state, "C").expect("parsed");
        assert_eq!(node.kind, NodeKind::Discarded);
        let stored = node.metadata.get(DISCARD_JUSTIFICATION_METADATA_KEY);
        assert_eq!(
            stored.map(|s| s.as_str()),
            Some("The string `please` is a politeness marker that adds no claim content.")
        );
    }

    #[test]
    fn adj28_discard_reason_string_parsed_into_enum() {
        // Each documented discard_reason string maps to the right
        // DiscardReason variant. Unknown strings fall back to
        // NonDomainContent rather than panicking.
        use adjudication_ir::DiscardReason::*;
        for (s, expected) in [
            ("Pleasantry", Pleasantry),
            ("DocumentMetadata", DocumentMetadata),
            ("NonDomainContent", NonDomainContent),
            ("Restatement", Restatement),
            ("Unparseable", Unparseable),
            ("AdministrativeOnly", AdministrativeOnly),
            ("ExplicitlyOutOfScope", ExplicitlyOutOfScope),
            ("not-a-real-reason", NonDomainContent),
            ("", NonDomainContent),
        ] {
            assert_eq!(parse_discard_reason(Some(s)), expected);
        }
        assert_eq!(parse_discard_reason(None), NonDomainContent);
    }
}
