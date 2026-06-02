# Changelog — iir-to-armv7

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-02 (A3 — crate skeleton)

### Added — `BKPT`-only emission

First release.  Implements item A3 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single ARMv7-A `BKPT #0xFFFF`
instruction (encoding `0xE12FFF7F`).

#### Public surface

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

#### Why an ARMv7 backend?

ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
the LANG VM backend lane.  It covers Cortex-A7/A8/A9-era SoCs and
many embedded boards (early Raspberry Pi, BeagleBone, Olimex
A20-OLinuXino) — vastly more deployed silicon than any single 8008
chip ever shipped, but architecturally a clean fixed-width 32-bit
RISC like RV32I with one big twist: every A32 instruction carries a
4-bit `cond` field that can predicate execution.

A3 establishes the third architecture backend alongside RV32I (A1)
and Intel 8008 (A2).  Together they exercise the IIR's neutrality
across genuinely different ISAs:

- **RV32I**: clean 32-bit RISC, load-store, no condition codes.
- **Intel 8008**: irregular 8-bit accumulator CISC, 14-bit address
  bus — historical fidelity for Oct.
- **ARMv7**: 32-bit RISC with cond prefixes on every instruction
  plus a barrel shifter on the second operand.

#### Why `Vec<u32>` output, not textual asm?

* **Round-trips with `arm-simulator`** — its decoder consumes raw
  little-endian 32-bit words directly.
* **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
  is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
  and ARMASM divergence we don't want to entangle with.
* **Trivial encoding shape** — every A32 instruction is exactly 4
  bytes (in stark contrast to the 8008's 1/2/3 byte variability).

#### The `BKPT #0xFFFF` encoding

`0xE12FFF7F`.  Bit layout (cond=AL):

```text
31..28  cond    = 0xE = 1110            (always — unconditional)
27..20          = 0001 0010 = 0x12      (BKPT opcode family)
19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
 7.. 4          = 0111 = 0x7            (BKPT opcode family)
 3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
```

Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
`0xE12FFF7F`.

#### Why BKPT and not WFI or `b .`?

| Candidate | Pros | Cons |
|-----------|------|------|
| `BKPT #imm16` | Semantically "stop"; every ARM debugger / emulator recognises it | None for skeleton purposes |
| `WFI`         | True halt | Requires kernel/hypervisor privilege; illegal in userspace |
| `B .`         | Pure userspace, no traps | Burns CPU; harder to detect without a host timeout |

BKPT wins on simplicity + emulator round-trip.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A3+) lowers `const` + `bx lr`.
* **No `lang-aot --emit=armv7`.**  Deferred to v0.4.0 (A3+++).
* **No external assembler / linker integration.**

#### Tests added (7 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_word`
* `lower_emits_the_canonical_bkpt_word` (exact `0xE12FFF7F`)
* `bkpt_constant_pinned_to_e12fff7f`
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
