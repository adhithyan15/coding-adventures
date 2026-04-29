# twig-jit-wasm

Twig source → JIT-specialised WebAssembly via the in-house
`jit-core` engine and `wasm-backend` (the same WASM backend the
Tetrad runtime uses).

This is the JIT compilation track for Twig — separate from the
`twig` package's plain interpreter on `vm-core`.  Same Twig
source code; an extra layer of `jit-core` watches for hot
functions, lowers them through `compiler-ir`, hands the IR to
`wasm-backend.compile()`, and runs the resulting WebAssembly
binary in `wasm-runtime`.

## Pipeline

```
Twig source
    ↓ twig.compile_program     (twig package)
IIRModule
    ↓ jit-core.JITCore         (this package wires it up)
    │
    ├── interpreter path (vm-core) — used for cold functions
    │
    └── JIT path (only for hot functions):
            jit-core.specialise → list[CIRInstr]
                ↓ wasm-backend.compile
            cir-to-compiler-ir → IrProgram
                ↓ ir-to-wasm-compiler
            WasmModule → wasm-module-encoder → bytes
                ↓ wasm-backend.run
            wasm-runtime → result
```

## Quick start

```python
from twig_jit_wasm import run_with_jit

# A Twig program; the JIT specialises hot functions to WASM,
# falls back to interpretation for the rest.
result = run_with_jit("""
    (define (square x) (* x x))
    (square 7)
""")
print(result)   # 49
```

## What this package adds

The repo already had:

- `jit-core` — generic JIT engine.
- `wasm-backend` — `BackendProtocol` over WebAssembly.
- `twig` — interpreter on `vm-core` with full closure / heap
  support.

What was missing was the **wiring** between Twig and `jit-core`.
That's all this package does — fewer than 100 lines of glue.

## Usage notes

- `run_with_jit(source)` — the simplest entry point.  Compiles
  Twig, runs through `jit-core` with a `WASMBackend()`, returns
  the result of evaluating the entry function.
- The JIT is transparent: functions the WASM backend can't
  compile (closures with captured state, anything touching the
  heap) fall back to interpretation under `vm-core`.  No silent
  failures.
- See [BEAM01](../../../specs/BEAM01-twig-on-real-erl.md) and
  [TW02-twig-jvm-compiler](../../../specs/TW02-twig-jvm-compiler.md)
  for the sister real-runtime tracks (BEAM and JVM respectively).

## Sister packages

- [`twig`](../twig/) — language frontend + `vm-core` interpreter.
- [`jit-core`](../jit-core/) — generic JIT engine.
- [`wasm-backend`](../wasm-backend/) — WebAssembly backend.
- [`twig-jvm-compiler`](../twig-jvm-compiler/) — JVM target.
- [`twig-beam-compiler`](../twig-beam-compiler/) — BEAM target.
