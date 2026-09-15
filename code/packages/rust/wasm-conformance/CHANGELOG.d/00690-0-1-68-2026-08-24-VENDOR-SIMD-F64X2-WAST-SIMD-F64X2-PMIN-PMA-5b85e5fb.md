## 0.1.68 — 2026-08-24 — vendor simd_f64x2.wast/simd_f64x2_pmin_pmax.wast: SIMD widen PR35 (task #220-222)

### Added

- Vendored `simd_f64x2.wast` and `simd_f64x2_pmin_pmax.wast` (both new
  files, not re-fetches of already-vendored ones) at the existing
  pinned commit `28864811cf03bdbf880733786148feaba339582d`. This PR
  implements the 5 opcodes needed to close f64x2's arithmetic family:
  `f64x2.abs` (`0xEC`), `f64x2.min` (`0xF4`), `f64x2.max` (`0xF5`),
  `f64x2.pmin` (`0xF6`), `f64x2.pmax` (`0xF7`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR, a direct structural mirror of PR34's f32x4
  closure. `simd_f64x2.wast` is the upstream corpus's general f64x2
  smoke-test file; the DIFFERENT, SIMPLER "pseudo-min"/"pseudo-max"
  semantics of `pmin`/`pmax` (a plain IEEE-754 `<`-based conditional
  select, no NaN canonicalization -- see wasm-opcodes'
  `SimdOpKind::PminF64x2`/`PmaxF64x2` doc comments) get their own much
  larger dedicated corpus file, `simd_f64x2_pmin_pmax.wast` -- together,
  4687 real directives across 2 files for 5 opcodes.
- `simd_f64x2.wast`: 793/793 `assert_return`, 8/8 `assert_invalid`, 2/2
  modules, ALL 100% passing on the first baseline regen after
  implementation. `simd_f64x2_pmin_pmax.wast`: 3872/3872 `assert_return`,
  6/6 `assert_invalid`, 8/8 `assert_malformed`, 1/1 module, ALL 100%
  passing -- including every one of the corpus's own NaN-operand-order
  vectors, the highest-risk correctness area for this PR (a `pmin`/
  `pmax` implementation that wrongly reused `min`/`max`'s
  NaN-canonicalization logic would have failed a meaningful chunk of
  that file's 3872 `assert_return` directives; it did not). Aggregate
  `assert_return` rose from 38655/38672 to 43320/43337 (+4665 pass,
  `fail` unchanged at 17); `assert_invalid` rose from 1852/1852 to
  1866/1866 (+14, still 100.0% of gradeable directives);
  `assert_malformed` rose from 351/351 to 359/359 (+8, still 100.0% of
  gradeable directives); `module` pass count rose from 1189/1190 to
  1192/1193 (+3, all passing). All deltas are EXACTLY these two files'
  own directive counts -- the pre-existing, unrelated baseline failures
  (17 `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`)
  are byte-for-byte unchanged, confirming zero regressions. See
  `tests/fixtures/testsuite/NOTICE` for the full accounting.

