# Changelog

All notable changes to this package will be documented in this file.

## [0.2.17] - 2026-08-18 - SIMD: boolean-reduction/bitmask family (task #153-155)

### Added

- 9 new `SIMD_OPS` entries -- `v128.any_true` (`0x53`) plus
  `ixNxM.all_true`/`bitmask` across all 4 lane widths: `i8x16.all_true`
  (`0x63`), `i8x16.bitmask` (`0x64`), `i16x8.all_true` (`0x83`),
  `i16x8.bitmask` (`0x84`), `i32x4.all_true` (`0xA3`), `i32x4.bitmask`
  (`0xA4`), `i64x2.all_true` (`0xC3`), `i64x2.bitmask` (`0xC4`) -- 90
  SIMD opcodes total, up from 81. `i64x2.all_true`/`bitmask` are the
  first `i64x2` opcodes in this table. Chosen over shift-op and
  i64x2-arithmetic candidates in a fresh prioritization survey: highest
  opcode count (9) behind a single new operand shape and one 72KB
  corpus file, and unlocks real use of the comparison families from
  earlier PRs (a `v128` mask result is otherwise inert without a
  reduction op to consume it). Each sub-opcode byte fetched live from
  the SIMD proposal's own `BinarySIMD.md` and cross-checked against the
  already-implemented `v128.bitselect` (`0x52`)/`i8x16.popcnt`
  (`0x62`)/`i16x8.abs` (`0x80`)/`neg` (`0x81`)/`i32x4.abs` (`0xA0`)/
  `neg` (`0xA1`) entries (every value landed exactly where the
  `abs`/`neg`/`[popcnt]`/`all_true`/`bitmask` per-lane-width pattern
  predicts).
- `SimdOpKind::AnyTrue`/`AllTrueI8x16`/`AllTrueI16x8`/`AllTrueI32x4`/
  `AllTrueI64x2`/`BitmaskI8x16`/`BitmaskI16x8`/`BitmaskI32x4`/
  `BitmaskI64x2`. The first `v128`-in/`i32`-out reduction shape besides
  `ExtractLane`, but with NO lane-index immediate (these reduce over
  ALL lanes, not select one).

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_boolean.wast` and the resulting baseline delta.

## [0.2.16] - 2026-08-18 - SIMD: v128 bitwise family (task #150-152)

### Added

- 6 new `SIMD_OPS` entries -- the lane-width-agnostic raw-byte bitwise
  family, a strategic pivot from "widen the next narrow per-lane-width
  family" to "close the highest-real-world-impact remaining gap" now
  that `i8x16`/`i16x8`/`i32x4` all have complete
  arith+cmp+arith2+widening coverage relative to each other:
  `v128.not` (`0x4D`), `v128.and` (`0x4E`), `v128.andnot` (`0x4F`),
  `v128.or` (`0x50`), `v128.xor` (`0x51`), `v128.bitselect` (`0x52`)
  -- 81 SIMD opcodes total, up from 75. Unlike every prior family,
  these have only ONE spelling each (no `i8x16`/`i16x8`/`i32x4`
  suffix) since they operate on raw bytes, not typed lanes. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i8x16.add` (`0x6E`)/`i32x4.add` (`0xAE`) entries.
- `SimdOpKind::Not`/`And`/`AndNot`/`Or`/`Xor`/`Bitselect`.
  `Bitselect` is the first TERNARY `SimdOpKind` in this crate (pops
  three `v128`s, pushes one): `(a AND c) OR (b AND (NOT c))`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_bitwise.wast` and the resulting baseline delta.

## [0.2.15] - 2026-08-18 - SIMD: i16x8-from-i8x16 widening family (task #147-149)

### Added

- 6 new `SIMD_OPS` entries -- `i16x8`'s own widening family, mirroring
  the already-implemented `i32x4`-from-`i16x8` widening family one
  lane width down, closing the last remaining gap between `i16x8` and
  `i8x16`'s coverage: `i16x8.extadd_pairwise_i8x16_s` (`0x7C`),
  `extadd_pairwise_i8x16_u` (`0x7D`), `extmul_low_i8x16_s` (`0x9C`),
  `extmul_high_i8x16_s` (`0x9D`), `extmul_low_i8x16_u` (`0x9E`),
  `extmul_high_i8x16_u` (`0x9F`) -- 75 SIMD opcodes total, up from 69.
  Unlike the `i32x4`-from-`i16x8` family, there is no
  `i16x8.dot_i8x16_s` -- WASM SIMD does not define a dot-product for
  this lane-width pair, so this family is 6 ops, not 7. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i8x16.add` (`0x6E`)/`i16x8.mul` (`0x95`)/`i16x8.avgr_u` (`0x9B`)/
  `i32x4.dot_i16x8_s` (`0xBA`)/`i8x16.popcnt` (`0x62`)/
  `i32x4.extadd_pairwise_i16x8_s` (`0x7E`) entries (all six matched
  exactly).
