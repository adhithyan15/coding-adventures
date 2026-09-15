## 0.1.58 — 2026-08-19 — vendor simd_select.wast + simd_address.wast: pure vendoring, zero new opcodes (task #186-187)

### Added

- Vendored `simd_select.wast` and `simd_address.wast` (both new files,
  not re-fetches of already-vendored ones) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. Unlike every prior PR in
  this SIMD widening campaign, this PR adds ZERO new opcodes to
  `wasm-opcodes` -- both files were confirmed, before vendoring, to use
  only opcodes already fully implemented: `simd_select.wast` exercises
  untyped `select` (`0x1B`) with `v128` operands (the parametric `select`
  handler and the validator's `select` type-check rule are both already
  fully generic over `ValueType`, `V128` included, with no SIMD-specific
  gating anywhere); `simd_address.wast` exercises `v128.load`/
  `v128.store` memarg offset/align edge cases, both implemented since
  PR15 (task #162-164). 100% pass on EVERY directive in both files:
  `simd_select.wast` (1/1 module, 6/6 assert_return); `simd_address.wast`
  (3/3 modules, 36/36 assert_return, 6/6 assert_trap, 2/2 assert_invalid,
  2/2 assert_malformed) -- 46 directives total, zero `NotYetSupported`
  anywhere. Aggregate `assert_return` rose from 24292/24309 to
  24334/24351 (+42 pass, +42 gradeable, exactly the two files' combined
  `assert_return` count); `assert_trap` +6, `assert_invalid` +2,
  `assert_malformed` +2 (all still 100.0% of gradeable directives);
  `module` pass count rose from 1164 to 1168 (+4). No source changes to
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`/`wasm-opcodes` --
  only this crate's fixtures/baseline/version/changelog are touched. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

