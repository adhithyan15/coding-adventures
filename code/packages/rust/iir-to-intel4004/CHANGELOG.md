# Changelog — iir-to-intel4004

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-06-02 (A4+ — `const` → `LDM n`; `ret`/`ret_void` → `JUN 0x000`)

### Added — first real instruction lowering

Extends v0.1.0's HALT_LOOP-only skeleton with three IIR ops:

| IIR op | 4004 lowering |
|--------|---------------|
| `const dest, Int(n)` (4-bit imm) | `LDM n` (`0xD0 \| n`, single byte) |
| `ret <var>` | `JUN 0x000` (halt sentinel — real RET in A4++) |
| `ret_void` | `JUN 0x000` |

The accumulator-only first slice: every `const` goes into the
4004's 4-bit accumulator.  Multi-register-pair allocation via the
8 register pairs (`r0r1..r14r15`) arrives in A4++ alongside
arithmetic.

#### Why `ret` → halt sentinel for now?

The 4004's real `RET` is `BBL` (Branch Back to Last, opcode
`1100 dddd`).  But `BBL` requires a corresponding `JMS` (Jump to
SubRoutine, opcode `0101 aaaa aaaaaaaa`) to have pushed the
return address onto the 4004's 3-deep internal call stack first.
Without proper call/return discipline (which lands in A4++),
`BBL` from a fresh-start ROM would pop a garbage address from
the stack and jump there — undefined behaviour on most 4004
simulators.

`JUN 0x000` gives the simulator a clean stopping point until A4++
wires up the call stack.  Same pattern as iir-to-intel8008
v0.2.0's HLT-for-ret stand-in.

#### New constant in the public surface

* `pub const LDM_OPCODE: u8 = 0xD0;` — Intel 4004 `LDM n` opcode
  high nibble.  OR in the 4-bit immediate (0..=15) to form the
  full byte (`LDM 0 = 0xD0`, `LDM 15 = 0xDF`).

#### Immediate nibble range

`const` accepts integers in `[-8, 15]` (signed 4-bit interval):
* `[0, 15]` cast straight to the low nibble.
* `[-8, -1]` reinterpreted as 4-bit two's-complement (`-1 → 0xF`).
* Anything outside → `InvalidOperand` with a precise message
  naming the 4-bit limit and pointing forward to A4++'s wider-
  immediate idiom (LDM/arithmetic-op pairs).

The 4004 has **no wide-immediate idiom** comparable to the 8008's
8-bit `MVI` or RV32I's 12-bit `addi` — `LDM` is exactly 4 bits.
Multi-nibble values require explicit decomposition.

#### Tests added (16 total, was 7)

* `ldm_opcode_pinned_to_0xd0` — sanity check on the constant.
* `const_5_then_ret_lowers_to_ldm_5_then_jun_self` — pinned
  3-byte sequence `0xD5 0x40 0x00`.
* `const_15_then_ret_lowers_to_ldm_15_then_jun_self` — max
  4-bit value `0xDF`.
* `const_0_then_ret_emits_ldm_0` — minimum positive (LDM 0 = 0xD0,
  the bare opcode).
* `const_negative_uses_twos_complement_nibble` (`-1 → 0xF`).
* `const_negative_minus_eight_uses_8_nibble` (`-8 → 0x8`, the
  minimum signed 4-bit value).
* `const_out_of_nibble_range_is_rejected` (`16` overflows).
* `ret_void_alone_emits_just_jun_self`.
* `unsupported_op_is_rejected_with_function_name` (`safepoint`
  rejected with function name preserved).

### What is NOT in v0.2.0 (deferred to A4++ and beyond)

* Register-pair allocator over `r0r1..r14r15` (the 4004's 8 register
  pairs — 16 4-bit registers organised as 8 pairs).
* Arithmetic via accumulator (`ADD`, `SUB`, `IAC`, `DAC`).
* Real `RET` via `BBL` + the 4004's 3-deep internal call stack.
* Conditional jumps via `JCN` (Jump on Condition).
* `lang-aot --emit=intel4004` wiring — A4+++.

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
