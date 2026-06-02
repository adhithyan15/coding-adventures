# Changelog — iir-to-ge225

All notable changes to this crate are documented here.

## v0.3.0 — 2026-06-02 — A5++ ACC-first GP allocator + `mov`

Second lowering increment.  Adds the GE-225 GP register file
(`r0..r15`), the `STA r` and `LD r` opcodes, the `mov dest, src`
instruction lowering, and an ACC-first linear allocator over the
17-slot pool (ACC + r0..r15).

| IIR op | GE-225 lowering |
|--------|-----------------|
| `const dest, Int(n)` | `(STA r_evict)?` + `LDA n` |
| `mov dest, src` | `(STA r_evict_src)?` + `LD r_src` + `STA r_dest` |
| `ret <var>` | `(LD r_var)?` + `HLT` |
| `ret_void` | `HLT` |

### Added

- `pub const STA_OPCODE_NIBBLE: u8 = 0x2` — GE-225 store-with-XCH-
  semantics opcode.  Word layout `[0x02, 0x00, r]` where `r` is the
  4-bit register index in the low nibble of byte 2.
- `pub const LD_OPCODE_NIBBLE: u8 = 0x3` — load-ACC-from-register.
  Word layout `[0x03, 0x00, r]`.  Pure copy (does not modify `r`).
- `IIRGe225Error::OutOfRegisters { function, name }` — fired when
  the 17-slot ACC + r0..r15 pool is exhausted (18th `const` of a
  function); memory spilling is not yet supported.

### Allocator strategy (ACC-first linear)

The first `const` of each function lands in the accumulator.  Each
subsequent `const` first evicts the current ACC owner to the
next-free GP register via `STA r`, then emits `LDA n`.  This
preserves the v0.2.0 6-byte trivial-case ROM for `const v; ret v`.

`mov dest, src` follows the same pattern as `iir-to-intel4004`
v0.3.0: if `src` lives in ACC, evict it first to a stable register
home, then `LD r_src` + `STA r_dest` (XCH semantics) places src's
value into a fresh register for `dest`.

`ret <var>` is now allocator-aware: if `var` is the current ACC
owner it emits just `HLT`; otherwise it reloads via `LD r_var`
first.

### Opcode map (cumulative through v0.3.0)

| Nibble | Mnemonic | Word |
|--------|----------|------|
| `0x0` | `HLT`   | `[0x00, 0x00, 0x00]` |
| `0x1` | `LDA n` | `[0x01, hi, lo]` |
| `0x2` | `STA r` | `[0x02, 0x00, r]` |
| `0x3` | `LD r`  | `[0x03, 0x00, r]` |

Future slices reserve `0x4..0xF` for `ADD`, `SUB`, `BR`, `BMI`,
`BNZ`, `JSR`, etc.

### A note on `STA` semantics

Real GE-225 silicon's `STA` was a pure store.  This skeleton models
`STA r` as exchange-with-ACC (XCH semantics) so the eviction
pattern is **one instruction** instead of two.  Documented as a
deliberate educational simplification; future versions may split
this back into pure `STA` + restore-via-`LD` if historical
fidelity becomes important.

### Tests (21 unit + 1 doctest, all passing)

New coverage:
- `sta_opcode_nibble_pinned_to_0x2`, `ld_opcode_nibble_pinned_to_0x3`.
- `two_consts_then_ret_of_current_acc_evicts_first_to_r0` — exact
  4-word sequence `LDA + STA + LDA + HLT`.
- `ret_of_evicted_var_emits_ld_to_reload` — 5-word sequence
  including the `LD r0` reload.
- `mov_when_src_in_acc_evicts_then_ld_sta` — 6-word `mov`-from-ACC.
- `mov_when_src_already_in_register_skips_eviction` — 8-word
  chained `mov` pair.
- `allocator_at_seventeenth_const_still_succeeds` — pool ceiling.
- `allocator_exhausts_on_eighteenth_const` — fails on v16 eviction.
- `mov_from_undefined_src_errors` — `UndefinedVariable`.
- `unsupported_op_add_errors` — `UnsupportedOp { op: "add" }`.

Regressions from v0.2.0 still pinned:
- `trivial_rom_is_still_six_bytes` (8 N values).
- `ret_void_only_still_emits_just_halt`.
- `const_negative_one_still_uses_twos_complement`.
- `const_out_of_range_still_errors`.
- All 6 `IIRGe225Error` variants Display correctly (incl. new
  `OutOfRegisters`).

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors `iir-to-intel4004` v0.3.0 (A4++) — same 17-slot pool
  capacity, same eviction-on-second-const allocator shape.

## v0.2.0 — 2026-06-02 — A5+ first real lowering

First lowering increment after the A5 skeleton.  Adds:

| IIR op | GE-225 lowering |
|--------|-----------------|
| `const dest, Int(n)` (16-bit signed/unsigned) | `LDA n` — 20-bit word: opcode nibble `0x1` + 16-bit immediate, packed `[0x01, hi, lo]` |
| `const dest, Bool(b)` | `LDA 0` or `LDA 1` |
| `ret <var>` | `HLT` (the all-zeros 20-bit word) — but only if `<var>` is the current ACC owner |
| `ret_void` | `HLT` |

