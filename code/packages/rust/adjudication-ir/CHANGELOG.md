# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2026-05-13 — ADJ25 PR-5: correlation IDs (additive helpers)

### Added

New types + helpers for the ADJ25 correlation vector. Every node
and edge can now carry a `CorrelationId` via metadata, threading
source-byte → IR node → engine clause → verdict citation
traceability through the IR.

- `CorrelationId(pub String)` — newtype around the opaque identifier
  with `new`, `is_empty`, `as_str`, `Display`.
- `CORRELATION_ID_METADATA_KEY = "adj.correlation_id"` — reserved
  metadata key under the framework's `adj.*` namespace.
- `node_correlation_id(node) -> Option<CorrelationId>` /
  `edge_correlation_id(edge)` — read helpers.
- `set_node_correlation_id(node, id)` /
  `set_edge_correlation_id(edge, id)` — write helpers (idempotent).
- `check_correlation_completeness(ir_doc)` — verifies every node
  carries a non-empty CorrelationId; returns the first violation
  (`NodeMissingCorrelation { node_id }` / `NodeEmptyCorrelation
  { node_id }`).

### Why metadata-keyed rather than a first-class field on `IRNode`

Adding a required field to `IRNode` is a SemVer-breaking change
that ripples through 400+ struct-literal construction sites in the
workspace, most of them tests. The `metadata: HashMap<String,
String>` field was designed for exactly this kind of additive
attribute. PR-7 (cutover) may promote to a dedicated struct field
once the workspace sweep is in scope.

### Tests

5 new test cases: metadata round-trip, completeness pass on fully
correlated IR, completeness rejects missing id, completeness
rejects empty id, metadata-key constant lock. Total tests: 34 → 39,
all passing.

### Notes

- Version: 0.4.0 → 0.5.0 (additive public surface).

## [0.4.0] - 2026-05-13 — ADJ25 PR-1: hierarchical-decomposition node kinds

### Added

New `NodeKind` variants per
[ADJ25](../../specs/ADJ25-hierarchical-decomposition.md), the
foundational reset to per-level coverage for source decomposition.
This PR is **additive only** — existing v3 kinds remain valid; the
PR-2/PR-7 sequence retires `Section` in favour of the explicit
level kinds once the foundation bench (PR-6) passes.

**Level-0 → level-3 skeleton kinds:**

- `NodeKind::Document` — the root of the hierarchical decomposition.
  Exactly one per IR document, span `[0, N)`.
- `NodeKind::Sentence` — a natural-language sentence in the source
  (level 1; children of `Document`).
- `NodeKind::Phrase` — a sub-sentence chunk that commits to one claim
  / uncertainty / question / discardable (level 2; children of
  `Sentence`).
- `NodeKind::Question` — an interrogative present in the source text,
  distinct from the engine-facing `Query` kind (level 3; children of
  `Phrase`).

**Level-4 typed-component slots (children of `Fact`):**

- `NodeKind::Quantity` — typed numerical literal `quantity(value,
  unit)`. Every numerical literal in a `Fact`'s span must surface as
  one of these (PR-2 will enforce; PR-1 only introduces the kind).
- `NodeKind::Polarity` — typed polarity slot, present when a `Fact`
  contains negation cues. The variant name shadows the lattice enum
  `Polarity`; disambiguate with `NodeKind::Polarity`.
- `NodeKind::Predicate` — the relation / verb of a `Fact`.
- `NodeKind::Comparator` — relational operator (`Eq`, `Lt`, `Le`,
  `Gt`, `Ge`, `Ne`).
- `NodeKind::TimeRef` — date, duration, or temporal phrase.
- `NodeKind::Modifier` — adjective / adverb refinement.

`NodeKind::Entity` is reused as the level-4 entity component
unchanged.

### Tests

Eight new test cases in the `tests` module covering construction +
validation of every new kind, plus a regression-guard on the
`discard_reason` rule for typed components. Total tests: 26 → 34.

### Notes

