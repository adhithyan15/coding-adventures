# wasm-backend

**WebAssembly 1.0 `BackendProtocol` implementation for the LANG JIT/AOT pipeline.**

This package is the final piece of the Tetrad compilation pipeline.  It wires
together two LANG milestones to produce a complete end-to-end path from Tetrad
source code to WebAssembly execution:

```
Tetrad source
  → tetrad-runtime.compile_to_iir()            → IIRModule
  → jit-core.specialise(fn, min_obs=0)         → list[CIRInstr]
  → WASMBackend.compile(cir)
        └── cir-to-compiler-ir (LANG21)         → IrProgram
        └── ir-to-wasm-compiler (LANG20)        → WasmModule
        └── wasm-module-encoder                 → bytes
  → WASMBackend.run(binary, [])
        └── wasm-runtime                        → result
```

## Quick start

```python
from tetrad_runtime import TetradRuntime
from wasm_backend import WASMBackend

rt = TetradRuntime()
result = rt.run_with_jit(
    "fn main() -> u8 { return 40 + 2; }",
    backend=WASMBackend(),
)
print(result)   # 42
```

## How it works

### BackendProtocol

`WASMBackend` implements the structural `BackendProtocol` from `codegen-core`
(two methods: `compile` and `run`).  No explicit inheritance is needed — the
protocol is checked structurally at runtime:

```python
from codegen_core import BackendProtocol
assert isinstance(WASMBackend(), BackendProtocol)   # True
```

### Return-value convention

The WASM compiler reads `IrRegister(1)` (the internal `_REG_SCRATCH` slot) at
`HALT` as the function return value.  LANG21's two-pass lowering assigns
registers by first-occurrence order, so the computation result may land in a
register other than 1.

`WASMBackend.compile()` fixes this automatically by inserting
`ADD_IMM IrRegister(1), result_reg, IrImmediate(0)` before HALT when needed.
This is a standard "MOV via ADD-zero" pattern used throughout the IrProgram
backends.

### Entry label

The default entry label is `"_start"` — the conventional name for a WASM
module's entry point.  Pass `entry_label="my_func"` to the constructor to use
a different name (useful for multi-function modules in future LANG specs).

## Integration with JITCore

```python
from jit_core import JITCore
from vm_core import VMCore
from wasm_backend import WASMBackend
from tetrad_runtime import compile_to_iir

module  = compile_to_iir("fn main() -> u8 { return 40 + 2; }")
backend = WASMBackend()
vm      = VMCore(opcodes={}, u8_wrap=True)
jit     = JITCore(vm, backend, min_observations=0)
result  = jit.execute_with_jit(module)
# result == 42
```

## Dependencies

| Package | Role |
|---------|------|
| `coding-adventures-codegen-core` | `CIRInstr`, `BackendProtocol` |
| `coding-adventures-compiler-ir` | `IrOp`, `IrRegister`, `IDGenerator`, … |
| `coding-adventures-cir-to-compiler-ir` | LANG21 lowering pass |
| `coding-adventures-ir-to-wasm-compiler` | LANG20 WASM codegen |
| `coding-adventures-wasm-module-encoder` | `encode_module()` → bytes |
| `coding-adventures-wasm-runtime` | `WasmRuntime().load_and_run()` |

## Limitations (V1)

- **Single function only.** The entry point is always a single flat CIR list
  mapped to one WASM function.  Multi-function support is planned for LANG22.
- **Void parameters.** The WASM function takes no parameters; all values must
  be embedded as constants in the CIR.  Parameter passing is planned for LANG22.
- **Integer and float results only.** `i32` return for integer ops, `f64` for
  float ops.  No multi-value returns.
- **No `call_runtime` / `io_in` / `io_out`.** These CIR ops trigger deoptimisation
  (compile returns `None`), falling back to the interpreter.
