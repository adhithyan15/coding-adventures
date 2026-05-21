# Changelog

All notable changes to the `coding-adventures-javascript-ast` crate will be documented in this file.

## [0.1.0] - 2026-05-21

### Added
- New crate scaffolded per CLOC02 Phase 1.
- `Program` struct: AST root carrying `cv: CvId`, `version: EsVersion`, and `source_type: SourceType`. Per CLOC02, the version tag lives only on `Program` — never on individual nodes.
- `SourceType` enum (`Script` / `Module`) with derives `Debug, Clone, Copy, PartialEq, Eq, Hash`.
- `CvId` type alias for `String` matching the current `correlation-vector` representation (see module-level docs for the migration plan to a true newtype).
- `Program::new(cv, version, source_type)` constructor.
- Module-level docs enumerate the six backend-agnostic invariants from CLOC02 and the dependency whitelist.
- Test suite covering: synthetic construction with each `SourceType`, `Clone` + `PartialEq`, compile-time `Copy` assertions on `SourceType` and `EsVersion`.

### Notes
- Dependencies are exactly `coding-adventures-correlation-vector` and `coding-adventures-javascript-tokens`. No serde for v1 — round-trippable JSON ships in a follow-up once consumers actually need it.
- The `Statement` / `Expression` / `Declaration` / class / module / literal variants from CLOC02 are deferred to follow-up PRs to keep this scaffolding small.
