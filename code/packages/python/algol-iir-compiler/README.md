# algol-iir-compiler

ALGOL 60 to InterpreterIR (IIR) compiler for the generic LANG pipeline.

This package is the bridge from the existing Python ALGOL parser/type-checker
lane into the shared VM/JIT/AOT/backend chain:

```text
ALGOL 60 source
    -> algol_parser.parse_algol
    -> algol_type_checker.assert_algol_typed
    -> algol_iir_compiler.compile_to_iir
    -> interpreter_ir.IIRModule
    -> vm_core / jit_core / aot_core / iir-to-wasm / iir-to-jvm / iir-to-cil / iir-to-beam / iir-to-llvm
```

The first slice is intentionally scalar and portable: it emits fully typed IIR
using `i32`, `f64`, and `bool`, plus the common opcodes already understood by
the Rust backend chain. Arrays, procedures, by-name thunks, switches, strings,
nonlocal gotos, and ALGOL runtime frame layout stay in the existing
`algol-ir-compiler`/WASM lane until they get dedicated IIR runtime lowering.

```python
from algol_iir_compiler import AlgolVM

vm = AlgolVM()
assert vm.run("begin integer result; result := 1 + 2 * 3 end") == 7
```
