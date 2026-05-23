# Changelog

All notable changes to the `coding-adventures-closure-emitter` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC07 emit-and-source-map spec — the back end of the Closure Compiler clone. Takes a finalized `Program` + sidecar and produces output JavaScript text + companion source-map blob.
- `emit(program: &Program, sidecar: &Sidecar, cv: &mut CVLog, opts: &EmitOptions) -> Result<EmitOutput, EmitError>` — the canonical entry point. Signature pinned.
- `EmitOptions` struct with three knobs:
  - `ascii_only: bool` (default `false`) — when `true`, escape non-ASCII codepoints to `\uXXXX` / `\u{XXXXXX}`.
  - `pretty: bool` (default `false`) — production default is minified; switch on for human-reviewed output.
  - `source_map: bool` (default `true`) — production default is to emit a companion `.js.map`.
- `EmitOutput` struct:
  - `code: String` — JavaScript bytes (UTF-8 or ASCII-restricted).
  - `source_map: Option<String>` — source-map v3 blob; `None` when `source_map = false`.
  - `contributions: Vec<Contribution>` — per-token "emitted" CV trail per CLOC03.
- `EmitError` enum (`#[non_exhaustive]`) with `Display` + `std::error::Error` impls:
  - `UnknownCvId { id, site }` — AST referenced a CV id the log doesn't know.
  - `UnsupportedSidecarType { id, kind }` — sidecar held a type the emitter can't render.
- v1 body: emits empty `code`, an empty source-map placeholder when `source_map = true`, no contributions. `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to render. The real AST walk lands once the AST grows `Statement` / `Expression` / `Declaration` variants.
- 9 tests covering: `EmitOptions::default()` values, identity emit on empty program with default opts (code empty, source_map present-but-empty, contributions empty), `source_map = false` drops the source-map field entirely, `ascii_only` flag accepted (output trivially ASCII when empty), `pretty` flag accepted, `EmitOptions` `Clone` + `PartialEq`, `EmitError::Display` formats for both variants include the id/site/kind they carry, `EmitError` implements `std::error::Error`.

### Notes
- Dependencies: `coding-adventures-javascript-ast` (`Program`), `coding-adventures-type-sidecar` (`Sidecar` for future emit hints), `coding_adventures_correlation_vector` (`CVLog`, `Contribution`), `serde` + `serde_json` (for future source-map serialization and `Contribution.meta`). Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`.
- The emitter does **not** depend on `closure-pass-pipeline` or any pass crate. It runs after the pipeline and only consumes the final `Program` shape — keeping that decoupling means future passes can be added without touching the emit dependency graph.
- v1 is scaffolding. The function signature, options struct, output struct, and error enum are the deliverable that the future `closurec` CLI (CLOC08) and the source-map generator (`closure-source-map`, CLOC07 Phase 2) link against. The body fills in once the AST grows variants.
