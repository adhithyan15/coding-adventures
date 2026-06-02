# Changelog — iir-to-intel8008

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.3.3] — 2026-06-02 (A2++.5.5 second slice — carry/borrow ALU `adc`/`sbb`)

### Added — carry-chained accumulator-target ALU

Extends v0.3.2 with two more accumulator-anchored ALU ops in family
`10 ooo sss` that read the carry/borrow flag set by a *prior*
flag-producing op:

| IIR op | Intel 8008 mnemonic | `ooo` | First byte |
|--------|---------------------|-------|------------|
| `adc dest, a, b` | `ACA b_reg` | `001` | `0x88 \| sss` |
| `sbb dest, a, b` | `SCA b_reg` | `011` | `0x98 \| sss` |

The lowering shape is identical to add/sub/and/or/xor — only the
`ooo` selector changes.  No new encoder code; `encode_alu(ooo, sss)`
from v0.3.1 carries the wider dispatch.

#### Carry-flag contract (front-end responsibility)

The 8008's `ACA`/`SCA` consume the carry flag bit set by a prior
flag-affecting ALU op (`ADD`, `SUB`, `ADC`, `SBB`, `ANA`, `ORA`,
`XRA`, `CMP`).  This backend emits instructions in source order with
no reordering — so if the IIR front-end emits

```
add r_lo lo_a lo_b   ; sets carry on overflow
adc r_hi hi_a hi_b   ; consumes that carry
```

the carry survives between the two.  However, the staging MOVs
inserted by the allocator (`MOV A, hi_a`) sit between them.  Per
Intel's 8008 docs MOV does NOT affect flags, so the carry survives
the MOV too.  Front-ends MUST NOT insert flag-clobbering ops between
the producer and the ADC/SBB consumer.

#### Why no `cmp` in this slice?

`cmp` in IIR is shaped `cmp dest, a, b` and produces a boolean dest.
The 8008's `CMP` (`ooo = 0b111`) computes `A - r`, sets flags, and
**discards** the result — there's no register dest in 8008-speak.
Lowering `cmp` therefore requires an additional sequence that
captures the resulting condition into a register, which is typically
done as part of the same lowering that handles conditional branches.
That work lands together in v0.3.4.

#### New opcode constants

* `const ALU_ADC: u8 = 0b001;`
* `const ALU_SBB: u8 = 0b011;`

#### Tests added (27 total, was 24)

* `adc_two_consts_emits_aca_b_after_mov` — pinned full sequence with
  `0x88` (ACA B).
* `sbb_two_consts_emits_sca_b_after_mov` — `0x98` (SCA B).
* `adc_when_lhs_is_already_in_a_skips_the_staging_mov` — `0x8F`
  (ACA A) for the self-ADC idiom, generalising the self-add/self-AND
  tests.

### What is NOT in v0.3.3 (deferred to v0.3.4 / v0.3.5 / A2+++)

* `cmp` — paired with the branch ops in v0.3.4.
* Real `RET` (`0x07`) via `CALL` (`0x46` + 14-bit address) + the
  internal return stack — v0.3.5.
* Conditional + unconditional jumps with 14-bit address backpatching
  — v0.3.4 (alongside `cmp`).
* `lang-aot --target=intel8008` wiring — A2+++.

## [0.3.2] — 2026-06-02 (A2++.5.5 first slice — bitwise ALU `and`/`or`/`xor`)

### Added — bitwise accumulator-target ALU

Extends v0.3.1 with three more accumulator-anchored ALU ops in family
`10 ooo sss`.  Identical lowering shape to add/sub — only the `ooo`
field changes:

| IIR op | Intel 8008 mnemonic | `ooo` | First byte |
|--------|---------------------|-------|------------|
| `and dest, a, b` | `ANA b_reg` | `100` | `0xA0 \| sss` |
| `xor dest, a, b` | `XRA b_reg` | `101` | `0xA8 \| sss` |
| `or  dest, a, b` | `ORA b_reg` | `110` | `0xB0 \| sss` |

