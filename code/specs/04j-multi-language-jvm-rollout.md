# 04j - Multi-Language JVM Rollout

## Overview

This spec defines the next JVM portability wave after the merged Python and Go
foundation work.

The goal is not to port the JVM lane to every implementation bucket at once.
The goal is to port it to every bucket that already has **honest Brainfuck and
Nib compiler lanes** and therefore has the prerequisites to host a real local
JVM backend.

For this rollout, the target buckets are:

- `go`
- `rust`
- `typescript`

Python is already complete enough to act as the source-of-truth reference for
the package shapes and behavior.

## Why These Buckets

These three buckets already have the necessary pre-JVM pieces:

- `brainfuck`
- `brainfuck-ir-compiler`
- at least one recognizable Brainfuck backend pipeline
- `nib-parser`
- `nib-type-checker` or a comparable Nib frontend lane
- `nib-ir-compiler`
- at least one recognizable Nib backend pipeline
- `compiler-ir`
- `bytecode-compiler`
- `jvm-simulator`

Buckets that only have frontends, or only have Brainfuck compiler lanes, are
explicitly out of scope for this slice.

## Required Packages Per Target Bucket

Each target bucket should end this rollout with the following JVM packages:

1. `jvm-class-file`
2. `ir-to-jvm-class-file`
3. `brainfuck-jvm-compiler`
4. `nib-jvm-compiler`

Go already has `jvm-class-file`, so its work starts at item 2.

Rust and TypeScript start from item 1.

## Pipeline Shape

The target package graph should look like this in every completed bucket:

```text
Brainfuck source
  -> brainfuck
  -> brainfuck-ir-compiler
  -> ir-to-jvm-class-file
  -> jvm-class-file
  -> .class

Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-to-jvm-class-file
  -> jvm-class-file
  -> .class
```

## Behavioral Contract

### `jvm-class-file`

Each bucket's `jvm-class-file` package should provide the same conservative
minimal surface already established by Python and Go:

- parse a conservative `.class` subset
- build a minimal one-class, one-method class file
- resolve class, field, method, name-and-type, and loadable constants

### `ir-to-jvm-class-file`

Each bucket's `ir-to-jvm-class-file` package should:

- consume local `compiler-ir`
- emit plain class bytes directly
- stay verifier-friendly and Graal-friendly
- avoid dynamic JVM features
- expose a write helper that maps the class name to a classpath path

### Source-language orchestrators

`brainfuck-jvm-compiler` and `nib-jvm-compiler` should stay thin and mirror the
existing Python JVM orchestrators and the existing WASM orchestrator ergonomics
in that bucket:

- compile source into local IR
- optionally optimize IR when the bucket already does that in similar pipelines
- lower through local `ir-to-jvm-class-file`
- parse the generated class through local `jvm-class-file`
- expose compile and write helpers

## Rollout Order Inside Each Bucket

To keep the architecture honest, each bucket should follow this order:

1. `jvm-class-file`
2. `ir-to-jvm-class-file`
3. `brainfuck-jvm-compiler`
4. `nib-jvm-compiler`

That order may proceed in parallel **across buckets**, but should not be
skipped **within a bucket**.

## Testing

Every new package still needs the repo-standard support files:

- `BUILD`
- `README.md`
- `CHANGELOG.md`

And every package should stay above 80% coverage.

Minimum validation per bucket:

1. `jvm-class-file` round trips and malformed-input coverage
2. `ir-to-jvm-class-file` lowering coverage against local `compiler-ir`
3. Brainfuck source-to-`.class` orchestration coverage
4. Nib source-to-`.class` orchestration coverage

Where the bucket already has a convenient Java runtime smoke-test pattern, that
is a bonus, but structural parseability through `jvm-class-file` is the minimum
portable contract.

## Safety Constraints

Every parser/builder implementation in this rollout must:

- reject malformed lengths before converting them to host-sized integers
- avoid unbounded recursion on attacker-controlled nested structures
- treat dynamic or unsupported JVM features as unsupported rather than trying to
  emulate them loosely

The Python and Go security lessons around malformed class-file parsing apply to
every new bucket in this rollout.

## Bottom Line

This rollout makes the JVM lane a real cross-language family instead of a
Python-only feature with an early Go foothold.

When complete, the repo should have:

- Go: generic backend + Brainfuck/Nib JVM orchestrators
- Rust: generic backend + Brainfuck/Nib JVM orchestrators
- TypeScript: generic backend + Brainfuck/Nib JVM orchestrators

all using local implementations, not shell-outs to Python.
