## 0.1.25 — 2026-08-15 — multi-memory; baseline regen (W16, task #85)

### Fixed

- `RegistryHost::resolve_memory` discarded the resolved export's memory
  index and always cloned `instance.memory` (the single memory that
  existed before `wasm-runtime` 0.6.2) -- harmless before multi-memory,
  a real latent bug afterward. Now indexes `instance.memories` by the
  real export index, matching `resolve_global`'s existing pattern.

### Changed

- Baseline regen: `memory_grow.wast` moves from a file-level parse
  failure (`$mem1` unresolved) to fully passing -- `module` 933/934 ->
  936/936 (100%), `register` 1/2 -> 2/2 (100%), `assert_return` 15523/
  16029 -> 15570/16029 (the 47-directive gap moving from `not_yet_
  supported` to real `pass`). Zero regressions anywhere else in the
  61-file vendored corpus (full before/after per-file/per-kind diff).
  This is the last remaining conformance gap in the corpus -- every
  graded directive kind is now at 100% pass with zero `fail`/`trap`
  anywhere.

See `code/specs/W16-wasm-multi-memory-first-slice.md` for the full design.

