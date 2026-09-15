## 0.1.33 — 2026-08-17 — baseline regen after call_indirect real table-index fix (task #107)

### Changed

- Baseline regen: fixing `call_indirect`/`return_call_indirect`'s
  explicit-table-index handling (see `wasm-execution`/`wasm-wast-parser`'s
  own CHANGELOG entries for this version) let most modules in
  `table_init.wast`/`table_copy.wast` build and run for the first time.
  Aggregate `assert_return` `NotYetSupported` dropped 983 -> 562,
  `assert_trap` 1789 -> 938, `module` 88 -> 65, with the pre-existing 17
  `assert_return`/2 `register`/1 `assert_unlinkable` failures byte-for-
  byte unchanged -- zero regressions, only new passes.

