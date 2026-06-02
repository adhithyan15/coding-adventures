# iir-to-ge225 — IIR → GE-225 machine code backend

**Status:** v0.1.0 — skeleton (A5)
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
| **v0.1.0 (A5 — this PR)** | crate skeleton: any module → single `HLT` (`0x00000`, packed `[0x00, 0x00, 0x00]`) | this PR |
| v0.2.0 (A5+) | `const dest, Int(n)` → `LDA n` (load accumulator immediate, 16-bit n) + `ret`/`ret_void` → HLT | future |
| v0.3.0 (A5++) | Accumulator-based arithmetic (`ADD`, `SUB`) + branch family (`BR`, `BMI`, `BNZ`) | future |
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
```

## Non-goals (v0.1.0)

* No instruction lowering — deferred to A5+.
* No `lang-aot --emit=ge225` wiring — deferred to A5+++.
* No external assembler / linker integration.
* No 4-bit-per-word truncation in middle words (we always pack
  3 bytes per word in v0.1.0).

## Tests (v0.1.0)

* `validate_returns_empty_for_empty_module` — stub validator behaves.
* `lower_emits_exactly_three_bytes` — output shape (one 20-bit word).
* `lower_emits_the_canonical_halt_word` — exact `[0x00, 0x00, 0x00]`.
* `halt_word_constant_pinned_to_zeros` — guards the constant.
* `default_config_has_nonempty_module_name` — config invariant.
* `new_sets_module_name` — builder contract.
* `errors_display_without_panic` — error formatting smoke.
