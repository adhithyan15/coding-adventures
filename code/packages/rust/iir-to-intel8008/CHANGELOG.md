# Changelog — iir-to-intel8008

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
