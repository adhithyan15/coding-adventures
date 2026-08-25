# java-to-semantic-ir

Java CST → narrow-waist Semantic IR. The first frontend for
[SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
nominal/static-dispatch OOP profile extension of the SIR10 narrow-waist IR.
See [JV02](../../../specs/JV02-java-to-semantic-ir.md) for this frontend's
full milestone plan (M0 + M1 + M2a + M2b here, through M9).

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

## Scope (v0.4.0 — JV02 milestones M0 + M1 + M2a + M2b)

Java requires an explicit `class`/`main`-method wrapper at the source level
(unlike Ruby/Python/JS, which allow bare top-level statements) — this crate
recognizes exactly that minimal shape: one top-level class declaring a
`public static void main(String[] args)` method. Supported so far: literal
expressions (`42`/`3.14`/`true`/`false`/`null`/`"str"`, M0); local variable
declarations (explicit primitive/`String` types, or `var` type inference),
re-assignment, arithmetic/comparison/logical operators, and `+`-based
string concatenation (M1); `if`/`else`, `while`, `do`/`while`, and compound-
assignment/increment/decrement as bare statements (M2a); classic `for`
(desugared to `while`, since SIR's `Stmt::ForRange` is a canonical counting
loop too narrow for Java's fully general three-clause form — mirrors
`c-to-semantic-ir`'s own precedent for C's equally general `for`) and
enhanced `for` (→ `Stmt::ForEach` directly, M2b) — every block, including a
classic `for`'s own init-declared variable's scope, is a real lexical
scope, mirroring the SIR validator's own block-scoping contract exactly (a
name declared inside one does not leak past it). Everything else —
`switch`/`break`/`continue` (SIR has no IR node for any of the three —
confirmed by a repo-wide grep, not assumed — so this needs a spec-level
design decision before any frontend can target it; note a bare `for (;;)`
loop genuinely cannot terminate without `break`, a real and permanent
limitation until it exists), method calls, field/array access, lambdas,
casts, `instanceof`, the ternary conditional, bitwise/shift operators,
additional classes/methods/fields, and every SIR29 construct
(`NominalClassDef`/`InterfaceDef`/`MethodDef`/`VirtualCall`) — is out of
scope so far and returns a clean `JavaLowerError` rather than being
silently mis-lowered. See `src/lower.rs`'s own module doc comment for the
exact boundary, and the JV02 spec's milestone table for what comes next.

### Testing

- `tests/test_lower.rs` — unit tests over every construct this crate
  supports (all four milestones) and every documented scope-boundary
  rejection, including block-scope leak prevention in both directions (a
  local declared inside an `if`/`do`-`while`/`for`/enhanced-`for` body is
  invisible after it; the outer scope's own name of the same spelling is
  unaffected). Every positive test also asserts the lowered `Module`
  passes `semantic_ir::validate()` — not just that lowering itself didn't
  error.
- `tests/e2e_python.rs` — execution-proof tests, per JV02's own
  "Verification" section. Real Java source lowers through this crate,
  then through the Python backend (`semantic-ir-to-python`, a dev-
  dependency), then runs under `python3`, asserting on real computed
  output — including the do-while "condition already false on entry, but
  the body still runs once" case a plain pretest `while` would get wrong,
  and a classic `for` reusing an already-declared loop variable (a
  different `for_init` grammar alternative from the usual declaration
  form). No execution-proof test exists for enhanced `for` (M1/M2 have no
  array/collection construction syntax yet, so there's no real Java
  expression that lowers to something Python's own `for x in xs:` codegen
  could actually iterate) or for `for (;;)` with empty clauses (it
  genuinely cannot terminate without `break` — an execution proof would
  just hang) — both are covered structurally in `tests/test_lower.rs`
  instead, honestly reflecting what's actually provable at this milestone.
  Python, not JavaScript: the JavaScript backend does not accept
  `Feature::StringInterpolation` yet, and M1's `+`-based string
  concatenation needs it. Since this crate has no way to produce
  observable output on its own terms yet (`System.out.println` is a
  method call, out of scope until M3), the harness redirects `main`'s
  trailing block value to its last statement's expression after lowering
  — a test-harness convenience, not a frontend behavior change — so the
  backend's own unconditional `return <block.value>` epilogue gives it
  something to observe. This test's `python3` dependency is unrelated to
  the JV02 spec's own `needs_java` CI toolchain-detection gap (already
  fixed, in `code/programs/go/build-tool`) — that gap is about getting a
  JDK for a future milestone's own `javac`/`java` oracle comparison, which
  only becomes meaningful once real Java source can produce output to
  compare (M3+); `python3` is a toolchain other cross-language backend
  tests in this repo already depend on. Gracefully skips when `python3` is
  absent from `PATH`.

## How it fits in the stack

Part of the [Java/C#/Kotlin SIR initiative](../../../specs/SIR29-nominal-static-oop-profile.md)'s
Phase B — Java frontend + backend. See
[JV01](../../../specs/JV01-java-grammars.md) for the versioned grammar
design `java-lexer`/`java-parser` (this crate's own dependencies)
implement.
