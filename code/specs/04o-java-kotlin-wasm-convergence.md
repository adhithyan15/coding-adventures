# 04o - Java and Kotlin Brainfuck/Nib Wasm Convergence

## Goal

Close the Java and Kotlin source-to-Wasm pipeline gap for the two compiler
frontends that the repo treats as convergence canaries:

```text
Brainfuck source -> brainfuck-wasm-compiler -> .wasm bytes
Nib source       -> nib-wasm-compiler       -> .wasm bytes
```

This batch intentionally follows the Haskell wave and precedes the C#/F# wave.
The Java and Kotlin package roots already have Wasm parser/runtime components,
but they do not yet have a Wasm module encoder or generic compiler IR stack.
For this convergence pass, each source-to-Wasm package may therefore emit the
small Wasm binary subset it owns directly.

## Package Set

The batch adds four publishable package directories:

- `code/packages/java/brainfuck-wasm-compiler`
- `code/packages/java/nib-wasm-compiler`
- `code/packages/kotlin/brainfuck-wasm-compiler`
- `code/packages/kotlin/nib-wasm-compiler`

Each package must include:

- `BUILD`
- `BUILD_windows`
- `.gitignore`
- `README.md`
- `CHANGELOG.md`
- `build.gradle.kts`
- `settings.gradle.kts`
- unit tests

## Brainfuck Contract

The Java and Kotlin Brainfuck packages must:

- accept Brainfuck source as a string
- ignore non-Brainfuck comment characters
- validate balanced loops
- expose `compileSource`, `packSource`, and `writeWasmFile`
- return a result object containing source text, normalized operations, Wasm
  bytes, and optional write path
- emit a valid Wasm module exporting `_start`

The emitted module must include linear memory and must support the complete
Brainfuck instruction alphabet: `>`, `<`, `+`, `-`, `.`, `,`, `[`, and `]`.
Runtime smoke tests may use non-I/O programs so they do not depend on host WASI
behavior.

## Nib Contract

The Java and Kotlin Nib packages must:

- accept Nib source as a string
- support the convergence subset:
  - top-level `fn`
  - zero or more `u4` parameters
  - `return`
  - integer literals in the `0..15` range
  - name references
  - function calls
  - wrapping addition via `+%`
- expose `compileSource`, `packSource`, and `writeWasmFile`
- derive Wasm exports from source functions
- return a result object containing source text, parsed function metadata, Wasm
  bytes, and optional write path

Nib functions compile to exported Wasm functions with `i32` parameters and an
`i32` result. The `_start` export is optional for this batch because Nib smoke
tests exercise named exported functions directly.

## Error Model

Each package must use stage-rich errors where practical:

- `parse`
- `validate`
- `encode`
- `write`

The packages should fail closed on malformed source rather than emitting a
partial module.

## Tests

Each package must cover:

- successful `compileSource`
- `packSource` alias behavior
- `writeWasmFile`
- malformed source errors
- runtime execution through the existing local Wasm runtime for at least one
  emitted module

Preferred runtime smokes:

- Brainfuck: `+` runs exported `_start`
- Nib: `fn answer() -> u4 { return 7; }` runs exported `answer`
- Nib: `fn add(a: u4, b: u4) -> u4 { return a +% b; }` runs exported `add`

## Non-Goals

This batch does not:

- add a Java/Kotlin generic compiler IR package
- add a Java/Kotlin Wasm module encoder package
- add Java/Kotlin JVM class-file generation
- complete the C#/F# convergence batch
- support the full Nib language beyond the convergence subset

## Completion Definition

The batch is complete when Java and Kotlin each have local Brainfuck-to-Wasm
and Nib-to-Wasm packages, package-local tests pass, the build-plan validator
does not report new Java/Kotlin dependency issues, and the branch passes
security review before push.
