# algol-wasm-compiler

Package the first ALGOL 60 compiler subset as WebAssembly.

This package is the orchestration layer for the first ALGOL 60 compiled lane:

```text
ALGOL source -> parse -> type-check -> IR -> WASM module -> WASM bytes
```

When the root block declares an integer scalar named `result`, `_start` returns
that value after the outer block finishes. Programs without that compatibility
variable now compile normally and return `0` from `_start`. Within that shape,
the current lane already supports a substantial ALGOL 60 surface:

- nested blocks and nested procedures with lexical access through static links
- `integer`, `boolean`, `real`, and `string` scalar values
- `own` scalars and arrays with static lifetime
- arrays of `integer`, `boolean`, `real`, and `string` values with runtime
  bounds and checked element access
- chained assignment, conditional expressions, tolerant trailing/repeated
  semicolons, numeric runtime failure guards, and
  ALGOL-left-associative exponentiation for integer exponents
- value and by-name parameters, including Jensen-style expression thunks,
  typed whole-array formals with copy or aliasing semantics, and formal
  procedure calls that forward scalar, whole-array, label, switch, or
  procedure arguments while validating nested procedure-formal contracts
- label, switch, and procedure formals in value or by-name mode
- bare no-argument typed procedure names as expression calls, matching ALGOL's
  omitted-parentheses call syntax for parameterless procedures
- labels, local and nonlocal `goto`/`go to`, switch designators, and conditional
  designational expressions, including nonlocal branches and nested
  recursive switch entries
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

print_only = compile_source("begin print('Hi') end")
captured_print: list[str] = []
runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured_print.append)))
assert runtime.load_and_run(print_only.binary, "_start", []) == [0]
assert "".join(captured_print) == "Hi"

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
captured_showcase: list[str] = []
runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured_showcase.append)))

assert runtime.load_and_run(showcase.binary, "_start", []) == [12]
assert "".join(captured_showcase) == "ALGOL 2 7.000"
```

## Golden Fixtures

The package test suite includes end-to-end golden ALGOL programs that compile
through the full parser -> type-checker -> IR -> WASM pipeline and run in the
local WASM runtime. Those fixtures cover:

- mixed scalar and array computation with output
- Jensen-style by-name procedure calls
- switch dispatch, procedure-formal dispatch, and procedure-crossing `goto`
- conditional expressions, chained assignment, and exponentiation in the same
  end-to-end program

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