The full sequence remains:

```text
if a_reg != A:    MOV A, a_reg
                  ANA/ORA/XRA b_reg     ; result lands in A
if dest_reg != A: MOV dest_reg, A
```

#### Code-gen shape (worked example)

`r = v & w` with v→A, w→B, r→C lowers to:

```
MVI A, v_imm
MVI B, w_imm
ANA B            ; 0xA0
MOV C, A         ; 0x4F
```

i.e. one byte of bitwise op plus the staging move (same as `add`/`sub`).

#### Self-op idiom

`and r v v` where `v` is already in `A` lowers to `ANA A` (`0xA7`,
family `10 100 111`) — same as the self-add shape: no leading
`MOV A, A`.

#### New opcode constants

* `const ALU_AND: u8 = 0b100;`
* `const ALU_XOR: u8 = 0b101;`
* `const ALU_OR:  u8 = 0b110;`

The `encode_alu(ooo, sss)` helper from v0.3.1 carries them all — no
new encoder code, just a wider dispatch in the lowering match arm.

#### Tests added (24 total, was 20)

* `and_two_consts_emits_ana_b_after_mov` — pinned full sequence with
  `0xA0` (ANA B).
* `or_two_consts_emits_ora_b_after_mov` — `0xB0` (ORA B).
* `xor_two_consts_emits_xra_b_after_mov` — `0xA8` (XRA B).
* `and_when_lhs_is_already_in_a_skips_the_staging_mov` — `0xA7`
  (ANA A) for the self-AND idiom, generalising the self-add test.

The `unsupported_op_is_rejected_with_function_name` test still probes
`safepoint`, which remains outside the whitelist.

### What is NOT in v0.3.2 (deferred to v0.3.3 / A2+++)

* `cmp`, `adc`, `sbb` — same family, different `ooo` codes
  (`cmp = 0b111`, `adc = 0b001`, `sbb = 0b011`).  `cmp` needs flag
  observation wiring; `adc`/`sbb` need the carry flag plumbed from a
  prior arithmetic op.  All three land together once the carry-flag
  story is settled.
* Real `RET` (`0x07`) via `CALL` + the internal return stack.
* Conditional + unconditional jumps with 14-bit address backpatching.
* `lang-aot --target=intel8008` wiring — A2+++.

## [0.3.1] — 2026-06-02 (A2++.5 first slice — `add`/`sub` on the accumulator)

### Added — accumulator-target ALU

Extends v0.3.0 with two ALU ops in family `10 ooo sss`.  The 8008's
ALU is *always* accumulator-anchored: left source AND destination are
`A`; only the right source comes from `sss`.

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `add dest, a, b` | (optional `MOV A, a_reg`) + `ADD b_reg` (`0x80 \| sss`) + (optional `MOV dest_reg, A`) |
| `sub dest, a, b` | (optional `MOV A, a_reg`) + `SUB b_reg` (`0x90 \| sss`) + (optional `MOV dest_reg, A`) |

#### Code-gen shape

```text
if a_reg != A:   MOV A, a_reg
                 ADD/SUB b_reg          ; result lands in A
if dest_reg != A: MOV dest_reg, A
```

The first const allocated to `A` and the next-allocated `dest_reg` ≠
`A` mean the typical sequence for `r = v + w` (where `v` was the first
const) is:

```
ADD b_reg
MOV C, A
```

i.e. two bytes of arithmetic plus the staging move.

#### Self-add idiom

`add r v v` where `v` is already in `A` lowers to `ADD A` (`0x87`,
family `10 000 111`) — the 8008 happily uses `A` as the right source.
No leading `MOV A, A` is emitted.

#### New encoder helper + opcode constants

* `fn encode_alu(ooo: u8, sss: u8) -> u8` — `0x80 | (ooo << 3) | sss`.
* `const ALU_ADD: u8 = 0b000;`
* `const ALU_SUB: u8 = 0b010;`

