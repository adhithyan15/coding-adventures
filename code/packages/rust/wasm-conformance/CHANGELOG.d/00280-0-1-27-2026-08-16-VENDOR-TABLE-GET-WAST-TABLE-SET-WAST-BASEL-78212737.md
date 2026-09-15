## 0.1.27 — 2026-08-16 — vendor table_get.wast/table_set.wast; baseline regen (task #96)

### Changed

- Baseline regen: vendored `table_get.wast`/`table_set.wast` (real
  cross-table type-checking, funcref+externref mix). Both fully pass --
  every directive kind at 100%. Zero regressions anywhere else in the
  now-64-file vendored corpus (full before/after per-file/per-kind diff).
  Deliberately excludes `table.wast` (hex-literal table limits + a real
  `spectest` import, task #99) and `table_size.wast`/`table_grow.wast`/
  `table_fill.wast` (need entirely unimplemented opcodes, task #98).

See `code/packages/rust/wasm-wast-parser/CHANGELOG.md` and `code/
packages/rust/wasm-validator/CHANGELOG.md` for the real bugs this
surfaced and fixed (table reftype parsing, per-table type-checking).

