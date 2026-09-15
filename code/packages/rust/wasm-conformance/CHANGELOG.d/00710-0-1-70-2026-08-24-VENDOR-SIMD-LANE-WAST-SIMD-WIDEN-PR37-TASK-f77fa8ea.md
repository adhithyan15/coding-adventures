## 0.1.70 — 2026-08-24 — vendor simd_lane.wast: SIMD widen PR37 (task #226-228)

### Added

- Vendored `simd_lane.wast` (a new file) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  remaining 10 extract_lane/replace_lane opcodes, CLOSING that family
  across all six SIMD vector shapes, implemented in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser` as part of the
  same PR. This PR also retrofits validation-time lane-index bounds
  checking onto the 4 pre-existing lane-immediate opcodes (previously
  runtime-only), and fixes `wasm-wast-parser`'s lane-index literal
  grammar to accept hex/underscore/leading-zero-decimal forms (was
  plain-decimal-only).
- 475 real directives: 12 module, 274 `assert_return`, 83
  `assert_invalid`, 106 `assert_malformed`. This upstream file also
  bundles `i8x16.shuffle` (a genuinely different, unimplemented opcode,
  out of this PR's scope -- 16 lane-index immediates across two source
  operands, its own future PR) into 4 of its 5 multi-function modules;
  because a WASM module builds as one atomic unit, those 4 modules (and
  the 268 `assert_return` directives invoking their exports, including
  ones that exercise an already-correct extract_lane/replace_lane
  export sitting right next to the unsupported shuffle export) grade
  `not_yet_supported`, a real capability gap, not a `Fail`. Every
  GRADEABLE directive in this file passes at 100%: 8/8 module, 6/6
  `assert_return` (the one shuffle-free multi-function module, memory
  load/store), 83/83 `assert_invalid` (every lane-index-out-of-range
  case, for all 14 lane-immediate opcodes old and new), 106/106
  `assert_malformed` (sign-prefixed/float/empty-argument lane-index
  literal rejection). See the vendored `NOTICE` file for the full
  breakdown and the exact aggregate before/after numbers, including an
  unplanned second effect: the literal-grammar fix also retroactively
  unlocks a previously-unbuildable module in the already-vendored
  `simd_splat.wast` (+1 module, +43 `assert_return`, both from
  `not_yet_supported` to passing).