#### Tests added (20 total, was 17)

* `add_two_consts_returns_their_sum_via_accumulator` — pinned 8-byte
  sequence ending in `0x80 0x4F 0x79 0x76`.
* `sub_two_consts_emits_sub_b_after_mov` — same shape with `0x90`.
* `add_when_lhs_is_already_in_a_skips_the_staging_mov` — pinned
  `ADD A = 0x87` for the self-add case.

The pre-existing `unsupported_op_is_rejected_with_function_name` test
flipped from probing `add` (now supported) to `safepoint` (still
outside the whitelist).

### What is NOT in v0.3.1 (deferred to A2++.5.5 / A2+++)

* `cmp`, `and`, `or`, `xor`, `adc`, `sbb` — same family, different
  `ooo` codes.
* Real `RET` (`0x07`) via `CALL` + the internal return stack.
* Conditional + unconditional jumps with 14-bit address backpatching.
* `lang-aot --target=intel8008` wiring — A2+++.

## [0.3.0] — 2026-06-02 (A2++ — multi-register `const` + `mov` + ret-value staging)

### Added — linear register allocator over A/B/C/D/E/H/L

Extends v0.2.0's accumulator-only `const` to a real allocator that
hands out the 8008's seven general-purpose registers in order:
`A, B, C, D, E, H, L`.  `A` comes first so the trivial `const v; ret v`
case stays at the 3-byte shape (`MVI A, n; HLT`) without an extra
`MOV A, X` round-trip.

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `const dest, Int(n)` | `MVI dest_reg, n` (`(rrr << 3) \| 0x06` + immediate byte) |
| `mov dest, src` | `MOV dest_reg, src_reg` (family `01 ddd sss`) |
| `ret <var>` | stage `var` into `A` via `MOV A, var_reg` if needed, then `HLT` |
| `ret_void` | `HLT` |

Register encoding: `A=7`, `B=0`, `C=1`, `D=2`, `E=3`, `H=4`, `L=5`,
`M=6` (memory pseudo-register, not allocated).  Pool: `[A, B, C, D,
E, H, L]`.

### New encoder helpers

* `encode_mvi(rrr: u8) -> u8` — `(rrr << 3) \| 0x06` for the
  immediate-load family.
* `encode_mov(ddd: u8, sss: u8) -> u8` — `0x40 \| (ddd << 3) \| sss`
  for the MOV family.

Both `debug_assert!` their inputs fit in 3 bits.

### New error variants

* `IIRIntel8008Error::UndefinedVariable` — `mov` or `ret` referenced
  a name that was never bound.
* `IIRIntel8008Error::OutOfRegisters` — 8th local exhausted the pool.
  Stack spilling lands in A2++.5 or later.

### Tests added (17 total, was 12)

* `two_consts_use_a_then_b_then_mov_a_b_before_hlt` — pinned exact
  6-byte sequence `MVI A,1; MVI B,2; MOV A,B; HLT` (`0x3E 0x01 0x06
  0x02 0x78 0x76`).
* `ret_of_first_const_omits_the_redundant_mov` — regression for the
  v0.2.0 3-byte trivial-case shape (A-first allocator preserves it).
* `mov_lowers_to_canonical_mov_ddd_sss` — `MOV B, A = 0x47`,
  `MOV A, B = 0x78`.
* `allocator_exhaustion_yields_out_of_registers` — 8 consts → fails
  on `v7` with `OutOfRegisters`.
* `undefined_variable_in_mov_is_rejected`.

### What is NOT in v0.3.0 (deferred to A2++.5)

* ALU on the accumulator (`ADD`/`SUB`/`CMP`/etc., family `10 ooo
  sss`).
* Real `RET` (`0x3F`) via `CALL` (`0x44` + 14-bit address) and the
  internal return stack.

## [0.2.0] — 2026-06-02 (A2+ — `const` → MVI A; `ret`/`ret_void` → HLT)

