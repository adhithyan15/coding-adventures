# PL03 - ALGOL 60 to WASM Compiler

## Overview

This spec starts the ALGOL 60-to-WASM lane by connecting the already-versioned
ALGOL frontend to the repository's existing compiler IR and WASM backend.

The grammar is already present and versioned:

- `code/grammars/algol/algol60.tokens`
- `code/grammars/algol/algol60.grammar`

Several language buckets already expose grammar-backed ALGOL lexers and
parsers. This wave does not invent a second grammar. It builds on the existing
`algol60` grammar and adds the missing semantic and code-generation path:

```text
ALGOL 60 source
  -> algol-lexer
  -> algol-parser
  -> algol-type-checker
  -> algol-ir-compiler
  -> algol-wasm-compiler
  -> .wasm bytes
```

The first implementation target is Python because that bucket already has the
most complete nearby examples for BASIC-to-IR, Nib-to-IR, IR-to-WASM, and
WASM execution.

## Package Set

This wave adds three Python packages:

- `code/packages/python/algol-type-checker`
- `code/packages/python/algol-ir-compiler`
- `code/packages/python/algol-wasm-compiler`

Each package must be publishable and include:

- `BUILD`
- `BUILD_windows`
- `README.md`
- `CHANGELOG.md`
- `pyproject.toml`
- tests with coverage above the repo threshold

The packages must use the existing Python package conventions for local
dependencies and editable installs. They must not add the new packages to the
Python uv workspace unless a separate spec explains why shared workspace
behavior is required.

## First Supported Subset

ALGOL 60 is historically important but semantically large. The first compiler
slice intentionally targets structured, numeric ALGOL programs that map cleanly
to the existing compiler IR.

### Declarations

- `integer` scalar declarations
- multiple names per declaration, such as `integer x, y`
- nested `begin ... end` blocks with lexical scoping

### Statements

- assignment with `:=`
- compound statements
- nested blocks
- `if ... then ... else ...`
- `for i := start step stride until limit do statement`

### Expressions

- integer literals
- boolean literals
- name references
- parenthesized expressions
- unary `+`, unary `-`, and `not`
- `+`, `-`, `*`, `div`, `mod`
- comparisons: `=`, `!=`, `<`, `<=`, `>`, `>=`
- boolean `and`, `or`

### Entrypoint Convention

The initial compiler treats the outermost block as `_start`. A program returns
its observable result by assigning to an integer variable named `result`.

Example:

```algol
begin
  integer result;
  result := 7
end
```

The compiler emits IR that leaves `result` in virtual register `v1` before
`HALT`. This matches the current lightweight runtime smoke pattern for other
compiler-lane packages.

## Explicitly Deferred

The following ALGOL 60 features remain parseable by the existing grammar but
are not required in the first compiler milestone:

- `real`, `boolean`, and `string` variables as stored user variables
- arrays and dynamic bounds
- switches and computed `goto`
- labels and unstructured `goto`
- procedures
- call-by-name parameters
- `own` declarations
- conditional expressions
- chained assignment beyond the simple `x := expr` form
- string output and runtime I/O

Unsupported features must fail with a clear diagnostic at type-check or IR
compile time. They must not silently miscompile.

## Type Checker Contract

`algol-type-checker` accepts the generic AST produced by `algol-parser` and
returns a `TypeCheckResult`.

The result must contain:

- the original AST root
- a typed scope tree
- a mapping from expression nodes to inferred types
- diagnostics with `message`, `line`, and `column`

The first checker supports these types:

| Source type | Internal type | Notes |
|-------------|---------------|-------|
| `integer` | `integer` | Lowered to WASM `i32` through compiler IR |
| boolean expressions | `boolean` | Used for branches, represented as 0 or 1 in IR |

Required checks:

- declarations introduce names in the current block scope
- redeclaring a name in the same scope is an error
- assignments target declared scalar variables
- assignment RHS type matches the target type
- arithmetic operators require integer operands
- comparisons require integer operands and produce boolean
- boolean operators require boolean operands
- `if` and `for while` conditions require boolean
- `for` start, step, and limit expressions require integer
- identifier references resolve through lexical parent scopes

## IR Compiler Contract

`algol-ir-compiler` lowers the typed AST into `compiler_ir.IrProgram`.

### Virtual Register Convention

- `v0`: reserved zero/scratch register
- `v1`: program result register
- `v2+`: variables and temporaries

Every declared scalar gets a stable virtual register for its lexical lifetime.
Expression temporaries use fresh virtual registers and are never recycled in
the first milestone.

### Required IR Shapes

Program prologue and epilogue:

```text
LABEL _start
... compiled statements ...
ADD_IMM v1, v_result, 0
HALT
```

Assignments:

```text
... compile RHS into v_tmp ...
ADD_IMM v_target, v_tmp, 0
```

Arithmetic:

```text
LOAD_IMM v_tmp, literal
ADD dst, lhs, rhs
SUB dst, lhs, rhs
MUL dst, lhs, rhs
DIV dst, lhs, rhs
```

Comparisons:

```text
CMP_EQ dst, lhs, rhs
CMP_NE dst, lhs, rhs
CMP_LT dst, lhs, rhs
CMP_GT dst, lhs, rhs
```

Less-than-or-equal and greater-than-or-equal may be lowered through the
available comparison and boolean inversion instructions if the IR does not
provide direct opcodes.

Structured `if` labels must follow the existing IR-to-WASM structured naming
convention:

```text
if_N_else
if_N_end
```

Structured loop labels must follow:

```text
loop_N_start
loop_N_end
```

This keeps the first compiler wave on the existing structured WASM lowering
path. A later wave may use the dispatch-loop strategy when labels and `goto`
are enabled.

## WASM Compiler Contract

`algol-wasm-compiler` is orchestration glue. It must expose:

- `compile_source`
- `pack_source`
- `write_wasm_file`
- `AlgolWasmResult`
- `AlgolWasmError`

The required pipeline is:

```text
parse
-> type-check
-> IR compile
-> IR-to-WASM validate
-> IR-to-WASM compile
-> WASM validate
-> WASM encode
```

The package should preserve intermediate artifacts in the result so tests and
future tools can inspect the AST, typed result, IR program, WASM module, and
encoded bytes.

## Tests

Package-local tests must cover:

- successful type checking of a minimal integer program
- undeclared identifier diagnostic
- assignment type mismatch diagnostic
- IR smoke test for `result := 7`
- IR smoke test for arithmetic, such as `result := 1 + 2 * 3`
- IR smoke test for an `if ... then ... else ...`
- IR smoke test for a `for ... step ... until ... do`
- end-to-end WASM packaging with non-empty bytes
- runtime smoke through the Python WASM runtime when the exported `_start`
  result can be observed in the local runtime

## Non-Goals

This spec does not require:

- changing `algol60.tokens` or `algol60.grammar`
- adding a second ALGOL grammar
- changing the generic compiler IR
- changing the IR-to-WASM backend ABI
- implementing full ALGOL 60 procedures or call-by-name semantics
- adding non-Python language buckets in the same PR

## Completion Definition

This wave is complete when:

- the spec is committed before implementation
- the three Python packages exist and build locally
- the first supported subset type-checks, lowers to IR, and packages to WASM
- unsupported ALGOL features fail with clear diagnostics
- README and CHANGELOG files describe the lane and current subset
- build-tool affected-package validation passes for the changed packages
- security review passes before push