### Added

- `pub const LDA_OPCODE_NIBBLE: u8 = 0x1` — the GE-225 `LDA` opcode
  nibble, lives in the low 4 bits of byte 0 in the 3-byte word
  packing.
- `IIRGe225Error::UndefinedVariable { function, name }` — fired
  when `ret <var>` references a var that is either never bound or
  no longer the current accumulator owner.  v0.2.0 has only the
  accumulator; multi-register liveness arrives in A5++.
- Real per-function lowering with accumulator tracking:
  `env: HashMap<String, u8>` (single sentinel `ACC_MARKER = 16`
  in v0.2.0) and `acc_owner: Option<String>`.  Mirrors the
  iir-to-intel4004 v0.2.0 → v0.3.0 progression.

### Acceptance test

The trivial-case ROM (`const v = N; ret v`) is always 6 bytes —
3 for `LDA N` + 3 for `HLT` — regardless of `N`.  Pinned by the
`trivial_rom_is_six_bytes` test across N ∈ {0, 1, 42, 255, 256,
32767, -1, -32768}.

### Word format pinned by this PR

```
byte 0: 0000 OOOO   (top 4 bits zero + 4-bit opcode nibble)
byte 1: IIII IIII   (high 8 bits of the 16-bit immediate)
byte 2: IIII IIII   (low  8 bits of the 16-bit immediate)
```

Opcodes used in v0.2.0: `0x0` (HLT), `0x1` (LDA).  Future opcodes
will populate `0x2..0xF`.

### Tests

21 unit + 1 doctest passing.  New coverage:
- LDA opcode nibble pinned.
- `const N; ret v` for N ∈ {0, 5}: exact byte sequence.
- max positive (32767), min negative (-32768), and -1: two's
  complement reinterpretation.
- 16-bit overflow (65536) errors with `InvalidOperand`.
- `Bool(true)` / `Bool(false)` lower to `LDA 1` / `LDA 0`.
- `ret_void`-only function: 3-byte HLT.
- Trivial-case 6-byte ROM size pinned across 8 value points.
- Multi-const where ret targets the current ACC owner: works.
- `ret` of a stale ACC owner: `UndefinedVariable`.
- `ret` of a never-defined variable: `UndefinedVariable`.
- `mov` (unsupported in v0.2.0): `UnsupportedOp { op: "mov" }`.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Mirrors iir-to-intel4004 v0.2.0 (A4+) exactly in spirit.

## v0.1.0 — 2026-06-02 — A5 skeleton

Initial release.  Establishes the IIR → GE-225 backend's public
surface as the fifth architecture-backend slot (after iir-to-riscv,
iir-to-intel8008, iir-to-armv7, iir-to-intel4004) — and the most
exotic by a wide margin (20-bit words, mainframe accumulator model,
1959 silicon).

### Added

- `IIRGe225Config` — module-name-carrying config (reserved for
  future symbol-table / `.bin` header use).
- `IIRGe225Error` — backend-side error type with four variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`,
  `InvalidOperand`.  Mirrors the iir-to-intel4004 / iir-to-armv7 /
  iir-to-intel8008 / iir-to-riscv error surface so callers can
  pattern-match identically across backends.
- `validate_for_ge225(&IIRModule) -> Vec<String>` — stub validator
  (always returns `[]` in v0.1.0).
- `lower_iir_to_ge225(&IIRModule, &IIRGe225Config) -> Result<Vec<u8>,
  IIRGe225Error>` — lowering entry point.  Currently emits the
  3-byte canonical HLT sentinel regardless of input.
- `pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00]` — the all-zeros
  20-bit GE-225 HLT word, packed big-endian.  Documented choice
  (vs branch-to-self / unimplemented-opcode) recorded in the spec
  and the constant's doc comment.

### Why the all-zeros HLT halt sentinel?

The GE-225's `HLT` instruction is the all-zeros 20-bit word.
Emitted at the start of program ROM, it halts the machine
deterministically — recognized by every GE-225 simulator and the
historical silicon.  Alternative halt idioms (branch-to-self) would
work but produce less visually obvious bytes.

### Word packing

GE-225 words are 20 bits; we pack each into 3 bytes (24 bits),
big-endian, with the top 4 bits of byte 0 always zero.  A downstream
simulator reads 3 bytes per instruction, masks off the top 4 bits,
and recovers the original 20-bit word.

### Scope notes

- No instruction lowering — deferred to v0.2.0 (A5+).
- No `lang-aot --emit=ge225` wiring — deferred to A5+++.
- No external assembler / linker integration.

### Tests

7 tests covering: empty-module validation, output shape, exact
halt bytes, `HALT_WORD` constant pinning, default-config invariant,
`IIRGe225Config::new` builder contract, and Display smoke for all
four `IIRGe225Error` variants.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
