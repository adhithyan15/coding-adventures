# Changelog

All notable changes to this project will be documented in this file.

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
