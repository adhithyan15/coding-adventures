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
- case-insensitive keywords, comments, and standard builtins,
  `!=`/`<>`/`≠` not-equal spelling, ALGOL publication symbols such as `≤`,
  `≥`, `↑`, `×`, `÷`, `¬`, `∧`, `∨`, `⊃`, `≡`, and single- or double-quoted
  string literals
- chained assignment, conditional expressions, tolerant trailing/repeated
  semicolons, numeric runtime failure guards, and
  ALGOL-left-associative exponentiation for integer or real exponents
- boolean `and`, `or`, and `impl` with short-circuiting RHS evaluation, plus
  strict `eqv`
- standard numeric functions `abs`, `sign`, `entier`, `sqrt`, `sin`, `cos`,
  `arctan`, `ln`, and `exp`; non-native real math is imported through the
  runtime's `compiler_math` host ABI
- value and by-name parameters, including Jensen-style expression thunks,
  typed whole-array formals with copy or aliasing semantics, report-style
  combined specs like `integer array a;` and `real procedure f;`, and formal
  procedure calls that forward scalar, whole-array, label, switch, or procedure
  arguments while validating nested procedure-formal contracts
- label, switch, and procedure formals in value or by-name mode
- explicit empty and bare no-argument procedure declarations/calls, matching
  ALGOL's omitted-parentheses call syntax for parameterless procedures
- forward sibling procedure calls and mutually recursive typed procedures
  within a block's declaration part
- switch entries that select later sibling switch declarations within the same
  declaration part
- labels, local and nonlocal `goto`/`go to`, switch designators, and conditional
  designational expressions, including nonlocal branches and nested
  recursive switch entries
- builtin `print(...)` / `output(...)` for one or more integers, booleans,
  reals, strings, and string variables

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

## Command Line

```bash
algol60-wasm program.alg -o program.wasm
python -m algol_wasm_compiler program.alg
algol60-wasm run program.alg --print-result
```

When `-o` is omitted, the compiler writes next to the source file with a
`.wasm` suffix. The command surface is declared with the repository's
`cli-builder` package, and compiler diagnostics are reported without Python
tracebacks using the same source-size, parse, type-check, IR, WASM validation,
and encoding stages as the Python API.

The `run` command compiles in memory and executes the generated module's
`_start` export through the local WASM runtime. Program output is forwarded to
stdout, `--print-result` writes `_start` return values to stderr, and
`--max-instructions` applies a host instruction budget so nonterminating ALGOL
programs fail with an explicit runtime diagnostic.

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

captured_many: list[str] = []
runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured_many.append)))
multi_print = compile_source(
    "begin integer result; print('Answer ', 40 + 2, ' ', true); result := 1 end"
)
assert runtime.load_and_run(multi_print.binary, "_start", []) == [1]
assert "".join(captured_many) == "Answer 42 true"

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
- lazy versus value label and switch formals across direct, forwarded, and
  formal-procedure calls
- conditional expressions, chained assignment, and exponentiation in the same
  end-to-end program
- lexical recursion with outer-frame mutation and fresh recursive frames
- procedure-formal closure dispatch that preserves the actual procedure's
  static link
- nonlocal procedure `goto` that unwinds dynamic array storage before the
  caller resumes at the target label
- dynamic multidimensional array bounds captured at block entry
- writable real, boolean, and string by-name scalar formals in one program
- real, boolean, and string by-name actuals through scalar storage,
  array-element storage, expression thunks, and formal-procedure forwarding
- runtime bounds failure through the zero-result failure path before later
  output executes
- mixed `own` scalar and array storage for real, boolean, and string values,
  plus value-array copies and by-name loop-control storage behavior
- boolean and string conditional values, conditional subscripts, terminal labels
  on empty statements, invalid switch indexes, array element caps, and heap
  exhaustion through the zero-result runtime guard path
- real-to-integer and real-output edge cases that would otherwise trap on
  overflow, infinity, or NaN now return through the same zero-result guard path
- a full-surface convergence program combining `own` scalars and arrays,
  default-real arrays, nested and single-statement procedures, value and
  by-name procedure formals, label and switch formals, multiple `for` element
  forms, parenthesized conditional designational expressions, numeric labels,
  boolean operators, real arithmetic, and output
- a surface-audit matrix for publication notation, procedure-call spellings,
  forward procedure and switch visibility, loop forms, typed formal procedures,
  value array copies, nonlocal switch/goto cleanup, and programs without the
  `result` compatibility scalar

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
