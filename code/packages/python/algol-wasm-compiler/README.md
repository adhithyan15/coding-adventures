# algol-wasm-compiler

Package the first ALGOL 60 compiler subset as WebAssembly.

This package is the orchestration layer for the first ALGOL 60 compiled lane:

```text
ALGOL source -> parse -> type-check -> IR -> WASM module -> WASM bytes
```

The current subset is intentionally small and structured. A program declares an
integer variable named `result`; `_start` returns that value after the outer
block finishes.

Scalar locals are now backed by the ALGOL frame model from PL04. Nested blocks
use static links to reach outer frame slots, so shadowing and outer-scope writes
exercise the same storage path later procedures will need.
Value-only integer procedures are compiled as WASM functions with explicit
static-link arguments, fresh frame allocation per call, and procedure-name
result slots for typed procedure returns.
Integer arrays compile through the Phase 4 descriptor path: bounds are
evaluated at block entry, descriptors live in frame slots, element storage lives
in a bounded ALGOL heap segment, and every element access performs runtime
bounds checks before touching WASM memory.
Scalar by-name parameters now execute through the same WASM memory path by
passing a storage pointer into the callee frame. A by-name formal assignment
therefore writes back to the caller's scalar slot. Read-only integer expression
actuals now compile as tagged eval thunk descriptors, so each by-name formal
read re-evaluates the expression using the caller frame captured at the call
site. Array-element actuals, expression thunks that read arrays, expression
thunks that call procedures, and expression thunk stores remain guarded by IR
compile-stage diagnostics until full Phase 5 store/helper coverage is available.

```python
from algol_wasm_compiler import compile_source
from wasm_runtime import WasmRuntime

compiled = compile_source("begin integer result; result := 7 end")
assert WasmRuntime().load_and_run(compiled.binary, "_start", []) == [7]

with_array = compile_source(
    "begin integer result; integer array a[1:3]; "
    "a[2] := 9; result := a[2] end"
)
assert WasmRuntime().load_and_run(with_array.binary, "_start", []) == [9]
```

## Dependencies

- algol-ir-compiler
- ir-to-wasm-validator
- ir-to-wasm-compiler
- wasm-validator
- wasm-module-encoder

## Development

```bash
# Run tests
bash BUILD
```
