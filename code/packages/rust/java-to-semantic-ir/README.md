# java-to-semantic-ir

Java CST → narrow-waist Semantic IR. The first frontend for
[SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
nominal/static-dispatch OOP profile extension of the SIR10 narrow-waist IR.
See [JV02](../../../specs/JV02-java-to-semantic-ir.md) for this frontend's
full milestone plan (M0 + M1 + M2a + M2b + M3a + M3b + M4a + M4b here, through M9).

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

## Scope (v0.8.0 — JV02 milestones M0 + M1 + M2a + M2b + M3a + M3b + M4a + M4b)

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
name declared inside one does not leak past it); every method in the class
body — static or instance, both lowering identically to a flat top-level
`Function` since there's no real object/receiver model yet — with typed
parameters, resolved via a two-pass scheme so forward references and
recursion between methods work regardless of textual order; bare
unqualified calls (`foo(a, b)` → `Expr::DirectCall`); and `return`, but
only as the literal last top-level statement of a method body (SIR has no
`Stmt::Return` primitive — a function's value is always its own body
`Block`'s trailing value) (M3a); lambda expressions with explicitly-typed
parameters (`(int x) -> x + 1` → `Expr::MakeClosure`, hoisting the body to
a synthesized top-level function), captures discovered *on-resolve* as the
body is lowered (mirrors `javascript-to-semantic-ir`'s identically-reasoned
approach — a bare name a lambda body references but doesn't itself declare
is captured from however many enclosing scopes/lambda-boundaries away it
actually lives), effectively-final enforcement (assigning to or
incrementing a captured local is rejected, matching real `javac`), and both
lambda-body shapes — an expression directly, or a block using the same
tail-position-only `return` rule methods use, but with no declared type to
validate the returned kind against (M3b — though a lambda value can only be
*created* and passed around this milestone, never actually *invoked*; see
below). Single-dimensional array types (`int[]`/`String[]`/etc.) with a
bare `{ ... }` literal initializer (`int[] xs = {1, 2, 3};`, or `var xs =
{1, 2, 3};` inferring the element kind from the literal itself) lower to
`Expr::SeqLit`; indexing reads (`xs[i]`) lower to `Expr::SeqIndex`; and
`.length` lowers to `Expr::SeqLen` — using SIR16's flat `Sequences`
primitives (`Feature::Sequences`) rather than SIR22's row-major-matrix-
shaped `NDArrays`/`ArrayLit`/`IndexGet` family, confirmed via direct
reading of `semantic-ir`'s own node/validator/Python-backend source to be
the better fit for Java's flat single-dimensional arrays — and the only
one of the two the Python backend already fully supports, which is what
makes a real execution-proof test possible this milestone (M4a). A new
`Kind::Array(ArrayElemKind)` variant (`ArrayElemKind` a small flat `Copy`
enum of `Int`/`Float`/`Bool`/`Str`, kept separate from `Kind` itself so
`Kind` doesn't need a recursive `Box` and lose its own `Copy` derive)
tracks an array-typed local/parameter/return's element kind. Plain indexed
assignment (`xs[i] = v;`) lowers to `Stmt::SeqSet` via a new
`indexed_assign_target` check that runs ahead of the existing bare-name
assignment-target resolution in `lower_expr_statement`, so a plain-name
target (unchanged since M1) and an indexed target (new) are distinguished
before either is lowered (M4b) — **narrowed during implementation**,
mirroring the earlier M2→M2a/M2b and M3→M3a/M3b splits: compound
assignment and increment/decrement on an indexed target (`xs[i] += v;`,
`xs[i]++;`) remain deferred, since naively lowering either would evaluate
the index expression twice (once to read, once to write), silently
double-evaluating any side effect a non-constant index expression carries.
Everything else — `switch`/`break`/`continue` (SIR has no IR node for any
of the three — confirmed by a repo-wide grep, not assumed — so this needs
a spec-level design decision before any frontend can target it; note a
bare `for (;;)` loop genuinely cannot terminate without `break`, a real
and permanent limitation until it exists), qualified calls (`x.foo(...)`),
method overloading, an early or branched `return` (in a method *or* a
lambda), untyped/`var`-inferred lambda parameters (Java infers these from
the lambda's own target functional-interface type, which this frontend has
no visibility into — no functional-interface declarations exist yet),
*invoking* a lambda value (`Expr::IndirectCall` isn't wired up),
multi-dimensional arrays, compound-assignment/increment-decrement on an
indexed target, `new`-based array creation (`new int[5]`/
`new int[]{...}`), List/Map collection literals, the array/String
method-call surface beyond `.length`, field access other than an array's
own `.length`, casts, `instanceof`, the ternary conditional, bitwise/shift
operators, fields/constructors/nested types, and every SIR29 construct
(`NominalClassDef`/`InterfaceDef`/`MethodDef`/`VirtualCall`) — is out of
scope so far and returns a clean `JavaLowerError` rather than being
silently mis-lowered. See `src/lower.rs`'s own module doc comment for the
exact boundary, and the JV02 spec's milestone table for what comes next.

### Testing

- `tests/test_lower.rs` — unit tests over every construct this crate
  supports (all six milestones) and every documented scope-boundary
  rejection, including block-scope leak prevention in both directions (a
  local declared inside an `if`/`do`-`while`/`for`/enhanced-`for` body is
  invisible after it; the outer scope's own name of the same spelling is
  unaffected), M3a's own method/call shapes (forward references,
  self- vs. mutual-recursion, tail-position-return validation,
  duplicate-method-name/wrong-arity/wrong-kind/unknown-callee rejection),
  and M3b's own lambda shapes (expression- and block-bodied, zero/one/
  multi-parameter, captures from both `Local`- and `Param`-scoped
  enclosing declarations, captures crossing *two* nested lambda
  boundaries, effectively-final rejection on assignment/increment to a
  captured local, and every untyped/`var`-parameter rejection), and
  M4a's own array shapes (literal declarations with an explicit type or
  `var`-inferred, empty-array-with-explicit-type, element-kind mismatch
  and empty-array-with-`var` rejection, indexing reads and their kind,
  indexing/`.length` on a non-array value rejection, non-`int` index
  rejection, array-typed method parameters and call-argument kind
  checking, `Feature::Sequences` manifest declaration, and every
  deferred-construct rejection — multi-dimensional array types, nested
  array initializers, and field access other than `.length`), and M4b's
  own indexed-assignment shapes (plain assignment with a constant and a
  variable index, inside a classic `for` loop's own update clause, on a
  `String` array, `Feature::Sequences` re-declaration, index/value kind-
  mismatch rejection, a plain-name assignment regression check alongside
  the new indexed path, and the still-deferred compound-assignment/
  increment-decrement-on-an-indexed-target and `new`-array-creation
  rejections). Every positive test also asserts the lowered `Module`
  passes `semantic_ir::validate()` — not just that lowering itself
  didn't error.
