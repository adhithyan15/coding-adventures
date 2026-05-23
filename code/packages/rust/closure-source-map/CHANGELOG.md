# Changelog

All notable changes to the `coding-adventures-closure-source-map` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC07 Phase 2 — source-map v3 generator. Companion to `closure-emitter`: receives per-token `(generated_line, generated_column, cv_id)` mappings from the emitter and produces a source-map v3 JSON blob the browser's devtools consume.
- `SourceMapBuilder`:
  - `new() -> Self` (also `Default`).
  - `set_file(&mut self, name: String) -> &mut Self` — generated-file name.
  - `set_source_root(&mut self, root: String) -> &mut Self` — prepended to each `sources` entry.
  - `add_mapping(&mut self, generated_line: u32, generated_column: u32, cv_id: &str) -> &mut Self` — record a token's origin. Fluent (returns `&mut Self` so callers can chain).
  - `raw_mapping_count(&self) -> usize` — debug/test introspection.
  - `build(self, cv: &CVLog) -> SourceMap` — finalize into the serializable form. `cv` parameter is in v1 unused; reserved for v2's VLQ encoder which walks the CV graph to resolve each `cv_id` into `(source_index, original_line, original_column)`.
- `SourceMap` struct with `Serialize` derived: `version: u32` (always 3), `file: String`, `source_root: String` (serializes as `sourceRoot` per v3 spec — camelCase), `sources: Vec<String>`, `names: Vec<String>`, `mappings: String`.
- `SourceMap::to_json(&self) -> String` — wire-format JSON string.
- v1 body: `build()` produces a valid empty v3 blob (`version = 3`, empty arrays, empty mappings string). The raw `(line, col, cv_id)` entries are still accumulated in the builder (visible via `raw_mapping_count`); the VLQ encoder that converts them lands in v2.
- 9 tests covering: builder `new()`/`Default` produce identical state, `add_mapping` accumulates in order (verified via `raw_mapping_count`), `set_file` + `set_source_root` round-trip through `build()` and `to_json`, fluent chaining works, `to_json` output is valid JSON, the top-level object has *exactly* the six v3-spec keys (`version`, `file`, `sourceRoot`, `sources`, `names`, `mappings`) — no extras, no missing, `sourceRoot` is camelCase (not `source_root`), `SourceMap` + `SourceMapBuilder` implement `Clone` + `Debug`.

### Notes
- Dependencies: `coding_adventures_correlation_vector` (`CVLog` consumed by `build()` for v2 VLQ encoding), `serde` + `serde_json` (JSON wire format).
- **Intentionally no `javascript-ast` or `type-sidecar` dependency.** This crate is backend-agnostic: anything that emits source text + per-token CV metadata can use it. Future Lispy / Prolog backends in this monorepo will produce maps the same way.
- v1 is scaffolding. The function signatures, struct shapes, and JSON wire format are the deliverable that `closure-emitter` and the future `closurec` CLI (CLOC08) link against. The VLQ encoder + CV-graph walk lands in v2.
