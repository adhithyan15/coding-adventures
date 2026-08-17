# iir-to-intel4004 — IIR → Intel 4004 machine code backend

> ⚠ **REMOVED (2026-08-17).** The `iir-to-intel4004` crate has been
> deleted. It lowered IIR directly to machine bytes, skipping type
> monomorphization and the `Backend` trait; it was superseded by
> `intel4004-encoder` + `intel4004-backend`, which lower CIR instead. See
> [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md).
> This spec is preserved as a historical record of the original design.

**Status:** v0.1.0 — skeleton (A4)
**Plan:** [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md) §A4
**Related:** [`iir-to-intel8008`][i8008], [`iir-to-riscv`][rv], [`iir-to-armv7`][arm]

[i8008]: ../packages/rust/iir-to-intel8008/
[rv]: ../packages/rust/iir-to-riscv/
[arm]: ../packages/rust/iir-to-armv7/

## Why a new crate?

The Intel 4004 (1971) was the **world's first commercial
microprocessor**.  Tiny ISA, 4-bit data, 12-bit ROM addresses,
single 4-bit accumulator, 16 4-bit registers organized as 8 register
pairs, tiny ROM (4 KiB max) and RAM (640 bits max).

In this codebase the 4004 is primarily a **Brainfuck fit** — BF's
minimal needs (single tape pointer, ±1 increment ops, conditional
jump on zero) actually do map cleanly to a 4004's accumulator-and-
loop programming model.

Adding the 4004 as a backend gives us:

1. **Historical fidelity.**  The 4004 is where the entire
   modern-microprocessor lineage began.  Compiling LANG VM
   programs to 4004 silicon is a small piece of computing
   history made queryable.
2. **A fourth architecture backend** alongside RV32I (A1),
   Intel 8008 (A2), and ARMv7 (A3).  The four sit at very
   different points in the design space:
   - RV32I: clean modern 32-bit RISC.
   - Intel 8008: irregular 8-bit accumulator CISC (Oct's native).
   - ARMv7: 32-bit RISC + cond-field on every instruction.
   - **Intel 4004**: 4-bit data, 12-bit ROM addresses, single
     accumulator, register pairs.  The most constrained target
     in the lane by a wide margin.
3. **Stress-tests the IIR's neutrality.**  If a 4-bit-data,
   4 KiB-ROM target can swallow the IIR's 64-bit Operand::Int
   shape (via truncation + range-rejection), every other backend
   can too.

## Why `Vec<u8>` output, not textual asm?

* **Round-trips with `intel-4004-assembler` and any in-tree
  4004 simulator.**  Both consume raw byte streams.
* **Deterministic test surface** — 4004 mnemonics have multiple
  historical spellings (Intel's original MCS-4 manual vs modern
  reverse-engineered docs).  Bytes are unambiguous.
* **Trivial output size** — 4004 instructions are 1 or 2 bytes;
  emitting bytes directly skips a textual-assembly round-trip.

## Pipeline

```text
IIRModule
  → validate_for_intel4004()      pre-flight, returns Vec<String>
  → lower_iir_to_intel4004()      returns Vec<u8> of 4004 opcodes
  → (optional)
      • intel-4004-assembler: round-trip via its decoder
      • write to .bin + a 4004 simulator for in-process testing
      • burn to a 1702/2708 EPROM + plug into a 4004 dev board
```

## Scope by version

| Version | Scope | Status |
|---------|-------|--------|
| v0.1.0 (A4) | crate skeleton: any module → single `JUN 0x000` (`0x40 0x00`) infinite-loop halt sentinel | **merged** |
| v0.2.0 (A4+) | `const dest, Int(n)` → `LDM n` + `ret`/`ret_void` → JUN-self | **merged** |
| v0.3.0 (A4++) | ACC-first linear allocator over `r0..r15` + `mov` + ret-value staging | **merged** |
| **A4+++ (this PR, in `lang-aot` v0.9.0 → v0.10.0)** | `lang-aot --emit=intel4004` (aliases `i4004`, `4004`) routes source → IIR → Intel 4004 `.bin` via `iir-to-intel4004`; cross-platform; no host gating; no version bump for the iir-to-intel4004 crate itself | this PR |
| v0.4.0 (A4++++) | Arithmetic via accumulator (`ADD`, `SUB`, `IAC`, `DAC`) + conditional jumps via `JCN` | future |
| v0.4.0 (A4+++) | `lang-aot --emit=intel4004` wiring + Brainfuck end-to-end | future |

## Public surface (v0.1.0)

```rust
pub struct IIRIntel4004Config { pub module_name: String }
impl IIRIntel4004Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRIntel4004Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_intel4004(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_intel4004(
    module: &IIRModule,
    cfg: &IIRIntel4004Config,
) -> Result<Vec<u8>, IIRIntel4004Error>;

pub const HALT_LOOP: [u8; 2] = [0x40, 0x00];
```

## The `JUN 0x000` halt encoding (v0.1.0 acceptance criterion)

The 4004 has no formal `HLT` instruction.  The canonical "halt"
idiom in 4004 ROM development is `JUN 0x000` — an unconditional
jump back to ROM address 0, which (when this instruction is
itself at address 0) loops on itself forever, simulating halt.

Bit layout: `0100 aaaa aaaaaaaa` — the high 4 bits of byte 1 are
the JUN opcode (`0100`), the low 4 bits hold the high 4 bits of
the 12-bit address, and byte 2 holds the low 8 bits.

For `JUN 0x000`:

```text
byte 1: 0100 0000 = 0x40   (JUN opcode + high addr nibble = 0)
byte 2: 0000 0000 = 0x00   (low addr byte = 0)
```

When emitted at ROM address 0 (where lowering starts), this
instruction's address is 0 and its target is 0, so the CPU
infinitely re-executes it.  Every 4004 simulator and any
oscilloscope hooked to the program counter recognises this
pattern as "the chip is stuck at address 0".

### Why JUN-self over NOP-cycle or unimplemented-opcode?

| Candidate | Pros | Cons |
|-----------|------|------|
| `JUN 0x000` (this) | Self-documenting; portable across all 4004 implementations | None for skeleton purposes |
| `NOP NOP NOP ...` (0x00 cycle) | Even simpler bytes | Doesn't halt — keeps running into whatever follows |
| Unimplemented opcode | Forces a trap | 4004 silicon executes most "unused" bit patterns as NOPs; not portable |

`JUN 0x000` wins on portability + clarity.

## Non-goals (v0.1.0)

* No instruction lowering — deferred to A4+.
* No `lang-aot --emit=intel4004` wiring — deferred to A4+++.
* No external assembler / linker integration.  Output is raw
  bytes; downstream loaders are the caller's responsibility.

## Tests (v0.1.0)

* `validate_returns_empty_for_empty_module` — stub validator behaves.
* `lower_emits_exactly_two_bytes` — output shape.
* `lower_emits_the_canonical_jun_self_bytes` — exact `0x40 0x00`.
* `halt_loop_constant_pinned_to_40_00` — guards the constant.
* `default_config_has_nonempty_module_name` — config invariant.
* `new_sets_module_name` — builder contract.
* `errors_display_without_panic` — error formatting smoke.
