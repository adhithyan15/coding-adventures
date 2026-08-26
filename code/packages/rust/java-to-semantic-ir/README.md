# java-to-semantic-ir

Java CST → narrow-waist Semantic IR. The first frontend for
[SIR29](../../../specs/SIR29-nominal-static-oop-profile.md), the
nominal/static-dispatch OOP profile extension of the SIR10 narrow-waist IR.
See [JV02](../../../specs/JV02-java-to-semantic-ir.md) for this frontend's
full milestone plan (M0 + M1 + M2a + M2b + M3a + M3b + M4a + M4b + M4c + M4d here, through M9; plus standalone tasks #54, wiring `Expr::IndirectCall`, and #64, `break`/`continue` support).

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

## Scope (v0.12.0 — JV02 milestones M0 + M1 + M2a + M2b + M3a + M3b + M4a + M4b + M4c + M4d, plus tasks #54 and #64)

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
validate the returned kind against (M3b). A lambda value can now also be
*invoked* (`f(5)` on a `Closure`-kinded local → `Expr::IndirectCall`, task
#54): `lower_call_expression` checks `resolve_name` on the bare callee
*before* falling back to `method_signatures`, mirroring real Java's own
name-resolution priority — a functional-interface-typed local in scope is
invoked directly through that binding, and a same-named top-level method
isn't reachable through this call syntax while such a local exists.
`Kind::Closure` gained a `u32` index into a new `Lowerer::closure_
signatures` side table (each lambda's own param kinds + return kind,
interned when the lambda is lowered) — needed so an indirect call can
type-check its arguments and pick the right result kind, without
embedding the signature inline on `Kind` itself (which would force it to
drop `Copy`, the same concern `Kind::Array` already navigates by staying
flat). A local that resolves but isn't `Closure`-kinded (`int x = 1;
x();`) is rejected rather than silently falling through to a same-named
method. Reassigning a `Closure`-kinded local is also rejected (found by
`/security-review`): since this crate only tracks a local's `Kind` at
declaration time, an unrejected reassignment would leave a later call
site type-checking against the *original* signature's interned index,
not whatever the variable was actually reassigned to. Single-dimensional
array types (`int[]`/`String[]`/etc.) with a
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
double-evaluating any side effect a non-constant index expression carries
(now its own tracked follow-up task, separate from M4c). `new`-based
array-creation expressions lower via two shapes (M4c): `new int[]{1, 2,
3}` delegates straight to the same array-literal lowering M4a already
built (semantically identical, just `new`-prefixed with an
always-explicit element type); `new int[N]` (sized, uninitialized) only
when `N` is a compile-time-constant, non-negative integer literal under a
`MAX_SIZED_ARRAY_LEN` element-count cap (a CWE-400/770-style resource-
exhaustion guard) and the element kind is numeric or boolean — a
non-constant `N` needs a real repeat/fill SIR primitive that doesn't
exist yet (confirmed by an exhaustive grep of every `Seq*` IR node), and
a reference-typed sized array (`new String[N]`) would need real Java's
own `null`-fill semantics, which this frontend's exact element-kind-match
invariant doesn't cleanly represent yet — both deferred rather than
attempted. Real multi-dimensional arrays (M4d): array *types*
(`int[][]`, capped at a small dimension limit) and explicitly-typed
literal declarations (`int[][] grid = {{1, 2}, {3, 4}};`, including
genuinely ragged rows), plus chained index reads (`grid[i][j]`) via a
generalized `lower_primary_expression` suffix-chain dispatch — a
multi-dimensional array is representationally just a nested sequence of
sequences, so `Kind::Array` gained a dimension count alongside its
existing element kind rather than becoming a boxed, recursive type.
**Narrowed during implementation**: a *mixed* index-then-`.length` chain
(`grid[i].length`) falls through to the pre-existing multi-suffix
rejection (the chained-index dispatch only fires when *every* suffix is
`[...]`-shaped — the sub-array's own `.length` remains reachable via an
intermediate local), and a *chained* indexed-assignment target
(`grid[i][j] = v;`) is not reachable at all — both real, disclosed gaps,
tracked as their own follow-up tasks. Bare (unlabeled) `break`/`continue`
now lower to `Stmt::Break`/`Stmt::Continue` (`Feature::LoopControl`,
task #64) inside any of `while`/`do`-`while`/classic-`for`/enhanced-`for`
— a `Lowerer::loop_depth` counter rejects one outside any loop with a
Java-flavored diagnostic, and is explicitly reset around a lambda body's
or a method body's own lowering so a `break`/`continue` written directly
inside either never resolves against an outer loop the declaration
merely happens to be lexically nested in (real Java forbids this too). A
labeled `break foo;`/`continue foo;` is rejected cleanly — SIR has no
loop-label vocabulary. Wiring `continue` support surfaced two genuine,
`/security-review`-caught non-termination bugs already latent in this
crate's own `do`-`while` and classic-`for` desugarings (both appended a
synthetic "bookkeeping" statement — a guard-flag clear, an update clause —
to the very end of the lowered body, which a `continue` anywhere earlier
would skip entirely): both are fixed by moving that bookkeeping *into*
the loop's own condition expression instead, the one position a
`continue` can never skip. **A second `/security-review` round on that
fix found a third bug**: the synthetic flag names it introduced
(`__do_while_N`/`__for_first_N`) were legal Java identifiers checked
only against locals visible *before* the loop, but their own reference
now lives inside the loop's *condition* — which several backends
(Python, Ruby) compile with flat scoping relative to the body — so a
body-declared local sharing the flag's exact name silently re-armed it
every iteration under those backends, the identical infinite-loop shape
again. Fixed by making the flag names unforgeable (`__do_while#N`/
`__for_first#N` — `#` is illegal in a Java identifier per JLS §3.8) so
no real Java source can ever collide with them, under any backend's
scoping, at any nesting depth — see `CHANGELOG.md`'s `[0.12.0]` entry
for the full before/after shapes and the real-Python-backend regression
tests this finding added. Everything else — `switch`
(SIR still has no IR node for it at all — confirmed by a repo-wide grep,
not assumed — so this needs its own spec-level design decision before
any frontend can target it, tracked as task #51; `break`/`continue`
themselves are supported as of task #64, see below), a labeled `break`/
`continue` (SIR has no loop-label vocabulary at all), qualified calls
(`x.foo(...)`), method overloading, an early
or branched `return` (in a method *or* a lambda), untyped/`var`-inferred
lambda parameters (Java infers these from the lambda's own target
functional-interface type, which this frontend has no visibility into —
no functional-interface declarations exist yet), calling a lambda-valued
*method parameter* (this frontend has no way to declare a method
parameter of a functional-interface type at all, so this is a boundary of
what's expressible, not a gap in invocation itself), `var`-inferred multi-
dimensional array literals, multi-dimensional `new` array-creation forms,
compound-assignment/increment-decrement on an indexed target, a
non-constant or reference-typed `new T[N]`, List/Map collection literals,
the array/String method-call surface beyond `.length`, field access
other than an array's own `.length`, casts, `instanceof`, the ternary
conditional, bitwise/shift operators, fields/constructors/nested types,
and every SIR29 construct (`NominalClassDef`/`InterfaceDef`/`MethodDef`/
`VirtualCall`) — is out of scope so far and returns a clean
`JavaLowerError` rather than being silently mis-lowered. See
`src/lower.rs`'s own module doc comment for the exact boundary, and the
JV02 spec's milestone table for what comes next.

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
  increment-decrement-on-an-indexed-target rejections), and M4c's own
  `new`-array-creation shapes (sized creation for every numeric/boolean
  element kind including a zero-length array, allocate-then-fill-by-index
  alongside M4b's own indexed assignment, negative-size and size-cap
  rejection, non-constant-size and reference-typed-sized-array deferral,
  `new`-with-initializer for both primitive and `String` element kinds,
  element-kind-mismatch and empty-initializer handling, multi-dimensional
  rejection for both `new` shapes, `Feature::Sequences` re-declaration,
  and a regression check that ordinary `new ClassName(...)` object
  construction — a structurally different `primary` alternative — remains
  correctly rejected), and M4d's own multi-dimensional shapes (2-D and
  3-D literal declarations, ragged rows, `String`-element 2-D arrays,
  element-kind mismatch across nested rows, a scalar where a nested
  array was expected, `var`-inference deferral, the dimension-cap
  rejection, chained index reads at 2 and 3 levels with the correctly-
  peeled result kind at each level, a single (non-chained) index read on
  a multi-dimensional array giving back a still-indexable sub-array, an
  out-of-dimension chained index rejection, the mixed index-then-`.`
  suffix-chain rejection, `.length` on a multi-dimensional array,
  `Feature::Sequences` re-declaration, and both the still-deferred
  chained-assignment-target rejection and the now-correctly-generalized
  single-index sub-array assignment), and task #54's own indirect-call
  shapes (zero/single/multi-argument calls with correct argument order,
  the call's own result kind usable in a further expression, a non-main
  method's own local lambda invocation, a captured closure invoked from
  within a nested lambda, wrong-argument-count and wrong-argument-kind
  rejection, calling a non-closure local rejection, and `Feature::
  Closures` re-declaration). Every positive test also asserts the
  lowered `Module` passes `semantic_ir::validate()` — not just that
  lowering itself didn't error.
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
  forever if actually run) — both are covered structurally in `tests/
  test_lower.rs` instead, honestly reflecting what's actually provable at
  that milestone. M3b's own lambdas (real, structurally-verified closure
  values with real capture threading, but no way to *invoke* them at the
  time) went unproven the same way — task #54 finally makes a real
  execution-proof test possible for them; see below. M4a
  adds 4 real execution-proof tests — the first milestone since M3a to
  add any, since unlike lambdas, arrays lower to a primitive
  (`Expr::SeqLit`/`SeqIndex`/`SeqLen`) the Python backend already fully
  supports: an array literal plus `.length`, an indexed read, a full
  indexed `for`-loop summing an array's elements, and a `var`-inferred
  array. M4b adds 3 more: a plain indexed assignment, one with a
  variable (non-constant) index, and a full indexed `for`-loop that
  fills each element by its own index then sums them — exercising
  `.length`, indexed reads, and indexed *writes* together. M4c adds 2
  more: a sized `new int[N]` array allocated then filled and summed by
  index (the realistic pattern M4b and M4c together exist to enable),
  and a `new int[]{...}`-with-initializer indexed read. M4d adds 3 more:
  a 2-D array literal with a chained index read, a nested indexed
  `for`-loop summing a 2-D array via an intermediate row local (since
  `grid[i].length` itself remains deferred — see below), and a genuinely
  ragged 2-D array's two rows' differing `.length`s summed via
  intermediate locals. No execution-proof test exists for a `var`-
  inferred multi-dimensional array literal, a chained (multi-
  dimensional) indexed-assignment target, or compound-assignment/
  increment-decrement on an indexed target (all remain deferred past
  M4d). Task #54 adds 4 more, the first *lambda*-execution proofs this
  crate has ever had: a lambda-valued local invoked with one argument, a
  multi-argument invocation, a nested lambda invoking a value captured
  from its own enclosing scope, and the realistic pattern this task
  exists to enable — the same closure value called repeatedly across
  loop iterations, its captured, effectively-final state read fresh each
  time. No execution-proof test exists for calling a lambda-valued
  *method parameter* (not constructible at all in this frontend's
  current scope — no functional-interface parameter type exists to
  declare one). Python,
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
- `tests/loop_control_java_execution.rs` (task #64): 6 `node`-execution-
  proof tests for `break`/`continue`, mirroring `e2e_python.rs`'s own
  "observe via `main`'s return value" harness but targeting the
  JavaScript backend instead — as of this crate's own v0.12.0, JavaScript
  is the only backend that accepts `Feature::LoopControl` (task #62), and
  none of these tests use string concatenation, so JS's own
  `StringInterpolation` gap (the reason `e2e_python.rs` picked Python)
  doesn't apply. Covers a `while` combining both `continue` and `break`,
  an enhanced-`for` `break`, and nested `while` loops proving `break`/
  `continue` each target only the *innermost* enclosing loop, not an
  outer one. Two tests are direct termination-regression tests for the
  `do`-`while`/classic-`for` bugs `CHANGELOG.md`'s `[0.12.0]` entry
  documents — before the fix, each hung forever rather than returning
  the wrong answer; a reintroduction of either bug would hang the
  corresponding test rather than fail it cleanly. Gracefully skips when
  `node` is absent from `PATH`.
- `tests/loop_control_flat_scoping_regression.rs` (task #64, added by a
  second `/security-review` round): 2 real-`python3`-execution regression
  tests for the flag-name-collision bug this file's own module doc
  comment and `CHANGELOG.md`'s `[0.12.0]` entry describe — the bug never
  reproduced through the JavaScript backend (real `let`/IIFE scoping
  there), only through backends that compile a loop condition/body pair
  with flat scoping, so these tests run the exact reported scenario
  through the real Python backend instead. Each has a hard 15-second
  wall-clock timeout (`Command::spawn` + polling `try_wait`, not the
  unbounded `Command::output()` every other harness here uses) so a
  reintroduction of this specific bug fails the test cleanly instead of
  hanging the whole suite. Gracefully skips when `python3` is absent.

## How it fits in the stack

Part of the [Java/C#/Kotlin SIR initiative](../../../specs/SIR29-nominal-static-oop-profile.md)'s
Phase B — Java frontend + backend. See
[JV01](../../../specs/JV01-java-grammars.md) for the versioned grammar
design `java-lexer`/`java-parser` (this crate's own dependencies)
implement.
