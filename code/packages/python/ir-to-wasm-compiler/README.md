# ir-to-wasm-compiler

Generic IR-to-WASM lowering package.

This package consumes a `compiler_ir.IrProgram` plus explicit function
signature hints and produces a `wasm_types.WasmModule`.

It is intentionally generic so it can be piped between frontends and encoders:

```
frontend -> IrProgram -> ir-to-wasm-compiler -> WasmModule -> encoder/runtime
```

## Usage

```python
from ir_to_wasm_compiler import FunctionSignature, IrToWasmCompiler

module = IrToWasmCompiler().compile(
    program,
    function_signatures=[
        FunctionSignature(label="_fn_main", param_count=0, export_name="main"),
    ],
)
```

## Register Conventions

Generated functions return `i32` values through virtual register `v1` and
`f64` values through virtual register `v31`. Calls mirror the same convention:
integer results are copied into `v1`, while real results are copied into `v31`.
The f64 lowering path includes arithmetic, comparisons, integer conversion,
truncation, unary square root via WASM `f64.sqrt`, and standard unary real
math through typed `compiler_math` imports (`f64_sin`, `f64_cos`, `f64_atan`,
`f64_ln`, `f64_exp`).
