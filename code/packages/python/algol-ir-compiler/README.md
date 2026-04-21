# algol-ir-compiler

Lower the first ALGOL 60 compiler subset to compiler IR.

The compiler accepts a checked ALGOL AST and emits `compiler_ir.IrProgram`
instructions that the existing structured IR-to-WASM lowerer can consume.
The outermost ALGOL block becomes `_start`. Integer variables live in planned
activation-frame slots, and scalar reads/writes lower to `LOAD_WORD` and
`STORE_WORD` through the semantic model's static-link metadata. The variable
named `result` is loaded into `v1` before `HALT` so the generated WASM function
returns it.

Value-only integer procedures lower to generated `_fn_algol_...` functions.
Calls pass an explicit static link followed by value arguments, procedure frames
are allocated from the module frame stack, and typed procedures return through
their procedure-name result slot.

Integer arrays lower to frame-stored descriptors backed by a separate bounded
ALGOL heap segment. The compiler evaluates integer lower/upper bounds once at
block entry, stores row-major stride metadata beside the descriptor, and emits
checked element loads/stores through the descriptor. Block and procedure exits
restore the heap pointer to the activation's entry mark, so dynamic arrays keep
ALGOL block lifetime instead of leaking through loops or recursive calls.

Scalar by-name parameters lower through a one-word storage pointer in the
callee frame. Passing a scalar variable as a by-name actual gives the callee a
delayed load/store cell, so assignments to the formal write back to the caller
slot while value parameters still remain isolated copies. General expression
actuals and array-element actuals still raise targeted `CompileError`
diagnostics until Phase 5 grows full eval/store thunk descriptors that
re-evaluate the actual expression on each access.

This phase keeps ALGOL frame memory and its 20-byte runtime state bounded to
one 64 KiB WASM page, and keeps array descriptors plus element storage inside a
separate 64 KiB heap segment. Larger semantic frame plans raise `CompileError`
before the WASM data encoder can materialize the memory image, dynamic
procedure recursion stops at the bounded frame stack, and invalid array bounds,
out-of-bounds subscripts, oversized arrays, or heap exhaustion return `0`.

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
