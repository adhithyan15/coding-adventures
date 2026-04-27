# 04K — Ruby, Elixir, and Lua Nib Convergence

## Goal

Close the most immediate cross-language Nib gap by giving the `ruby`,
`elixir`, and `lua` buckets the same recognizable end-to-end lane shape they
already have for Brainfuck:

```text
Nib source
  -> nib_lexer / nib-lexer
  -> nib_parser / nib-parser
  -> nib_type_checker / nib-type-checker
  -> nib_ir_compiler / nib-ir-compiler
  -> nib_wasm_compiler / nib-wasm-compiler
  -> .wasm bytes
```

This wave is intentionally about convergence, not about proving every corner
of the full Python Nib frontend in one shot.

## Why This Slice

Ruby, Elixir, and Lua already have:

- a local Nib lexer/parser lane
- a local `compiler_ir`
- a local `ir_to_wasm_compiler`
- a local `ir_to_wasm_validator`
- a local WASM encoder / validator stack
- a local Brainfuck-to-WASM orchestration package

That means the only missing pieces are the Nib semantic and orchestration
packages. We should add those directly rather than widening the surface area
with unrelated substrate work.

## Package Set

This wave adds nine packages:

- Ruby:
  - `code/packages/ruby/nib_type_checker`
  - `code/packages/ruby/nib_ir_compiler`
  - `code/packages/ruby/nib_wasm_compiler`
- Elixir:
  - `code/packages/elixir/nib_type_checker`
  - `code/packages/elixir/nib_ir_compiler`
  - `code/packages/elixir/nib_wasm_compiler`
- Lua:
  - `code/packages/lua/nib_type_checker`
  - `code/packages/lua/nib_ir_compiler`
  - `code/packages/lua/nib_wasm_compiler`

Each package must ship with:

- `BUILD`
- `README.md`
- `CHANGELOG.md`
- language-native package metadata
- tests with coverage comfortably above 80%

## Supported Nib Subset

The first convergence wave supports the subset already exercised by the
existing Nib-to-WASM smoke tests and by the current Rust portability lane.

### Declarations

- top-level `fn`
- top-level `const`
- top-level `static`
- local `let`

### Statements

- assignment
- `return`
- `for`
- expression statements for calls

### Expressions

- integer literals
- hex literals
- `true` / `false`
- name references
- function calls
- additive expressions using `+`, `+%`, and `-`

### Explicitly Required Scenarios

The new packages must successfully compile and run these shapes:

- `fn answer() -> u4 { return 7; }`
- `fn add(a: u4, b: u4) -> u4 { return a +% b; }`
- `fn main() -> u4 { return add(3, 4); }`
- `fn count_to(n: u4) -> u4 { let acc: u4 = 0; for i: u4 in 0..n { acc = acc +% 1; } return acc; }`

## Type Checker Contract

Each new `nib_type_checker` package must:

- accept the generic AST shape produced by the local `nib_parser`
- return a `TypeCheckResult`
- expose a typed AST representation that later stages can consume honestly

The typed AST representation may differ by implementation language:

- Ruby may annotate nodes or return a wrapper
- Elixir should return a wrapper, because parser nodes are immutable structs
- Lua may annotate nodes in place or return a wrapper

What matters is the contract:

- later stages can recover the original root
- later stages can recover the inferred type for any checked expression node

### Required Checks

- `let` initializer type matches declared type
- assignment RHS type matches the previously declared variable type
- function call arity matches the declared parameter count
- function call argument types match the declared parameter types
- return expression type matches the declared function return type
- identifiers are declared before use
- `for` loop bounds type-check as numeric

### Diagnostic Shape

Diagnostics should be plain and stage-friendly:

- message
- line
- column

The package does not own backend or hardware constraints.

## IR Compiler Contract

Each new `nib_ir_compiler` package must lower the typed AST into the local
`compiler_ir` package without shelling out to another language.

### Register Convention

Use the same simple Nib v1 convention as the source-of-truth lane:

- `v0`: zero constant or reserved scratch for generic comparisons
- `v1`: expression result and return register
- `v2+`: parameters and locals

Exact internal bookkeeping may vary by implementation language, but the
generated IR must preserve this observable ABI:

- function arguments arrive in `v2`, `v3`, `v4`, ...
- return values leave in `v1`

### Required IR Shapes

- program entry:
  - `LABEL _start`
  - call `_fn_main` when a `main` function exists
  - `HALT`
- function entry:
  - `LABEL _fn_NAME`
- literals:
  - `LOAD_IMM`
- variable copies:
  - `ADD_IMM dst, src, 0`
- addition:
  - `ADD` or `ADD_IMM`
- subtraction:
  - `SUB`
- loops:
  - label / branch / jump pattern reducible by the existing WASM backend
- function calls:
  - caller places arguments into `v2+`
  - emits `CALL _fn_NAME`
- returns:
  - result in `v1`
  - `RET`

Static declarations may emit `.data` entries when the local implementation
can do so cleanly in this wave. This is encouraged but not required for the
initial smoke-test subset.

## WASM Compiler Contract

Each new `nib_wasm_compiler` package must mirror the existing Brainfuck package
shape in its language bucket.

### Public API

- `compile_source`
- `pack_source`
- `write_wasm_file`
- package-specific `PackageResult`
- package-specific `PackageError`

### Required Pipeline

```text
parse
-> type-check
-> IR compile
-> ir-to-wasm validate
-> ir-to-wasm compile
-> wasm validate
-> wasm encode
```

If a local bucket does not yet have an honest IR optimizer, `optimized_ir`
may equal `raw_ir`, just like the existing Brainfuck WASM packages.

### Signature Extraction

The WASM package must derive function signatures from the typed AST:

- `_start` exported as `_start`
- `_fn_NAME` exported as `NAME`
- parameter count inferred from the source-level parameter list

## Testing Requirements

Each language bucket must add package-local tests for:

- type checker success cases
- at least one type error case
- IR compiler smoke coverage
- end-to-end WASM packaging
- running a compiled Nib module through the local WASM runtime

Minimum end-to-end runtime assertions:

- direct function return:
  - compile `answer() -> 7`
  - run exported `answer`
  - observe `7`
- `_start` path:
  - compile `main() -> add(3, 4)`
  - run `_start`
  - observe `7`
- loop path:
  - compile `count_to(5)`
  - run exported `count_to`
  - observe `5`

## Non-Goals

This wave does not attempt to:

- prove total parity with every Python Nib checker rule
- introduce a local IR optimizer where one does not exist
- add JVM or Intel 4004 orchestration to these buckets
- close Perl, Swift, WASM, Java, Kotlin, C#, or F# in the same PR

## Completion Definition

This convergence wave is complete when:

- Ruby has real local `nib_type_checker`, `nib_ir_compiler`, and `nib_wasm_compiler`
- Elixir has real local `nib_type_checker`, `nib_ir_compiler`, and `nib_wasm_compiler`
- Lua has real local `nib_type_checker`, `nib_ir_compiler`, and `nib_wasm_compiler`
- all nine packages build locally
- package-local tests pass
- the new packages provide a recognizable, honest Nib source-to-WASM lane in
  each bucket
