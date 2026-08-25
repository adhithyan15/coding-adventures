# java-to-semantic-ir

Java CST → narrow-waist Semantic IR. The first frontend for
[SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
nominal/static-dispatch OOP profile extension of the SIR10 narrow-waist IR.
See [JV02](../../../specs/JV02-java-to-semantic-ir.md) for this frontend's
full milestone plan (M0 + M1 here, through M9).

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

## Scope (v0.2.0 — JV02 milestones M0 + M1)

Java requires an explicit `class`/`main`-method wrapper at the source level
(unlike Ruby/Python/JS, which allow bare top-level statements) — this crate
recognizes exactly that minimal shape: one top-level class declaring a
`public static void main(String[] args)` method, whose body is a flat
sequence of statements. Supported so far: literal expressions (`42`/`3.14`/
`true`/`false`/`null`/`"str"`, M0); local variable declarations (explicit
primitive/`String` types, or `var` type inference), re-assignment,
arithmetic/comparison/logical operators, and `+`-based string concatenation
(M1). Everything else — control flow, method calls, field/array access,
lambdas, casts, `instanceof`, the ternary conditional, bitwise/shift
operators, additional classes/methods/fields, and every SIR29 construct
(`NominalClassDef`/`InterfaceDef`/`MethodDef`/`VirtualCall`) — is out of
scope so far and returns a clean `JavaLowerError` rather than being
silently mis-lowered. See `src/lower.rs`'s own module doc comment for the
exact boundary, and the JV02 spec's milestone table for what M2 onward
adds.

### Testing

- `tests/test_lower.rs` — unit tests over every construct this crate
  supports (both milestones) and every documented scope-boundary rejection.
  Every positive test also asserts the lowered `Module` passes
  `semantic_ir::validate()` — not just that lowering itself didn't error.
- `tests/e2e_python.rs` — this crate's first execution-proof test, per
  JV02's own "Verification" section. Real Java source lowers through this
  crate, then through the Python backend (`semantic-ir-to-python`, a dev-
  dependency), then runs under `python3`, asserting on real computed
  output. Python, not JavaScript: the JavaScript backend does not accept
  `Feature::StringInterpolation` yet, and M1's `+`-based string
  concatenation needs it. Since M1 has no way to produce observable output
  on its own terms (`System.out.println` is a method call, out of scope
  until M3), the harness redirects `main`'s trailing block value to its
  last statement's expression after lowering — a test-harness convenience,
  not a frontend behavior change — so the backend's own unconditional
  `return <block.value>` epilogue gives it something to observe. This
  test's `python3` dependency is unrelated to the JV02 spec's own
  `needs_java` CI toolchain-detection gap (already fixed, in `code/
  programs/go/build-tool`) — that gap is about getting a JDK for a future
  milestone's own `javac`/`java` oracle comparison, which only becomes
  meaningful once real Java source can produce output to compare (M3+);
  `python3` is a toolchain other cross-language backend tests in this repo
  already depend on. Gracefully skips when `python3` is absent from
  `PATH`.

## How it fits in the stack

Part of the [Java/C#/Kotlin SIR initiative](../../../specs/SIR29-nominal-static-oop-profile.md)'s
Phase B — Java frontend + backend. See
[JV01](../../../specs/JV01-java-grammars.md) for the versioned grammar
design `java-lexer`/`java-parser` (this crate's own dependencies)
implement.
