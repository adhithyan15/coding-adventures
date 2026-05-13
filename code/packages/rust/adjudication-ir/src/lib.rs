//! # adjudication-ir — the typed IR for rule-based adjudication (v3).
//!
//! Reference implementation of
//! [`ADJ01 v3`](../../../specs/ADJ01-adjudication-ir-grammar.md).
//! Defines node shapes, edge shapes, the closed-set edge-relation
//! taxonomy, the polarity / modality lattices, and a total
//! [`validate`] function that enforces every well-formedness
//! invariant before any downstream component touches the document.
//!
//! ## What v3 changes
//!
//! v2's hierarchical decomposition tree (with `part_of` and
//! `lowered_from` fields and the content-free `TextRun` grouping
//! kind) becomes a single **multi-directed acyclic graph** of typed
//! nodes and typed [`IREdge`]s. The tree was sufficient for small
//! narratives but cannot represent the relationships that emerge at
//! scale: rule citations, cross-references, table-row membership,
//! provenance chains, exception scoping, temporal supersession.
//! v3 promotes each of those to a first-class IR edge with a typed
//! [`EdgeRelation`] from a closed eleven-group taxonomy.
//!
//! Key v2 → v3 differences:
//!
//! - [`IRNode`] drops `part_of` and `lowered_from`. Both move into
//!   typed edges ([`EdgeRelation::Contains`] and
//!   [`EdgeRelation::Clarifies`] respectively).
//! - [`NodeKind`] drops `TextRun`; replaced by [`NodeKind::Section`]
//!   (a *meaningful* structural grouping that carries its type in
//!   the node's term) and [`NodeKind::Entity`] (a deduplicated
//!   reference target).
//! - New top-level [`IREdge`] alongside [`IRNode`]; the on-wire
//!   shape gains an `edges: [IREdge]` field.
//! - Coverage v3 is a *flat tile* of `(nodes ∪ edges).source_spans`
//!   against `[0, len(document))`; no recursive tree descent.
//! - A **DAG acyclicity** invariant is added across all edge
//!   relations.
//! - Propagation runs along [`EdgeRelation::Contains`] edges with a
//!   multi-parent [`ValidationError::PropagationConflict`] check.
//!
//! ## Layer Position
//!
//! ```text
//!    logic-core (LP00)            ← Term, LogicVar, Substitution, unify
//!         │
//!         ▼
//!    adjudication-ir              ← this crate (ADJ01 v3)
//!         │
//!         ├── ADJ02 coverage + DAG check
//!         ├── ADJ03 polarity/modality propagation
//!         ├── ADJ04 round-trip
//!         ├── ADJ05 adversarial
//!         ├── ADJ06 clarification
//!         └── ADJ09 / ADJ14 rule compilation + elicitation
//! ```
//!
//! ## Worked example
//!
//! See [`ADJ01 §"Worked Example (Graph Form)"`](../../../specs/ADJ01-adjudication-ir-grammar.md)
//! for the canonical TSA worked example end-to-end. The same example
//! appears in the integration tests below.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};

use logic_core::Term;

// ===========================================================================
// Identifiers
// ===========================================================================

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

/// Stable identifier for an [`IRNode`] within a document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Stable identifier for an [`IREdge`] within a document.
///
/// Convention: `"E1", "E2", ...` for human-readable edges, mirroring
/// the `"N1", "N2", ...` convention for nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeId(pub String);

impl EdgeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

// ===========================================================================
// Spans
// ===========================================================================

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
        Self { document_id, start, end }
    }

    /// `true` iff `self.start < self.end`.
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

// ===========================================================================
// Lattices: Polarity, Modality
// ===========================================================================

/// Whether the node or edge asserts, denies, or records uncertainty
/// about itself. The lattice is flat — no element subsumes another,
/// and there is no `Unknown` value. The absence of evidence is
/// represented by the absence of a node/edge (coverage enforces this).
///
/// [`Polarity::Inherit`] on a *node* with [`EdgeRelation::Contains`]
/// parents means "use the parent's effective polarity" — propagation
/// per ADJ03 v3.
///
/// [`Polarity::Inherit`] on an *edge* has no well-defined meaning and
/// is rejected by [`validate`] (see
/// [`ValidationError::InheritOnEdge`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    Affirmed,
    Denied,
    Uncertain,
    /// Defer to the propagation ancestor's effective polarity. Valid
    /// only on nodes that have at least one `Contains` parent.
    Inherit,
}

/// The temporal / hypothetical / ownership context of the term. Flat
/// lattice. Combining modalities requires multiple nodes, not a join.
///
/// `RuledOut` and `Denied` (a polarity value) are **not** synonyms.
/// See [`ADJ01 §"Modality"`](../../../specs/ADJ01-adjudication-ir-grammar.md).
///
/// [`Modality::Inherit`] follows the same propagation rule as
/// [`Polarity::Inherit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modality {
    Present,
    Past,
    Future,
    Hypothetical,
    FamilyHistory,
    RuledOut,
    Conditional,
    /// Defer to the propagation ancestor's effective modality.
    Inherit,
}

// ===========================================================================
// Node kinds, discard reasons
// ===========================================================================

/// The role a node plays in the IR.
///
/// **v3** dropped `TextRun` (replaced by [`Section`](NodeKind::Section)
/// which carries meaningful structural metadata) and added
/// [`Entity`](NodeKind::Entity) for deduplicated reference targets.
///
/// **ADJ25 (PR-1, additive)** adds the hierarchical decomposition
/// kinds required by `code/specs/ADJ25-hierarchical-decomposition.md`:
/// [`Document`](NodeKind::Document), [`Sentence`](NodeKind::Sentence),
/// [`Phrase`](NodeKind::Phrase), and [`Question`](NodeKind::Question)
/// form the level-0 → level-3 skeleton; the level-4 typed-component
/// kinds [`Quantity`](NodeKind::Quantity),
/// [`Polarity`](NodeKind::Polarity),
/// [`Predicate`](NodeKind::Predicate),
/// [`Comparator`](NodeKind::Comparator),
/// [`TimeRef`](NodeKind::TimeRef), and
/// [`Modifier`](NodeKind::Modifier) decompose a `Fact`'s content into
/// structured slots. [`Entity`](NodeKind::Entity) doubles as the
/// level-4 entity component. PR-1 is additive only — the v3 kinds
/// remain valid; PR-7 retires `Section` and the demoted kinds after
/// the foundation bench passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A claim about the world.
    Fact,
    /// A question the adjudication is asked. Engine-facing — distinct
    /// from [`Question`](NodeKind::Question), which is an interrogative
    /// present in the source text (per ADJ25 level 3).
    Query,
    /// The source explicitly raised a question without answering it.
    Uncertainty,
    /// Produced by the rule-compilation pipeline (`ADJ09`) or
    /// rulebook elicitation (`ADJ14`).
    Rule,
    /// A carve-out attached to one or more Rules via `Excepts` edges.
    Exception,
    /// An explicit span the extractor judged irrelevant.
    Discarded,
    /// A structural unit: paragraph, sentence, subsection, list item,
    /// table, row, column, cell, heading. The node's term names the
    /// kind (e.g., `paragraph(_)`, `row(3)`); its source_spans cover
    /// the *meta-text* (heading, numbering, delimiters), not the
    /// content. Content is reached via `Contains` edges.
    ///
    /// **ADJ25**: retiring in favour of the explicit level kinds
    /// [`Sentence`](NodeKind::Sentence) and
    /// [`Phrase`](NodeKind::Phrase). Deprecation lands with PR-7
    /// (cutover) once the foundation bench (PR-6) passes.
    Section,
    /// A deduplicated reference target for an atom or compound term
    /// mentioned at multiple sites. Mentioning nodes connect via
    /// `Mentions` edges. May have empty source_spans if synthesized.
    ///
    /// **ADJ25**: also serves as the level-4 entity component when
    /// it is a child of a `Fact` via `Contains` (a non-synthesized
    /// Entity with non-empty spans inside a Fact's bytes).
    Entity,

    // -----------------------------------------------------------------
    // ADJ25 — hierarchical decomposition skeleton (levels 0 → 3)
    // -----------------------------------------------------------------
    /// **ADJ25 level 0.** The root of the hierarchical decomposition.
    /// Exactly one per IR document. Its span covers `[0, N)` where
    /// `N = len(normalized_text)`. Its children are `Sentence` (and
    /// document-scope `Discarded`) nodes that tile the full range.
    Document,
    /// **ADJ25 level 1.** A natural-language sentence in the source.
    /// Decomposition granularity is model-determined subject to the
    /// per-level coverage check. Tiles part of a [`Document`]'s span;
    /// children are [`Phrase`] and (sentence-scope) [`Discarded`] nodes
    /// that tile the Sentence's span.
    Sentence,
    /// **ADJ25 level 2.** A sub-sentence chunk — a coherent unit of
    /// meaning the model commits to as *"this stretch contributes one
    /// claim (or one uncertainty / question / discardable)."* Tiles
    /// part of a [`Sentence`]'s span; children are level-3 claim
    /// nodes ([`Fact`], [`Uncertainty`], [`Question`],
    /// phrase-scope [`Discarded`]) tiling the Phrase's span.
    Phrase,
    /// **ADJ25 level 3.** An interrogative present in the source text
    /// ("Is this allowed?", "How many batteries can I bring?").
    /// Distinct from [`Query`](NodeKind::Query), which is an
    /// engine-facing posed question synthesized downstream of the
    /// source decomposition.
    Question,

    // -----------------------------------------------------------------
    // ADJ25 — level-4 typed-component slots (children of `Fact`)
    // -----------------------------------------------------------------
    /// **ADJ25 level 4.** A typed numerical literal — `Quantity(value,
    /// unit)`. Every numerical literal in the source within a
    /// [`Fact`]'s span MUST surface as a `Quantity` child of that
    /// Fact. Flattening the literal into an atom name (e.g.,
    /// `battery_50_wh`) is rejected by the no-flattening rule.
    Quantity,
    /// **ADJ25 level 4.** A typed polarity slot — `Polarity(Affirmed |
    /// Denied)`. Exists when a [`Fact`]'s span contains negation cues
    /// ("no", "not", "denies", "without"). The structural slot is
    /// gated; the *value* the model assigns is not (per
    /// `feedback_adjudication_no_interpretive_gating`).
    ///
    /// Note: the variant name shadows the lattice enum
    /// [`Polarity`](crate::Polarity); use `NodeKind::Polarity` to
    /// disambiguate.
    Polarity,
    /// **ADJ25 level 4.** The relation / verb of a [`Fact`]. Often a
    /// single source word ("carry", "deliver", "exceeds"). Decomposes
    /// the Fact's predicate from its arguments.
    Predicate,
    /// **ADJ25 level 4.** A relational operator — `Eq`, `Lt`, `Le`,
    /// `Gt`, `Ge`, `Ne`. Used in threshold conditions ("blade length
    /// > 2.36 inches", "battery capacity ≤ 100 Wh").
    Comparator,
    /// **ADJ25 level 4.** A date, duration, or temporal phrase
    /// ("by 2026-12-01", "within 30 days", "Q3 2024").
    TimeRef,
    /// **ADJ25 level 4.** An adjective or adverb refinement that
    /// doesn't fit the other level-4 slots ("strike-anywhere",
    /// "disposable", "carry-on", "promptly").
    Modifier,
}

