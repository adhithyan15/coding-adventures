# Changelog

All notable changes to `journal-wasm` are documented here.

## [0.1.0] - Unreleased

### Added

- The linear-memory WASM ABI over `journal-core` (J2b of #14416, spec
  `code/specs/journal-wasm.md`): `init`, `load`, `snapshot`, `apply`,
  `journals`, `entry`, `timeline`, `on_this_day`, `search`, `tag_counts`,
  `month_activity`, `import_legacy`, plus the repo-standard `alloc`/`dealloc`
  and `reset`.
- Every export answers with a JSON envelope carrying a stable camelCase `code`
  on failure; nothing traps the boundary, including calls before `init`.
- Error text never repeats untrusted input: ids are checked before any
  "not found" error, and parse failures report category and position only
  (found by the pre-push security review).
- `load` runs `JournalState::validate` and keeps the current state unless the
  snapshot both parses and validates — the loader obligation from the
  journal-core spec.
- `import_legacy` migrates the TypeScript Journal's `Entry[]`, skipping and
  reporting (index + code) rows the core refuses instead of failing the file.
- `js/journal-engine.mjs` (dependency-free accessor, one method per export) and
  `js/smoke.mjs` (end-to-end Node smoke test); `build-wasm.sh` /
  `build-wasm.ps1` write `pkg/journal_engine.wasm` (not committed).
