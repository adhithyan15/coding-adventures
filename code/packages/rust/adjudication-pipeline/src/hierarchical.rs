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

/// Hard cap on parsed source-span count per LLM response. A
/// well-behaved response carries one span per node; this exists to
/// blunt an adversarial response that emits millions of spans.
const MAX_SPANS_PER_NODE: usize = 64;

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
            let gap_description = render_gap_description(gap);
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
    let parent_span = parent.source_spans.first().cloned();
    let id_prefix = match level {
        DecompLevel::DocumentToSentence => "S",
        DecompLevel::SentenceToPhrase => "P",
        DecompLevel::PhraseToClaim => "C",
        DecompLevel::FactToTypedComponent => "T",
    };
    let mut accepted_children: Vec<(NodeId, NodeKind)> = Vec::new();
    for raw in nodes_raw.into_iter().take(PER_LEVEL_DISPATCH_CAP) {
        let Some(node) =
            parse_child_node(&raw, &ir.document_id, parent_span.as_ref(), allowed, id_state, id_prefix)
        else {
            continue;
        };
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
    let parent_node = match find_node(ir, parent_id) {
        Some(n) => n,
        None => {
            return serde_json::json!({ "nodes": nodes });
        }
    };
    let parent_start = parent_node
        .source_spans
        .first()
        .map(|s| s.start)
        .unwrap_or(0);
    for edge in &ir.edges {
        if edge.relation != EdgeRelation::Contains || edge.source != *parent_id {
            continue;
        }
        let Some(child) = find_node(ir, &edge.target) else {
            continue;
        };
        let span_relative: Vec<serde_json::Value> = child
            .source_spans
            .iter()
            .map(|s| {
                serde_json::json!({
                    "start": s.start.saturating_sub(parent_start),
                    "end": s.end.saturating_sub(parent_start),
                })
            })
            .collect();
        nodes.push(serde_json::json!({
            "id": child.id.0,
            "kind": format!("{:?}", child.kind),
            "term": term_to_json(&child.term),
            "polarity": polarity_to_str(child.polarity),
            "modality": modality_to_str(child.modality),
            "source_spans": span_relative,
        }));
    }
    let _ = full_source;
    serde_json::json!({ "nodes": nodes })
}

fn render_gap_description(gap: &HierarchicalGap) -> String {
    match &gap.kind {
        HierarchicalGapKind::UncoveredBytes { ranges } => format!(
            "the following byte range(s) of the parent were not covered by any child: {ranges:?}"
        ),
        HierarchicalGapKind::Overlap { ranges, participants } => format!(
            "two or more children overlapped on byte range(s) {ranges:?} (participants: {ids:?})",
            ids = participants.iter().map(|n| &n.0).collect::<Vec<_>>()
        ),
        HierarchicalGapKind::EmptyChildSpan { child_id } => format!(
            "child {} was emitted with an empty span; every child must cover \
             at least one byte (synthesized Entity is the only exception)",
            child_id.0
        ),
        HierarchicalGapKind::ChildSpansEscape { child_id, outside } => format!(
            "child {} produced a span outside the parent's bounds: {outside:?}",
            child_id.0
        ),
        HierarchicalGapKind::NoChildrenAtLevel => {
            "the parent has no children — the decomposition must produce at least one"
                .to_string()
        }
        HierarchicalGapKind::WrongChildKindForLevel {
            child_id,
            child_kind,
        } => format!(
            "child {} has kind {:?}, which is not allowed at this decomposition level",
            child_id.0, child_kind
        ),
        HierarchicalGapKind::FlattenedAtom { atom, reason, .. } => format!(
            "the atom \"{atom}\" smuggles source content into a name ({reason:?}); \
             surface the underlying values as typed components instead"
        ),
    }
}

// ---------------------------------------------------------------------------
// JSON-to-IR parsing
// ---------------------------------------------------------------------------