/// Controlled vocabulary for [`NodeKind::Discarded`] nodes'
/// `discard_reason` field.
///
/// `Unparseable` is always a coverage failure: an extractor that
/// produces a `Discarded(Unparseable)` triggers ADJ06 clarification
/// rather than shipping the node.
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

// ===========================================================================
// Edge relation taxonomy (closed set + escape hatch)
// ===========================================================================

/// Closed-set typed relation between two nodes. Adding a new variant
/// is an ADJ01 version bump (v3 → v4). The
/// [`EdgeRelation::DomainSpecific`] escape hatch accommodates
/// deployments that need a relation the framework doesn't yet ship.
///
/// Grouped per ADJ01 v3 §"The Edge-Relation Taxonomy". The grouping
/// is documentation, not enforcement: the validator only checks the
/// per-relation source/target kind invariants
/// (see [`relation_endpoint_constraints`] below for the table).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeRelation {
    // --- 1. Structural ---
    Contains,
    Precedes,
    Heading,

    // --- 2. Identity ---
    Mentions,
    SameAs,
    Refers,

    // --- 3. Rule modification ---
    Excepts,
    Refines,
    Generalizes,
    Supersedes,
    Restricts,

    // --- 4. Application ---
    AppliesTo,
    AppliesWhen,
    Concludes,

    // --- 5. Provenance ---
    DerivedFrom,
    JustifiedBy,
    ElicitedFrom,

    // --- 6. Tabular ---
    RowOf,
    ColumnOf,
    HeaderOf,
    CellOf,

    // --- 7. Temporal ---
    Before,
    After,
    During,
    EffectiveAt,
    SupersededAt,

    // --- 8. Cross-source ---
    ConflictsWith,
    Confirms,
    DependsOn,

    // --- 9. Discourse ---
    Defines,
    Restates,
    Cites,

    // --- 10. Refinement (clarification lineage) ---
    Clarifies,

    // --- 11. Escape hatch ---
    /// Deployment-specific relation. The name MUST be non-empty and
    /// MUST NOT collide with the names of any closed-set relation
    /// (case-sensitive comparison). Validators record it but do not
    /// interpret it.
    DomainSpecific(String),
}

impl EdgeRelation {
    /// Stable kebab-case name for audit-trail records.
    pub fn as_str(&self) -> &str {
        match self {
            EdgeRelation::Contains => "contains",
            EdgeRelation::Precedes => "precedes",
            EdgeRelation::Heading => "heading",
            EdgeRelation::Mentions => "mentions",
            EdgeRelation::SameAs => "same-as",
            EdgeRelation::Refers => "refers",
            EdgeRelation::Excepts => "excepts",
            EdgeRelation::Refines => "refines",
            EdgeRelation::Generalizes => "generalizes",
            EdgeRelation::Supersedes => "supersedes",
            EdgeRelation::Restricts => "restricts",
            EdgeRelation::AppliesTo => "applies-to",
            EdgeRelation::AppliesWhen => "applies-when",
            EdgeRelation::Concludes => "concludes",
            EdgeRelation::DerivedFrom => "derived-from",
            EdgeRelation::JustifiedBy => "justified-by",
            EdgeRelation::ElicitedFrom => "elicited-from",
            EdgeRelation::RowOf => "row-of",
            EdgeRelation::ColumnOf => "column-of",
            EdgeRelation::HeaderOf => "header-of",
            EdgeRelation::CellOf => "cell-of",
            EdgeRelation::Before => "before",
            EdgeRelation::After => "after",
            EdgeRelation::During => "during",
            EdgeRelation::EffectiveAt => "effective-at",
            EdgeRelation::SupersededAt => "superseded-at",
            EdgeRelation::ConflictsWith => "conflicts-with",
            EdgeRelation::Confirms => "confirms",
            EdgeRelation::DependsOn => "depends-on",
            EdgeRelation::Defines => "defines",
            EdgeRelation::Restates => "restates",
            EdgeRelation::Cites => "cites",
            EdgeRelation::Clarifies => "clarifies",
            EdgeRelation::DomainSpecific(name) => name.as_str(),
        }
    }

    /// `true` iff this relation is the closed set (not `DomainSpecific`).
    pub fn is_closed_set(&self) -> bool {
        !matches!(self, EdgeRelation::DomainSpecific(_))
    }
}

// ===========================================================================
// IRNode and IREdge
// ===========================================================================

/// A single node in the IR. Every field corresponds 1:1 to a field
/// in [`ADJ01 §"IR Nodes"`](../../../specs/ADJ01-adjudication-ir-grammar.md).
///
/// Three properties are non-negotiable (enforced by [`validate`]):
///
/// 1. Every IR node is span-grounded — `source_spans` is non-empty
///    except for [`NodeKind::Query`] (synthesized queries are
///    allowed) and [`NodeKind::Entity`] (synthesized entities are
///    allowed).
/// 2. `polarity` and `modality` are always set; may be `Inherit` on a
///    node that has at least one `Contains` parent.
/// 3. [`NodeKind::Discarded`] is an explicit node citing both the
///    span being discarded and the reason ([`DiscardReason`]);
///    silently omitting a span is not a valid representation of
///    "irrelevant".
#[derive(Debug, Clone, PartialEq)]
pub struct IRNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub term: Term,
    pub polarity: Polarity,
    pub modality: Modality,
    pub source_spans: Vec<Span>,
    /// Extractor's self-reported confidence. Informational only; not
    /// used by [`validate`].
    pub confidence: f64,
    /// Required iff `kind == Discarded`; must be `None` otherwise.
    pub discard_reason: Option<DiscardReason>,
    /// Free-form extension for downstream consumers. The framework
    /// reserves any key beginning with `adj.` for future use.
    pub metadata: HashMap<String, String>,
}

/// A single directed edge in the IR.
///
/// Edges are **directed**: `source` and `target` are not symmetric.
/// Multiple edges may exist between the same pair of nodes provided
/// they have different [`EdgeRelation`] values (hence
/// *multi-directed*).
///
/// Edge `source_spans` cover the **textual marker** that signals the
/// relation — "except", "see §5", ",", "and" — not the spans of the
/// related nodes themselves. A synthesized edge (engine-emitted, not
/// extracted from text) carries empty `source_spans`; it is recorded
/// in the audit trail but does not tile any source bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct IREdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub relation: EdgeRelation,
    /// Edge polarity. `Inherit` is *not* allowed (rejected by
    /// [`validate`]); use `Affirmed` for the default.
    pub polarity: Polarity,
    /// Edge modality. `Inherit` is *not* allowed.
    pub modality: Modality,
    pub source_spans: Vec<Span>,
    pub confidence: f64,
    pub metadata: HashMap<String, String>,
}

// ===========================================================================
// ADJ25 — Correlation IDs (PR-5)
// ===========================================================================
//
// ADJ25 requires every IR object to carry a `CorrelationId` that flows
// source byte → IR node → engine clause → verdict citation. PR-5 lands
// the type, the metadata-key contract, and the helpers; downstream
// crates (audit trail, connector) read the helpers to propagate IDs
// into engine clauses and audit-trail records.
//
// Why metadata-keyed rather than a first-class field on `IRNode`:
// adding a required field to `IRNode` is a SemVer-breaking change
// that ripples through 400+ struct-literal construction sites in the
// workspace, most of them tests. The `metadata: HashMap<String,
// String>` field was designed for exactly this kind of additive
// attribute. Future PRs (PR-7 cutover, or beyond) may promote to a
// dedicated struct field once the workspace is ready for the sweep.
//
// The framework's `metadata.adj.*` namespace is reserved; PR-5
// reserves the key [`CORRELATION_ID_METADATA_KEY`] within that space.

/// A correlation identifier — a stable, document-scoped string that
/// ties every IR object and every downstream artifact derived from
/// it back to a single source-span anchor.
///
/// Per the [ADJ25 spec](../../../specs/ADJ25-hierarchical-decomposition.md)
/// §"Correlation vector", granularity is **per source span**, not
/// per byte: every node (and every downstream artifact) carries one
/// `CorrelationId`. Byte-level provenance is derivable from the
/// `(correlation_id, span)` pair by recursion through `Contains`
/// edges.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    /// Construct a [`CorrelationId`] from any string-convertible
    /// value. The framework treats the inner string as opaque; the
    /// orchestrator that assigns IDs picks whatever scheme it likes
    /// (deterministic from `NodeId`, UUIDv4, hash of the span,
    /// etc.).
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// True iff the identifier is empty. Empty correlation IDs are
    /// rejected by [`check_correlation_completeness`] — every node
    /// MUST carry a non-empty ID.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The metadata key under which a [`CorrelationId`] is stored on
/// `IRNode::metadata` and `IREdge::metadata`. Reserved by the
/// framework under the `adj.*` namespace per the IRNode metadata
/// contract.
pub const CORRELATION_ID_METADATA_KEY: &str = "adj.correlation_id";

