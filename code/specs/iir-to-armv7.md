# iir-to-armv7 — IIR → ARMv7 (A32) machine code backend

> ⚠ **REMOVED (2026-08-17).** The `iir-to-armv7` crate has been
> deleted. It lowered IIR directly to machine bytes, skipping type
> monomorphization and the `Backend` trait; it was superseded by
> `armv7-encoder` + `armv7-backend`, which lower CIR instead. See
> [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md).
> This spec is preserved as a historical record of the original design.

**Status:** v0.1.0 — skeleton (A3)
**Plan:** [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md) §A3
**Related:** [`iir-to-riscv`][rv], [`iir-to-intel8008`][i8008]

[rv]: ../packages/rust/iir-to-riscv/
[i8008]: ../packages/rust/iir-to-intel8008/

## Why a new crate?

ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
the LANG VM backend lane.  It covers Cortex-A7/A8/A9-era SoCs and
many embedded boards (early Raspberry Pi, BeagleBone, Olimex
A20-OLinuXino) — far more deployed silicon than any single 8008
chip ever shipped, but architecturally a clean fixed-width 32-bit
RISC like RV32I.

Adding ARMv7 as a backend gives us:

1. **A third architecture backend** alongside RV32I (A1) and Intel
   8008 (A2).  The three sit at meaningfully different points in
   the design space:
   - RV32I: clean modern 32-bit RISC, load-store, condition-codes-
     less.
   - Intel 8008: irregular 8-bit accumulator-based CISC with 14-bit
     addressing — historical fidelity for Oct.
   - **ARMv7 (A32)**: 32-bit RISC with condition-code prefixes on
     every instruction (the `cond` field every A32 instruction
     carries) plus a barrel shifter on the second operand.  Same
     word-width as RV32I but a fundamentally different ISA-level
     design.

2. **Foundation for native phone-OS targets.**  Once the AOT
   wiring lands, the same LANG VM source program can be cross-
   compiled to ARMv7 Linux executables (BeagleBone, Raspberry Pi
   1/Zero, qemu-arm) without changing front-end code.

3. **Round-trip with the in-tree [`arm-simulator`].**  The
   `Vec<u32>` output drops directly into the simulator for
   in-process tests.

## Why `Vec<u32>` output, not textual asm?

* **Round-trips with `arm-simulator`** — its decoder consumes raw
  little-endian 32-bit words directly.
* **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
  is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
  and ARMASM divergence we don't want to entangle with.
