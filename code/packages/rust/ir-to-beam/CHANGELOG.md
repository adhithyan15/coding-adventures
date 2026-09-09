# Changelog — ir-to-beam

## [0.3.0] — 2026-09-09

### Fixed (large positive `I`-tagged literals silently went negative)

- `encode_compact_term`'s "Large form" (`value >= 2048`) built the operand's
  big-endian byte string with `value_to_be_bytes`, which stripped every
  leading `0x00` byte regardless of the operand's tag. That rule is correct
  for `U` (unsigned) operands — the loader reads them as a plain magnitude —
  but wrong for `I` (signed) operands, which the loader reads as big-endian
  **two's complement**: if the stripped leading byte's high bit ends up set,
  a large positive literal is misread back as negative.
  - Discovered by VM-040's COBOL BEAM signed/algebra probe: an ordinary
    `COMPUTE`'s scale-12 intermediate multiplied a literal by `10^12`, and
    `2 * 1_000_000_000_000`'s minimal 5-byte magnitude (`E8 D4 A5 10 00`) has
    its leading byte's high bit set, so it came back as a large *negative*
    number on real `erl` — corrupting every arithmetic result downstream.
  - `value_to_be_bytes` now takes a `signed: bool`. The `U` path is
    unchanged (strip every leading `0x00`). The new `I` path finds the
    minimal two's-complement encoding directly: strip a leading `0x00` only
    while the *next* byte's high bit is still clear (a positive value), and
    symmetrically strip a leading `0xFF` only while the next byte's high bit
    is still set (an already-negative value) — the same rule ASN.1 DER uses
    for `INTEGER`. Values from `i32::MAX + 1` through the low bits of
    `u32::MAX`, and every scale-12-style intermediate whose magnitude's
    leading byte happened to have its high bit set, were affected; `2^32`
    exactly and other top-bit-clear magnitudes already round-tripped
    correctly and are unchanged.
  - The compact-term "Large form" header can only express byte lengths
    `2..=9` (`len_field = length - 2` in 3 bits), so the signed minimal
    length is floored at 2 bytes even when the true minimal two's-complement
    encoding of a SMALL magnitude (e.g. a small negative literal like `-7`,
    whose full `u64` bit pattern is enormous and so reaches the Large form
    purely by its sign, not its size) would otherwise fit in 1 — the naive
    signed-minimal rule alone underflowed `(length - 2) as u8` for exactly
    this case.
- New tests: `test_signed_large_form_pads_high_bit`,
  `test_signed_large_form_round_trips_via_i64_reader` (round-trips a table of
  boundary values, including `i64::MIN`/`i64::MAX`, through a real
  sign-extending big-endian reader), and
  `test_encode_compact_term_small_negative_does_not_underflow` pin the fix
  and its length floor. `test_value_to_be_bytes_zero`/`_256` updated for the
  new `signed` parameter.

## [0.2.1] — 2026-06-02

### Fixed (atom table now loads on OTP 27 *and* 28)

