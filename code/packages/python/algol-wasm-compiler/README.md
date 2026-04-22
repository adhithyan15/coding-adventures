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
site. Integer array-element by-name actuals now compile as tagged descriptors
with eval/store helpers, so reads and assignments re-compute the current element
address every time the formal is used. Read-only expression thunks can also
read array elements, including Jensen-style terms such as `a[i] * i`.
Expression thunks can call procedures, including procedures that receive nested
by-name descriptors, and runtime failures from those callees propagate through
the by-name formal read. Expression thunk stores remain guarded by IR
compile-stage diagnostics until full Phase 5 store-helper coverage is
available. The integer by-name acceptance tests cover the supported scalar,
array, expression, nested-procedure, and Jensen's-device surface; full ALGOL
labels, switches, procedure-valued parameters, whole-array by-name values, and
non-integer by-name formals remain later phases.

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
