# iir-to-ge225 — IIR → GE-225 machine code backend

**Status:** v0.9.0 — neg via 0-src, BASIC unary minus works end-to-end (A5++++++++++)
**Plan:** [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md) §A5
**Related:** [`iir-to-intel4004`][i4004], [`iir-to-intel8008`][i8008], [`iir-to-riscv`][rv], [`iir-to-armv7`][arm]

[i4004]: ../packages/rust/iir-to-intel4004/
[i8008]: ../packages/rust/iir-to-intel8008/
[rv]: ../packages/rust/iir-to-riscv/
[arm]: ../packages/rust/iir-to-armv7/

## Why a new crate?

The **GE-225** (1959) was the General Electric mainframe at
Dartmouth College where **John Kemeny and Thomas Kurtz designed
Dartmouth BASIC in 1964**.  BASIC ran on this very machine — the
1.7 microsecond cycle time and 20-bit word size shaped the
language's defaults in ways still visible 60 years later.

In this codebase the GE-225 is primarily a **BASIC fit** per
MULTILANG-ARCHITECTURE-BACKENDS.md §A5.  Compiling BASIC source
to GE-225 bytes is a small piece of computing history made
queryable through the LANG VM pipeline.

Adding the GE-225 gives us:

1. **Historical fidelity for Dartmouth BASIC.**  BASIC programs
   round-trip to the silicon they were designed for.
