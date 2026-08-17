# iir-to-riscv — IIR → RV32I machine code backend

> ⚠ **REMOVED (2026-08-17).** The `iir-to-riscv` crate has been
> deleted. It lowered IIR directly to machine bytes, skipping type
> monomorphization and the `Backend` trait; it was superseded by
> `riscv-encoder` + `riscv-backend`, which lower CIR instead. See
> [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md).
> This spec is preserved as a historical record of the original design.

**Status:** v0.1.0 — skeleton (A1)
**Plan:** [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md) §A1
**Related:** [`iir-to-llvm`][llvm], [`riscv-simulator`][sim]

[llvm]: ../packages/rust/iir-to-llvm/
[sim]: ../packages/rust/riscv-simulator/

## Why a new crate?

The existing IIR backends (wasm / JVM / CLR / BEAM / LLVM) all target
*software* runtimes that own register allocation, memory layout, GC, and
exception handling.  RISC-V is a different beast: real hardware ISA,
decoded directly by `riscv-simulator` (RV32I + M-mode traps) or by
QEMU / a physical SiFive / Espressif chip.

Adding RISC-V as a backend gives us:

1. **A real hardware target** alongside the software runtimes.
2. **A first architecture backend.**  Once A1 works, A2 (Intel 8008),
   A3 (ARMv7), A4 (Intel 4004), and A5 (GE-225) follow the same shape.
3. **A bridge to the broader RISC-V ecosystem** — QEMU, OpenSBI, Linux,
   FreeRTOS, all decode our output.

## Why **`Vec<u32>` output**, not textual asm?

* **Round-trips with the simulator.** `riscv-simulator::execute` takes
  raw 32-bit words; emitting them directly skips the assembler step.
* **Deterministic test surface.** `assert!(words[0] == 0x00008067)`
  reads cleanly in test failures.
* **No textual-format coupling.** GNU vs LLVM assembler syntaxes
  diverge on edge cases; raw words avoid both.

A textual `.s` emitter can be added as a sibling later without breaking
callers — but only when there's a concrete consumer that prefers it.

## Pipeline

```text
IIRModule
  → validate_for_riscv()       pre-flight, returns Vec<String>
  → lower_iir_to_riscv()       returns Vec<u32> of RV32I words
  → (optional)
      • riscv-simulator::execute() for in-process testing
      • write to .bin + qemu-riscv32 for headless host runs
      • link with `ld` + load on a SiFive / ESP32-C3 board
```

## Scope by version

| Version | Scope | Status |
|---------|-------|--------|
| v0.1.0 (A1) | crate skeleton: any module → single `ret` (`0x0000_8067`) | **merged** |
| v0.2.0 (A1+) | const/mov/add/sub/ret + linear register allocator | **merged** |
| v0.3.0 (A1++ first slice) | wide consts (`lui+addi`) + comparisons + `ecall print_i64` | **merged** |
| v0.3.1 (A1++.5 control-flow slice) | `label` / `jmp` / `jmp_if_*` with two-pass label resolution | **merged** |
| v0.3.2 (A1++.5.5 first slice) | cross-function `call` (0-arg, void only) + module-level resolution + per-fn prologue/epilogue | **merged** |
| **v0.3.3 (A1++.5.5.5 — this PR)** | call arguments (up to 8, two-phase mv-through-temp) + non-void return values | this PR |
| v0.4.0 (A1++.6) | stack-spilling register allocator + i64 register pairs + stack arg passing for > 8 args | future |
| **lang-aot v0.7.0 (A1+++ — this PR)** | `--emit=riscv32` flag + `compile_file_to_riscv32_bin` API + e2e smoke | this PR |
| (later) | RV32M (mul/div), RV32A (atomics), RV32F (floats), DWARF emission via aot-debug | future |

## Public surface (v0.1.0)

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

## The `ret` encoding (v0.1.0 acceptance criterion)

RISC-V `ret` is the standard pseudo-instruction for "return from
function".  It encodes as `jalr x0, x1, 0`:

| Field | Value | Meaning |
|-------|-------|---------|
| `rd`  | `x0`  | Discard the next PC (don't write it anywhere) |
| `rs1` | `x1 (ra)` | Jump to the address held in the standard return-address register |
| `imm[11:0]` | `0` | No offset |
| `funct3` | `0` | JALR uses funct3=0 |
| `opcode` | `0b1100111` | JALR |

Bit pattern: `0000_0000_0000_00001_000_00000_1100111` = **`0x0000_8067`**.

Verify in RISC-V User-Level ISA spec Volume I §2.5 (page 30 in the
2019-12-13 ratified edition).  v0.1.0's
`lower_emits_the_canonical_ret_word` test pins this exactly so any
future revision of `riscv-simulator::encoding::encode_jalr` that breaks
the encoding will surface immediately.

## Non-goals (v0.1.0)

* No instruction lowering — deferred to A1+.
* No `lang-aot --target=riscv32` wiring — deferred to A1+++.
* No external linker integration.  Output is raw words; downstream
  loaders are the caller's responsibility.
* No DWARF emission.  Once AOT-DBG-02 lands, the bridge can wrap this
  crate's output in a debug-augmented ELF file via `aot-debug`.

## Tests (v0.1.0)

* `validate_returns_empty_for_empty_module` — stub validator behaves.
* `lower_emits_exactly_one_word` — output shape.
* `lower_emits_the_canonical_ret_word` — exact `0x0000_8067`.
* `default_config_has_nonempty_module_name` — config invariant.
* `new_sets_module_name` — builder contract.
* `errors_display_without_panic` — error formatting smoke.