* **Trivial encoding shape** — every A32 instruction is exactly 4
  bytes (in stark contrast to the 8008's 1/2/3 byte variability).
  `Vec<u32>` is the natural representation.

## Pipeline

```text
IIRModule
  → validate_for_armv7()      pre-flight, returns Vec<String>
  → lower_iir_to_armv7()      returns Vec<u32> of A32 words
  → (optional)
      • arm-simulator: in-process testing
      • write to .bin + qemu-arm for cross-platform execution
      • objcopy + linker for an ELF on a phone-class Linux board
```

## Scope by version

| Version | Scope | Status |
|---------|-------|--------|
| v0.1.0 (A3) | crate skeleton: any module → single `BKPT #0xFFFF` (`0xE12FFF7F`) | **merged** |
| v0.2.0 (A3+) | `const` → `MOV r0, #imm8` (accumulator-only) + `ret`/`ret_void` → `BX LR` (AAPCS return) | **merged** |
| v0.3.0 (A3++) | Linear register allocator over `r0..r12` + multi-register `const` + `mov` + ret-value staging | **merged** |
| v0.4.0 (A3++.5) | `add`/`sub` on the data-processing-register family — 3-register `Rd = Rn op Rm` | **merged** |
| v0.4.1 (A3++.5.5 first slice) | Bitwise DP-register ops `and` (AND `0xE000_0000`), `or` (ORR `0xE180_0000`), `xor` (EOR `0xE020_0000`) | **merged** |
| v0.4.2 (A3++.5.5 second slice) | Carry-chained DP-register ops `adc` (`0xE0A0_0000`) and `sbb` (`0xE0C0_0000`) | **merged** |
| v0.4.3 (A3++.5.5 third slice) | `cmp` equality with flag-to-bool capture via `MOVEQ` (`0x03A0_0000`) | **merged** |
| v0.4.4 (A3++.5.5 fourth slice) | Full comparison family `cmp_ne`/`cmp_lt`/`cmp_gt`/`cmp_gte`/`cmp_lte` via condition prefixes | **merged** |
| v0.4.5 (A3++.5.5 fifth slice) | Control flow: `label` + `jmp` + `jmp_if_true`/`jmp_if_false` via Bcond + two-pass backpatching | **merged** |
| v0.4.6 (A3++.6) | Real `call` via `BL` + module-level call-backpatching.  ARMv7 backend feature-complete for AOT. | **merged** |
| **A3+++ (this PR, in `lang-aot` v0.8.0 → v0.9.0)** | `lang-aot --emit=armv7` (aliases `arm`, `arm32`) routes source → IIR → ARMv7 (A32) `.bin` via `iir-to-armv7`; flattens `Vec<u32>` to little-endian bytes; cross-platform; no host gating; no version bump for the iir-to-armv7 crate itself | this PR |
| v0.3.x (A3++.5.5) | Comparisons + conditional branches via the cond-field on every A32 instruction (a stronger version of the 8008's flag-based jumps) | future |
| v0.3.x (A3++.6) | Function calls via `bl` with PC-relative offsets + stack spilling | future |
| v0.4.0 (A3+++) | `lang-aot --emit=armv7` wiring | future |

## Public surface (v0.1.0)

```rust
pub struct IIRArmv7Config { pub module_name: String }
impl IIRArmv7Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRArmv7Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_armv7(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_armv7(
    module: &IIRModule,
    cfg: &IIRArmv7Config,
) -> Result<Vec<u32>, IIRArmv7Error>;

pub const BKPT: u32 = 0xE12FFF7F;
```

## The `BKPT` encoding (v0.1.0 acceptance criterion)

The chosen "halt sentinel" is `BKPT #0xFFFF`, encoded as
`0xE12FFF7F`.  Bit layout:

```text
31..28  cond     = 0xE = 1110          (always — unconditional)
27..20            = 0001 0010 = 0x12   (BKPT opcode family)
19..8   imm12    = 0xFFF                (top 12 bits of imm16)
 7..4            = 0111 = 0x7           (BKPT opcode family)
 3..0   imm4     = 0xF                  (bottom 4 bits of imm16)
```

Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
`0xE12FFF7F`.

### Why BKPT and not WFI or `b .`?

| Candidate | Pros | Cons |
|-----------|------|------|
| `BKPT #imm16` | Semantically "stop execution"; every ARM debugger / emulator recognises it; emits a defined trap | None for skeleton purposes |
| `WFI` | "Wait For Interrupt" — true halt | Requires kernel/hypervisor privilege; not legal in userspace |
| `B .` (infinite loop) | Pure userspace, no traps | Burns CPU; harder to detect from a host emulator without timeout |

BKPT wins on simplicity + emulator round-trip.  The
`arm-simulator`'s decoder flags it as `bkpt` and stops single-stepping.

## Non-goals (v0.1.0)

* No instruction lowering — deferred to A3+.
* No `lang-aot --emit=armv7` wiring — deferred to A3+++.
* No external assembler / linker integration.  Output is raw
  little-endian 32-bit words; downstream loaders are the caller's
  responsibility.

## Tests (v0.1.0)

* `validate_returns_empty_for_empty_module` — stub validator behaves.
* `lower_emits_exactly_one_word` — output shape.
* `lower_emits_the_canonical_bkpt_word` — exact `0xE12FFF7F`.
* `bkpt_constant_pinned_to_e12fff7f` — guards the constant.
* `default_config_has_nonempty_module_name` — config invariant.
* `new_sets_module_name` — builder contract.
* `errors_display_without_panic` — error formatting smoke.
