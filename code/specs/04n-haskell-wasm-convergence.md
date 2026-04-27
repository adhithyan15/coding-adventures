# 04n - Haskell Brainfuck and Nib Wasm Convergence

## Goal

Close the Haskell end-to-end Wasm gap for the two source languages that are
already part of the generic IR effort:

```text
Brainfuck source
  -> brainfuck
  -> brainfuck-ir-compiler
  -> ir-to-wasm-compiler
  -> wasm-module-encoder
  -> .wasm bytes

Nib source
  -> nib-lexer
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-to-wasm-compiler
  -> wasm-module-encoder
  -> .wasm bytes
```

This Haskell wave is intentionally separate from the Java/Kotlin and C#/F#
waves because Haskell package builds are sensitive to `cabal.project`
dependency ordering in CI.

## Package Set

This wave adds the missing Haskell packages needed for local source-to-Wasm
pipelines:

- `code/packages/haskell/compiler-ir`
- `code/packages/haskell/brainfuck`
- `code/packages/haskell/brainfuck-ir-compiler`
- `code/packages/haskell/ir-optimizer`
- `code/packages/haskell/ir-to-wasm-validator`
- `code/packages/haskell/ir-to-wasm-compiler`
- `code/packages/haskell/brainfuck-wasm-compiler`
- `code/packages/haskell/nib-type-checker`
- `code/packages/haskell/nib-ir-compiler`
- `code/packages/haskell/nib-wasm-compiler`

Every package must include `BUILD`, `BUILD_windows`, `README.md`,
`CHANGELOG.md`, a `.cabal` file, a `cabal.project`, and tests.

## Build Footgun Rule

Every Haskell `cabal.project` in this wave must list transitive local packages
explicitly in leaf-to-root order. Do not rely on Cabal discovering sibling
packages implicitly. The repo build validator and CI run many packages in
clean, package-local contexts, so missing transitive project entries should be
treated as build failures even when a developer machine has cached packages.

## Brainfuck Contract

The Haskell Brainfuck lane must:

- tokenize and parse Brainfuck source into a local AST
- lower Brainfuck operations into `compiler-ir`
- lower that IR into a Wasm module
- validate the Wasm module
- encode raw Wasm bytes
- expose `compileSource`, `packSource`, and `writeWasmFile`

The minimum supported source subset is the complete Brainfuck instruction set:
`>`, `<`, `+`, `-`, `.`, `,`, `[`, and `]`.

## Nib Contract

The Haskell Nib lane must use the existing Haskell `nib-lexer` and `nib-parser`
and add the semantic/compiler pieces needed for the convergence smoke subset:

- top-level `fn`
- local `let`
- assignment
- `return`
- integer literals
- name references
- function calls
- wrapping addition for `u4`

The package must derive Wasm exports from source functions:

- `_start` exports as `_start`
- `_fn_NAME` exports as `NAME`
- function parameters map to the generic IR ABI registers `v2+`
- return values leave functions in `v1`

## Public Orchestrator Results

Both source-to-Wasm packages should return stage-rich result values containing:

- source text
- frontend AST
- raw IR
- optimized IR
- Wasm module
- validated module
- encoded bytes
- optional write path

Nib results must additionally retain the typed AST.

## Error Model

Each orchestrator must report stage-labeled errors. Required stages are:

- `parse`
- `type-check` for Nib
- `ir-compile`
- `validate-ir`
- `lower`
- `validate-wasm`
- `encode`
- `write`

## Tests

Each package should have focused package-local tests. The orchestrator packages
must cover:

- successful `compileSource`
- `packSource` alias behavior
- `writeWasmFile`
- stage-labeled failures
- runtime execution through the local Haskell Wasm runtime when the emitted
  module does not require unsupported host behavior

Preferred runtime smoke programs:

- Brainfuck: simple arithmetic/loop output or a non-IO pointer/cell program
- Nib: `fn answer() -> u4 { return 7; }`
- Nib: `fn add(a: u4, b: u4) -> u4 { return a +% b; }`

## Non-Goals

This wave does not:

- add Haskell JVM class-file generation
- change the generic IR opcode contract
- complete Java/Kotlin or C#/F# convergence
- require the full Nib language beyond the convergence smoke subset

## Completion Definition

The wave is complete when Haskell has honest local Brainfuck-to-Wasm and
Nib-to-Wasm packages, the package builds pass locally, the build metadata
validator does not report new Haskell dependency issues, and the branch passes
security review before push.