- `SimdOpKind::ExtaddPairwiseI8x16S`/`ExtaddPairwiseI8x16U`/
  `ExtmulLowI8x16S`/`ExtmulHighI8x16S`/`ExtmulLowI8x16U`/
  `ExtmulHighI8x16U`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_extadd_pairwise_i8x16.wast`/`simd_i16x8_extmul_i8x16.wast`
corpus files this widening unblocks.

## [0.2.14] - 2026-08-18 - SIMD: i16x8 abs/min/max/avgr_u family (task #144-146)

### Added

- 6 new `SIMD_OPS` entries -- `i16x8`'s own "arith2" family, closing
  the same gap PR8 (task #141-143) just closed for `i8x16` (no
  `i16x8.popcnt` -- WASM SIMD only defines `popcnt` for `i8x16`):
  `i16x8.abs` (`0x80`), `min_s` (`0x96`), `min_u` (`0x97`), `max_s`
  (`0x98`), `max_u` (`0x99`), `avgr_u` (`0x9B`) -- 69 SIMD opcodes
  total, up from 63. All six sub-opcodes are >= 128 (2-byte LEB128),
  same shape as `i16x8`'s own `add`/`sub`/`mul`/`neg`, unlike
  `i8x16`'s own arith2 family (all < 128). Each sub-opcode byte
  fetched live from the SIMD proposal's own `BinarySIMD.md` and
  cross-checked against the already-implemented `i16x8.neg`
  (`0x81`)/`add` (`0x8E`)/`sub` (`0x91`)/`mul` (`0x95`) entries (all
  four matched exactly).
- `SimdOpKind::AbsI16x8`/`MinSI16x8`/`MinUI16x8`/`MaxSI16x8`/
  `MaxUI16x8`/`AvgrUI16x8`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_arith2.wast` corpus file this widening unblocks.

## [0.2.13] - 2026-08-18 - SIMD: i8x16 abs/popcnt/min/max/avgr_u family (task #141-143)

### Added

- 7 new `SIMD_OPS` entries -- `i8x16`'s own "arith2" family, mirroring
  `i32x4`'s own `abs`/`min_s`/`min_u`/`max_s`/`max_u` widening, plus
  two op SHAPES with no `i32x4`/`i16x8` precedent in this table:
  `i8x16.abs` (`0x60`), `popcnt` (`0x62`, lane-wise Hamming weight --
  WASM SIMD only defines `popcnt` for `i8x16`), `min_s` (`0x76`),
  `min_u` (`0x77`), `max_s` (`0x78`), `max_u` (`0x79`), `avgr_u`
  (`0x7B`, lane-wise unsigned rounding average `(a + b + 1) >> 1` --
  WASM SIMD defines `avgr_u` for `i8x16`/`i16x8` but not `i32x4`) -- 63
  SIMD opcodes total, up from 56. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against the already-implemented `i8x16.add` (`0x6E`)/`i8x16.neg`
  (`0x61`)/`i8x16.sub` (`0x71`) entries (all three matched exactly).
- `SimdOpKind::AbsI8x16`/`PopcntI8x16`/`MinSI8x16`/`MinUI8x16`/
  `MaxSI8x16`/`MaxUI8x16`/`AvgrUI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i8x16_arith2.wast` corpus file this widening unblocks.

## [0.2.12] - 2026-08-18 - SIMD: i8x16 comparison family (task #137-140)

### Added

