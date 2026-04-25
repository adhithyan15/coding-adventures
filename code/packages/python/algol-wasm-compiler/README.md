# algol-wasm-compiler

Package the first ALGOL 60 compiler subset as WebAssembly.

This package is the orchestration layer for the first ALGOL 60 compiled lane:

```text
ALGOL source -> parse -> type-check -> IR -> WASM module -> WASM bytes
```

Programs still declare an integer variable named `result`; `_start` returns
that value after the outer block finishes. Within that shape, the current lane
already supports a substantial ALGOL 60 surface:

- nested blocks and nested procedures with lexical access through static links
- `integer`, `boolean`, `real`, and `string` scalar values
- `own` scalars and arrays with static lifetime
- arrays of `integer`, `boolean`, `real`, and `string` values with runtime
  bounds and checked element access
- value and by-name parameters, including Jensen-style expression thunks and
  typed whole-array, label, and switch formals
- labels, local and nonlocal `goto`, switch designators, and conditional
  designational expressions, including nonlocal branches and nested
  non-recursive switch entries
- builtin `print(...)` / `output(...)` for integers, booleans, reals, strings,
  and string variables

The implementation is not yet a full ALGOL 60 conformance lane, but it is well
beyond a toy arithmetic subset and is now useful for representative block- and
procedure-heavy programs.

```python
from algol_wasm_compiler import compile_source
from wasm_runtime import WasiConfig, WasiHost, WasmRuntime

compiled = compile_source("begin integer result; result := 7 end")
assert WasmRuntime().load_and_run(compiled.binary, "_start", []) == [7]

with_array = compile_source(
    "begin integer result; integer array a[1:3]; "
    "a[2] := 9; result := a[2] end"
)
assert WasmRuntime().load_and_run(with_array.binary, "_start", []) == [9]

showcase = compile_source(
    "begin own integer counter; integer array a[1:3]; real average; "
    "string msg; integer result; "
    "integer procedure inc(x); value x; integer x; begin inc := x + 1 end; "
    "procedure bump; begin counter := counter + 1 end; "
    "msg := 'ALGOL'; "
    "a[1] := 2; a[2] := inc(4); a[3] := 7; "
    "bump; bump; "
    "average := (a[1] + a[2] + a[3]) / 2; "
    "print(msg); output(' '); print(counter); output(' '); print(average); "
    "result := a[2] + a[3] "
    "end"
)
captured: list[str] = []
runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

assert runtime.load_and_run(showcase.binary, "_start", []) == [12]
assert "".join(captured) == "ALGOL 2 7.000"
```

## Golden Fixtures

The package test suite includes end-to-end golden ALGOL programs that compile
through the full parser -> type-checker -> IR -> WASM pipeline and run in the
local WASM runtime. Those fixtures cover:

- mixed scalar and array computation with output
- Jensen-style by-name procedure calls
- switch dispatch and procedure-crossing `goto`

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
