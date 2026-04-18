# 04h — Source-to-JVM Pipeline Packages

## Overview

This spec defines the first **language-facing JVM pipeline packages** that sit
on top of `ir-to-jvm-class-file`.

The generic backend already lowers `compiler_ir.IrProgram` into real JVM
`.class` bytes. The next layer is the same orchestration layer we already use
for WASM:

```text
source language frontend
  -> generic IR
  -> optional IR optimizer
  -> generic JVM class-file backend
  -> parse/validate generated class file
  -> optional write-to-classpath helper
```

For the first slice, this spec covers two packages:

- `brainfuck-jvm-compiler`
- `nib-jvm-compiler`

These packages are **orchestrators**, not new backends. They should stay thin,
language-specific at the frontend boundary, and reuse the existing generic JVM
backend as-is.

## Package Goals

1. Preserve the repo's "small packages connected in order" architecture.
2. Give Brainfuck and Nib a direct `.class` output path.
3. Keep the generated output runnable on ordinary `java` and suitable for later
   GraalVM `native-image` use.
4. Match the existing WASM pipeline package ergonomics so the user experience is
   familiar across targets.

## Package Names

### Python

- `code/packages/python/brainfuck-jvm-compiler`
- `code/packages/python/nib-jvm-compiler`

Import names:

- `brainfuck_jvm_compiler`
- `nib_jvm_compiler`

### Cross-language naming target

When the generic JVM backend is ported to the other implementation buckets, the
same recognizable package names should be used where the language allows:

- Go: `brainfuck-jvm-compiler`, `nib-jvm-compiler`
- Rust: `brainfuck-jvm-compiler`, `nib-jvm-compiler`
- Ruby: `brainfuck_jvm_compiler`, `nib_jvm_compiler`
- Elixir: `brainfuck_jvm_compiler`, `nib_jvm_compiler`
- Lua: `brainfuck_jvm_compiler`, `nib_jvm_compiler`
- Perl: `brainfuck-jvm-compiler`, `nib-jvm-compiler`
- TypeScript: `brainfuck-jvm-compiler`, `nib-jvm-compiler`
- Swift: same logical names, adapted to SwiftPM conventions

## Brainfuck Pipeline

`brainfuck-jvm-compiler` packages the following pipeline:

```text
Brainfuck source
  -> brainfuck
  -> brainfuck-ir-compiler
  -> ir-optimizer (optional)
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> JVMClassArtifact / .class bytes
```

Default policy:

- default source filename: `program.bf`
- default JVM class name: `BrainfuckProgram`
- IR optimization enabled by default
- emit a `main(String[] args)` wrapper by default

The package should expose:

- a compiler object with instance defaults
- `compile_source(...)`
- `pack_source(...)` as an alias for `compile_source(...)`
- `write_class_file(...)`

## Nib Pipeline

`nib-jvm-compiler` packages the following pipeline:

```text
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer (optional)
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> JVMClassArtifact / .class bytes
```

Default policy:

- default JVM class name: `NibProgram`
- IR optimization enabled by default
- emit a `main(String[] args)` wrapper by default

The package should expose:

- a compiler object with instance defaults
- `compile_source(...)`
- `pack_source(...)`
- `write_class_file(...)`

## Result Objects

Both packages should return a package-specific `PackageResult` that preserves
the important artifacts from each pipeline stage.

### Brainfuck result

Recommended fields:

- `source`
- `filename`
- `class_name`
- `ast`
- `raw_ir`
- `optimization`
- `optimized_ir`
- `artifact`
- `parsed_class`
- `class_bytes`
- `class_file_path` (optional, populated only after write)

### Nib result

Recommended fields:

- `source`
- `class_name`
- `ast`
- `typed_ast`
- `raw_ir`
- `optimization`
- `optimized_ir`
- `artifact`
- `parsed_class`
- `class_bytes`
- `class_file_path` (optional)

## Error Model

Like the WASM pipeline packages, each orchestrator should use a small
stage-labeled error type:

- `parse`
- `type-check` (Nib only)
- `ir-compile`
- `lower-jvm`
- `validate-class`
- `write`

The purpose is not to hide exceptions, but to make failures attributable to a
clear pipeline stage.

## Validation Step

The orchestrator packages should immediately parse the generated class bytes
through `jvm-class-file`.

That gives each package a lightweight self-check analogous to the WASM
validator/runtime path:

- the generic backend emits `.class` bytes
- the parser confirms they are structurally parseable
- tests can assert on class names, helper methods, and callable entrypoints

This does **not** replace execution smoke tests. It just makes parseability part
of the pipeline contract.

## Write Semantics

`write_class_file(...)` should accept a classpath root directory, not a fully
qualified file path.

Reason:

- Java and GraalVM care about package/classpath layout
- `ir-to-jvm-class-file.write_class_file(...)` already knows how to map
  `demo.Example` to `demo/Example.class`
- the orchestrator package should preserve that behavior

## Testing

Each package must stay above 80% coverage and should exercise:

1. successful compilation
2. alias behavior for `pack_source`
3. write helper behavior
4. parsed class-file structure
5. at least one end-to-end execution smoke test when GraalVM is available
6. stage-labeled failure behavior for invalid input

### Brainfuck smoke tests

- compile `+.` and inspect parsed class
- compile a program that writes `A`
- when `GRAALVM_HOME` is available, run the generated class with `java`

### Nib smoke tests

- compile a tiny function and inspect parsed class
- type error raises `PackageError(stage="type-check", ...)`
- parse error raises `PackageError(stage="parse", ...)`
- when `GRAALVM_HOME` is available, invoke generated code through a tiny Java
  driver class and observe the returned value

## Cross-Language Rollout

Python is the first implementation bucket because `origin/main` already has:

- `jvm-class-file`
- `ir-to-jvm-class-file`
- Brainfuck and Nib frontend lanes

The other implementation buckets should follow **after** a truthful generic JVM
backend exists in that bucket. The rollout order should be:

1. port `jvm-class-file` support where missing
2. port `ir-to-jvm-class-file`
3. add `brainfuck-jvm-compiler`
4. add `nib-jvm-compiler`

This avoids creating thin wrapper packages that secretly depend on Python or on
shelling out to a different implementation language.

## Bottom Line

`ir-to-jvm-class-file` gives the repo a real generic JVM backend.

This spec adds the next thin layer:

- source-language pipeline packages for Brainfuck and Nib
- Python implementation first
- same recognizable package names across other languages once the generic JVM
  backend has been ported honestly