/// Read a node's `CorrelationId` from its metadata. Returns `None`
/// when the metadata key is absent (the orchestrator has not yet
/// assigned an ID, or this is an un-correlated legacy node).
pub fn node_correlation_id(node: &IRNode) -> Option<CorrelationId> {
    node.metadata
        .get(CORRELATION_ID_METADATA_KEY)
        .map(|s| CorrelationId(s.clone()))
}

/// Read an edge's `CorrelationId`. Same metadata contract as
/// [`node_correlation_id`].
pub fn edge_correlation_id(edge: &IREdge) -> Option<CorrelationId> {
    edge.metadata
        .get(CORRELATION_ID_METADATA_KEY)
        .map(|s| CorrelationId(s.clone()))
}

/// Write a `CorrelationId` onto a node's metadata. Idempotent —
/// overwrites any prior value at the same key.
pub fn set_node_correlation_id(node: &mut IRNode, id: CorrelationId) {
    node.metadata
        .insert(CORRELATION_ID_METADATA_KEY.to_string(), id.0);
}

/// Write a `CorrelationId` onto an edge's metadata.
pub fn set_edge_correlation_id(edge: &mut IREdge, id: CorrelationId) {
    edge.metadata
        .insert(CORRELATION_ID_METADATA_KEY.to_string(), id.0);
}

/// Errors returned by [`check_correlation_completeness`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrelationCompletenessError {
    /// A node lacks the `adj.correlation_id` metadata entry. PR-5
    /// makes correlation a hard requirement on the hierarchical
    /// orchestrator's output; pre-hierarchical IRs are exempt and
    /// callers can choose whether to invoke this check.
    NodeMissingCorrelation { node_id: NodeId },
    /// A node has the metadata entry but its value is empty.
    NodeEmptyCorrelation { node_id: NodeId },
}

impl std::fmt::Display for CorrelationCompletenessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeMissingCorrelation { node_id } => {
                write!(
                    f,
                    "node {} is missing the `{}` metadata key",
                    node_id.0, CORRELATION_ID_METADATA_KEY
                )
            }
            Self::NodeEmptyCorrelation { node_id } => write!(
                f,
                "node {} has an empty correlation id at `{}`",
                node_id.0, CORRELATION_ID_METADATA_KEY
            ),
        }
    }
}

impl std::error::Error for CorrelationCompletenessError {}

/// Verify that every node in the IR carries a non-empty
/// `CorrelationId`. Returns the first violation encountered (or
/// `Ok(())` when the IR is fully correlated).
///
/// Per the ADJ25 spec, this check is intended for the output of the
/// hierarchical orchestrator (every node produced by
/// `decompose_hierarchical` MUST be correlated). Callers handling
/// legacy / pre-hierarchical IR can skip the check.
pub fn check_correlation_completeness(
    doc: &IRDocument,
) -> Result<(), CorrelationCompletenessError> {
    for node in &doc.nodes {
        match node_correlation_id(node) {
            None => {
                return Err(CorrelationCompletenessError::NodeMissingCorrelation {
                    node_id: node.id.clone(),
                });
            }
            Some(id) if id.is_empty() => {
                return Err(CorrelationCompletenessError::NodeEmptyCorrelation {
                    node_id: node.id.clone(),
                });
            }
            Some(_) => {}
        }
    }
    Ok(())
}

// ===========================================================================
// IRDocument
// ===========================================================================

/// An IR document is a container of nodes *and* edges belonging to
/// one input document. The `document_id` matches the input's
/// identifier; nodes' and edges' `source_spans` reference offsets
/// into that document's normalized text.
///
/// Two top-level collections, populated independently. The graph
/// structure emerges from edges referring to nodes by id; nodes do
/// not know which edges connect them. (Callers that need an
/// adjacency view can build one with [`adjacency_in`] /
/// [`adjacency_out`].)
#[derive(Debug, Clone, PartialEq)]
pub struct IRDocument {
    pub document_id: DocumentId,
    pub nodes: Vec<IRNode>,
    pub edges: Vec<IREdge>,
}

impl IRDocument {
    /// New empty document.
    pub fn new(document_id: DocumentId) -> Self {
        Self {
            document_id,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Look up a node by id. `None` if no node has this id.
    pub fn node(&self, id: &NodeId) -> Option<&IRNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    /// Look up an edge by id.
    pub fn edge(&self, id: &EdgeId) -> Option<&IREdge> {
        self.edges.iter().find(|e| &e.id == id)
    }

    /// Outgoing edges from a node. Returns an empty iterator if the
    /// node has no outgoing edges (or doesn't exist).
    pub fn adjacency_out<'a>(&'a self, node_id: &'a NodeId) -> impl Iterator<Item = &'a IREdge> {
        self.edges.iter().filter(move |e| &e.source == node_id)
    }

    /// Incoming edges to a node.
    pub fn adjacency_in<'a>(&'a self, node_id: &'a NodeId) -> impl Iterator<Item = &'a IREdge> {
        self.edges.iter().filter(move |e| &e.target == node_id)
    }
}

// ===========================================================================
// Validation: ValidationError
// ===========================================================================

/// Every reason an IR document can fail well-formedness.
///
/// Each variant corresponds to a numbered rule from
/// [`ADJ01 §"Well-Formedness Summary"`](../../../specs/ADJ01-adjudication-ir-grammar.md).
/// Returning a specific variant rather than a generic error lets
/// callers (especially ADJ06 clarification) surface precise feedback
/// to the extractor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    // ----- Node-shape violations -----

    /// A node's `source_spans` was empty when the kind requires at
    /// least one span.
    MissingSourceSpans {
        node_id: NodeId,
        kind: NodeKind,
    },

    /// A span had `start >= end` (degenerate range).
    InvalidSpan {
        location: SpanLocation,
        start: usize,
        end: usize,
    },

    /// A span cites a document other than the IRDocument's
    /// `document_id`. Cross-document references are out of scope.
    SpanDocumentMismatch {
        location: SpanLocation,
        expected: DocumentId,
        found: DocumentId,
    },

    /// `kind = Fact`, but `polarity` is `Uncertain`. Facts cannot be
    /// uncertain by construction.
    FactWithUncertainPolarity { node_id: NodeId },

    /// `kind = Uncertainty`, but `polarity` is not `Uncertain`.
    UncertaintyWithDefinitePolarity {
        node_id: NodeId,
        polarity: Polarity,
    },

    /// `kind = Query`, but `polarity` is not `Affirmed` (or
    /// `Inherit`).
    QueryWithNonAffirmedPolarity {
        node_id: NodeId,
        polarity: Polarity,
    },

    /// `kind = Exception`, but `polarity` is not `Affirmed` (or
    /// `Inherit`).
    ExceptionWithNonAffirmedPolarity {
        node_id: NodeId,
        polarity: Polarity,
    },

    /// `kind = Discarded`, but `discard_reason` is absent.
    DiscardedWithoutReason { node_id: NodeId },

    /// `kind != Discarded`, but `discard_reason` is set. Only
    /// Discarded nodes carry a discard reason.
    NonDiscardedWithReason {
        node_id: NodeId,
        kind: NodeKind,
    },

    /// Two nodes share the same `NodeId`.
    DuplicateNodeId { id: NodeId },

    /// Two edges share the same `EdgeId`.
    DuplicateEdgeId { id: EdgeId },

    // ----- Edge well-formedness -----

    /// An edge's `source` references a node that doesn't exist.
    DanglingEdgeSource {
        edge_id: EdgeId,
        missing: NodeId,
    },

    /// An edge's `target` references a node that doesn't exist.
    DanglingEdgeTarget {
        edge_id: EdgeId,
        missing: NodeId,
    },

    /// An edge connects a node to itself. v3 forbids self-loops; the
    /// DAG check catches them but this variant gives a sharper
    /// diagnostic when the cycle is length-1.
    SelfLoopEdge { edge_id: EdgeId, node: NodeId },

    /// An edge carries `Polarity::Inherit` or `Modality::Inherit` —
    /// neither is valid on edges (only on nodes that have at least
    /// one `Contains` parent).
    InheritOnEdge {
        edge_id: EdgeId,
        offending: InheritField,
    },

    /// An [`EdgeRelation::DomainSpecific`] variant has an empty name
    /// or a name that collides with a closed-set relation.
    InvalidDomainSpecificName {
        edge_id: EdgeId,
        name: String,
    },

    /// The relation's source-kind constraint is violated. See
    /// [`relation_endpoint_constraints`] for the full table.
    InvalidRelationSourceKind {
        edge_id: EdgeId,
        relation: EdgeRelation,
        source_kind: NodeKind,
    },

    /// The relation's target-kind constraint is violated.
    InvalidRelationTargetKind {
        edge_id: EdgeId,
        relation: EdgeRelation,
        target_kind: NodeKind,
    },

    // ----- Graph-level violations -----

    /// The graph (nodes + edges) contains a cycle.
    GraphCycle { participants: Vec<NodeId> },

    /// A `Clarifies` edge connects nodes whose kinds are not
    /// compatible under the refinement-kind compatibility table.
    IncompatibleClarification {
        edge_id: EdgeId,
        child_kind: NodeKind,
        parent_kind: NodeKind,
    },

    /// An `Exception` node has no outgoing `Excepts` edge.
    UnattachedException { node_id: NodeId },

    // ----- Coverage -----

    /// Some byte of the document is not in any node's or edge's
    /// `source_spans`. Lists the uncovered ranges.
    CoverageGap { missing_ranges: Vec<(usize, usize)> },

    /// Some byte appears in more than one source_span (across nodes
    /// and edges, after exempting synthesized objects). Lists the
    /// overlapping byte ranges and the participants.
    CoverageOverlap {
        ranges: Vec<(usize, usize)>,
        participants: Vec<NodeOrEdgeId>,
    },

    // ----- Propagation -----

    /// A node has `polarity = Inherit` but no `Contains` parent to
    /// inherit from.
    InheritWithoutParent {
        node_id: NodeId,
        field: InheritField,
    },

    /// A node with `polarity = Inherit` has multiple `Contains`
    /// parents whose effective polarities disagree.
    PropagationConflict {
        node_id: NodeId,
        field: InheritField,
        candidates: Vec<(NodeId, &'static str)>,
    },
}

/// Where a span lives — on a node or an edge. Used by
/// [`ValidationError`] to point at the offending entity in
/// span-related variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanLocation {
    Node(NodeId),
    Edge(EdgeId),
}

/// Which of polarity / modality triggered an `Inherit`-related
/// violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InheritField {
    Polarity,
    Modality,
}

