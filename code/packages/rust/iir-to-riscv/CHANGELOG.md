# Changelog — iir-to-riscv

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-06-02 (A1+ — const/mov/add/sub/ret + linear register allocator)

### Added — first real instruction lowering

| IIR op | RV32I lowering |
|--------|----------------|
| `const dest, Int(n)` (12-bit imm) | `addi rd, x0, n` |
| `add dest, a, b`  | R-type `add rd, rs1, rs2` |
| `sub dest, a, b`  | R-type `sub rd, rs1, rs2` |
| `mov dest, src`   | `addi rd, rs1, 0` (canonical move) |
| `ret <var>` (int) | `addi a0, var_reg, 0` (skipped when var already in a0) + `jalr x0, x1, 0` |
| `ret_void`        | `jalr x0, x1, 0` |

#### Register allocation (linear, no spilling)

* Function parameters land in `a0..a7` (`x10..x17`) per the RISC-V
  calling convention; the validator caps params at 8 (`TooManyParams`
  error otherwise).
* Locals get the next free temp from
  `[t0, t1, t2, t3, t4, t5, t6]` = `[x5, x6, x7, x28, x29, x30, x31]`.
  Pool exhaustion → `OutOfRegisters`.  A real stack-spilling allocator
  lands in A1++.

#### Type rules

Supported: `void`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`.  Everything is
treated as a 32-bit value at this scope (RV32I native width).
`i64`/`u64`/`f32`/`f64` are deferred — 64-bit needs register pairs and
floats need the F-extension.

#### Why skip the `mv` when `ret`'ing the first param

`identity(x: i32) -> i32 { ret x }` lowers to just one word — the
canonical `ret` (`0x0000_8067`).  We don't emit `mv a0, a0` because
`x` already lives in `a0` (it's the first param).  This is a small but
visible win for the trivial pass-through case.

#### Public error variants added

* `IIRRiscvError::UndefinedVariable` — var used before bound.
* `IIRRiscvError::TooManyParams` — `>8` function params.
* `IIRRiscvError::OutOfRegisters` — `>7` locals after params.
* `IIRRiscvError::ImmediateOutOfRange` — `const` value outside
  `[-2048, 2047]`.  `lui+addi` synthesis lands in A1++.

#### Tests added (17 total, was 6)

* Validator (5): empty, accept, reject-op, reject-type, reject-too-many-params.
* Empty module emits no words (contract for trivial input).
* `ret_void`-only function emits just the canonical `0x0000_8067`.
* `const` + `ret`: pinned the exact three-word sequence
  `addi t0, x0, 7; addi a0, t0, 0; jalr x0, x1, 0`.
* `const` of `-2048` (smallest valid imm12) lowers cleanly.
* `const` of `4096` (overflow) → `ImmediateOutOfRange`.
* `add` of two params: pinned the exact `add t0, a0, a1` word.
* `sub` of two params: pinned the exact `sub t0, a0, a1` word.
* `mov` produces the canonical `addi rd, rs1, 0`.
* Identity-of-first-param skips the redundant `mv` (one-word output).
* Register-pool exhaustion is rejected with `OutOfRegisters`.
* Config + error-display smoke.

#### Why scope branches & comparisons to A1++

The data-flow core (arith + ret with register allocation) is its own
review surface.  Branches add label resolution and PC-relative offset
computation — orthogonal concerns that benefit from landing as a
separate slice.

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md

## [0.1.0] — 2026-06-01 (A1 — crate skeleton)

### Added — `ret`-only emission

First release.  Implements item A1 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single RV32I `ret` instruction
(`jalr x0, x1, 0`, encoded as `0x0000_8067`).

#### Public surface

```rust
pub struct IIRRiscvConfig { pub module_name: String }
impl IIRRiscvConfig {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRRiscvError {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_riscv(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_riscv(
    module: &IIRModule,
    cfg: &IIRRiscvConfig,
) -> Result<Vec<u32>, IIRRiscvError>;
```

#### Why an architecture backend?

The wasm / JVM / CLR / BEAM / LLVM backends all target *software*
runtimes that own register allocation and instruction selection.
RISC-V is the first **architecture** backend: output is real hardware
ISA, decodeable by the in-tree `riscv-simulator` (RV32I + M-mode
traps), QEMU, or a physical SiFive / Espressif RISC-V chip.

Strategic priority: RISC-V is the most open of the architecture
candidates (royalty-free spec, broad simulator availability, growing
hardware footprint).  A2-A5 (Intel 8008, ARMv7, Intel 4004, GE-225)
follow the same shape once A1's lessons are baked in.

#### Why `Vec<u32>` output, not textual assembly?

* **Round-trips with `riscv-simulator`** — it consumes raw 32-bit words.
* **Deterministic test surface** — `assert!(words[0] == 0x0000_8067)`
  is unambiguous; assembly syntax has GNU vs LLVM divergence.
* **No textual-format coupling.**

A textual `.s` emitter can be added as a sibling later without
breaking callers.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A1+) lowers function
  entry/exit prologue/epilogue + arithmetic + cmp + control flow.
* **No `lang-aot --target=riscv32`.**  Deferred to v0.4.0 (A1+++).
* **No external linker integration.**  Output is raw words; downstream
  linkers / loaders are the caller's responsibility.

#### Tests added (6 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_word`
* `lower_emits_the_canonical_ret_word` (exact `0x0000_8067`)
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
