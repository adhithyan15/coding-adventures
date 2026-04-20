# algol-ir-compiler

Lower the first ALGOL 60 compiler subset to compiler IR.

The compiler accepts a checked ALGOL AST and emits `compiler_ir.IrProgram`
instructions that the existing structured IR-to-WASM lowerer can consume.
The outermost ALGOL block becomes `_start`. Integer variables live in planned
activation-frame slots, and scalar reads/writes lower to `LOAD_WORD` and
`STORE_WORD` through the semantic model's static-link metadata. The variable
named `result` is loaded into `v1` before `HALT` so the generated WASM function
returns it.

This phase keeps the static frame image bounded to one 64 KiB WASM page. Larger
semantic frame plans raise `CompileError` before the WASM data encoder can
materialize the memory image.

```python
from algol_ir_compiler import compile_algol
from algol_parser import parse_algol

ir = compile_algol(parse_algol("begin integer result; result := 1 + 2 end"))
assert ir.program.entry_label == "_start"
assert ir.variable_slots["result@block0"] == 20
```

## Dependencies

- algol-type-checker
- compiler-ir

## Development

```bash
# Run tests
bash BUILD
```