/// Identifier discriminator for `CoverageOverlap` participants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeOrEdgeId {
    Node(NodeId),
    Edge(EdgeId),
}

// ===========================================================================
// Validation: top-level entry point
// ===========================================================================

/// Validate an IR document.
///
/// Returns `Ok(())` iff every rule in
/// [`ADJ01 §"Well-Formedness Summary"`](../../../specs/ADJ01-adjudication-ir-grammar.md)
/// is satisfied; otherwise returns the first violation found.
///
/// Validation is **total** — no partial well-formedness. A caller
/// that receives `Err(_)` must not pass the document to any
/// downstream component.
///
/// Order of checks (each early-returns on failure):
///
/// 1. Duplicate node ids.
/// 2. Duplicate edge ids.
/// 3. Per-node structural rules.
/// 4. Per-edge structural rules (incl. endpoint existence,
///    `Inherit` rejection, DomainSpecific name validation,
///    relation-kind constraints).
/// 5. `Clarifies`-edge kind compatibility.
/// 6. Exception attachment (every Exception has an outgoing
///    `Excepts` edge).
/// 7. Graph acyclicity (DFS, all edges treated uniformly).
/// 8. Coverage tiling (flat, across nodes and edges).
/// 9. Propagation consistency (multi-parent agreement on
///    `Contains`-inherited polarity / modality).
pub fn validate(doc: &IRDocument) -> Result<(), ValidationError> {
    check_duplicate_node_ids(doc)?;
    check_duplicate_edge_ids(doc)?;

    for n in &doc.nodes {
        validate_per_node(n, doc)?;
    }
    for e in &doc.edges {
        validate_per_edge(e, doc)?;
    }

    check_clarifies_compatibility(doc)?;
    check_exception_attachment(doc)?;

    check_acyclicity(doc)?;

    check_coverage(doc)?;

    check_propagation(doc)?;

    Ok(())
}

// ===========================================================================
// Validation: id uniqueness
// ===========================================================================

fn check_duplicate_node_ids(doc: &IRDocument) -> Result<(), ValidationError> {
    let mut seen: HashSet<&NodeId> = HashSet::new();
    for n in &doc.nodes {
        if !seen.insert(&n.id) {
            return Err(ValidationError::DuplicateNodeId { id: n.id.clone() });
        }
    }
    Ok(())
}

fn check_duplicate_edge_ids(doc: &IRDocument) -> Result<(), ValidationError> {
    let mut seen: HashSet<&EdgeId> = HashSet::new();
    for e in &doc.edges {
        if !seen.insert(&e.id) {
            return Err(ValidationError::DuplicateEdgeId { id: e.id.clone() });
        }
    }
    Ok(())
}

// ===========================================================================
// Validation: per-node rules
// ===========================================================================

fn validate_per_node(n: &IRNode, doc: &IRDocument) -> Result<(), ValidationError> {
    // Span basics. Query and Entity may have empty source_spans
    // (synthesized objects); everything else must cite at least one
    // span and every span must be valid + cite this document.
    let synthesizable = matches!(n.kind, NodeKind::Query | NodeKind::Entity);
    if !synthesizable && n.source_spans.is_empty() {
        return Err(ValidationError::MissingSourceSpans {
            node_id: n.id.clone(),
            kind: n.kind,
        });
    }
    for span in &n.source_spans {
        if !span.is_valid() {
            return Err(ValidationError::InvalidSpan {
                location: SpanLocation::Node(n.id.clone()),
                start: span.start,
                end: span.end,
            });
        }
        if span.document_id != doc.document_id {
            return Err(ValidationError::SpanDocumentMismatch {
                location: SpanLocation::Node(n.id.clone()),
                expected: doc.document_id.clone(),
                found: span.document_id.clone(),
            });
        }
    }

    // discard_reason iff Discarded.
    match (n.kind, n.discard_reason.is_some()) {
        (NodeKind::Discarded, false) => {
            return Err(ValidationError::DiscardedWithoutReason {
                node_id: n.id.clone(),
            });
        }
        (k, true) if k != NodeKind::Discarded => {
            return Err(ValidationError::NonDiscardedWithReason {
                node_id: n.id.clone(),
                kind: k,
            });
        }
        _ => {}
    }

    // Kind-specific polarity rules. `Inherit` is admitted as a
    // "delegate to parent" sentinel; propagation will check it has a
    // parent and the agreement holds.
    match n.kind {
        NodeKind::Fact | NodeKind::Rule => {
            if n.polarity == Polarity::Uncertain {
                return Err(ValidationError::FactWithUncertainPolarity {
                    node_id: n.id.clone(),
                });
            }
        }
        NodeKind::Query => {
            if !matches!(n.polarity, Polarity::Affirmed | Polarity::Inherit) {
                return Err(ValidationError::QueryWithNonAffirmedPolarity {
                    node_id: n.id.clone(),
                    polarity: n.polarity,
                });
            }
        }
        NodeKind::Exception => {
            if !matches!(n.polarity, Polarity::Affirmed | Polarity::Inherit) {
                return Err(ValidationError::ExceptionWithNonAffirmedPolarity {
                    node_id: n.id.clone(),
                    polarity: n.polarity,
                });
            }
        }
        NodeKind::Uncertainty => {
            if !matches!(n.polarity, Polarity::Uncertain | Polarity::Inherit) {
                return Err(ValidationError::UncertaintyWithDefinitePolarity {
                    node_id: n.id.clone(),
                    polarity: n.polarity,
                });
            }
        }
        NodeKind::Section
        | NodeKind::Entity
        | NodeKind::Discarded
        // ADJ25 PR-1: hierarchical-decomposition skeleton kinds. These
        // are structural slots — no polarity constraint at the kind
        // level. Per-level coverage rules (PR-2) will enforce structural
        // invariants. The model-assigned polarity *value* lives in a
        // dedicated `Polarity` child node, not on these structural
        // ancestors.
        | NodeKind::Document
        | NodeKind::Sentence
        | NodeKind::Phrase
        | NodeKind::Question
        | NodeKind::Quantity
        | NodeKind::Polarity
        | NodeKind::Predicate
        | NodeKind::Comparator
        | NodeKind::TimeRef
        | NodeKind::Modifier => {
            // No polarity constraint beyond "set to something legal";
            // the lattice itself is the legal set.
        }
    }

    Ok(())
}

// ===========================================================================
// Validation: per-edge rules
// ===========================================================================

fn validate_per_edge(e: &IREdge, doc: &IRDocument) -> Result<(), ValidationError> {
    // Endpoint existence.
    let by_id: HashMap<&NodeId, &IRNode> = doc.nodes.iter().map(|n| (&n.id, n)).collect();
    let Some(source) = by_id.get(&e.source) else {
        return Err(ValidationError::DanglingEdgeSource {
            edge_id: e.id.clone(),
            missing: e.source.clone(),
        });
    };
    let Some(target) = by_id.get(&e.target) else {
        return Err(ValidationError::DanglingEdgeTarget {
            edge_id: e.id.clone(),
            missing: e.target.clone(),
        });
    };

    // Self-loops are always wrong (v3 forbids cycles uniformly; a
    // self-loop is a length-1 cycle).
    if e.source == e.target {
        return Err(ValidationError::SelfLoopEdge {
            edge_id: e.id.clone(),
            node: e.source.clone(),
        });
    }

    // `Inherit` rejection on edges.
    if e.polarity == Polarity::Inherit {
        return Err(ValidationError::InheritOnEdge {
            edge_id: e.id.clone(),
            offending: InheritField::Polarity,
        });
    }
    if e.modality == Modality::Inherit {
        return Err(ValidationError::InheritOnEdge {
            edge_id: e.id.clone(),
            offending: InheritField::Modality,
        });
    }

    // DomainSpecific name validation.
    if let EdgeRelation::DomainSpecific(name) = &e.relation {
        if name.is_empty() || closed_set_names().any(|known| known == name) {
            return Err(ValidationError::InvalidDomainSpecificName {
                edge_id: e.id.clone(),
                name: name.clone(),
            });
        }
    }

    // Span validity (edges have spans too; empty is allowed for
    // synthesized edges).
    for span in &e.source_spans {
        if !span.is_valid() {
            return Err(ValidationError::InvalidSpan {
                location: SpanLocation::Edge(e.id.clone()),
                start: span.start,
                end: span.end,
            });
        }
        if span.document_id != doc.document_id {
            return Err(ValidationError::SpanDocumentMismatch {
                location: SpanLocation::Edge(e.id.clone()),
                expected: doc.document_id.clone(),
                found: span.document_id.clone(),
            });
        }
    }

    // Relation endpoint-kind constraints.
    let (allow_source, allow_target) = relation_endpoint_constraints(&e.relation);
    if !allow_source(source.kind) {
        return Err(ValidationError::InvalidRelationSourceKind {
            edge_id: e.id.clone(),
            relation: e.relation.clone(),
            source_kind: source.kind,
        });
    }
    if !allow_target(target.kind) {
        return Err(ValidationError::InvalidRelationTargetKind {
            edge_id: e.id.clone(),
            relation: e.relation.clone(),
            target_kind: target.kind,
        });
    }

    Ok(())
}

/// Names of all closed-set relations as kebab-case strings. Used to
/// validate that a `DomainSpecific(name)` doesn't collide with a
/// known relation.
fn closed_set_names() -> impl Iterator<Item = &'static str> {
    const NAMES: &[&str] = &[
        "contains",
        "precedes",
        "heading",
        "mentions",
        "same-as",
        "refers",
        "excepts",
        "refines",
        "generalizes",
        "supersedes",
        "restricts",
        "applies-to",
        "applies-when",
        "concludes",
        "derived-from",
        "justified-by",
        "elicited-from",
        "row-of",
        "column-of",
        "header-of",
        "cell-of",
        "before",
        "after",
        "during",
        "effective-at",
        "superseded-at",
        "conflicts-with",
        "confirms",
        "depends-on",
        "defines",
        "restates",
        "cites",
        "clarifies",
    ];
    NAMES.iter().copied()
}

