## 0.1.15 — 2026-08-15 — real assert_unlinkable grading via registry-backed HostInterface (WASM05)

### Added

- `RegistryHost`: a `HostInterface` backed by `Executor`'s own module
  registry, letting a module import a function/memory/table/global from
  a `register`ed sibling module -- the shape the real corpus's own
  `assert_unlinkable`/linking cases use. Function imports resolve to a
  real `CrossModuleFunction` wrapper whose `call()` re-enters
  `WasmRuntime::call_typed` against the *callee's own* instance state,
  reusing already-tested machinery for genuine cross-instance calls, not
  just link-time type declarations. See `code/specs/
  W10-wasm-real-linking-and-unlinkable.md`.
- `assert_unlinkable` is now graded for real (`grade_assert_unlinkable`)
  instead of unconditionally `NotYetSupported` -- build failure,
  structural-validation failure, or a genuine link failure via
  `RegistryHost` all count as the expected outcome (matching
  `grade_assert_invalid`'s own precedent: the harness only needs the
  OUTCOME category to match, not the specific reason).
- `Executor.current_link_failed` replaces the old blanket
  `current_has_imports` gate: a module that fails to LINK for a genuine
  capability gap (an import from a host module `RegistryHost` doesn't
  know about, e.g. `spectest`) now cascades to `NotYetSupported` for
  subsequent `invoke`/`get` actions targeting it -- same outcome as
  before for every currently-vendored file, but for the real, specific
  reason rather than "any import present at all."
- 7 new tests: totally-unknown-module/unknown-export/type-mismatch
  `assert_unlinkable` cases (all now real `Pass`es), and a genuine
  cross-instance function call round-trip proving the positive linking
  path works end to end.

### Known limitation, named not silent

- `RegistryHost::resolve_memory`/`resolve_table` return a real CLONE of
  the exporting instance's memory/table, not a live shared view (both
  `HostInterface` methods return owned values by their existing
  signature) -- link-time limits compatibility is still checked for
  real, but a write through the importing instance won't become visible
  to the exporting one. None of the corpus vendored so far exercises
  that; revisit if a future vendored file needs it.

### Deferred, not silently dropped

- The real, pinned-commit `imports.wast` (93 `assert_unlinkable` cases,
  mostly `register`-based sibling-module linking this PR's
  `RegistryHost` can grade for real) was fetched and attempted, but
  fails to PARSE entirely: its "auxiliary modules to import from"
  section uses `(tag ...)` declarations (WebAssembly exceptions
  proposal syntax) `wasm-wast-parser` has no grammar support for at all.
  Not vendored this PR -- see the new backlog item tracking this
  (needs at least minimal structural `tag` parsing support first). The
  real linking/`assert_unlinkable` machinery itself (`RegistryHost`,
  `CrossModuleFunction`, `wasm-runtime`'s link-failure path) is fully
  implemented and verified via hand-written in-crate test scripts
  instead, and remains ready to grade `imports.wast` for real the
  moment it can parse.

