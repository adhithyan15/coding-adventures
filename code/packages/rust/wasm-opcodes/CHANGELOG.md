# Changelog

All notable changes to this package will be documented in this file.

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