- Adding variants to a non-`#[non_exhaustive]` enum is a SemVer
  breaking change for downstream exhaustive matches. The workspace
  consumers (`adjudication-connector`) have been updated in the same
  PR. External consumers may need updates; this is consistent with
  the 0.x convention of minor-bumps for breaking changes.
- No coverage or per-level invariants are enforced yet — those land
  in PR-2 (per-level coverage check). PR-1 deliberately ships only
  the type-level additions so each subsequent PR has a tight scope.

## [0.3.0] - 2026-05-12 — ADJ01 v3 multi-directed acyclic graph

### Changed (breaking)

The crate's surface area is rebuilt to match
[ADJ01 v3](../../specs/ADJ01-adjudication-ir-grammar.md). v2's
hierarchical decomposition tree (with `part_of` and `lowered_from`
fields on `IRNode` and the content-free `TextRun` grouping kind)
becomes a single multi-directed acyclic graph of typed nodes and
typed `IREdge`s. The tree was sufficient for small narratives but
cannot represent the relationships that emerge at scale: rule
citations, cross-references, table-row membership, provenance
chains, exception scoping, temporal supersession.

The closed-set edge-relation taxonomy organizes 30+ relations into
eleven groups plus a `DomainSpecific(name)` escape hatch:

  1. Structural — `Contains`, `Precedes`, `Heading`
  2. Identity — `Mentions`, `SameAs`, `Refers`
  3. Rule modification — `Excepts`, `Refines`, `Generalizes`,
     `Supersedes`, `Restricts`
  4. Application — `AppliesTo`, `AppliesWhen`, `Concludes`
  5. Provenance — `DerivedFrom`, `JustifiedBy`, `ElicitedFrom`
  6. Tabular — `RowOf`, `ColumnOf`, `HeaderOf`, `CellOf`
  7. Temporal — `Before`, `After`, `During`, `EffectiveAt`,
     `SupersededAt`
  8. Cross-source — `ConflictsWith`, `Confirms`, `DependsOn`
  9. Discourse — `Defines`, `Restates`, `Cites`
 10. Refinement — `Clarifies` (replaces v2's `lowered_from`)
 11. Escape hatch — `DomainSpecific(name)`

### Added

- `IREdge { id, source, target, relation, polarity, modality,
  source_spans, confidence, metadata }` — first-class edge struct
  alongside `IRNode`.
- `EdgeId` opaque identifier mirroring `NodeId`.
- `EdgeRelation` enum with all 30+ closed-set variants plus
  `DomainSpecific(String)` escape hatch and an `as_str()` method
  for audit-trail records.
- `NodeKind::Section` — meaningful structural unit (paragraph,
  sentence, table, row, cell, heading). Carries the structural type
  in its `term`; its `source_spans` cover only the meta-text
  (heading, numbering, delimiters), not the content.
- `NodeKind::Entity` — deduplicated reference target for atoms or
  compound terms mentioned at multiple sites. May have empty
  `source_spans` (synthesized).
- `IRDocument.edges: Vec<IREdge>` top-level collection.
- `IRDocument::adjacency_out(node_id)` / `adjacency_in(node_id)`
  helpers for callers that want a graph-adjacency view.
- `IRDocument::node(id)` / `edge(id)` lookup helpers.
- `SpanLocation { Node(NodeId), Edge(EdgeId) }` discriminator on
  span-related `ValidationError`s so callers can attribute span
  failures to either a node or an edge.
- `InheritField { Polarity, Modality }` discriminator on
  `Inherit`-related errors.
- `NodeOrEdgeId { Node(NodeId), Edge(EdgeId) }` for coverage-
  overlap participant attribution.
- Eleven new `ValidationError` variants for edge well-formedness
  (`DanglingEdgeSource`, `DanglingEdgeTarget`, `SelfLoopEdge`,
  `DuplicateEdgeId`, `InheritOnEdge`, `InvalidDomainSpecificName`,
  `InvalidRelationSourceKind`, `InvalidRelationTargetKind`,
  `IncompatibleClarification`, `UnattachedException`, `GraphCycle`)
  and three for coverage and propagation
  (`CoverageGap`, `CoverageOverlap`, `InheritWithoutParent`,
  `PropagationConflict`).

### Removed (breaking)

- `NodeKind::TextRun` — replaced by `NodeKind::Section` with a
  meaningful `term`.
- `IRNode.part_of: Option<NodeId>` — replaced by
  `EdgeRelation::Contains` edges. A node's structural parent is no
  longer a field on the node; it's an edge in the graph.
- `IRNode.lowered_from: Option<NodeId>` — replaced by
  `EdgeRelation::Clarifies` edges.
- v2's `ValidationError` variants tied to the tree shape:
  `LoweringCycle`, `DanglingLoweredFrom`, `LoweringExpandsSpans`,
  `IncompatibleLowering`, `DanglingPartOf`, `PartOfCycle`,
  `NonTextRunHasChildren`, `ChildSpansExceedParent`,
  `ChildrenDoNotTileParent` — replaced by `GraphCycle`,
  `IncompatibleClarification`, `CoverageGap`, `CoverageOverlap`,
  etc.
- `validate_structural_tree` and `validate_lowering_dag`
  module-private helpers — replaced by the graph-shaped
  `check_acyclicity`, `check_coverage`, `check_propagation`, and
  per-relation invariant checks.

### Invariants (v3 well-formedness summary)

`validate` enforces all nine of:

  1. Node-kind constraints (e.g., `Fact.polarity != Uncertain`).
  2. Edge well-formedness (source/target exist, relation legal,
     `Inherit` rejected on edges, DomainSpecific name validation).
  3. Identifier uniqueness (nodes and edges separately).
  4. Span validity (offsets are valid byte ranges in this document).
  5. Coverage — flat tile of `(nodes ∪ edges).source_spans` against
     `[0, max_end)`, with synthesized-object exemption (empty-span
     Query, Entity, and synthesized edges).
  6. Acyclicity — `(Nodes, Edges)` is a DAG across ALL edge
     relations.
  7. Propagation consistency — for every node with
     `polarity = Inherit`, all `Contains` parents agree on the
     effective polarity (and similarly for modality).
  8. Edge-relation endpoint kind constraints (e.g., `Excepts` must
     go `Exception -> Rule`; `RowOf` must target a `Section`).
  9. Exception attachment — every Exception node is the source of
     at least one `Excepts` edge.

### Migration

Backwards compatibility is deliberately broken. v2 IR documents do
not load as v3. Downstream consumers that depend on this crate
(adjudication-coverage, adjudication-polarity-modality,
adjudication-round-trip, adjudication-adversarial,
adjudication-connector, adjudication-pipeline, the demos, and
llm-primitives) are temporarily excluded from the workspace
`members` list while their migration PRs land. Each consumer's
migration is sized to land as its own PR per the sequence in
[`ADJ01 v3 §"Status"`](../../specs/ADJ01-adjudication-ir-grammar.md).

### Tests

24 unit tests cover empty doc, single-node tiling, duplicate ids
(node + edge), per-kind polarity rules, edge endpoint existence,
self-loop, relation source/target kind constraints, `Inherit` on
edges, `Inherit` on nodes without parent, graph cycle, unattached
exception, coverage gap (mid-document and at-start), coverage
overlap, synthesized-Query exemption, Entity dedup with Mentions
edge tiling, single-parent Contains propagation, multi-parent
`PropagationConflict`, `DomainSpecific` name collision, `Clarifies`
kind incompatibility, adjacency helpers, Discarded-without-reason,
and a worked TSA-style example end-to-end.

## [0.2.0] - 2026-05-11 — ADJ01 v2 hierarchical decomposition

### Added

- `NodeKind::TextRun` — non-leaf decomposition node carrying source
  spans and (optionally) polarity / modality. Used to build the
  structural tree the LLM produces.
- `IRNode.part_of: Option<NodeId>` — structural parent in the
  decomposition tree. A node with `part_of = None` is at a document
  root. Orthogonal to the existing `lowered_from` (refinement DAG).
- `Polarity::Inherit` and `Modality::Inherit` — defer to the
  ancestor's effective value. Resolved by ADJ03 v2's propagation
  pass before downstream consumers see the value.
- `validate_structural_tree` enforces five v2 invariants beyond v1's
  checks:
    1. `part_of` references existing nodes (`DanglingPartOf`).
    2. `part_of` edges form a forest (`PartOfCycle`).
    3. Only `TextRun` nodes have children (`NonTextRunHasChildren`).
    4. Child spans fit inside parent spans (`ChildSpansExceedParent`).
    5. Each `TextRun`'s children's spans tile its own spans
       (`ChildrenDoNotTileParent`, with `missing_ranges`).
- New `ValidationError` variants: `DanglingPartOf`, `PartOfCycle`,
  `NonTextRunHasChildren`, `ChildSpansExceedParent`,
  `ChildrenDoNotTileParent`.
- 9 new tests for v2 invariants (textrun-with-tiling-passes, gap
  fails with missing_ranges, empty textrun, fact-with-child rejected,
  dangling part_of, child spans exceeding parent, Inherit accepted on
  each kind, nested decomposition).

### Changed

- Per-kind polarity rules now accept `Polarity::Inherit` (e.g., a
  Query may declare `Affirmed` or `Inherit`; an Uncertainty may
  declare `Uncertain` or `Inherit`).
- Downstream `adjudication-connector` adds a fall-back path for
  `Polarity::Inherit` (treats it as Affirmed if reached without a
  prior propagation pass).

### Unchanged

- The seven leaf node kinds (Fact, Query, Uncertainty, Rule,
  Exception, Discarded), their polarity / modality rules, and the
  refinement DAG (`lowered_from`) semantics.
- `Polarity` / `Modality` lattice values (just gained `Inherit`).
- `DiscardReason` vocabulary.

### Migration

v1 IR documents wrap into a single `TextRun` covering the whole
document and become valid v2 documents. A `migrate_v1_to_v2(doc)`
helper will land alongside the JSON-schema work.

## [0.1.0] - 2026-05-11

### Added

- `NodeId`, `DocumentId` newtypes for stable identifiers.
- `Span { document_id, start, end }` — byte-offset ranges into a
  document's normalized text, per `ADJ01`.
- `Polarity` enum: `Affirmed`, `Denied`, `Uncertain`. Flat lattice;
  no `Unknown` (absence of evidence is the absence of a node).
- `Modality` enum: `Present`, `Past`, `Future`, `Hypothetical`,
  `FamilyHistory`, `RuledOut`, `Conditional`. Flat lattice. `RuledOut`
  and `Denied` are explicitly distinct because clinical and legal
  practice treats them as non-synonyms.
- `NodeKind` enum: `Fact`, `Query`, `Uncertainty`, `Rule`, `Exception`,
  `Discarded`.
- `DiscardReason` controlled vocabulary: `Pleasantry`, `DocumentMetadata`,
  `NonDomainContent`, `Restatement`, `Unparseable`, `AdministrativeOnly`,
  `ExplicitlyOutOfScope`.
- `IRNode { id, kind, term, polarity, modality, source_spans,
  confidence, lowered_from, discard_reason, metadata }` carrying every
  field from `ADJ01`'s grammar.
- `IRDocument { document_id, nodes }` — the container.
- `validate(doc)` — total well-formedness check enforcing every rule
  from `ADJ01 §"Well-Formedness Summary"`: per-kind constraints, DAG
  acyclicity of `lowered_from`, kind-compatibility under lowering,
  span-superset invariant on lowered nodes, Exception's `applies_to`
  metadata pointing to an existing Rule.
- `ValidationError` enum naming every violation class precisely so
  callers can branch on them.
- 22 tests covering each well-formedness rule and the lowering DAG
  invariants.

### Scope

This is the first slice of [`ADJ01`](../../../specs/ADJ01-adjudication-ir-grammar.md).
JSON serialization and the canonical IR schema (a forthcoming
`code/specs/schemas/adjudication-ir.schema.json`) are deferred to a
subsequent slice — the structural types should stabilize first.

### Notes

The IR's term language is `logic_core::Term` unchanged (per `ADJ01`).
This crate adds metadata around terms without modifying the term layer.