- `tests/e2e_python.rs` — execution-proof tests, per JV02's own
  "Verification" section. Real Java source lowers through this crate,
  then through the Python backend (`semantic-ir-to-python`, a dev-
  dependency), then runs under `python3`, asserting on real computed
  output — including the do-while "condition already false on entry, but
  the body still runs once" case a plain pretest `while` would get wrong,
  a classic `for` reusing an already-declared loop variable (a different
  `for_init` grammar alternative from the usual declaration form), a
  method call, a call resolving a forward reference, and a void call
  running harmlessly alongside a real trailing value. No execution-proof
  test exists for enhanced `for` (M1/M2 have no array/collection
  construction syntax yet, so there's no real Java expression that lowers
  to something Python's own `for x in xs:` codegen could actually
  iterate), `for (;;)` with empty clauses (it genuinely cannot terminate
  without `break` — an execution proof would just hang), recursion
  (plain or mutual — a genuinely *terminating* recursive call needs a
  base case, which needs a branched/early `return`, out of scope for
  M3a, so any recursive call this milestone can express would recurse
  forever if actually run), or lambdas (M3b lowers a real, structurally-
  verified closure value with real capture threading, but has no way to
  *invoke* it — that needs `Expr::IndirectCall`, not wired up this
  milestone — so there is nothing a lambda-using program could do that
  produces *observably different* output than not using one at all) —
  all four are covered structurally in `tests/test_lower.rs` instead,
  honestly reflecting what's actually provable at this milestone. M4a
  adds 4 real execution-proof tests — the first milestone since M3a to
  add any, since unlike lambdas, arrays lower to a primitive
  (`Expr::SeqLit`/`SeqIndex`/`SeqLen`) the Python backend already fully
  supports: an array literal plus `.length`, an indexed read, a full
  indexed `for`-loop summing an array's elements, and a `var`-inferred
  array. M4b adds 3 more: a plain indexed assignment, one with a
  variable (non-constant) index, and a full indexed `for`-loop that
  fills each element by its own index then sums them — exercising
  `.length`, indexed reads, and indexed *writes* together. No
  execution-proof test exists for `new`-based array creation, multi-
  dimensional arrays, or compound-assignment/increment-decrement on an
  indexed target (all remain deferred past M4b). Python,
  not JavaScript: the JavaScript backend does not accept `Feature::
  StringInterpolation` yet, and M1's `+`-based string concatenation needs
  it. The harness redirects `main`'s trailing block value to its last
  statement's expression after lowering — a test-harness convenience, not
  a frontend behavior change — so the backend's own unconditional
  `return <block.value>` epilogue gives it something to observe. This
  test's `python3` dependency is unrelated to the JV02 spec's own
  `needs_java` CI toolchain-detection gap (already fixed, in
  `code/programs/go/build-tool`) — that gap is about getting a JDK for a
  future milestone's own `javac`/`java` oracle comparison, which only
  becomes meaningful once real Java source can produce output to compare;
  `python3` is a toolchain other cross-language backend tests in this
  repo already depend on. Gracefully skips when `python3` is absent from
  `PATH`.

## How it fits in the stack

Part of the [Java/C#/Kotlin SIR initiative](../../../specs/SIR29-nominal-static-oop-profile.md)'s
Phase B — Java frontend + backend. See
[JV01](../../../specs/JV01-java-grammars.md) for the versioned grammar
design `java-lexer`/`java-parser` (this crate's own dependencies)
implement.