### Added — first real instruction lowering

Extends v0.1.0's HLT-only skeleton with three IIR ops:

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `const dest, Int(n)` | `MVI A, n` (`0x3E` + immediate byte) |
| `ret <var>` | `HLT` (real RET lands in A2++) |
| `ret_void` | `HLT` |

The accumulator-only first slice: every `const` goes into `A`.  Multi-
register allocation (B/C/D/E/H/L) lands in A2++ alongside `MOV r1, r2`
(family `11 ddd sss`) and ALU on the accumulator.

#### Why `ret` → `HLT` for now

Intel 8008's real `RET` (`0x3F`) requires the CPU to have a non-empty
internal return stack — which means proper `CALL` semantics first.
A2++ adds the `CALL/RET` stack discipline; until then, `HLT` gives the
simulator a clean stopping point for trivial test programs.

#### New constant in the public surface

* `pub const MVI_A: u8 = 0x3E;` — Intel 8008 `MVI A, imm8` first byte
  (bit pattern `00 111 110`, immediate-load family `00 rrr 110` with
  `rrr = 111 = A`).

#### Immediate byte range

`const` accepts integers in `[-128, 255]`:
* `[0, 255]` cast straight to `u8`.
* `[-128, -1]` reinterpreted as two's-complement (`-1 → 0xFF`).
* Anything outside → `InvalidOperand` with a precise message naming
  the 8-bit limit.  The 8008 has no wide-immediate idiom comparable
  to RV32I's `lui` — A2++ will split wider values into multiple
  `MVI` sequences across multiple registers.

#### Tests added (12 total, was 6)

* `const_42_then_ret_lowers_to_mvi_a_42_then_hlt` — pinned exact
  3-byte sequence `0x3E 0x2A 0x76`.
* `mvi_a_constant_pinned_to_0x3e`
* `const_negative_uses_twos_complement_byte` (`-1 → 0xFF`)
* `const_out_of_byte_range_is_rejected` (`1000` overflows)
* `ret_void_alone_emits_just_hlt`
* `unsupported_op_is_rejected_with_function_name` (`add` rejected
  with function name preserved in the error)

## [0.1.0] — 2026-06-02 (A2 — crate skeleton)

### Added — `HLT`-only emission

First release.  Implements item A2 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single Intel 8008 `HLT` instruction
(opcode `0x76`).

#### Public surface

```rust
pub struct IIRIntel8008Config { pub module_name: String }
impl IIRIntel8008Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRIntel8008Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_intel8008(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_intel8008(
    module: &IIRModule,
    cfg: &IIRIntel8008Config,
) -> Result<Vec<u8>, IIRIntel8008Error>;

pub const HLT: u8 = 0x76;
```

#### Why an Intel 8008 backend?

The 8008 (1972) is the first commercial 8-bit microprocessor and is
**Oct's native target** — Oct programs are written specifically to
round-trip through 8008 silicon (or the in-tree `intel8008-simulator`).
A2 establishes the second architecture backend alongside RV32I (A1) and
lays the groundwork for A4 (Intel 4004), which shares the historical-
microprocessor backend shape.

#### Why `Vec<u8>` output, not textual asm?

* **Round-trips with `intel8008-simulator`** — `Simulator::run` takes
  raw `&[u8]` instruction streams directly.
* **Deterministic test surface** — `assert_eq!(bytes, vec![0x76])` is
  unambiguous; 8008 mnemonics have Intel-spec vs MCS-8 historical
  divergence.
* **Trivial output size** — 8008 instructions are 1, 2, or 3 bytes;
  textual round-trip contributes nothing.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A2+) lowers MVI / MOV / basic
  arithmetic.
* **No `lang-aot --target=intel8008`.**  Deferred to v0.4.0 (A2+++).
* **No external assembler / linker integration.**

#### Tests added (6 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_byte`
* `lower_emits_the_canonical_hlt_byte` (exact `0x76`)
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
