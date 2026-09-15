## 0.1.29 — 2026-08-16 — vendor memory_copy.wast/memory_fill.wast; baseline regen (task #94)

### Changed

- Baseline regen: vendored `memory_copy.wast`/`memory_fill.wast` -- the
  first vendored files from the bulk-memory proposal. Both fully pass --
  every directive kind at 100%. Zero regressions anywhere else in the
  now-66-file vendored corpus (full before/after per-file/per-kind diff).
  Surfaced a real bug in `wasm-execution::LinearMemory::copy()`/`fill()`
  (see that crate's own CHANGELOG): a zero-length copy/fill skipped
  bounds-checking entirely instead of still requiring `dest`/`src` to sit
  at or before the end of memory. Deliberately excludes their sibling
  `bulk.wast`: it mixes memory.copy/memory.fill with memory.init/
  data.drop (task #95) and table.init/elem.drop/table.copy (task #97) in
  the same file, all still unimplemented.

See `code/packages/rust/wasm-wast-parser/CHANGELOG.md` and `code/
packages/rust/wasm-execution/CHANGELOG.md` for the parsing and
interpreter-layer changes this vendoring pass needed and surfaced.

