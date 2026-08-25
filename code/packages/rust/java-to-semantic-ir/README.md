# java-to-semantic-ir

Java CST → narrow-waist Semantic IR. The first frontend for
[SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
nominal/static-dispatch OOP profile extension of the SIR10 narrow-waist IR.
See [JV02](../../../specs/JV02-java-to-semantic-ir.md) for this frontend's
full milestone plan (M0 here, through M9).

## Where this fits

```
Java source
   │
   ▼  coding_adventures_java_parser::parse_java(src, "21")
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  java_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR17 + SIR29)
```

The lowered `Module` can then be validated (`semantic_ir::validate`) and,
once `semantic-ir-to-java` exists (a later slice — see JV02), handed to
that backend or any other SIR backend — "write the Java frontend once,
target every SIR backend" is the whole point of this narrow-waist design.

## Usage

```rust
use java_to_semantic_ir::compile_source;

let module = compile_source(
    "class Main { public static void main(String[] args) { 42; } }",
    "demo",
)?;
```

## Scope (v0.1.0 — JV02 milestone M0)

Java requires an explicit `class`/`main`-method wrapper at the source level
(unlike Ruby/Python/JS, which allow bare top-level statements) — this
milestone recognizes exactly that minimal shape: one top-level class
declaring a `public static void main(String[] args)` method, whose body is
a flat sequence of literal expression statements (`42;`/`3.14;`/`true;`/
`false;`/`null;`/`"str";`). Everything else — variable references,
operators (including unary `-`/`+`/`!`), control flow, method calls,
additional classes/methods/fields, and every SIR29 construct
(`NominalClassDef`/`InterfaceDef`/`MethodDef`/`VirtualCall`) — is out of
scope for this milestone and returns a clean `JavaLowerError` rather than
being silently mis-lowered. See `src/lower.rs`'s own module doc comment for
the exact boundary, and the JV02 spec's milestone table for what M1 onward
adds.

### Testing

- `tests/test_lower.rs` — unit tests over every literal kind this
  milestone supports, the synthesized `main` function's shape, and every
  documented scope-boundary rejection (a variable reference, a binary
  operator, unary minus, a method call, more than one top-level class, a
  missing `main` method). Every positive test also asserts the lowered
  `Module` passes `semantic_ir::validate()` — not just that lowering
  itself didn't error.
- This crate's own tests are pure Rust today (no `javac`/`java` process
  invocation) — M0 only exercises the parser + this frontend's own
  lowering logic. Once a later milestone's tests need to actually execute
  lowered Java (e.g. round-tripping through `semantic-ir-to-java` once it
  exists), add a `# needs-toolchain: java` line to this crate's `BUILD`
  file (see `code/programs/go/build-tool`'s own README/CHANGELOG for that
  mechanism) so CI sets up a JDK for them.

## How it fits in the stack

Part of the [Java/C#/Kotlin SIR initiative](../../../specs/SIR29-nominal-static-oop-profile.md)'s
Phase B — Java frontend + backend. See
[JV01](../../../specs/JV01-java-grammars.md) for the versioned grammar
design `java-lexer`/`java-parser` (this crate's own dependencies)
implement.
