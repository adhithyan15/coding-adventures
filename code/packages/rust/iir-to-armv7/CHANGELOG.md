# Changelog — iir-to-armv7

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.3.0] — 2026-06-02 (A3++ — linear register allocator + `mov` + ret-value staging)

### Added — linear register allocator over r0..r12

Extends v0.2.0's accumulator-only `const` to a real allocator that
hands out the 13 ARMv7 general-purpose registers in order:
`r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12`.  `r0` comes
first so the trivial `const v; ret v` case stays at the 2-word shape
(`MOV r0, #imm; BX LR`) without an extra `MOV r0, X` round-trip.

`r13` (`sp`), `r14` (`lr`), and `r15` (`pc`) are NOT in the pool —
touching them as locals would break the calling convention's stack
discipline, the return address, or the instruction pointer.

| IIR op | A32 lowering |
|--------|--------------|
| `const dest, Int(n)` | `MOV rrr, #n` (`MOV_IMM_R0_BASE | (rrr << 12) | n`) |
| `mov dest, src` | `MOV Rd, Rm` (`MOV_REG_BASE | (Rd << 12) | Rm`) |
| `ret <var>` | stage `var` into `r0` via `MOV r0, var_reg` if needed, then `BX LR` |
| `ret_void` | `BX LR` |

### New constants

* `pub const MOV_REG_BASE: u32 = 0xE1A0_0000;` — `MOV r0, r0` base
  encoding for the register-to-register form.  OR in `(Rd << 12) |
  Rm` for arbitrary register pairs.

CAREFUL: bit-25 distinguishes this from `MOV_IMM_R0_BASE` (which is
`0xE3A0_0000`).  Data-processing-immediate has bit-25 set; register-
form doesn't.

### New encoder helper

* `encode_mov_reg(rd: u8, rm: u8) -> u32` — emits `MOV Rd, Rm` with
  `debug_assert!`s on both 4-bit register selectors.

### New error variants

* `IIRArmv7Error::UndefinedVariable` — `mov` or `ret` referenced a
  name that was never bound.
* `IIRArmv7Error::OutOfRegisters` — 14th local exhausted the 13-
  register pool.  Stack spilling lands in A3++.5 or later.

### Tests added (21 total, was 14)

* `mov_reg_base_pinned_to_0xe1a00000` — guards the new constant.
* `two_consts_use_r0_then_r1_then_mov_r0_r1_before_bx_lr` — pinned
  exact 4-word sequence including `MOV r1, #2 = 0xE3A0_1002` (the
  `1` in the `Rd` nibble) and `MOV r0, r1 = 0xE1A0_0001`.
* `ret_of_first_const_omits_the_redundant_mov` — regression for
  the v0.2.0 2-word trivial-case shape (r0-first allocator preserves
  it).
* `mov_lowers_to_canonical_mov_rd_rm` — `MOV r1, r0 = 0xE1A0_1000`
  and `MOV r0, r1 = 0xE1A0_0001` shown side-by-side.
* `allocator_exhaustion_yields_out_of_registers` — 14 consts → fails
  on `v13` with `OutOfRegisters`.
* `undefined_variable_in_mov_is_rejected`.
* `errors_for_new_variants_display_without_panic`.

### What is NOT in v0.3.0 (deferred to A3++.5 and beyond)

* Arithmetic (`add`/`sub`/`mul`) and the data-processing-register
  family those expand into.
* Function calls via `bl` with PC-relative offsets.
* Comparisons + conditional branches via the `cond` field every
  A32 instruction carries.
* Wider immediates via `movw`/`movt` (the ARMv7 16-bit-imm pair).
* `lang-aot --emit=armv7` wiring — A3+++.

## [0.2.0] — 2026-06-02 (A3+ — `const` → `MOV r0, #imm8`; `ret`/`ret_void` → `BX LR`)

### Added — first real instruction lowering

Extends v0.1.0's BKPT-only skeleton with three IIR ops:

