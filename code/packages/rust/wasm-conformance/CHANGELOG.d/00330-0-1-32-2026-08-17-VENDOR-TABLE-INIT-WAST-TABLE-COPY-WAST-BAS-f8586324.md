## 0.1.32 — 2026-08-17 — vendor table_init.wast + table_copy.wast; baseline regen (task #97)

### Changed

- Baseline regen: vendored `table_init.wast` and `table_copy.wast` --
  every directive that actually runs passes (module 17/17 and 23/23,
  assert_invalid 67/67, assert_trap 8/8 and 14/14, assert_return 9/9
  combined), zero regressions anywhere else in the corpus (the
  pre-existing 17 `assert_return`/2 `register`/1 `assert_unlinkable`
  failures are byte-for-byte unchanged from the prior baseline).
- Both files' `module`/`assert_return`/`assert_trap` directive
  coverage is real but partial: most modules in each file use
  `(call_indirect $t0 (type N) ...)` -- the reference-types proposal's
  explicit-table-index text syntax -- which `wasm-wast-parser` doesn't
  parse yet (it hardcodes table 0 for every `call_indirect`, see task
  #107, logged as new backlog work discovered by this vendoring pass).
  A module using that syntax fails to build entirely and every
  directive against it grades `NotYetSupported`, not `Fail` -- this is
  the SAME "vendor now with a real, non-silent capability-gap count;
  widen coverage later" pattern this crate already uses for
  `simd_const.wast`'s partial opcode coverage.

