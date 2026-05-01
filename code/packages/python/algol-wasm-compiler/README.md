# algol-wasm-compiler

Package the Python ALGOL 60 compiler lane as WebAssembly.

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
- case-insensitive keywords and comments, `!=`/`<>`/`≠` not-equal spelling,
  ALGOL publication symbols such as `≤`, `≥`, `↑`, `¬`, `∧`, `∨`, `⊃`, `≡`,
  and single- or double-quoted string literals
- chained assignment, conditional expressions, tolerant trailing/repeated
  semicolons, numeric runtime failure guards, and
  ALGOL-left-associative exponentiation for integer or real exponents
- boolean `and`, `or`, and `impl` with short-circuiting RHS evaluation, plus
  strict `eqv`
- standard numeric functions `abs`, `sign`, `entier`, `sqrt`, `sin`, `cos`,
  `arctan`, `ln`, and `exp`; non-native real math is imported through the
  runtime's `compiler_math` host ABI
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

Within the repository's ASCII `algol60` grammar, the executable
declaration, statement, expression, procedure, array, by-name, and designational
surface is now implemented and covered by golden WASM fixtures. It is not a
claim of historical whole-environment compatibility with every ALGOL system:
the package intentionally keeps the observable I/O surface to `print(...)` /
`output(...)`, supports the bundled ASCII spelling plus common ALGOL
publication-symbol notation, and applies explicit source, semantic,
generated-state, memory, and host execution limits for untrusted programs.

The convenience APIs reject source strings larger than 256 KiB before parsing.
The downstream type checker also enforces configurable AST, block-nesting, and
procedure-nesting limits so recursive semantic analysis fails with explicit
diagnostics instead of exhausting Python recursion on hostile inputs.
At execution time, run compiled modules with `WasmRuntime`'s
`WasmExecutionLimits` when the ALGOL source is untrusted; the instruction
budget stops nonterminating control-flow programs before they monopolize the
host process.

```python
from algol_wasm_compiler import compile_source
from wasm_runtime import WasiConfig, WasiHost, WasmExecutionLimits, WasmRuntime

compiled = compile_source("begin integer result; result := 7 end")
assert WasmRuntime().load_and_run(compiled.binary, "_start", []) == [7]

bounded_runtime = WasmRuntime(limits=WasmExecutionLimits(max_instructions=100_000))
assert bounded_runtime.load_and_run(compiled.binary, "_start", []) == [7]

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

with_math = compile_source(
    "begin integer result; real x; "
    "x := sin(0) + cos(0) + arctan(1) + ln(exp(1)) + 4 ^ 0.5; "
    "result := entier(x * 100) end"
)
assert WasmRuntime(host=WasiHost()).load_and_run(with_math.binary, "_start", []) == [478]

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
- a full-surface convergence program combining `own` scalars and arrays,
  default-real arrays, nested and single-statement procedures, value and
  by-name procedure formals, label and switch formals, multiple `for` element
  forms, parenthesized conditional designational expressions, numeric labels,
  boolean operators, real arithmetic, and output

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