| IIR op | A32 lowering |
|--------|--------------|
| `const dest, Int(n)` (8-bit imm) | `mov r0, #n` (`0xE3A0_00NN`) |
| `ret <var>` (int) | `bx lr` (`0xE12FFF1E`) — `var` is already in `r0` |
| `ret_void` | `bx lr` |

The accumulator-only first slice: every `const` goes into `r0`.
Multi-register allocation (`r1..r12`) and a real linear allocator
land in A3++ alongside arithmetic.

#### Why `BX LR` for both `ret` and `ret_void`?

In the ARM AAPCS calling convention, `r0` (a.k.a. `a1`) is the
return-value register.  Since every `const` already lands in `r0`,
`ret <var>` doesn't need any staging MOV — the value is in the
right place by construction.  `ret_void` simply doesn't care about
the value, so the same `bx lr` works.

(Compared to the 8008 backend's v0.2.0 which used `HLT` for `ret`
as a temporary stand-in until v0.3.9 wired up real `RET`, ARMv7's
`bx lr` is a proper return from the start.)

#### Why `BX LR` over `MOV PC, LR`?

`MOV PC, LR` would work on pure A32 cores but doesn't switch
instruction sets when the lr's low bit signals "return to Thumb
mode".  `BX LR` reads the low bit, switches mode accordingly, and
branches — the canonical AAPCS return.  On a pure A32 module
emitted by this crate, the low bit will always be 0 so both
behave identically, but emitting `BX LR` keeps the output
interop-correct with any caller that mixes A32 and Thumb code.

#### New constants in the public surface

* `pub const BX_LR: u32 = 0xE12FFF1E;` — branch-to-link-register
  (AAPCS return).
* `pub const MOV_IMM_R0_BASE: u32 = 0xE3A0_0000;` — `mov r0, #0`
  base; OR in the 8-bit immediate to form `MOV r0, #N`.

CAREFUL: `BX_LR = 0xE12FFF1E` vs `BKPT = 0xE12FFF7F` — bit-7
differs.  Both share the same `12F_FF` family bits, so the bit-7
nibble difference is the canonical confusion point.

#### Immediate byte range

`const` accepts integers in `[-128, 255]`:
* `[0, 255]` cast straight to `u8`.
* `[-128, -1]` reinterpreted as two's-complement (`-1 → 0xFF`).
* Anything outside → `InvalidOperand` with a message naming the
  8-bit limit and pointing forward to `movw`/`movt` (the ARMv7+
  wide-immediate idiom that lands in A3++).

#### New internal helper

* `encode_mov_imm(rd: u8, imm8: u8) -> u32` — encodes the full
  `MOV Rd, #imm8` word.  `debug_assert!`s on the 4-bit `rd` range.

#### Tests added (14 total, was 7)

* `bx_lr_constant_pinned_to_e12fff1e` — guards against the
  `BKPT ↔ BX_LR` (bit-7) confusion.
* `mov_imm_r0_base_pinned_to_0xe3a00000`.
* `const_42_then_ret_lowers_to_mov_r0_42_then_bx_lr` — pinned
  exact 2-word sequence `0xE3A0002A 0xE12FFF1E`.
* `const_negative_uses_twos_complement_byte` (`-1 → 0xFF`).
* `const_out_of_byte_range_is_rejected` (1000 overflows).
* `ret_void_alone_emits_just_bx_lr`.
* `unsupported_op_is_rejected_with_function_name` — `safepoint`
  rejected with function name preserved in the error.

### What is NOT in v0.2.0 (deferred to A3++ and beyond)

* Multi-register allocator over `r1..r12` + `mov rd, rs` register-
  to-register lowering.
* Arithmetic (`add`/`sub`/`mul`).
* Wider immediates via `movw`/`movt` (ARMv7's 16-bit-imm pair) or
  rotated 8-bit values (the A32 12-bit-imm field's full power).
* Function calls via `bl` and PC-relative offsets.
* Comparisons + conditional branches via the `cond` field every
  A32 instruction carries (a stronger version of the 8008's
  flag-based jumps).
* `lang-aot --emit=armv7` wiring — A3+++.

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