- 10 new `SIMD_OPS` entries -- `i8x16`'s own comparison family, closing
  the same gap PR6 closed for `i16x8`: `i8x16.add`/`sub`/`neg` (PR4)
  landed without a comparison family. `i8x16.eq` (`0x23`), `ne`
  (`0x24`), `lt_s` (`0x25`), `lt_u` (`0x26`), `gt_s` (`0x27`), `gt_u`
  (`0x28`), `le_s` (`0x29`), `le_u` (`0x2A`), `ge_s` (`0x2B`), `ge_u`
  (`0x2C`) -- same boolean-mask convention and signed/unsigned split as
  `i16x8`'s and `i32x4`'s own comparison families, just at the
  narrowest lane width -- 56 SIMD opcodes total, up from 46. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i8x16.add` (`0x6E`)/`i16x8.eq` (`0x2D`) entries (both matched
  exactly), same verification discipline as every widening above.
- `SimdOpKind::EqI8x16`/`NeI8x16`/`LtSI8x16`/`LtUI8x16`/`GtSI8x16`/
  `GtUI8x16`/`LeSI8x16`/`LeUI8x16`/`GeSI8x16`/`GeUI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i8x16_cmp.wast` corpus file this widening unblocks.

## [0.2.11] - 2026-08-18 - SIMD: i16x8 comparison family (task #133-136)

### Added

- 10 new `SIMD_OPS` entries -- `i16x8`'s own comparison family, closing
  the gap left when `i16x8.add`/`sub`/`mul`/`neg` (PR5) landed without
  one (unlike `i32x4`, which got arith+cmp together): `i16x8.eq`
  (`0x2D`), `ne` (`0x2E`), `lt_s` (`0x2F`), `lt_u` (`0x30`), `gt_s`
  (`0x31`), `gt_u` (`0x32`), `le_s` (`0x33`), `le_u` (`0x34`), `ge_s`
  (`0x35`), `ge_u` (`0x36`) -- same boolean-mask convention and signed/
  unsigned split as `i32x4`'s own comparison family, just at `i16x8`'s
  narrower lane width -- 46 SIMD opcodes total, up from 36. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i16x8.add` (`0x8E`)/`i32x4.eq` (`0x37`) entries (both matched
  exactly), same verification discipline as every widening above.
- `SimdOpKind::EqI16x8`/`NeI16x8`/`LtSI16x8`/`LtUI16x8`/`GtSI16x8`/
  `GtUI16x8`/`LeSI16x8`/`LeUI16x8`/`GeSI16x8`/`GeUI16x8`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_cmp.wast` corpus file this widening unblocks.

## [0.2.10] - 2026-08-18 - SIMD: i16x8 first primary-lane slice (task #129-132)

### Added

- 4 new `SIMD_OPS` entries -- the first opcodes in this table where
  `i16x8` is a PRIMARY lane width (produces `i16x8` results), not just
  an INPUT to an `i32x4`-producing widening op (`extadd_pairwise`/
  `dot`/`extmul`, already implemented): `i16x8.neg` (`0x81`),
  `i16x8.add` (`0x8E`), `i16x8.sub` (`0x91`), `i16x8.mul` (`0x95`).
  Unlike `i8x16` (whose spec defines no `mul`), WASM SIMD DOES define
  `i16x8.mul` -- included here since the real upstream corpus file
  bundles all four ops together -- 36 SIMD opcodes total, up from 32.
  Each sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i32x4.add` (`0xAE`)/`i8x16.add` (`0x6E`) entries (both matched
  exactly), same verification discipline as every prior addition.
