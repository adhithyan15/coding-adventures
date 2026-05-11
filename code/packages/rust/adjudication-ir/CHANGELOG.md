# Changelog

All notable changes to this project will be documented in this file.

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
