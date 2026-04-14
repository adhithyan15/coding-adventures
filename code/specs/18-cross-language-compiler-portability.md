# Cross-Language Compiler Portability

## Goal

Make it possible to build compiler toolchains in this repo using any supported
implementation language, while preserving a recognizable pipeline shape:

```text
lexer -> parser -> type checker -> IR compiler -> optimizer -> backend validator -> backend compiler -> assembler/packager
```

## Supported Implementation Buckets

- `python`
- `typescript`
- `go`
- `rust`
- `ruby`
- `swift`
- `elixir`
- `lua`
- `perl`
- `starlark`
- `wasm`

## Porting Strategy

1. Port shared infrastructure first.
   Start with packages like `type-checker-protocol` that do not depend on a
   particular source language.
2. Port one full frontend sequence in a non-Python language.
   TypeScript is the first good candidate because the repo already has
   `typescript-lexer` and `typescript-parser`.
3. Keep target constraints out of frontend semantics.
   Language type checkers should remain target-agnostic; hardware constraints
   belong in backend validators.
4. Preserve package naming symmetry where possible.
   A compiler written in another implementation language should still use
   recognizable package names such as `nib-type-checker` or
   `intel-4004-ir-validator`.

## Immediate Rollout

- Python:
  source of truth today for the Nib pipeline
- TypeScript:
  first non-Python shared semantic-analysis infrastructure
- Go / Rust:
  next strong candidates for shared compiler infrastructure ports
- Remaining languages:
  follow the same package shape once the protocol/framework pattern settles

## Near-Term Package Targets

Shared infrastructure:

- `type-checker-protocol`
- future generic IR support packages

Language-specific pipeline candidates:

- `nib-lexer`
- `nib-parser`
- `nib-type-checker`
- `nib-ir-compiler`
- `nib-compiler`

Backend-specific pipeline candidates:

- `intel-4004-ir-validator`
- `ir-to-intel-4004-compiler`
- `intel-4004-assembler`
- `intel-4004-packager`