/// Per-relation source/target kind constraints, per
/// [`ADJ01 §"Relation-Specific Invariants"`](../../../specs/ADJ01-adjudication-ir-grammar.md).
/// Returns a pair of predicates `(source_ok, target_ok)`.
fn relation_endpoint_constraints(
    rel: &EdgeRelation,
) -> (fn(NodeKind) -> bool, fn(NodeKind) -> bool) {
    fn any(_: NodeKind) -> bool {
        true
    }
    fn section_only(k: NodeKind) -> bool {
        matches!(k, NodeKind::Section)
    }
    fn rule_only(k: NodeKind) -> bool {
        matches!(k, NodeKind::Rule)
    }
    fn exception_only(k: NodeKind) -> bool {
        matches!(k, NodeKind::Exception)
    }
    fn entity_only(k: NodeKind) -> bool {
        matches!(k, NodeKind::Entity)
    }
    fn rule_or_entity(k: NodeKind) -> bool {
        matches!(k, NodeKind::Rule | NodeKind::Entity)
    }
    fn entity_or_fact(k: NodeKind) -> bool {
        matches!(k, NodeKind::Entity | NodeKind::Fact)
    }
    fn fact_or_entity(k: NodeKind) -> bool {
        matches!(k, NodeKind::Fact | NodeKind::Entity)
    }
    fn fact_only(k: NodeKind) -> bool {
        matches!(k, NodeKind::Fact)
    }
    fn rule_or_fact(k: NodeKind) -> bool {
        matches!(k, NodeKind::Rule | NodeKind::Fact)
    }
    fn fact_or_query(k: NodeKind) -> bool {
        matches!(k, NodeKind::Fact | NodeKind::Query)
    }
    fn non_entity(k: NodeKind) -> bool {
        !matches!(k, NodeKind::Entity)
    }

    match rel {
        EdgeRelation::Contains => (section_only, any),
        EdgeRelation::Precedes => (section_only, section_only),
        EdgeRelation::Heading => (section_only, section_only),

        EdgeRelation::Mentions => (non_entity, entity_only),
        EdgeRelation::SameAs => (entity_only, entity_only),
        EdgeRelation::Refers => (any, any),

        EdgeRelation::Excepts => (exception_only, rule_only),
        EdgeRelation::Refines => (rule_only, rule_only),
        EdgeRelation::Generalizes => (rule_only, rule_only),
        EdgeRelation::Supersedes => (rule_only, rule_only),
        EdgeRelation::Restricts => (rule_only, rule_only),

        EdgeRelation::AppliesTo => (rule_only, entity_or_fact),
        EdgeRelation::AppliesWhen => (rule_only, any),
        EdgeRelation::Concludes => (rule_only, fact_or_entity),

        EdgeRelation::DerivedFrom => (fact_only, fact_only),
        EdgeRelation::JustifiedBy => (fact_or_query, rule_or_fact),
        EdgeRelation::ElicitedFrom => (rule_only, entity_only),

        EdgeRelation::RowOf => (any, section_only),
        EdgeRelation::ColumnOf => (any, section_only),
        EdgeRelation::HeaderOf => (section_only, section_only),
        EdgeRelation::CellOf => (any, section_only),

        EdgeRelation::Before => (any, any),
        EdgeRelation::After => (any, any),
        EdgeRelation::During => (any, any),
        EdgeRelation::EffectiveAt => (rule_only, entity_only),
        EdgeRelation::SupersededAt => (rule_only, entity_only),

        EdgeRelation::ConflictsWith => (rule_only, rule_only),
        EdgeRelation::Confirms => (rule_only, rule_only),
        EdgeRelation::DependsOn => (rule_or_entity, rule_or_entity),

        EdgeRelation::Defines => (any, entity_only),
        EdgeRelation::Restates => (any, any),
        EdgeRelation::Cites => (any, entity_only),

        EdgeRelation::Clarifies => (any, any),

        EdgeRelation::DomainSpecific(_) => (any, any),
    }
}

// ===========================================================================
// Validation: Clarifies kind compatibility
// ===========================================================================

fn check_clarifies_compatibility(doc: &IRDocument) -> Result<(), ValidationError> {
    let by_id: HashMap<&NodeId, &IRNode> = doc.nodes.iter().map(|n| (&n.id, n)).collect();
    for e in &doc.edges {
        if e.relation != EdgeRelation::Clarifies {
            continue;
        }
        let child = by_id[&e.source];
        let parent = by_id[&e.target];
        // Per ADJ01 v3, child ←Clarifies← parent allowed pairs:
        //   (parent=Fact,        child=Fact)
        //   (parent=Uncertainty, child=Fact)
        //   (parent=Uncertainty, child=Uncertainty)
        //   (parent=Query,       child=Query)
        //   (parent=Rule,        child=Rule)
        // Note: source = child (clarified), target = parent (original).
        let compat = matches!(
            (parent.kind, child.kind),
            (NodeKind::Fact, NodeKind::Fact)
                | (NodeKind::Uncertainty, NodeKind::Fact)
                | (NodeKind::Uncertainty, NodeKind::Uncertainty)
                | (NodeKind::Query, NodeKind::Query)
                | (NodeKind::Rule, NodeKind::Rule)
        );
        if !compat {
            return Err(ValidationError::IncompatibleClarification {
                edge_id: e.id.clone(),
                child_kind: child.kind,
                parent_kind: parent.kind,
            });
        }
    }
    Ok(())
}

// ===========================================================================
// Validation: Exception attachment
// ===========================================================================

fn check_exception_attachment(doc: &IRDocument) -> Result<(), ValidationError> {
    for n in &doc.nodes {
        if n.kind != NodeKind::Exception {
            continue;
        }
        let has_excepts = doc
            .edges
            .iter()
            .any(|e| e.source == n.id && e.relation == EdgeRelation::Excepts);
        if !has_excepts {
            return Err(ValidationError::UnattachedException {
                node_id: n.id.clone(),
            });
        }
    }
    Ok(())
}

// ===========================================================================
// Validation: graph acyclicity
// ===========================================================================

fn check_acyclicity(doc: &IRDocument) -> Result<(), ValidationError> {
    // Adjacency: source -> [target ...]. Uses NodeId for keys.
    let mut adj: HashMap<&NodeId, Vec<&NodeId>> = HashMap::new();
    for n in &doc.nodes {
        adj.insert(&n.id, Vec::new());
    }
    for e in &doc.edges {
        if let Some(targets) = adj.get_mut(&e.source) {
            targets.push(&e.target);
        }
    }

    // Iterative DFS with three colors. Captures the participating
    // cycle when one is found.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }
    let mut color: HashMap<&NodeId, Color> = doc
        .nodes
        .iter()
        .map(|n| (&n.id, Color::White))
        .collect();
    let mut path: Vec<&NodeId> = Vec::new();
    let mut on_path: HashSet<&NodeId> = HashSet::new();

    for start in doc.nodes.iter().map(|n| &n.id) {
        if color[start] != Color::White {
            continue;
        }
        // Iterative DFS to avoid blowing the stack on long chains.
        // Frame = (node, index into its adj list).
        let mut stack: Vec<(&NodeId, usize)> = vec![(start, 0)];
        path.push(start);
        on_path.insert(start);
        color.insert(start, Color::Gray);

        while let Some(&(node, idx)) = stack.last() {
            let neighbors = adj.get(node).map(|v| v.as_slice()).unwrap_or(&[]);
            if idx < neighbors.len() {
                let next = neighbors[idx];
                // Advance the index on this frame.
                stack.last_mut().unwrap().1 += 1;
                match color[next] {
                    Color::White => {
                        color.insert(next, Color::Gray);
                        path.push(next);
                        on_path.insert(next);
                        stack.push((next, 0));
                    }
                    Color::Gray => {
                        // Cycle. Reconstruct it: from `next` to end of
                        // `path` is the cycle.
                        let start_idx = path.iter().position(|p| *p == next).unwrap();
                        let participants: Vec<NodeId> =
                            path[start_idx..].iter().map(|&n| n.clone()).collect();
                        return Err(ValidationError::GraphCycle { participants });
                    }
                    Color::Black => {
                        // Already finalized; skip.
                    }
                }
            } else {
                // Done with this node.
                color.insert(node, Color::Black);
                let popped = path.pop().unwrap();
                on_path.remove(popped);
                stack.pop();
            }
        }
    }

    Ok(())
}

// ===========================================================================
// Validation: coverage (flat tiling)
// ===========================================================================