- Reverted `encode_atu8` to the **classic positive-count `AtU8` format** (a
  big-endian `u32` atom count, then a single raw length byte per atom). The
  0.2.0 "OTP 25+" rewrite (negative count + `(len << 4)` / `[0x08, len]`
  nibble-packed lengths) was based on a misdiagnosis: OTP 28 accepts *both*
  formats, but **OTP 27 rejects the nibble-packed form** with
  `beam_load.c: corrupt atom table`. Every BEAM module the encoder produced
  therefore failed to load on OTP 27 (CI's pinned runtime) with `undef`,
  breaking `iir-to-beam`'s `test_65`/`test_66` real-`erl` round-trips.
- Verified empirically by round-tripping `iir_arith_test:main/0` through `erl`
  on both runtimes:

  | atom form    | OTP 27 load | OTP 28 load |
  |--------------|-------------|-------------|
  | nibble-packed| `badfile` ✗ | `ok` ✓      |
  | classic      | `ok` ✓      | `ok` ✓      |

  The classic form covers every atom we emit (capped at 255 bytes by
  `validate_for_beam`), so it is now emitted unconditionally.

## [0.2.0] — 2026-05-12

### Fixed (OTP 25+ BEAM file format compatibility)

#### `encode_atu8` — new atom-table format (OTP 25+)

- Previously wrote the **old** `AtU8` format: a big-endian `u32` count
  followed by each atom's raw byte-length prefix.  OTP 25 introduced a
  breaking change: the count field is now a **negative signed `i32`** (the
  negation signals the new format), and each atom's length is stored as
  `(len << 4)` for lengths 0–15 or `[0x08, len]` for lengths 16–255.
- `encode_atu8` is rewritten to produce the new format.  OTP 28 C-loader
  rejects files with the old positive-count encoding with "compiled for an
  old version of the runtime system".

#### `encode_beam` — required Attr, CInf, and Meta chunks

- Added three chunks that OTP 25+ requires before the module is loadable:
  - **`Attr`** — module attributes, ETF-encoded as an empty proplist `[]`
    (ETF nil: `0x6a`).
  - **`CInf`** — compiler info, same empty proplist.
  - **`Meta`** — OTP 25+ mandatory metadata chunk containing
    `[{enabled_features,[]}]` as a canonical ETF binary.
- Without these chunks OTP 28 reports "compiled for an old version of the
  runtime system" at the C-loader level even after the AtU8 fix.

#### New constants

- `ETF_NIL` — one-byte ETF nil marker (`0x6a`), used in Attr / CInf payloads.
- `ETF_META` — hard-coded ETF payload for `[{enabled_features,[]}]`.

#### Tests

- Replaced the single `test_encode_atu8` round-trip test with 5 focused
  tests: `test_encode_atu8_empty`, `test_encode_atu8_single_short_atom`,
  `test_encode_atu8_long_atom`, `test_encode_atu8_multiple_atoms`,
  `test_encode_atu8_atom_exactly_16_bytes`.
- Added `test_encode_beam_contains_cinf_chunk`,
  `test_encode_beam_contains_meta_chunk`,
  `test_etf_nil_is_correct`, `test_etf_meta_starts_with_version_tag`,
  `test_meta_chunk_payload_is_correct_etf`.

---

## [0.1.0] — 2026-04-28

### Added

- **`encoder` module** — BEAM IFF container builder.
  - `BEAMTag` — 3-bit compact-term type tags (U, I, A, X, Y, F).
  - `BEAMOperand` / `BEAMInstruction` — typed instruction representation.
  - `BEAMImport` / `BEAMExport` — import and export table row types.
  - `BEAMModule` — complete in-memory BEAM module.
  - `encode_compact_term()` — variable-width BEAM operand encoding
    (small / medium / large forms).
  - `encode_beam()` — serialize a `BEAMModule` to a complete `.beam` binary
    with AtU8, Code, StrT, ImpT, ExpT, LocT, Attr, CInf chunks.

- **`backend` module** — IR → BEAM lowering pass.
  - `BEAMBackendConfig` — lowering configuration (module name).
  - `BEAMBackendError` — typed errors (ValidationFailed, UnsupportedOp,
    InvalidOperand, UndefinedLabel).
  - `validate_for_beam()` — pre-flight validation (detects unsupported ops,
    empty entry label).
  - `lower_ir_to_beam()` — two-pass lowering:
    - Pass 1: collect LABEL instructions → assign BEAM label numbers (starting
      at 3; 1 and 2 are reserved for the `func_info` preamble).
    - Pass 2: translate each IR instruction to BEAM bytecode using the
      mapping described below.
  - Supported opcodes: LABEL, LOAD_IMM, ADD, ADD_IMM, SUB, AND, AND_IMM,
    JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET, HALT, NOP, COMMENT.
  - Synthesised ops: ADD_IMM / AND_IMM are expanded to MOVE + GC_BIF2;
    BRANCH_Z → `is_ne_exact`; BRANCH_NZ → `is_eq_exact`.
  - Unsupported (validation errors): LOAD_BYTE, STORE_BYTE, LOAD_WORD,
    STORE_WORD, LOAD_ADDR, SYSCALL, CMP_EQ, CMP_NE, CMP_LT, CMP_GT.

- **`codegen` module** — LANG20 adapter.
  - `BEAMCodeGenerator` — implements `CodeGenerator<IrProgram, BEAMModule>`.
    - `name()` → `"beam"`.
    - `validate()` → delegates to `validate_for_beam`.
    - `generate()` → delegates to `lower_ir_to_beam`, panics on invalid IR.
  - `BEAMCodeGenerator::new(module_name)` and `::default_module()`.

- **Tests** — 14 encoder tests + 24 backend tests + 11 codegen tests = 49 total.