- `SimdOpKind::AddI16x8`/`SubI16x8`/`MulI16x8`/`NegI16x8`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_arith.wast` corpus file this slice unblocks.

## [0.2.9] - 2026-08-18 - SIMD: i8x16 first slice (task #125-128)

### Added

- 3 new `SIMD_OPS` entries -- this table's first `i8x16` lane-width
  opcodes, a brand-new "first slice" (following the same pattern
  `i32x4` itself started with, not a widening of an existing lane
  width): `i8x16.add` (`0x6E`), `i8x16.sub` (`0x71`), `i8x16.neg`
  (`0x61`). No `i8x16.mul` -- WASM SIMD defines none (8-bit lanes are
  too narrow for a useful lane-wise multiply). No `i8x16.splat`/
  `extract_lane` either: unlike `i32x4`'s original 5-opcode slice,
  they aren't needed, since `v128.const i8x16 ...` (already supported
  for all 6 shapes since PR1b) covers both operand construction and
  result comparison for this slice's own test corpus on its own -- 32
  SIMD opcodes total, up from 29. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against the already-implemented `i32x4.add` (`0xAE`)/`i32x4.abs`
  (`0xA0`) entries (both matched exactly), same verification
  discipline as every widening above.
- `SimdOpKind::AddI8x16`/`SubI8x16`/`NegI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i8x16_arith.wast` corpus file this slice unblocks.

## [0.2.8] - 2026-08-18 - SIMD widening: i32x4-from-i16x8 family (task #121-124)

### Added

- 7 new `SIMD_OPS` entries -- the first opcodes in this table whose INPUT
  lane width (16-bit `i16x8`) differs from their OUTPUT lane width
  (32-bit `i32x4`): `i32x4.extadd_pairwise_i16x8_s` (`0x7E`)/`_u`
  (`0x7F`) (pairwise-add adjacent `i16x8` lanes into `i32x4`),
  `i32x4.dot_i16x8_s` (`0xBA`) (pairwise signed multiply-accumulate
  across all 8 `i16x8` lanes into 4 `i32x4` results), and
  `i32x4.extmul_low_i16x8_s` (`0xBC`)/`extmul_high_i16x8_s` (`0xBD`)/
  `extmul_low_i16x8_u` (`0xBE`)/`extmul_high_i16x8_u` (`0xBF`) (widening
  multiply over only the low or high 4 `i16x8` lanes of each operand) --
  29 SIMD opcodes total, up from 22. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked against
  the already-implemented `i32x4.eq` (`0x37`)/`i32x4.add` (`0xAE`)
  entries (both matched exactly), same verification discipline as every
  widening pass above.
- `SimdOpKind::ExtaddPairwiseI16x8S`/`ExtaddPairwiseI16x8U`/`DotI16x8S`/
  `ExtmulLowI16x8S`/`ExtmulHighI16x8S`/`ExtmulLowI16x8U`/
  `ExtmulHighI16x8U`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i32x4_extadd_pairwise_i16x8.wast`/`simd_i32x4_dot_i16x8.wast`/
`simd_i32x4_extmul_i16x8.wast` corpus files this widening unblocks.

## [0.2.7] - 2026-08-18 - SIMD widening: i32x4 abs/min/max family (task #118-120)

### Added

- 5 new `SIMD_OPS` entries widening the `i32x4` lane coverage further:
  `i32x4.abs` (the second UNARY kind, alongside `neg`) plus the
  `min_s`/`min_u`/`max_s`/`max_u` family (same `v128,v128->v128` binary
  shape as `add`/`sub`/`mul`) -- 22 SIMD opcodes total, up from 17. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i32x4.eq` (`0x37`)/`i32x4.add` (`0xAE`) entries (both matched
  exactly), same verification discipline as every widening pass above.
- `SimdOpKind::Abs`/`MinS`/`MinU`/`MaxS`/`MaxU`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i32x4_arith2.wast` corpus file this widening unblocks.

## [0.2.6] - 2026-08-18 - SIMD widening: i32x4 arithmetic + comparison family (task #113-117)

### Added

- 12 new `SIMD_OPS` entries widening the first slice's `i32x4` lane
  coverage: `i32x4.mul`/`neg`/`sub` (joining the already-implemented
  `add`) and the full comparison family `ne`/`lt_s`/`lt_u`/`gt_s`/
  `gt_u`/`le_s`/`le_u`/`ge_s`/`ge_u` (joining `eq`) -- 17 SIMD opcodes
  total, up from 5. Each sub-opcode byte fetched live from the SIMD
  proposal's own `BinarySIMD.md` and cross-checked against the already-
  implemented `i32x4.eq` (`0x37`)/`i32x4.add` (`0xAE`) entries (both
  matched exactly), same verification discipline as the original 5.
