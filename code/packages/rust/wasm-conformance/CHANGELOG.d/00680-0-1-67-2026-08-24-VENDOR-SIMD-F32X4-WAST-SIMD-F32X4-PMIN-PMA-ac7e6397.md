## 0.1.67 — 2026-08-24 — vendor simd_f32x4.wast/simd_f32x4_pmin_pmax.wast: SIMD widen PR34 (task #217-219)

### Added

- Vendored `simd_f32x4.wast` and `simd_f32x4_pmin_pmax.wast` (both new
  files, not re-fetches of already-vendored ones) at the existing
  pinned commit `28864811cf03bdbf880733786148feaba339582d`. This PR
  implements the 3 opcodes needed to close f32x4's arithmetic family:
  `f32x4.max` (`0xE9`), `f32x4.pmin` (`0xEA`), `f32x4.pmax` (`0xEB`),
  implemented in `wasm-opcodes`/`wasm-execution`/`wasm-validator`/
  `wasm-wast-parser` as part of the same PR. `simd_f32x4.wast` is the
  upstream corpus's general f32x4 smoke-test file; the DIFFERENT,
  SIMPLER "pseudo-min"/"pseudo-max" semantics of `pmin`/`pmax` (a plain
  IEEE-754 `<`-based conditional select, no NaN canonicalization -- see
  wasm-opcodes' `SimdOpKind::PminF32x4`/`PmaxF32x4` doc comments) get
  their own much larger dedicated corpus file, `simd_f32x4_pmin_pmax
  .wast` -- together, 4674 real directives across 2 files for 3
  opcodes, the best directive-per-opcode ratio in this campaign so far.
- `simd_f32x4.wast`: 772/772 `assert_return`, 8/8 `assert_invalid`,
  8/8 `assert_malformed`, 2/2 modules, ALL 100% passing on the first
  baseline regen after implementation. `simd_f32x4_pmin_pmax.wast`:
  3872/3872 `assert_return`, 6/6 `assert_invalid`, 8/8
  `assert_malformed`, 1/1 module, ALL 100% passing -- including every
  one of the corpus's own NaN-operand-order vectors, the highest-risk
  correctness area for this PR (a `pmin`/`pmax` implementation that
  wrongly reused `min`/`max`'s NaN-canonicalization logic would have
  failed a meaningful chunk of that file's 3872 `assert_return`
  directives; it did not). Aggregate `assert_return` rose from
  34011/34028 to 38655/38672 (+4644 pass, `fail` unchanged at 17);
  `assert_invalid` rose from 1838/1838 to 1852/1852 (+14, still 100.0%
  of gradeable directives); `assert_malformed` rose from 335/335 to
  351/351 (+16, still 100.0% of gradeable directives); `module` pass
  count rose from 1186/1187 to 1189/1190 (+3, all passing). All deltas
  are EXACTLY these two files' own directive counts -- the pre-existing,
  unrelated baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register`) are byte-for-byte unchanged,
  confirming zero regressions. See `tests/fixtures/testsuite/NOTICE`
  for the full accounting.

