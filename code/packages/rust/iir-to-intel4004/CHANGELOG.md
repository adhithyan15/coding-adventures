# Changelog — iir-to-intel4004

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-02 (A4 — crate skeleton)

### Added — `JUN 0x000`-only emission

First release.  Implements item A4 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to the canonical 2-byte 4004-ROM halt
sentinel `JUN 0x000` (bytes `0x40 0x00`).

#### Public surface

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

#### Why an Intel 4004 backend?

The Intel 4004 (1971) was the **world's first commercial
microprocessor**.  Tiny ISA, 4-bit data, 12-bit ROM addresses,
single 4-bit accumulator, 16 4-bit registers organised as 8 register
pairs, tiny ROM (4 KiB max) and RAM (640 bits max).

In this codebase the 4004 is primarily a Brainfuck fit per
MULTILANG-ARCHITECTURE-BACKENDS.md §A4.

A4 establishes the fourth architecture backend alongside RV32I
(A1), Intel 8008 (A2), and ARMv7 (A3).  The 4004 is the most
constrained target in the lane by a wide margin — 4-bit data, 4 KiB
ROM, single accumulator, no formal HLT.

#### Why `Vec<u8>` output, not textual asm?

* **Round-trips with the in-tree intel-4004-assembler and any
  4004 simulator** — both consume raw byte streams.
* **Deterministic test surface** — 4004 mnemonics have multiple
  historical spellings (Intel MCS-4 manual vs modern reverse-
  engineered docs).  Bytes are unambiguous.
* **Trivial output size** — 4004 instructions are 1 or 2 bytes;
  emitting bytes directly skips a textual-assembly round-trip.

#### The `JUN 0x000` halt encoding

The 4004 has no formal `HLT` instruction.  The canonical "halt"
idiom in 4004 ROM development is `JUN 0x000` — an unconditional
jump back to ROM address 0, which (when this instruction is itself
at address 0) loops forever, simulating halt.

`0x40 0x00`.  Bit layout:

```text
byte 1: 0100 0000 = 0x40   (JUN opcode + high nibble of 12-bit addr = 0)
byte 2: 0000 0000 = 0x00   (low byte of address = 0)
```

#### Why JUN-self over NOP-cycle or unimplemented-opcode?

| Candidate | Pros | Cons |
|-----------|------|------|
| `JUN 0x000` | Self-documenting; portable across all 4004 implementations | None for skeleton purposes |
| `NOP NOP NOP ...` (0x00 cycle) | Even simpler bytes | Doesn't halt — keeps running into whatever follows |
| Unimplemented opcode | Forces a trap | 4004 silicon executes most "unused" bit patterns as NOPs; not portable |

`JUN 0x000` wins on portability + clarity.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A4+) lowers `LDM` (load
  immediate) + `ret`/`ret_void`.
* **No `lang-aot --emit=intel4004`.**  Deferred to v0.4.0 (A4+++).
* **No external assembler / linker integration.**

#### Tests added (7 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_two_bytes`
* `lower_emits_the_canonical_jun_self_bytes` (exact `[0x40, 0x00]`)
* `halt_loop_constant_pinned_to_40_00`
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
