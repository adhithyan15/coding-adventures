## 0.1.40 — 2026-08-18 — vendor simd_i16x8_arith.wast; baseline regen (task #129-132)

### Changed

- Baseline regen: vendored `simd_i16x8_arith.wast` -- the first file
  where `i16x8` is a PRIMARY lane width (produces `i16x8` results)
  rather than merely an input to an `i32x4`-producing widening op.
  `i16x8.add`/`sub`/`mul`/`neg` are the first such opcodes this repo
  implements, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on
  every directive kind (2/2 modules, 181/181 assert_return, 11/11
  assert_invalid). Aggregate `assert_return` rose from 21577/21594 to
  21758/21775; `assert_invalid` from 1460 to 1471.