- `SimdOpKind::Neg` -- the first UNARY kind (pops one `v128`, pushes
  one), unlike every other kind so far (all binary, pop two push one).

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast` corpus files this widening
unblocks.

## [0.2.5] - 2026-08-15 - SIMD first slice: SIMD_OPS table (0xFD prefix)

### Added

- `SIMD_OPS`/`get_simd_op`/`get_simd_op_by_name`/`SimdOpInfo`/`SimdOpKind`:
  5 SIMD opcodes for this first slice -- `v128.const` (`0x0C`),
  `i32x4.extract_lane` (`0x1B`), `i32x4.splat` (`0x11`), `i32x4.eq`
  (`0x37`), `i32x4.add` (`0xAE`) -- verified against the SIMD proposal's
  own `BinarySIMD.md`, cross-checked against the W3C core spec for 4 of
  the 5. Structurally different from `ATOMIC_OPS`: SIMD's sub-opcode is a
  **LEB128-encoded `u32`**, not a raw byte (`i32x4.add`'s real value, 174,
  doesn't fit in one byte) -- `SimdOpInfo::sub_opcode` is `u32`, not `u8`.
  `i32x4.extract_lane` is the one opcode beyond the original 4-opcode
  spec scope, added because it's the only way to observe a `v128`
  result's contents as a scalar (see `code/specs/
  W13-wasm-simd-v128-first-slice.md`). 3 new tests, including one pinning
  `i32x4.add`'s value specifically as the multi-byte-LEB128 case (the
  other 4 opcodes are all single-byte-safe).

## [0.2.4] - 2026-08-15 - tail-call opcodes: return_call/return_call_indirect (WASM16)

### Added

- `return_call` (`0x12`) and `return_call_indirect` (`0x13`) -- confirmed
  free/unclaimed sub-opcodes, same immediate shapes as `call`/
  `call_indirect` respectively (a `funcidx`, or a `typeidx`+`tableidx`
  pair). 1 new unit test. See `code/specs/W11-wasm-tail-calls.md`.

## [0.2.3] - 2026-08-15 - atomic memory opcode table, 0xFE prefix (WASM18)

### Added

- New `AtomicOpKind` enum: `Load`, `Store`, `Rmw`, `Cmpxchg`, `Fence`,
  `Notify`, `Wait` -- each documenting its own pop/push shape.
- New `AtomicOpInfo` struct (`name`, `sub_opcode`, `kind`, `value_type`,
  `natural_align`) and `pub static ATOMIC_OPS: &[AtomicOpInfo]`, a single
  shared 67-entry table covering the entire `0xFE`-prefixed atomics
  family in one place -- unlike the `0xFB`/`0xFC` prefix families, whose
  sub-opcode dispatch is duplicated ad hoc per consumer, `wasm-execution`
  and `wasm-validator` both look atomic ops up here rather than
  hand-rolling their own tables, since this family is large (67 entries)
  but fully regular.
- `get_atomic_op(sub_opcode: u8)` / `get_atomic_op_by_name(name: &str)`
  lookup helpers.
- 4 new unit tests, including a count/no-duplicates check
  (`3 + 1 + 7 + 7 + 7*7 == 67`: notify/wait32/wait64 + fence + 7 loads +
  7 stores + 7 RMW-op-kinds × 7 width variants) and a byte-range
  contiguity check (`0x00-0x03` and `0x10-0x4E` present, `0x04-0x0F`
  absent).

### Corrected (implementation-time, vs. the merged W09 spec)

- The merged spec described `memory.atomic.notify`/`wait32`/`wait64` as
  "deliberately absent -- meaningless without real threads." Reading the
  real, pinned-commit `atomic.wast` testsuite file directly showed these
  three instructions are declared inline in the SAME top-level module as
  every other atomic op under test, so omitting them would have made
  `wasm-wast-parser` fail to *parse* the entire file on the first
  occurrence -- losing all conformance value from vendoring it, not just
  leaving 3 opcodes ungraded as intended. They also turn out to have
  fully deterministic, single-agent-computable semantics: `notify`
  always returns 0 woken (no second agent can ever be waiting);
  `wait32`/`wait64` return 1 ("not-equal") or 2 ("timed-out") based on a
  plain memory comparison, since nothing can ever notify a
  single-threaded VM. Both are implemented for real here, not stubbed.

## [0.2.2] - 2026-08-15 - reference-types table/ref.func opcodes (WASM17)

### Added

- `table.get` (0x25) and `table.set` (0x26) -- the previously-reserved MVP
  gap the reference-types proposal fills, `tableidx` immediate. `ref.func`
  (0xD2) -- `funcidx` immediate. All three use the crate's normal metadata
  table with no special-casing (`tableidx`/`funcidx` were already generic
  immediate kinds this crate's decoders understood).
- 1 new unit test (`test_reference_types_opcodes`) round-tripping all three
  by byte and name, and asserting `ref.null`/`ref.is_null` (0xD0/0xD1) are
  deliberately absent from this table -- both already have working
  `wasm-execution`/`wasm-validator` handlers from before WASM17 and were
  never entries here, matching the existing `0xFB`/`0xFC` GC/misc-prefix
  precedent. See `code/specs/W08-wasm-funcref-externref.md`.

## [0.2.1] - 2026-08-13 - sign-extension opcodes (WASM03)

### Added

- The 5 single-byte opcodes from the "sign-extension operators" proposal
  (widely implemented, MVP-adjacent, still single-byte unlike later
  0xFC-prefixed proposals): `i32.extend8_s` (0xC0), `i32.extend16_s`
  (0xC1), `i64.extend8_s` (0xC2), `i64.extend16_s` (0xC3),
  `i64.extend32_s` (0xC4). Each pops one int, sign-extends its low N bits
  to the full i32/i64 width, pushes one int -- category `"conversion"`,
  matching every other unary numeric conversion already in the table.
- 1 new unit test (`test_sign_extension_opcodes`) round-tripping all 5 by
  both byte and name; the existing `test_conversions_stack_effects`
  automatically covers their pop=1/push=1/no-immediates invariant since
  they share the `"conversion"` category.

### Not added (deliberately)

- The same proposal's 8 `trunc_sat` opcodes are NOT in this table -- they
  use a two-byte `0xFC <sub-opcode>` encoding this crate's single-byte
  `OpcodeInfo`/`get_opcode(byte: u8)` model doesn't fit (consistent with
  this crate's existing, pre-2.0 scoping decision to leave `0xFC`-prefixed
  opcodes to their callers -- see the module doc comment). `wasm-wast-parser`
  and `wasm-execution` special-case the `0xFC` prefix directly, the same
  way they already did for bulk-memory's `memory.copy`/`memory.fill`.

### Fixed (also this release, no version-worthy behavior change here)

- `Cargo.toml`'s `version` had drifted to `0.1.0` while this changelog's
  last real entry was already `0.2.0` -- corrected to continue from the
  changelog's actual history, not a code change.

## [0.2.0] - 2026-03-23

### Added

- Complete WASM 1.0 MVP opcode table: 172 instructions from 0x00 to 0xBF
- `OpcodeInfo` struct with `name`, `opcode`, `category`, `immediates`,
  `stack_pop`, `stack_push` fields (all `&'static` for zero heap allocation)
- `OPCODES` static slice — entire table lives in read-only memory at compile time
- `get_opcode(byte: u8) -> Option<&'static OpcodeInfo>` — lookup by byte value
- `get_opcode_by_name(name: &str) -> Option<&'static OpcodeInfo>` — lookup by name
- 17 unit tests covering: count, byte/name lookups, stack effects, immediates,
  uniqueness, category spot checks, category-wide invariants, and doc-tests
- Literate inline comments explaining: stack machine model, immediates encoding,
  memarg format, structured control flow, signedness conventions, IEEE 754 notes,
  and type conversion semantics

### Notes

- The WASM 1.0 MVP has exactly 172 instructions; the "~183" figure sometimes cited
  includes post-MVP proposals (SIMD, bulk-memory, etc.) that use a 0xFC prefix
  encoding not covered here
- Lookup is a linear scan over 183 entries — negligible cost, avoids runtime map

## [0.1.0] - 2026-03-23

### Added

- Initial package scaffolding generated by scaffold-generator
