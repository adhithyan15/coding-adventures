# Changelog — coding_adventures_correlation_vector

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added - public limit configuration validation (CV02)

`GraphLimits::validate` exposes the existing safe metadata-depth ceiling to
compiler configuration before input work. Limits support `PartialEq`/`Eq` for
configuration comparison; numeric fields and default values are unchanged.

### Fixed — CV02 boundary review

- Charge object-key work before decoding; validate borrowed key role, duplicates
  and encoded metadata bytes before copying keys into owned storage.

- Retain allocator-only import status through mutation/save/reload. Canonical
  exports include unchecked_import:true; strict queries/validation and checked
  import reject that state instead of upgrading discarded/defaulted evidence.

### Added

- Public `dispose_metadata` lets evidence consumers drain pending owned payloads
  iteratively without changing record types' field-move compatibility.

- Enforce enabled recording, complete compact allocation coverage and stage-set
  consistency at fallible query boundaries, including generic compatibility
  reloads. Reject declared views in both identity modes before their markers
  could be discarded (independent review findings with regression evidence).

- CV02 checked library foundation: explicit `GraphLimits`, fully recorded compact
  construction/import, independently validated iterative DAG queries and borrowed
  parent-before-child lineage. Unknown parents, incomplete allocation coverage,
  graph/schema defects and exceeded limits fail without success-shaped fallbacks.
- Transaction accounting before node/event mutation; repeated edges remain data.
  Checked deletion is permanent and records its stage. Graph fields are private
  with read-only accessors; generic toggles use `set_enabled`.
- Bounded checked JSON parsing rejects nested duplicate decoded keys before
  replacement values, unknown/missing record fields and declared partial views.
  Import shares structural work across parse, conversion and validation.
- Canonical bounded JSON encoding borrows evidence, sorts every object level,
  retains arrays/numbers and shares validation/encoding work. Compatibility
  snapshots retain allocator-only semantics with finite payload/output bounds.
- Iterative disposal of rejected owned metadata, including early failures and
  disabled no-ops, avoids recursive destruction of arbitrary caller nesting.
- Exact/over-cap resource tests, integer-overflow rejection, isolated-process
  65,536-level cleanup regression, 4,096/8,192-node chain and 10,002-node wide
  graph checks. Compiler formats/publication integration and typed execution
  chronology remain separate work; the library foundation does not close CCR-065.
- CV01 opt-in `CVLog::new_compact`: fixed 20-byte per-log identities, shared
  create/derive/merge sequence and explicit compact-v1 allocation watermark.
  Save/reload preserves disabled allocations and rejects malformed identity
  state and projected views. Default hierarchical clients retain their format.
- Reject present null allocator state, including empty/disabled snapshots,
  instead of silently switching to legacy allocation. Reject duplicate entry
  identities before their values can overwrite evidence, in both modes and
  with equivalent escaped JSON keys. Full graph/schema validation remains pending.
- Fallible allocation methods reject counter exhaustion and collisions before
  mutation. String-returning methods fail fast instead of wrapping/overwriting.
- Deep direct-library chain, mode-toggle/reload, origin/history/tombstone,
  malformed-state and unchanged-on-error tests. Graph budgets, validation,
  canonical serialization and chronological events remain pending foundation work.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of the Correlation Vector (CV00) spec in Rust.
- `CVLog` — the central log struct with `entries`, `pass_order`, and `enabled` fields.
- `CVEntry` — full record for one tracked entity: id, parent_ids, origin, contributions, deleted.
- `Origin` — where/when an entity was born (source, location, timestamp, meta).
- `Contribution` — a stage's record of processing an entity (source, tag, meta).
- `DeletionRecord` — permanent record of an intentional entity removal.
- `CVLog::new(enabled)` — create an empty log, optionally disabled.
- `CVLog::create(origin)` — create a root CV with SHA-256-based ID.
- `CVLog::contribute(cv_id, source, tag, meta)` — append a contribution; errors on deleted entities.
- `CVLog::derive(parent_cv_id, origin)` — create a child CV with dot-extended ID.
- `CVLog::merge(parent_cv_ids, origin)` — create a CV from multiple parents.
- `CVLog::delete(cv_id, source, reason, meta)` — mark an entity as deleted.
- `CVLog::passthrough(cv_id, source)` — record a no-change stage visit.
- `CVLog::get(cv_id)` — retrieve a CVEntry by ID.
- `CVLog::ancestors(cv_id)` — BFS walk of parent chain, nearest first.
- `CVLog::descendants(cv_id)` — reverse index scan for all children/grandchildren.
- `CVLog::history(cv_id)` — ordered list of contributions.
- `CVLog::lineage(cv_id)` — full ancestor chain + entity, oldest first.
- `CVLog::to_json_string()` — serialize to compact JSON.
- `CVLog::from_json_string(s)` — deserialize from JSON, reconstructing counters.
- 30+ unit tests covering all 7 spec groups (>95% coverage).
- Full literate-programming documentation with inline diagrams and examples.
