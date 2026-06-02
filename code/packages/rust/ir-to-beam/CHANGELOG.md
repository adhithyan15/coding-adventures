# Changelog — ir-to-beam

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