fn check_coverage(doc: &IRDocument) -> Result<(), ValidationError> {
    // Gather every span that participates in tiling. Synthesized
    // objects (Query nodes, Entity nodes, edges with empty spans) are
    // exempt.
    let mut spans: Vec<((usize, usize), NodeOrEdgeId)> = Vec::new();
    for n in &doc.nodes {
        let synthesizable = matches!(n.kind, NodeKind::Query | NodeKind::Entity);
        if synthesizable && n.source_spans.is_empty() {
            continue;
        }
        for s in &n.source_spans {
            spans.push(((s.start, s.end), NodeOrEdgeId::Node(n.id.clone())));
        }
    }
    for e in &doc.edges {
        for s in &e.source_spans {
            spans.push(((s.start, s.end), NodeOrEdgeId::Edge(e.id.clone())));
        }
    }

    // If there's nothing to tile (empty document or all synthesized),
    // we're done.
    if spans.is_empty() {
        return Ok(());
    }

    // Determine the document's byte span as the smallest interval
    // covering all participants. (We don't have a stored
    // `document_length` field; this approximates "0 to max(end)" and
    // requires min start = 0. A future API may take an explicit
    // length.)
    let min_start = spans.iter().map(|((s, _), _)| *s).min().unwrap();
    let max_end = spans.iter().map(|((_, e), _)| *e).max().unwrap();

    if min_start > 0 {
        // The document doesn't start at byte 0 — that's a gap of
        // [0, min_start).
        return Err(ValidationError::CoverageGap {
            missing_ranges: vec![(0, min_start)],
        });
    }

    // Sort spans by start to detect overlaps and gaps.
    spans.sort_by_key(|((s, _), _)| *s);

    let mut prev_end: usize = 0;
    let mut overlap_ranges: Vec<(usize, usize)> = Vec::new();
    let mut overlap_participants: Vec<NodeOrEdgeId> = Vec::new();
    let mut last_participant: Option<NodeOrEdgeId> = None;
    for ((start, end), owner) in &spans {
        if *start > prev_end {
            // Gap from prev_end to start (within the document range).
            return Err(ValidationError::CoverageGap {
                missing_ranges: vec![(prev_end, *start)],
            });
        }
        if *start < prev_end {
            // Overlap. Record the overlap range and the two
            // participants.
            overlap_ranges.push((*start, prev_end.min(*end)));
            if let Some(prev) = &last_participant {
                if !overlap_participants.contains(prev) {
                    overlap_participants.push(prev.clone());
                }
            }
            if !overlap_participants.contains(owner) {
                overlap_participants.push(owner.clone());
            }
        }
        prev_end = prev_end.max(*end);
        last_participant = Some(owner.clone());
    }

    if !overlap_ranges.is_empty() {
        return Err(ValidationError::CoverageOverlap {
            ranges: overlap_ranges,
            participants: overlap_participants,
        });
    }

    if prev_end < max_end {
        // shouldn't happen given how max_end is computed; defensive.
        return Err(ValidationError::CoverageGap {
            missing_ranges: vec![(prev_end, max_end)],
        });
    }

    Ok(())
}

// ===========================================================================
// Validation: propagation consistency
// ===========================================================================

fn check_propagation(doc: &IRDocument) -> Result<(), ValidationError> {
    let by_id: HashMap<&NodeId, &IRNode> = doc.nodes.iter().map(|n| (&n.id, n)).collect();

    // Build a reverse index: child -> [parents via Contains].
    let mut contains_parents: HashMap<&NodeId, Vec<&NodeId>> = HashMap::new();
    for e in &doc.edges {
        if e.relation != EdgeRelation::Contains {
            continue;
        }
        contains_parents.entry(&e.target).or_default().push(&e.source);
    }

    for n in &doc.nodes {
        check_inherit_one_field(n, &by_id, &contains_parents, InheritField::Polarity)?;
        check_inherit_one_field(n, &by_id, &contains_parents, InheritField::Modality)?;
    }
    Ok(())
}

fn check_inherit_one_field(
    n: &IRNode,
    by_id: &HashMap<&NodeId, &IRNode>,
    contains_parents: &HashMap<&NodeId, Vec<&NodeId>>,
    field: InheritField,
) -> Result<(), ValidationError> {
    let is_inherit = match field {
        InheritField::Polarity => n.polarity == Polarity::Inherit,
        InheritField::Modality => n.modality == Modality::Inherit,
    };
    if !is_inherit {
        return Ok(());
    }

    // Find Contains parents.
    let parents = match contains_parents.get(&n.id) {
        Some(ps) if !ps.is_empty() => ps,
        _ => {
            return Err(ValidationError::InheritWithoutParent {
                node_id: n.id.clone(),
                field,
            });
        }
    };

    // Effective value lookup with memoization (per-field). Each lookup
    // walks Contains parents transitively; cycles can't happen here
    // because acyclicity already passed.
    let mut memo: HashMap<&NodeId, EffectivePolarityOrModality> = HashMap::new();
    let mut candidates: Vec<(NodeId, &'static str)> = Vec::new();
    for p_id in parents {
        let v = resolve_effective(p_id, by_id, contains_parents, field, &mut memo);
        match v {
            EffectivePolarityOrModality::Polarity(p) => {
                candidates.push(((*p_id).clone(), polarity_name(p)));
            }
            EffectivePolarityOrModality::Modality(m) => {
                candidates.push(((*p_id).clone(), modality_name(m)));
            }
            EffectivePolarityOrModality::StillInherit => {
                // A parent itself has Inherit but no further parent —
                // earlier check (InheritWithoutParent) would have
                // caught it. Defensive: surface as conflict candidate.
                candidates.push(((*p_id).clone(), "Inherit"));
            }
        }
    }

    // Multi-parent agreement check.
    if candidates.len() >= 2 {
        let first = candidates[0].1;
        if candidates.iter().any(|(_, v)| *v != first) {
            return Err(ValidationError::PropagationConflict {
                node_id: n.id.clone(),
                field,
                candidates,
            });
        }
    }
    Ok(())
}

enum EffectivePolarityOrModality {
    Polarity(Polarity),
    Modality(Modality),
    StillInherit,
}

/// Iterative walk along `Contains` parents. Iterative on purpose:
/// `check_acyclicity` already guarantees the graph is a DAG, but a
/// long linear chain of `Contains` edges (each node carrying
/// `Inherit` polarity) is still a legitimate input shape and would
/// otherwise grow the call stack proportional to chain length. The
/// O(N) `visited` `Vec` here grows on the heap, so worst-case
/// behaviour is graceful allocation failure rather than stack
/// overflow.
fn resolve_effective<'a>(
    start: &'a NodeId,
    by_id: &HashMap<&NodeId, &'a IRNode>,
    contains_parents: &HashMap<&'a NodeId, Vec<&'a NodeId>>,
    field: InheritField,
    memo: &mut HashMap<&'a NodeId, EffectivePolarityOrModality>,
) -> EffectivePolarityOrModality {
    let mut visited: Vec<&'a NodeId> = Vec::new();
    let mut cursor: &'a NodeId = start;

    let final_value = loop {
        if let Some(v) = memo.get(cursor) {
            break clone_eff(v);
        }
        let node = by_id[cursor];
        let is_inherit = match field {
            InheritField::Polarity => node.polarity == Polarity::Inherit,
            InheritField::Modality => node.modality == Modality::Inherit,
        };
        if !is_inherit {
            break match field {
                InheritField::Polarity => {
                    EffectivePolarityOrModality::Polarity(node.polarity)
                }
                InheritField::Modality => {
                    EffectivePolarityOrModality::Modality(node.modality)
                }
            };
        }
        // Inherit: walk to the first Contains parent (if any).
        visited.push(cursor);
        match contains_parents.get(cursor).and_then(|ps| ps.first()) {
            Some(parent) => cursor = parent,
            None => break EffectivePolarityOrModality::StillInherit,
        }
    };

    // Backfill memo for every node we walked so the next caller
    // short-circuits at the first hit.
    for id in visited {
        memo.insert(id, clone_eff(&final_value));
    }
    final_value
}

fn clone_eff(v: &EffectivePolarityOrModality) -> EffectivePolarityOrModality {
    match v {
        EffectivePolarityOrModality::Polarity(p) => EffectivePolarityOrModality::Polarity(*p),
        EffectivePolarityOrModality::Modality(m) => EffectivePolarityOrModality::Modality(*m),
        EffectivePolarityOrModality::StillInherit => EffectivePolarityOrModality::StillInherit,
    }
}

fn polarity_name(p: Polarity) -> &'static str {
    match p {
        Polarity::Affirmed => "Affirmed",
        Polarity::Denied => "Denied",
        Polarity::Uncertain => "Uncertain",
        Polarity::Inherit => "Inherit",
    }
}

