# Cross-Language Compiler Portability

## Goal

Make it possible to build compiler toolchains in this repo using any supported
implementation language, while preserving a recognizable pipeline shape:

```text
lexer -> parser -> type checker -> IR compiler -> optimizer -> backend validator -> backend compiler -> assembler/packager
```

Near-term concrete scope:

- every supported implementation language should eventually have a real
  `nib-lexer` and `nib-parser`
- the JVM class-file lane should eventually have a real generic backend and
  matching source-language pipeline packages in each supported bucket
- later compiler stages can remain abstract until each language bucket has the
  prerequisites to support them honestly
- `starlark` is intentionally excluded from this portability push

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

1. Port language lanes instead of isolated packages.
   A useful slice is a runnable `nib-lexer` + `nib-parser`, not just a shared
   utility package in isolation.
2. Reuse the shared grammar files whenever possible.
   `code/grammars/nib.tokens` and `code/grammars/nib.grammar` remain the source
   of truth; language-specific wrappers should stay thin.
3. Keep target constraints out of frontend semantics.
   Language type checkers should remain target-agnostic; hardware constraints
   belong in backend validators.
4. Preserve package naming symmetry where possible.
   A ported Nib frontend should still use recognizable package names such as
   `nib-lexer`, `nib-parser`, and later `nib-type-checker`.

## Immediate Rollout

- Python:
  source of truth today for the Nib pipeline
- TypeScript:
  first non-Python Nib frontend lane (`type-checker-protocol`, `nib-lexer`,
  `nib-parser`)
- Go / Rust / Ruby / Elixir / Lua / Perl / Swift / WASM:
  port `nib-lexer` and `nib-parser` next, using each bucket's existing generic
  lexer/parser substrate where available
- Remaining languages:
  follow the same package shape once the Nib frontend lane pattern settles

## Near-Term Package Targets

Shared infrastructure:

- `type-checker-protocol`
- future generic IR support packages

Concrete language-specific pipeline targets:

- `nib-lexer`
- `nib-parser`

Abstract / later-stage candidates:

- `nib-type-checker`
- `nib-ir-compiler`
- `nib-compiler`

Backend-specific pipeline candidates:

- `intel-4004-ir-validator`
- `ir-to-intel-4004-compiler`
- `intel-4004-assembler`
- `intel-4004-packager`
- `jvm-class-file`
- `ir-to-jvm-class-file`
- `brainfuck-jvm-compiler`
- `nib-jvm-compiler`

## JVM Rollout Note

For the JVM lane, portability should follow the same honesty rule as the rest
of the compiler stack:

- Python is allowed to be the first source-of-truth implementation
- Go now has the first truthful portability foothold through `jvm-class-file`
- other implementation buckets should not grow `brainfuck-jvm-compiler` or
  `nib-jvm-compiler` wrappers until they also have a real local
  `ir-to-jvm-class-file`
- orchestration packages should sit on top of a real backend in the same
  language bucket, not shell out to Python behind the user's back

That means the next Go JVM step should be:

```text
compiler-ir -> ir-to-jvm-class-file -> jvm-class-file -> .class
```

The next honest cross-language JVM wave targets the buckets that already have
both recognizable Brainfuck and Nib compiler lanes:

- Go
- Rust
- TypeScript

The rollout contract for that wave lives in `04j-multi-language-jvm-rollout.md`.
