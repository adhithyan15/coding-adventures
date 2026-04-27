# 04L — Perl Brainfuck and Nib WASM Convergence

## Goal

Close the next cross-language compiler gap by giving the `perl` bucket the
same recognizable end-to-end WASM lane shape that now exists in
`python`, `typescript`, `go`, `rust`, `ruby`, `elixir`, and `lua`.

The resulting Perl lanes should look like this:

```text
Brainfuck source
  -> brainfuck
  -> brainfuck-ir-compiler
  -> brainfuck-wasm-compiler
  -> .wasm bytes

Nib source
  -> nib-lexer
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> nib-wasm-compiler
  -> .wasm bytes
```

## Why Perl Next

Perl is the highest-leverage next wave because it already has:

- `brainfuck`
- `brainfuck-ir-compiler`
- `nib-lexer`
- `nib-parser`
- `compiler-ir`
- `type-checker-protocol`
- most of the WASM runtime stack:
  - `wasm-leb128`
  - `wasm-types`
  - `wasm-opcodes`
  - `wasm-module-parser`
  - `wasm-validator`
  - `wasm-execution`
  - `wasm-runtime`
  - `wasm-simulator`

That means the missing work is a coherent compiler slice, not a full language
bootstrap.

## Package Set

This convergence wave adds seven packages:

- `code/packages/perl/wasm-module-encoder`
- `code/packages/perl/ir-to-wasm-compiler`
- `code/packages/perl/ir-to-wasm-validator`
- `code/packages/perl/brainfuck-wasm-compiler`
- `code/packages/perl/nib-type-checker`
- `code/packages/perl/nib-ir-compiler`
- `code/packages/perl/nib-wasm-compiler`

Each package must ship with:

- `BUILD`
- `BUILD_windows` when needed by existing Perl package patterns
- `README.md`
- `CHANGELOG.md`
- `Makefile.PL`
- `cpanfile`
- tests

## Honesty Rule

The Perl packages in this wave must be honest local implementations:

- no shelling out to Python, Go, Ruby, Elixir, Lua, or Rust to do the real work
- no embedding pre-generated `.wasm` fixtures as fake compiler output
- no delegating compilation to external tools outside the Perl packages

It is acceptable to follow the existing Perl object and hashref conventions,
even when they differ from the source-of-truth Python package APIs.

## Required Brainfuck Lane

The new Perl `brainfuck-wasm-compiler` package must mirror the existing
Brainfuck orchestration packages in other buckets:

```text
parse
-> compile to compiler-ir
-> validate IR against the WASM lowering rules
-> lower to a WASM module structure
-> validate the WASM module
-> encode the module to raw bytes
```

### Public API

- `compile_source`
- `pack_source`
- `write_wasm_file`
- package-specific `PackageResult`
- package-specific `PackageError`

### Minimum Brainfuck Runtime Scenarios

- compile `+++++++.` to a non-empty binary
- compile a program with a loop and run it through the local WASM runtime
- compile a program that writes a byte via the existing syscall/WASI path

## Required Nib Lane

The new Perl Nib lane must follow the same subset used in the Ruby/Elixir/Lua
convergence wave.

### Supported Nib Subset

#### Declarations

- top-level `fn`
- top-level `const`
- top-level `static`
- local `let`

#### Statements

- assignment
- `return`
- `for`
- expression statements for calls

#### Expressions

- integer literals
- hex literals
- `true` / `false`
- name references
- function calls
- additive expressions using `+`, `+%`, and `-`

### Required Success Shapes

- `fn answer() -> u4 { return 7; }`
- `fn add(a: u4, b: u4) -> u4 { return a +% b; }`
- `fn main() -> u4 { return add(3, 4); }`
- `fn count_to(n: u4) -> u4 { let acc: u4 = 0; for i: u4 in 0..n { acc = acc +% 1; } return acc; }`

## Type Checker Contract

`nib-type-checker` must:

- accept the generic AST produced by Perl `nib-parser`
- return a plain stage-friendly result hash with:
  - `ok`
  - `errors`
  - `typed_ast`
- expose enough type information for the IR compiler to recover expression
  types without recomputing semantic analysis from scratch

Perl may choose either of these implementation shapes:

- annotate AST hashrefs directly
- return a wrapper that keeps the original AST root plus a node-identity map

What matters is the observable contract:

- later stages can recover the original root
- later stages can recover the inferred type of every checked expression node

### Required Checks

- `let` initializer type matches the declared type
- assignment RHS type matches the previously declared variable type
- function call arity matches the declared parameter count
- function call argument types match the declared parameter types
- return expression type matches the declared function return type
- identifiers are declared before use
- `for` loop bounds type-check as numeric

### Diagnostic Shape

Diagnostics must stay plain and portable:

- `message`
- `line`
- `column`

## IR Compiler Contract

`nib-ir-compiler` must lower the checked Nib program into the local
`compiler-ir` package without leaving the Perl bucket.

### Register Convention

Use the same simple Nib v1 convention already established in the source-of-truth
lane:

- `v0`: reserved zero/scratch register
- `v1`: expression result and return register
- `v2+`: parameters and locals

Observable ABI:

- function arguments arrive in `v2`, `v3`, `v4`, ...
- return values leave in `v1`

### Required IR Shapes

- program entry:
  - `LABEL _start`
  - call `_fn_main` when `main` exists
  - `HALT`
- function entry:
  - `LABEL _fn_NAME`
- literals:
  - `LOAD_IMM`
- variable copies:
  - `ADD_IMM dst, src, 0`
- arithmetic:
  - `ADD`, `ADD_IMM`, `SUB`
- loops:
  - label / branch / jump pattern already reducible by the current WASM backend
- function calls:
  - caller places arguments into `v2+`
  - emits `CALL _fn_NAME`
- returns:
  - result in `v1`
  - `RET`

Static declarations may emit `.data` entries when that keeps the Perl port
clean, but the initial smoke-test subset does not require full static-memory
coverage.

## WASM Module Encoder Contract

Perl currently has a module parser and validator but not an encoder, so this
wave must add `wasm-module-encoder`.

The encoder must accept the existing Perl WASM module shape and emit raw
WebAssembly 1.0 bytes. At minimum it must support the section shapes produced
by the new Perl lowering layer:

- type section
- import section
- function section
- memory section
- export section
- code section
- data section

Support for tables, globals, start, elements, and custom sections is
encouraged when it keeps the encoder aligned with the source-of-truth Python
package and with the existing Perl module parser.

## IR-to-WASM Compiler Contract

The Perl `ir-to-wasm-compiler` package must conservatively follow the current
Python backend contract rather than invent a new lowering model.

### Required Opcode Support

For this convergence wave, support the opcodes already exercised by the Perl
Brainfuck and Nib subsets:

- `LABEL`
- `COMMENT`
- `LOAD_IMM`
- `LOAD_ADDR`
- `LOAD_BYTE`
- `STORE_BYTE`
- `LOAD_WORD`
- `STORE_WORD`
- `ADD`
- `ADD_IMM`
- `SUB`
- `AND`
- `AND_IMM`
- `CMP_EQ`
- `CMP_NE`
- `CMP_LT`
- `CMP_GT`
- `BRANCH_Z`
- `BRANCH_NZ`
- `JUMP`
- `CALL`
- `RET`
- `HALT`
- `NOP`
- `SYSCALL`

### Control Flow Model

The lowerer may reject arbitrary unstructured control flow. It only needs to
support the structured patterns already emitted by the current Brainfuck and
Nib IR compilers:

- canonical loop labels:
  - `loop_<n>_start`
  - `loop_<n>_end`
- canonical conditional labels:
  - `if_<n>_else`
  - `if_<n>_end`

### Function Signatures

The lowerer must support explicit source-provided signatures for:

- `_start`
- `_fn_NAME`

And it may also infer Nib signatures from debug comments, matching the current
Python backend.

### Syscall Support

Support the existing syscall subset already used by the WASM convergence lanes:

- write
- read
- exit

Lower these through WASI imports into the same conservative scratch-memory
scheme used by the Python backend.

## IR-to-WASM Validator Contract

`ir-to-wasm-validator` may stay intentionally thin:

- attempt to lower the IR using the local Perl `ir-to-wasm-compiler`
- return an empty list on success
- return a structured validation error list on failure

This mirrors the current Python validator package and keeps the rule surface in
one place.

## Testing Requirements

Each new Perl package must include package-local tests.

### Encoder Tests

- encode a minimal empty-ish module with one function
- encode imported WASI function metadata
- round-trip parser sanity where practical:
  - encode
  - parse with local `wasm-module-parser`
  - validate with local `wasm-validator`

### Lowering Tests

- lower a minimal `HALT` program
- lower a memory-touching Brainfuck-style program
- lower a simple function call
- reject unsupported or malformed IR with a structured error

### Brainfuck End-to-End Tests

- compile source into non-empty `.wasm` bytes
- run a simple write program through Perl `wasm-runtime`
- run a loop-containing program through Perl `wasm-runtime`

### Nib End-to-End Tests

- type checker success case
- at least one type error case
- IR compiler smoke coverage
- end-to-end WASM packaging
- run compiled Nib through the local WASM runtime

Minimum runtime assertions:

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

- add Perl JVM convergence in the same PR
- close Haskell, WASM, Swift, Java, Kotlin, C#, or F#
- prove full parity with every Python Nib checker rule
- add a Perl IR optimizer

## Completion Definition

This Perl convergence wave is complete when:

- Perl has real local `wasm-module-encoder`
- Perl has real local `ir-to-wasm-compiler`
- Perl has real local `ir-to-wasm-validator`
- Perl has a real local `brainfuck-wasm-compiler`
- Perl has real local `nib-type-checker`, `nib-ir-compiler`, and `nib-wasm-compiler`
- the new packages build locally
- package-local tests pass
- both Brainfuck and Nib have recognizable honest source-to-WASM lanes in the
  Perl bucket
