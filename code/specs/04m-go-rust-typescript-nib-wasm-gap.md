# 04m — Go, Rust, and TypeScript Nib-to-Wasm Gap Closure

## Purpose

The repository now has honest Brainfuck-to-Wasm lanes in Go, Rust, and
TypeScript, and each of those language buckets already contains local Nib
frontend pieces:

```
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-to-wasm-compiler
  -> wasm-module-encoder
  -> .wasm bytes
```

The missing convergence piece is the final orchestration package,
`nib-wasm-compiler`, in each language. This wave closes that gap without
changing the generic IR or Wasm contracts.

## Packages

This wave adds:

- `code/packages/go/nib-wasm-compiler`
- `code/packages/rust/nib-wasm-compiler`
- `code/packages/typescript/nib-wasm-compiler`

The Go and TypeScript Nib IR compilers already cover the smoke-test subset used
by the existing convergence lanes. Rust has the same package names, but its
current IR compiler is smaller, so this wave may extend Rust's
`nib-ir-compiler` only where required for the orchestrator to compile useful
Nib programs.

## Orchestrator Contract

Each `nib-wasm-compiler` package is glue, not a new backend. It must:

- parse Nib source with the local `nib-parser`
- type-check with the local `nib-type-checker`
- lower to generic compiler IR with the local `nib-ir-compiler`
- optionally run the local `ir-optimizer` when available in that language lane
- derive Wasm function signatures from the typed AST
- validate/lower IR with `ir-to-wasm-validator` and `ir-to-wasm-compiler`
- encode the resulting module with `wasm-module-encoder`
- validate the module with `wasm-validator`
- expose `compile_source`, `pack_source`, and `write_wasm_file` style helpers
  matching the existing local Brainfuck-to-Wasm package conventions

## Signature Derivation

The IR-to-Wasm lowerers need explicit signatures for Nib function labels:

- `_start` has `0` parameters and exports as `_start`
- each `fn name(...)` maps to label `_fn_name`
- each source function exports as `name`
- the parameter count is the number of parsed Nib parameters

All prototype Nib functions return one `i32`, matching the current
IR-to-Wasm backend ABI.

## Supported Smoke Subset

This convergence wave targets the subset already exercised by the mature
Python/Ruby/Elixir/Lua/Perl Nib-to-Wasm lanes:

- simple numeric literals
- `u4`, `u8`, `bcd`, and `bool` checks as implemented locally
- function definitions and calls
- `return`
- wrapping addition where already supported by the local IR compiler
- basic counted loops where already supported by the local IR compiler

Rust may start with a narrower, explicit subset if the local frontend cannot
yet represent every mature-lane scenario, but the package must still perform a
real parse/type-check/IR/lower/encode pipeline.

## Test Requirements

Each new package must include package-local tests that cover:

- successful `compile_source` with retained intermediate artifacts
- `pack_source` as an alias
- `write_wasm_file` persisting the produced bytes
- a type-check or parse failure reported as a package-stage error
- at least one runtime smoke test through the local Wasm runtime when that
  language already has a runtime capable of executing the output

The preferred runtime smoke tests are:

- direct function call: `answer() -> 7`
- entrypoint path: `main() -> add(3, 4)` or the nearest local equivalent
- loop path: `count_to(5)` where the local Nib IR compiler supports loops

## Build Metadata

Each package must be publishable in its local ecosystem and must keep BUILD
metadata aligned with declared dependencies:

- Go: `go.mod` requires every sibling visited by `BUILD`; run `go mod tidy`
  in the new package and any transitively affected packages if needed.
- Rust: add the crate to the workspace manifest exactly once and run
  package-scoped formatting/builds.
- TypeScript: declare every local `file:` dependency in `package.json`, install
  transitive siblings in BUILD leaf-to-root order, and do not commit generated
  `.js`, `.d.ts`, or source-map artifacts.

## Non-Goals

This wave does not:

- add missing Brainfuck-to-Wasm lanes for Dart, Haskell, JVM, CLR, Swift, or
  other full-support languages
- redesign the generic IR
- replace the existing IR-to-Wasm backend ABI
- require full Nib language coverage beyond the convergence smoke subset

## Completion Definition

This wave is complete when:

- Go has a local `nib-wasm-compiler` package that builds and tests
- Rust has a local `nib-wasm-compiler` package that builds and tests
- TypeScript has a local `nib-wasm-compiler` package that builds and tests
- the new packages expose the same orchestration surface as their local
  Brainfuck-to-Wasm siblings
- package READMEs and CHANGELOGs describe the new lanes
- security review passes before push