fn modality_name(m: Modality) -> &'static str {
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
    use logic_core::{atom, compound};

    fn span(doc: &DocumentId, start: usize, end: usize) -> Span {
        Span::new(doc.clone(), start, end)
    }

    fn fact(id: &str, doc: &DocumentId, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: compound("p", vec![atom("a")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(doc, start, end)],
            confidence: 0.9,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn section(id: &str, doc: &DocumentId, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Section,
            term: compound("paragraph", vec![]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(doc, start, end)],
            confidence: 1.0,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn entity(id: &str, name: &str) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Entity,
            term: atom(name),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![],
            confidence: 1.0,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn edge(
        id: &str,
        source: &str,
        target: &str,
        relation: EdgeRelation,
        spans: Vec<Span>,
    ) -> IREdge {
        IREdge {
            id: EdgeId::new(id),
            source: NodeId::new(source),
            target: NodeId::new(target),
            relation,
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: spans,
            confidence: 1.0,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn empty_document_is_well_formed() {
        let doc = IRDocument::new(DocumentId::new("doc1"));
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn single_well_formed_fact_tiling_document() {
        let did = DocumentId::new("doc1");
        let n = fact("F1", &did, 0, 10);
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn duplicate_node_id_rejected() {
        let did = DocumentId::new("doc1");
        let doc = IRDocument {
            document_id: did.clone(),
            nodes: vec![fact("F1", &did, 0, 5), fact("F1", &did, 5, 10)],
            edges: vec![],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DuplicateNodeId { .. })
        ));
    }

    #[test]
    fn duplicate_edge_id_rejected() {
        let did = DocumentId::new("doc1");
        let mut n1 = fact("F1", &did, 0, 5);
        n1.kind = NodeKind::Exception;
        let mut n2 = fact("R1", &did, 5, 10);
        n2.kind = NodeKind::Rule;
        let doc = IRDocument {
            document_id: did.clone(),
            nodes: vec![n1, n2],
            edges: vec![
                edge("E1", "F1", "R1", EdgeRelation::Excepts, vec![]),
                edge("E1", "F1", "R1", EdgeRelation::Excepts, vec![]),
            ],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DuplicateEdgeId { .. })
        ));
    }

    #[test]
    fn fact_with_uncertain_polarity_rejected() {
        let did = DocumentId::new("doc1");
        let mut n = fact("F1", &did, 0, 10);
        n.polarity = Polarity::Uncertain;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::FactWithUncertainPolarity { .. })
        ));
    }

    #[test]
    fn dangling_edge_source_rejected() {
        let did = DocumentId::new("doc1");
        let r = {
            let mut r = fact("R1", &did, 0, 10);
            r.kind = NodeKind::Rule;
            r
        };
        let doc = IRDocument {
            document_id: did,
            nodes: vec![r],
            edges: vec![edge("E1", "GHOST", "R1", EdgeRelation::Excepts, vec![])],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DanglingEdgeSource { .. })
        ));
    }

    #[test]
    fn dangling_edge_target_rejected() {
        let did = DocumentId::new("doc1");
        let mut excpt = fact("X1", &did, 0, 10);
        excpt.kind = NodeKind::Exception;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![excpt],
            edges: vec![edge("E1", "X1", "GHOST", EdgeRelation::Excepts, vec![])],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DanglingEdgeTarget { .. })
        ));
    }

    #[test]
    fn self_loop_edge_rejected() {
        let did = DocumentId::new("doc1");
        let r = {
            let mut r = fact("R1", &did, 0, 10);
            r.kind = NodeKind::Rule;
            r
        };
        let doc = IRDocument {
            document_id: did,
            nodes: vec![r],
            edges: vec![edge("E1", "R1", "R1", EdgeRelation::Refines, vec![])],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::SelfLoopEdge { .. })
        ));
    }

    #[test]
    fn invalid_relation_source_kind_rejected() {
        let did = DocumentId::new("doc1");
        let f = fact("F1", &did, 0, 5);
        let mut r = fact("R1", &did, 5, 10);
        r.kind = NodeKind::Rule;
        // Excepts requires source kind = Exception; F1 is Fact.
        let doc = IRDocument {
            document_id: did,
            nodes: vec![f, r],
            edges: vec![edge("E1", "F1", "R1", EdgeRelation::Excepts, vec![])],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::InvalidRelationSourceKind { .. })
        ));
    }

    #[test]
    fn invalid_relation_target_kind_rejected() {
        let did = DocumentId::new("doc1");
        let mut x = fact("X1", &did, 0, 5);
        x.kind = NodeKind::Exception;
        let f = fact("F1", &did, 5, 10);
        // Excepts requires target kind = Rule; F1 is Fact.
        let doc = IRDocument {
            document_id: did,
            nodes: vec![x, f],
            edges: vec![edge("E1", "X1", "F1", EdgeRelation::Excepts, vec![])],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::InvalidRelationTargetKind { .. })
        ));
    }

    #[test]
    fn inherit_polarity_on_edge_rejected() {
        let did = DocumentId::new("doc1");
        let mut x = fact("X1", &did, 0, 5);
        x.kind = NodeKind::Exception;
        let mut r = fact("R1", &did, 5, 10);
        r.kind = NodeKind::Rule;
        let mut bad = edge("E1", "X1", "R1", EdgeRelation::Excepts, vec![]);
        bad.polarity = Polarity::Inherit;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![x, r],
            edges: vec![bad],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::InheritOnEdge {
                offending: InheritField::Polarity,
                ..
            })
        ));
    }

    #[test]
    fn graph_cycle_rejected() {
        // Three rules R1 -> R2 -> R3 -> R1 with Refines edges.
        let did = DocumentId::new("doc1");
        let r1 = {
            let mut r = fact("R1", &did, 0, 5);
            r.kind = NodeKind::Rule;
            r
        };
        let r2 = {
            let mut r = fact("R2", &did, 5, 10);
            r.kind = NodeKind::Rule;
            r
        };
        let r3 = {
            let mut r = fact("R3", &did, 10, 15);
            r.kind = NodeKind::Rule;
            r
        };
        let doc = IRDocument {
            document_id: did,
            nodes: vec![r1, r2, r3],
            edges: vec![
                edge("E1", "R1", "R2", EdgeRelation::Refines, vec![]),
                edge("E2", "R2", "R3", EdgeRelation::Refines, vec![]),
                edge("E3", "R3", "R1", EdgeRelation::Refines, vec![]),
            ],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::GraphCycle { .. })
        ));
    }

    #[test]
    fn unattached_exception_rejected() {
        let did = DocumentId::new("doc1");
        let mut x = fact("X1", &did, 0, 10);
        x.kind = NodeKind::Exception;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![x],
            edges: vec![],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::UnattachedException { .. })
        ));
    }

    #[test]
    fn coverage_gap_detected() {
        let did = DocumentId::new("doc1");
        let doc = IRDocument {
            document_id: did.clone(),
            nodes: vec![fact("F1", &did, 0, 5), fact("F2", &did, 10, 15)],
            edges: vec![],
        };
        let err = validate(&doc).unwrap_err();
        match err {
            ValidationError::CoverageGap { missing_ranges } => {
                assert_eq!(missing_ranges, vec![(5, 10)]);
            }
            other => panic!("expected CoverageGap, got {other:?}"),
        }
    }

    #[test]
    fn coverage_starts_at_nonzero_detected() {
        let did = DocumentId::new("doc1");
        let doc = IRDocument {
            document_id: did.clone(),
            nodes: vec![fact("F1", &did, 5, 10)],
            edges: vec![],
        };
        let err = validate(&doc).unwrap_err();
        match err {
            ValidationError::CoverageGap { missing_ranges } => {
                assert_eq!(missing_ranges, vec![(0, 5)]);
            }
            other => panic!("expected CoverageGap, got {other:?}"),
        }
    }

    #[test]
    fn coverage_overlap_detected() {
        let did = DocumentId::new("doc1");
        let doc = IRDocument {
            document_id: did.clone(),
            nodes: vec![fact("F1", &did, 0, 10), fact("F2", &did, 5, 15)],
            edges: vec![],
        };
        let err = validate(&doc).unwrap_err();
        match err {
            ValidationError::CoverageOverlap { .. } => (),
            other => panic!("expected CoverageOverlap, got {other:?}"),
        }
    }

    #[test]
    fn synthesized_query_does_not_break_tiling() {
        let did = DocumentId::new("doc1");
        // F1 tiles [0,10); Q1 is synthesized with empty spans.
        let f = fact("F1", &did, 0, 10);
        let mut q = fact("Q1", &did, 0, 0);
        q.kind = NodeKind::Query;
        q.source_spans.clear();
        let doc = IRDocument {
            document_id: did,
            nodes: vec![f, q],
            edges: vec![],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn entity_dedup_with_mentions_edge_validates() {
        let did = DocumentId::new("doc1");
        // F1 spans [0,30); entity E_p (no spans); a Mentions edge from
        // F1 to E_p with spans=[5,14) (the bytes "passenger"). Coverage
        // tiles via F1's [0,30); the edge's mention span is INSIDE
        // F1's span — which is an overlap. That's a coverage violation
        // by design: edges and nodes can't double-cover.
        //
        // The correct shape is: F1's span doesn't include "passenger",
        // and the edge tiles those bytes. Re-do.
        let f1 = fact("F1", &did, 0, 5);
        let f2 = fact("F2", &did, 14, 30);
        let e_p = entity("Ep", "passenger");
        let mention = edge(
            "E1",
            "F1",
            "Ep",
            EdgeRelation::Mentions,
            vec![span(&did, 5, 14)],
        );
        let doc = IRDocument {
            document_id: did,
            nodes: vec![f1, f2, e_p],
            edges: vec![mention],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn contains_propagation_single_parent_passes() {
        let did = DocumentId::new("doc1");
        // Section S1 (Denied) -> Contains F1 with Inherit polarity.
        let mut s1 = section("S1", &did, 0, 5);
        s1.polarity = Polarity::Denied;
        let mut f1 = fact("F1", &did, 5, 10);
        f1.polarity = Polarity::Inherit;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![s1, f1],
            edges: vec![edge("E1", "S1", "F1", EdgeRelation::Contains, vec![])],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn contains_propagation_conflict_detected() {
        let did = DocumentId::new("doc1");
        // Two Sections cover F1 via Contains, with disagreeing
        // polarities. F1's Inherit triggers a conflict.
        let mut s1 = section("S1", &did, 0, 5);
        s1.polarity = Polarity::Affirmed;
        let mut s2 = section("S2", &did, 10, 15);
        s2.polarity = Polarity::Denied;
        let mut f1 = fact("F1", &did, 5, 10);
        f1.polarity = Polarity::Inherit;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![s1, s2, f1],
            edges: vec![
                edge("E1", "S1", "F1", EdgeRelation::Contains, vec![]),
                edge("E2", "S2", "F1", EdgeRelation::Contains, vec![]),
            ],
        };
        let err = validate(&doc).unwrap_err();
        match err {
            ValidationError::PropagationConflict {
                field: InheritField::Polarity,
                ..
            } => (),
            other => panic!("expected PropagationConflict, got {other:?}"),
        }
    }

    #[test]
    fn inherit_without_parent_rejected() {
        let did = DocumentId::new("doc1");
        let mut f = fact("F1", &did, 0, 10);
        f.polarity = Polarity::Inherit;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![f],
            edges: vec![],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::InheritWithoutParent { .. })
        ));
    }

    #[test]
    fn domain_specific_collision_rejected() {
        let did = DocumentId::new("doc1");
        let mut x = fact("X1", &did, 0, 5);
        x.kind = NodeKind::Exception;
        let mut r = fact("R1", &did, 5, 10);
        r.kind = NodeKind::Rule;
        let bad_edge = edge(
            "E1",
            "X1",
            "R1",
            EdgeRelation::DomainSpecific("excepts".to_string()),
            vec![],
        );
        let doc = IRDocument {
            document_id: did,
            nodes: vec![x, r],
            edges: vec![bad_edge],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::InvalidDomainSpecificName { .. })
        ));
    }

    #[test]
    fn clarifies_kind_compatibility_enforced() {
        // Fact ←Clarifies← Uncertainty is OK; Uncertainty ←Clarifies← Fact is NOT.
        let did = DocumentId::new("doc1");
        let mut original = fact("F1", &did, 0, 10);
        // Original is a Fact; clarified is an Uncertainty — forbidden.
        let mut clarified = fact("U1", &did, 0, 10);
        clarified.kind = NodeKind::Uncertainty;
        clarified.polarity = Polarity::Uncertain;
        // Two nodes with the same span overlap. To make coverage clean,
        // use distinct spans.
        original.source_spans = vec![span(&did, 0, 5)];
        clarified.source_spans = vec![span(&did, 5, 10)];
        let doc = IRDocument {
            document_id: did,
            nodes: vec![original, clarified],
            edges: vec![edge(
                "E1",
                "U1",
                "F1",
                EdgeRelation::Clarifies,
                vec![],
            )],
        };
        // Should fail because Fact ←Clarifies← Uncertainty is not allowed:
        // (parent=Fact, child=Uncertainty) is missing from the
        // compatibility table.
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::IncompatibleClarification { .. })
        ));
    }

    #[test]
    fn adjacency_helpers_work() {
        let did = DocumentId::new("doc1");
        let f1 = fact("F1", &did, 0, 5);
        let f2 = fact("F2", &did, 5, 10);
        let e = edge("E1", "F1", "F2", EdgeRelation::DerivedFrom, vec![]);
        let doc = IRDocument {
            document_id: did,
            nodes: vec![f1, f2],
            edges: vec![e],
        };
        let n1 = NodeId::new("F1");
        let n2 = NodeId::new("F2");
        assert_eq!(doc.adjacency_out(&n1).count(), 1);
        assert_eq!(doc.adjacency_out(&n2).count(), 0);
        assert_eq!(doc.adjacency_in(&n1).count(), 0);
        assert_eq!(doc.adjacency_in(&n2).count(), 1);
    }

    #[test]
    fn discarded_without_reason_rejected() {
        let did = DocumentId::new("doc1");
        let mut n = fact("D1", &did, 0, 10);
        n.kind = NodeKind::Discarded;
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        assert!(matches!(
            validate(&doc),
            Err(ValidationError::DiscardedWithoutReason { .. })
        ));
    }

    #[test]
    fn worked_example_tsa_validates() {
        // Mini version of the ADJ01 v3 §"Worked Example" using a
        // shorter source and a subset of the relationships. Spans are
        // illustrative; the goal is to exercise the validator on a
        // realistic-shaped graph.
        let did = DocumentId::new("tsa-1540-111a");
        let mut s1 = section("N1", &did, 0, 12);
        s1.term = compound("section", vec![atom("1")]);
        let mut s2 = section("N2", &did, 12, 28);
        s2.term = compound("heading", vec![atom("2")]);
        let mut r3 = fact("N3", &did, 28, 96);
        r3.kind = NodeKind::Rule;
        r3.metadata
            .insert("as_of".to_string(), "2026-05-12".to_string());
        let mut x4 = fact("N4", &did, 100, 168);
        x4.kind = NodeKind::Exception;
        let e_passenger = entity("Ep", "passenger");

        // Coverage-tiling spans:
        //   N1: [0, 12)   structural marker "§1540.111(a) "
        //   N2: [12, 28)  heading
        //   N3: [28, 96)  rule body
        //   gap to fill:  [96, 100) — a connective span
        //   N4: [100, 168) exception
        let excepts_edge_marker = edge(
            "E1",
            "N4",
            "N3",
            EdgeRelation::Excepts,
            vec![span(&did, 96, 100)],
        );
        let mention_passenger = edge(
            "E2",
            "N3",
            "Ep",
            EdgeRelation::Mentions,
            vec![],
        );
        let doc = IRDocument {
            document_id: did,
            nodes: vec![s1, s2, r3, x4, e_passenger],
            edges: vec![excepts_edge_marker, mention_passenger],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    // -----------------------------------------------------------------
    // ADJ25 PR-1 — additive node-kind smoke tests
    //
    // PR-1 only introduces the new variants; the per-level coverage
    // invariants and Contains-edge tiling are PR-2's responsibility.
    // These tests confirm the additive change is sound: each new kind
    // can be constructed, validates as a stand-alone node tiling a
    // document, and respects the discard_reason rules.
    // -----------------------------------------------------------------

    fn typed_node(id: &str, kind: NodeKind, did: &DocumentId, start: usize, end: usize) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind,
            term: atom("placeholder"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(did, start, end)],
            confidence: 1.0,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn assert_validates_as_root_node(node: IRNode) {
        let did = node.source_spans[0].document_id.clone();
        let doc = IRDocument {
            document_id: did,
            nodes: vec![node],
            edges: vec![],
        };
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn adj25_document_node_validates() {
        let did = DocumentId::new("d");
        let n = typed_node("Doc", NodeKind::Document, &did, 0, 24);
        assert_validates_as_root_node(n);
    }

    #[test]
    fn adj25_sentence_node_validates() {
        let did = DocumentId::new("d");
        let n = typed_node("S1", NodeKind::Sentence, &did, 0, 24);
        assert_validates_as_root_node(n);
    }

    #[test]
    fn adj25_phrase_node_validates() {
        let did = DocumentId::new("d");
        let n = typed_node("P1", NodeKind::Phrase, &did, 0, 16);
        assert_validates_as_root_node(n);
    }

    #[test]
    fn adj25_question_node_validates() {
        let did = DocumentId::new("d");
        let n = typed_node("Q1", NodeKind::Question, &did, 0, 17);
        assert_validates_as_root_node(n);
    }

    #[test]
    fn adj25_quantity_component_validates() {
        let did = DocumentId::new("d");
        let n = IRNode {
            id: NodeId::new("Q200wh"),
            kind: NodeKind::Quantity,
            term: compound("quantity", vec![atom("200"), atom("wh")]),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(&did, 0, 6)],
            confidence: 1.0,
            discard_reason: None,
            metadata: HashMap::new(),
        };
        assert_validates_as_root_node(n);
    }

    #[test]
    fn adj25_polarity_component_validates() {
        let did = DocumentId::new("d");
        let n = IRNode {
            id: NodeId::new("PolDenied"),
            kind: NodeKind::Polarity,
            term: atom("denied"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(&did, 0, 2)],
            confidence: 1.0,
            discard_reason: None,
            metadata: HashMap::new(),
        };
        assert_validates_as_root_node(n);
    }

    #[test]
    fn adj25_typed_components_reject_discard_reason() {
        let did = DocumentId::new("d");
        let mut n = typed_node("Pred", NodeKind::Predicate, &did, 0, 5);
        n.discard_reason = Some(DiscardReason::NonDomainContent);
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        match validate(&doc) {
            Err(ValidationError::NonDiscardedWithReason { node_id, kind }) => {
                assert_eq!(node_id.0, "Pred");
                assert_eq!(kind, NodeKind::Predicate);
            }
            other => panic!("expected NonDiscardedWithReason, got {:?}", other),
        }
    }

    #[test]
    fn adj25_all_new_kinds_round_trip_through_match() {
        // Sanity that the exhaustive match in validate_per_node was
        // updated for every new variant. If a new kind is added later
        // without updating the match, this test still passes (compile
        // error would catch it earlier); the test documents the
        // expected per-kind validation behaviour.
        let did = DocumentId::new("d");
        for (kind, end) in [
            (NodeKind::Document, 10usize),
            (NodeKind::Sentence, 10),
            (NodeKind::Phrase, 10),
            (NodeKind::Question, 10),
            (NodeKind::Quantity, 10),
            (NodeKind::Polarity, 10),
            (NodeKind::Predicate, 10),
            (NodeKind::Comparator, 10),
            (NodeKind::TimeRef, 10),
            (NodeKind::Modifier, 10),
        ] {
            let n = typed_node("X", kind, &did, 0, end);
            let doc = IRDocument {
                document_id: did.clone(),
                nodes: vec![n],
                edges: vec![],
            };
            assert_eq!(validate(&doc), Ok(()), "kind {:?} failed validation", kind);
        }
    }

    // -----------------------------------------------------------------
    // ADJ25 PR-5 — CorrelationId helpers + completeness check
    // -----------------------------------------------------------------

    #[test]
    fn adj25_correlation_id_round_trip_through_metadata() {
        let did = DocumentId::new("d");
        let mut n = typed_node("X", NodeKind::Fact, &did, 0, 5);
        // No id before set.
        assert!(node_correlation_id(&n).is_none());
        let id = CorrelationId::new("corr.test-1");
        set_node_correlation_id(&mut n, id.clone());
        assert_eq!(node_correlation_id(&n), Some(id));
        // Idempotent overwrite.
        set_node_correlation_id(&mut n, CorrelationId::new("corr.test-2"));
        assert_eq!(
            node_correlation_id(&n).map(|c| c.0),
            Some("corr.test-2".to_string())
        );
    }

    #[test]
    fn adj25_correlation_completeness_passes_when_every_node_has_an_id() {
        let did = DocumentId::new("d");
        let mut n = typed_node("F1", NodeKind::Fact, &did, 0, 5);
        set_node_correlation_id(&mut n, CorrelationId::new("corr.F1"));
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        assert_eq!(check_correlation_completeness(&doc), Ok(()));
    }

    #[test]
    fn adj25_correlation_completeness_rejects_missing_id() {
        let did = DocumentId::new("d");
        let n = typed_node("F1", NodeKind::Fact, &did, 0, 5);
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        match check_correlation_completeness(&doc) {
            Err(CorrelationCompletenessError::NodeMissingCorrelation { node_id }) => {
                assert_eq!(node_id.0, "F1");
            }
            other => panic!("expected NodeMissingCorrelation; got {:?}", other),
        }
    }

    #[test]
    fn adj25_correlation_completeness_rejects_empty_id() {
        let did = DocumentId::new("d");
        let mut n = typed_node("F1", NodeKind::Fact, &did, 0, 5);
        set_node_correlation_id(&mut n, CorrelationId::new(""));
        let doc = IRDocument {
            document_id: did,
            nodes: vec![n],
            edges: vec![],
        };
        match check_correlation_completeness(&doc) {
            Err(CorrelationCompletenessError::NodeEmptyCorrelation { node_id }) => {
                assert_eq!(node_id.0, "F1");
            }
            other => panic!("expected NodeEmptyCorrelation; got {:?}", other),
        }
    }

    #[test]
    fn adj25_correlation_metadata_key_is_stable() {
        assert_eq!(CORRELATION_ID_METADATA_KEY, "adj.correlation_id");
    }
}