fn parse_child_node(
    v: &serde_json::Value,
    doc_id: &DocumentId,
    parent_span: Option<&Span>,
    allowed_kinds: &[NodeKind],
    id_state: &mut IdState,
    id_prefix: &str,
) -> Option<IRNode> {
    let obj = v.as_object()?;
    let kind = parse_kind(obj.get("kind").and_then(|v| v.as_str())?)?;
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
    let source_spans = parse_spans(obj.get("source_spans"), doc_id, parent_span);
    let discard_reason = if kind == NodeKind::Discarded {
        Some(adjudication_ir::DiscardReason::NonDomainContent)
    } else {
        None
    };
    let mut node = IRNode {
        id,
        kind,
        term,
        polarity,
        modality,
        source_spans,
        confidence: 1.0,
        discard_reason,
        metadata: HashMap::new(),
    };
    // ADJ25 PR-5: assign a CorrelationId to every parsed child. The
    // ID derives from the (assigned) NodeId so the correlation tree
    // mirrors the Contains-edge hierarchy.
    let corr = correlation_id_for_node(&node.id);
    set_node_correlation_id(&mut node, corr);
    Some(node)
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

fn parse_spans(
    v: Option<&serde_json::Value>,
    doc_id: &DocumentId,
    parent_span: Option<&Span>,
) -> Vec<Span> {
    let Some(arr) = v.and_then(|v| v.as_array()) else {
        return vec![];
    };
    let mut out: Vec<Span> = Vec::new();
    let bounded = if arr.len() > MAX_SPANS_PER_NODE {
        &arr[..MAX_SPANS_PER_NODE]
    } else {
        arr.as_slice()
    };
    let parent_start = parent_span.map(|s| s.start).unwrap_or(0);
    let parent_end = parent_span
        .map(|s| s.end)
        .unwrap_or(usize::MAX);
    for s in bounded {
        let obj = match s.as_object() {
            Some(o) => o,
            None => continue,
        };
        let Some(start_v) = obj.get("start").and_then(|x| x.as_u64()) else {
            continue;
        };
        let Some(end_v) = obj.get("end").and_then(|x| x.as_u64()) else {
            continue;
        };
        let start = start_v as usize;
        let end = end_v as usize;
        if start >= end {
            continue;
        }
        // The LLM is asked to report spans relative to the parent's
        // text (offset 0 = parent's first byte). Translate to
        // document-absolute by adding the parent's start. Clamp to
        // the parent's range so a misbehaving response cannot
        // produce a span that extends past the parent.
        let abs_start = parent_start.saturating_add(start).min(parent_end);
        let abs_end = parent_start.saturating_add(end).min(parent_end);
        if abs_start >= abs_end {
            continue;
        }
        out.push(Span::new(doc_id.clone(), abs_start, abs_end));
    }
    out
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

    fn child_node(
        id: &str,
        kind: &str,
        start: usize,
        end: usize,
        atom_name: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "kind": kind,
            "term": { "atom": atom_name },
            "polarity": "Affirmed",
            "modality": "Present",
            "source_spans": [{ "start": start, "end": end }]
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
            one_node_response(child_node("Sx", "Sentence", 0, 7, "sent")),
            one_node_response(child_node("Px", "Phrase", 0, 7, "phr")),
            one_node_response(child_node("Fx", "Fact", 0, 7, "matches")),
            one_node_response(child_node("Ex", "Entity", 0, 7, "matches")),
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
    fn adj25_parse_spans_translates_to_document_absolute() {
        // Unit-level test for the parent-relative → document-absolute
        // span translation. The LLM emits spans relative to the
        // parent's text; the orchestrator translates to absolute
        // offsets by adding the parent's `span.start`.
        let doc_id = DocumentId::new("doc1");
        let parent_span = Span::new(doc_id.clone(), 4, 10); // doc [4..10)
        let llm_emitted_spans = serde_json::json!([
            { "start": 0, "end": 3 },
            { "start": 3, "end": 6 }
        ]);
        let parsed =
            parse_spans(Some(&llm_emitted_spans), &doc_id, Some(&parent_span));
        assert_eq!(parsed.len(), 2);
        // Relative 0..3 → absolute 4..7
        assert_eq!(parsed[0].start, 4);
        assert_eq!(parsed[0].end, 7);
        // Relative 3..6 → absolute 7..10
        assert_eq!(parsed[1].start, 7);
        assert_eq!(parsed[1].end, 10);
    }

    #[test]
    fn adj25_parse_spans_clamps_past_parent_end() {
        // A misbehaving response that emits a relative span
        // extending past the parent's end is clamped to the parent's
        // bounds. (The coverage check will still catch the resulting
        // gap, but at least the orchestrator doesn't pollute the IR
        // with spans escaping the parent.)
        let doc_id = DocumentId::new("doc1");
        let parent_span = Span::new(doc_id.clone(), 0, 5);
        let too_far = serde_json::json!([{ "start": 0, "end": 100 }]);
        let parsed = parse_spans(Some(&too_far), &doc_id, Some(&parent_span));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].start, 0);
        assert_eq!(parsed[0].end, 5); // clamped
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
            one_node_response(child_node("Px", "Phrase", 0, 5, "phr")),
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
            one_node_response(child_node("S1", "Sentence", 0, 3, "s")),
            // Phrase / Claim / TypedComp for the 0..3 sub-tree
            // so the inner levels can complete cleanly.
            one_node_response(child_node("P1", "Phrase", 0, 3, "p")),
            one_node_response(child_node("F1", "Fact", 0, 3, "f")),
            one_node_response(child_node("E1", "Entity", 0, 3, "e")),
            // Retry on Document — still only [0..3).
            one_node_response(child_node("S2", "Sentence", 0, 3, "s")),
            // Inner levels for the replacement sub-tree.
            one_node_response(child_node("P2", "Phrase", 0, 3, "p")),
            one_node_response(child_node("F2", "Fact", 0, 3, "f")),
            one_node_response(child_node("E2", "Entity", 0, 3, "e")),
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
            one_node_response(child_node("Sx", "Sentence", 0, 7, "sent")),
            one_node_response(child_node("Px", "Phrase", 0, 7, "phr")),
            one_node_response(child_node("Fx", "Fact", 0, 7, "matches")),
            one_node_response(child_node("Ex", "Entity", 0, 7, "matches")),
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
            one_node_response(child_node("Sx", "Sentence", 0, 7, "sent")),
            one_node_response(child_node("Px", "Phrase", 0, 7, "phr")),
            one_node_response(child_node("Fx", "Fact", 0, 7, "matches")),
            one_node_response(child_node("Ex", "Entity", 0, 7, "matches")),
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
}
