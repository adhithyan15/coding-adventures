# iir-to-riscv

IIR → RV32I machine code backend.  Emits `Vec<u32>` of encoded 32-bit
RISC-V instructions, suitable to drop into the in-tree `riscv-simulator`
or to write out as a flat `.bin` for `qemu-riscv32`.

**Status: v0.1.0 (A1 skeleton).**  Any module lowers to a single `ret`
(`jalr x0, x1, 0` → `0x0000_8067`).  Real instruction lowering arrives
in A1+, A1++, A1+++.

## Why an architecture backend?

The existing IIR backends target *software* runtimes:

| Backend | Target |
|---------|--------|
| iir-to-wasm | WebAssembly 1.0 |
| iir-to-jvm-class-file | JVM bytecode |
| iir-to-cil-bytecode | CLR CIL |
| iir-to-beam | Erlang BEAM |
| iir-to-llvm | LLVM textual IR |
| **iir-to-riscv (this)** | **RV32I machine code** |

RISC-V is the first **architecture** backend — output is real hardware
ISA, decoded directly by `riscv-simulator` (RV32I + M-mode), QEMU, or a
physical SiFive / Espressif RISC-V chip.

## Why `Vec<u32>` (not textual `.s`)?

- **Round-trips with the simulator.**  `riscv-simulator::execute` takes
  raw 32-bit words.
- **Deterministic test surface.**  `assert!(words[0] == 0x00008067)` is
  easier to debug than parsing assembly.
- **No textual-format coupling.**  GNU and LLVM assembler syntaxes
  diverge on edge cases.

A textual `.s` emitter could land as a sibling later without breaking
callers.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_riscv::{validate_for_riscv, lower_iir_to_riscv, IIRRiscvConfig};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_riscv(&module).is_empty());

let words = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
    .expect("lowering should succeed");
// 0x0000_8067 == `jalr x0, x1, 0` == RV32I `ret`.
assert_eq!(words, vec![0x0000_8067]);
```

## Roadmap

| Version | Scope |
|---------|-------|
| v0.1.0 (A1)  | Crate skeleton: any module → single `ret`. *(this release)* |
| v0.2.0 (A1+) | Function entry/exit prologue/epilogue + arith + cmp + control flow |
| v0.3.0 (A1++)| Calls + locals on stack + `ecall` for print |
| v0.4.0 (A1+++)| `lang-aot --target=riscv32` wiring |

See [`code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md`](../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md)
§A1 for the full plan.

## Tests

```sh
cargo test -p iir-to-riscv
```

6 tests at v0.1.0 covering validator stub, `ret` encoding (exact word),
config defaults, error display.