2. **A fifth architecture backend** alongside RV32I (A1), Intel
   8008 (A2), ARMv7 (A3), and Intel 4004 (A4).  The five span
   width / age / programming-model diversity:
   - RV32I: 32-bit clean RISC (modern).
   - Intel 8008: 8-bit accumulator CISC (Oct's native, 1972).
   - ARMv7: 32-bit RISC + cond-prefix (phone-class, 2005).
   - Intel 4004: 4-bit accumulator (world's first commercial μP, 1971).
   - **GE-225**: 20-bit mainframe (Dartmouth BASIC's birthplace, 1959).

3. **Stress-tests the IIR's neutrality across the most exotic
   target** in the lane: 20-bit words don't fit cleanly in
   bytes, and the GE-225's accumulator + magnetic-drum-style
   memory model is unlike anything else.

## Why `Vec<u8>` output for a 20-bit-word machine?

* **Cross-backend uniformity.**  Every other backend emits
  `Vec<u8>` (Intel 8008, Intel 4004) or a `Vec<u32>` that's
  trivially flattened to bytes by `lang-aot` (RV32I, ARMv7).
  Bytes round-trip through every host filesystem without
  alignment surprises.
* **3-byte word packing.**  Each 20-bit GE-225 word is emitted
  as 3 bytes (24 bits total) with the top 4 bits zero,
  big-endian-style.  This wastes 4 bits per word (~17 % overhead)
  but means a downstream simulator can read 3 bytes, mask off
  the top 4 bits, and have the original 20-bit word.

## The halt sentinel — all-zeros HLT

The GE-225's `HLT` instruction is the all-zeros 20-bit word.
Emitted at the start of a program ROM, this immediately halts
the machine.

```text
20-bit word: 0000_0000_0000_0000_0000
   ↓ packed into 3 big-endian bytes (top 4 bits of byte 0 are zero)
[0x00, 0x00, 0x00]
```

This is the documented HLT encoding in the GE-225 reference
manual.  Alternative halt idioms (e.g. unconditional branch to
self, `BR $.` in mnemonics) would also work but produce less
visually obvious bytes; the all-zeros HLT is preferred for
skeleton purposes.

## Pipeline

```text
IIRModule
  → validate_for_ge225()      pre-flight, returns Vec<String>
  → lower_iir_to_ge225()      returns Vec<u8> (20-bit words packed 3 bytes each)
  → (optional)
      • a GE-225 simulator (mostly historical now, but a few exist)
      • write to .bin + custom emulator
```

## Scope by version

| Version | Scope | Status |
|---------|-------|--------|
| v0.1.0 (A5) | crate skeleton: any module → single `HLT` (`0x00000`, packed `[0x00, 0x00, 0x00]`) | **merged** |
| v0.2.0 (A5+) | `const dest, Int(n)` → `LDA n` + `ret`/`ret_void` → HLT.  Single-ACC liveness model. | **merged** |
| v0.3.0 (A5++) | ACC-first GP register allocator over ACC + r0..r15 (17-slot pool) + `STA r` (0x2, XCH semantics) + `LD r` (0x3) + `mov dest, src` lowering | **merged** |
| v0.4.0 (A5+++) | Accumulator-based arithmetic: `add` and `sub` IIR ops → `ADD r` (0x4) and `SUB r` (0x5) opcodes preceded by `LD r_lhs` staging | **merged** |
| (A5++++ in lang-aot v0.11.0) | `lang-aot --emit=ge225` wiring (aliases `ge-225`, `225`) | **merged** |
| v0.5.0 (A5+++++) | Branch family `BR` (0x6), `BNZ` (0x7), `BZ` (0x8) + per-function label backpatching for `label`, `jmp`, `jmp_if_true`, `jmp_if_false` IIR ops | **merged** |
| v0.6.0 (A5++++++) | Call/return discipline: `JSR` (0x9), `RTS` (0xA) + module-level `call` backpatching; `BMI` (0xB) reserved; non-entry-fn ret emits RTS instead of HLT | **merged** |
| v0.7.0 (A5+++++++) | Six comparison ops `cmp_lt`/`cmp_eq`/`cmp_ne`/`cmp_le`/`cmp_gt`/`cmp_ge` via SUB-then-test boolean materialization.  Activates `BMI` (0xB) for the lt/le/gt/ge family.  Operand-swap pattern handles gt/ge with no new code | **merged** |
| (A5++++++++ in lang-aot tests) | BASIC end-to-end smoke tests: `LET A = 5`, `LET A = 1 + 2`, PRINT-gap documentation | **merged** |
| v0.8.0 (A5+++++++++) | `call_builtin` no-op lowering: closes BASIC PRINT gap.  No-dest case emits zero bytes; with-dest case emits `LDA 0` placeholder | **merged** |
| **v0.9.0 (A5++++++++++ — this PR)** | `neg dest, src` lowering via `LDA 0 + SUB r_src` (0 - src = -src).  Closes BASIC unary-minus gap.  15-byte trivial ROM for `const v=N; neg w, v; ret w` | this PR |
| v0.4.0 (A5+++) | `lang-aot --emit=ge225` wiring + BASIC end-to-end | future |

## Public surface (v0.1.0)

```rust
pub struct IIRGe225Config { pub module_name: String }
impl IIRGe225Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRGe225Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_ge225(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_ge225(
    module: &IIRModule,
    cfg: &IIRGe225Config,
) -> Result<Vec<u8>, IIRGe225Error>;

pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00];
pub const LDA_OPCODE_NIBBLE: u8 = 0x1;  // v0.2.0
pub const STA_OPCODE_NIBBLE: u8 = 0x2;  // v0.3.0 — XCH semantics
pub const LD_OPCODE_NIBBLE:  u8 = 0x3;  // v0.3.0 — pure copy
pub const ADD_OPCODE_NIBBLE: u8 = 0x4;  // v0.4.0 — ACC ← ACC + r
pub const SUB_OPCODE_NIBBLE: u8 = 0x5;  // v0.4.0 — ACC ← ACC - r
pub const BR_OPCODE_NIBBLE:  u8 = 0x6;  // v0.5.0 — unconditional branch
pub const BNZ_OPCODE_NIBBLE: u8 = 0x7;  // v0.5.0 — branch if ACC ≠ 0
pub const BZ_OPCODE_NIBBLE:  u8 = 0x8;  // v0.5.0 — branch if ACC = 0
pub const JSR_OPCODE_NIBBLE: u8 = 0x9;  // v0.6.0 — jump subroutine
pub const RTS_OPCODE_NIBBLE: u8 = 0xA;  // v0.6.0 — return from subroutine
pub const BMI_OPCODE_NIBBLE: u8 = 0xB;  // v0.6.0 — reserved (no IIR op yet)
pub const RTS_WORD: [u8; 3] = [0x0A, 0x00, 0x00];  // v0.6.0
```

## Word format

```
byte 0: 0000 OOOO   (top 4 bits zero + 4-bit opcode nibble)
byte 1: IIII IIII   (high 8 bits of the 16-bit immediate)
byte 2: IIII IIII   (low  8 bits of the 16-bit immediate)
```

Opcodes assigned through v0.6.0: `0x0` (HLT), `0x1` (LDA),
`0x2` (STA — XCH semantics), `0x3` (LD), `0x4` (ADD), `0x5` (SUB),
`0x6` (BR), `0x7` (BNZ), `0x8` (BZ), `0x9` (JSR), `0xA` (RTS),
`0xB` (BMI — reserved).  Future slices take `0xC..0xF`.

## Non-goals (v0.7.0)

* No call arguments — calls are still zero-arg, single-return-value
  (via ACC).  Argument-passing arrives in a future slice.
* No memory spilling beyond the 17-slot ACC + r0..r15 pool — future.
* No external assembler / linker integration.
* No peephole optimisation: `add c, c, x` always emits the
  `LD r_c` even when `c` is already in ACC.  Same for cmp's LD.
* No cross-function branches — labels are per-function.
* No `mul` / `div` / shift opcodes — future ISA extensions.

## Tests (v0.2.0 — 21 unit + 1 doctest)

* `validate_returns_empty_for_empty_module` — stub validator behaves.
* `empty_module_still_emits_the_canonical_halt_word` — v0.1.0 contract preserved.
* `halt_word_constant_pinned_to_zeros` — `HALT_WORD` constant.
* `lda_opcode_nibble_pinned_to_0x1` — `LDA_OPCODE_NIBBLE` constant.
* `default_config_has_nonempty_module_name` / `new_sets_module_name` — config invariants.
* `errors_display_without_panic` — Display covers all 5 error variants.
* `const_5_then_ret_lowers_to_lda_5_then_halt` — canonical 6-byte ROM.
* `const_0_then_ret_emits_lda_zero` — LDA 0 byte 0 visibly has opcode nibble.
* `const_max_positive_16bit_emits_correct_bytes` — n=32767 packed correctly.
* `const_min_negative_16bit_emits_correct_bytes` — n=-32768 → 0x8000.
* `const_negative_one_uses_twos_complement` — n=-1 → 0xFFFF.
* `const_bool_true_emits_lda_one` / `const_bool_false_emits_lda_zero`.
* `const_out_of_range_errors` — 65536 → `InvalidOperand`.
* `ret_void_only_emits_just_halt` — 3-byte HLT.
* `trivial_rom_is_six_bytes` — 6-byte invariant across 8 N values.
* `multiple_consts_then_ret_of_current_acc_works` — `LDA; LDA; HLT` = 9 bytes.
* `ret_of_stale_acc_owner_errors_in_v0_2_0` — `UndefinedVariable`.
* `ret_of_completely_undefined_variable_errors` — `UndefinedVariable`.
* `unsupported_op_errors_with_op_name` — `mov` → `UnsupportedOp`.
