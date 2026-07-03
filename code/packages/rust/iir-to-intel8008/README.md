# iir-to-intel8008

IIR → Intel 8008 machine code backend. Emits `Vec<u8>` of encoded
8-bit Intel 8008 opcodes — Oct's native target.

**Status: v0.1.0 (A2 skeleton).** Any module lowers to a single `HLT`
(`0x76`). Real instruction lowering arrives in A2+ / A2++.

## Why an Intel 8008 backend?

The Intel 8008 (1972) is the first commercial 8-bit microprocessor. In
this codebase it's **Oct's native target** — Oct programs are written
specifically to round-trip through the 8008.

| Backend | Target |
|---------|--------|
| iir-to-wasm | WebAssembly 1.0 (software) |
| iir-to-jvm-class-file | JVM bytecode (software) |
| iir-to-cil-bytecode | CLR CIL (software) |
| iir-to-beam | Erlang BEAM (software) |
| iir-to-llvm | LLVM IR (textual → AOT) |
| iir-to-riscv | RV32I machine code (hardware) |
| **iir-to-intel8008 (this)** | **Intel 8008 machine code (Oct's native target)** |

## Why `Vec<u8>` (not textual `.asm`)?

- **Round-trips with `intel8008-simulator`** — its `Simulator::run`
  consumes raw `&[u8]` instruction streams directly.
- **Deterministic test surface** — `assert_eq!(bytes, vec![0x76])`.
- **Trivial output size** — Intel 8008 instructions are 1, 2, or 3
  bytes; emitting bytes skips the textual round-trip.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_intel8008::{validate_for_intel8008, lower_iir_to_intel8008, IIRIntel8008Config};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_intel8008(&module).is_empty());

let bytes = lower_iir_to_intel8008(&module, &IIRIntel8008Config::default())
    .expect("lowering should succeed");
// 0x76 == Intel 8008 HLT.
assert_eq!(bytes, vec![0x76]);
```

## Roadmap

| Version | Scope |
|---------|-------|
| v0.1.0 (A2)  | Crate skeleton: any module → single `HLT`. *(this release)* |
| v0.2.0 (A2+) | MVI (immediate load) + MOV (register-register) + arithmetic |
| v0.3.0 (A2++)| Conditional + unconditional jumps, calls, stack frame |
| v0.4.0 (A2+++)| `lang-aot --target=intel8008` wiring |

See [`code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md`](../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md)
§A2 for the full plan.

## Tests

```sh
cargo test -p iir-to-intel8008
```

6 tests at v0.1.0 covering the validator stub, the exact `HLT`
encoding, config defaults, and error display.
