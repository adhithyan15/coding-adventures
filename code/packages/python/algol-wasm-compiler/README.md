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

```python
from algol_wasm_compiler import compile_source
from wasm_runtime import WasmRuntime

compiled = compile_source("begin integer result; result := 7 end")
assert WasmRuntime().load_and_run(compiled.binary, "_start", []) == [7]
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
