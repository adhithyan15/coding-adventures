## 0.1.93 — 2026-08-26 — table64 proposal, first slice (W26)

### Added

- Vendored `table64.wast` (757 bytes) AND `memory64-imports.wast` (6728
  bytes, W25's own deliberately-deferred file — entangled because roughly
  half of it is table64 import/export cases) to `TESTSUITE_FILES`, both
  from the same pinned SHA `28864811cf03bdbf880733786148feaba339582d`.
  `table64` is no longer entangled once `TableType`/`Table` gain the same
  `is64` plumbing `MemoryType`/`LinearMemory` already have (W25), so both
  of W25's two deferred items resolve together this slice.

### Fixed

- Real corpus, measured (`cargo run --bin wasm_conformance_report`, full
  baseline diff — confirmed via JSON diff that ONLY these two new files'
  stats moved, zero regressions anywhere else): `memory64-imports.wast`
  grades `module` 40/40, `register` 8/8, `assert_unlinkable` 30/30 — ALL
  real `Pass`. `table64.wast` grades `assert_invalid` 2/2 (real `Pass`)
  and `module` 10 `Pass` + 1 `Trap` (a spec-valid `(module definition
  (table i64 0xffff_ffff_ffff_ffff funcref))` declaration — table64's own
  real spec ceiling is `u64::MAX`, but this interpreter's own practical
  resource limit, `wasm_execution::MAX_TABLE_ELEMENTS`, correctly refuses
  to actually allocate a table anywhere near that size, matching the
  identical "spec allows more than we'll actually allocate" shape W25
  established for a `2^48`-page memory64 declaration) + 1
  `NotYetSupported` (`(table (import "spectest" "table64") ...)` — this
  crate's `RegistryHost` has no `spectest` support, the same pre-existing,
  documented gap every other `spectest`-referencing corpus file already
  hits).

See `code/specs/W26-wasm-table64-first-slice.md`.

