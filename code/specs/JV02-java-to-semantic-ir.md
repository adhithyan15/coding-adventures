# JV02 — Java → Semantic IR

## Status

Defines the `java-to-semantic-ir` frontend's milestone sequence (M0–M9),
the `java-lexer`/`java-parser` hardening this frontend's own CI needs
before it can trust those crates, and the CI toolchain-detection gap this
frontend's own execution-proof tests expose. This is the second slice of
the Java frontend/backend initiative, immediately after
[SIR29](SIR29-nominal-static-oop-profile.md) (Slice 0, merged) — the
third slice, `semantic-ir-to-java`, follows once this frontend is usable
end-to-end.

**Implementation progress**: M0 (hardening + `main` + literals), M1
(local variable declarations, re-assignment, arithmetic/comparison/
logical operators, `+`-based string concatenation), M2a (`if`/`else`,
`while`, `do`/`while`, compound-assignment/increment/decrement as bare
statements), M2b (classic `for`, desugared to `while`; enhanced `for`, →
`Stmt::ForEach` directly), M3a (every method in the class body, typed
parameters, two-pass forward-reference/recursion resolution, bare
unqualified calls → `Expr::DirectCall`, `return` in tail position only),
and M3b (lambda expressions with explicitly-typed parameters →
`Expr::MakeClosure`, hoisting the body to a synthesized top-level
function; captures discovered on-resolve, effectively-final enforced;
both lambda-body shapes — though a lambda value can only be created and
passed around this milestone, never actually invoked, since `Expr::
IndirectCall` isn't wired up yet) are merged — see `code/packages/rust/
java-to-semantic-ir`'s own `CHANGELOG.md` for the exact per-milestone
construct list and the real correctness bugs each milestone's own test
suite caught before shipping. M2's own scope split into two PRs (M2a;
M2b) once implementation revealed how much scope-stack infrastructure
`if`/`while`/`do`-`while` alone already needed; M3 similarly split into
M3a and M3b (this section) once research showed M3's combined scope —
multi-function tables, typed params, tail-position return, *and* lambda
capture analysis — was comparably large to M2's own combined scope.
`switch` was also discovered, during M2a, to have no corresponding SIR
IR node at all (confirmed by a repo-wide grep, not assumed) — it needs
its own spec-level design decision (Java's fall-through semantics in
particular) before any frontend can target it, tracked as a separate
backlog item rather than folded into "M2"/"M3" implicitly; `break`/
`continue` have the identical gap. M4 onward are pending.

## Motivation

This is the first SIR frontend for a nominally-typed, static-dispatch
source language — every existing frontend (Ruby/Python/JS/C/Twig/the
math-language family) lowers onto SIR25 §2's dynamic-OOP profile or has
no OOP surface at all. `java-to-semantic-ir` is the intended first
consumer of [SIR29](SIR29-nominal-static-oop-profile.md), lowering Java
`class`/`interface`/`extends`/`implements` source directly onto
`Stmt::NominalClassDef`/`InterfaceDef`/`MethodDef` and an overridden-
method call site onto `Expr::VirtualCall` with a frontend-computed `slot`
— the first real test of whether SIR29's design, confirmed only by
feasibility sketch in that spec, holds up against a real, non-trivial
source language.

## Existing groundwork

`code/packages/rust/java-lexer` / `java-parser` already exist (PR #647, an
earlier, unrelated effort — see [JV01](JV01-java-grammars.md) for the
versioned-grammar design those two crates implement), built on the same
shared `grammar-tools`/
`GrammarLexer`/`GrammarParser` framework every other frontend's own
lexer/parser pair uses. `java_parser::parse_java(source: &str, version:
&str) -> Result<GrammarASTNode, String>` is structurally identical to
every other frontend's own parse entry point — `java-to-semantic-ir` is a
straightforward new `lower.rs` consumer, no new grammar-tools work needed
for the parsing side.

Two gaps this spec budgets for rather than treats as pre-solved:

- **`SUPPORTED_VERSIONS` is 10 curated milestone versions** (`1.0, 1.1,
  1.4, 5, 7, 8, 10, 14, 17, 21`), not continuous 1.0–21 coverage.
  `DEFAULT_VERSION = "21"`. This frontend targets `"21"` by default,
  matching the lexer/parser's own default and the JDK version this repo's
  CI already sets up (`.github/workflows/ci.yml`'s `actions/setup-java@v4`
  step, JDK 21).
- **Test coverage is smoke-level only** — 3 tests in `java-lexer`, 5 in
  `java-parser`, none exercising real class/interface/generic/lambda/
  exception source. M1 (below) budgets time to harden this with real
  source snippets covering every construct this frontend's own milestones
  need, rather than discovering gaps mid-lowering.

## Pipeline

```text
Java source
   │
   ▼  java_parser::parse_java(source, "21")
parser::grammar_parser::GrammarASTNode (generic CST)
   │
   ▼  java_to_semantic_ir::compile_source
semantic_ir::Module                          (per SIR10 + SIR17 + SIR29)
```

## Public API

```rust
pub fn compile(
    tree:        &GrammarASTNode,
    module_name: &str,
) -> Result<semantic_ir::Module, JavaLowerError>;

pub fn compile_source(
    source:      &str,
    module_name: &str,
) -> Result<semantic_ir::Module, JavaLowerError>;  // parse + lower

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaLowerError {
    pub message: String,
    pub line:    usize,
    pub column:  usize,
}
```

Mirrors every existing frontend's own `compile`/`compile_source` shape
(see [SIR19](SIR19-javascript-to-semantic-ir.md)'s identical pair) —
deliberately unremarkable, so a reader who already knows one frontend's
public API knows this one.

## Milestone sequence

Each milestone is its own PR: subset coverage table extended, lowering
implemented, tests added (unit tests per construct + a `compile_source`
round-trip test), `sir-conformance` or a dedicated `tests/e2e_*.rs`
proving the *combination* of that milestone's constructs actually lowers
to a runnable program via at least one backend.

**M0 — hardening + `main`.** Harden `java-lexer`/`java-parser`'s smoke-
level test coverage with real class/interface/generic/lambda/exception
source snippets (the "Existing groundwork" gap above) — this is
prerequisite work, not this frontend's own lowering, but blocks trusting
either crate for anything past the trivial cases the existing 3+5 tests
cover. Also lands the CI `needs_java`-inference extension (see "CI
toolchain-detection gap" below), so this crate's own `javac`/`java`
execution-proof tests (M1 onward) get a deterministic JDK 21 in CI rather
than whatever the runner image happens to ship. Establishes the crate
skeleton: `compile`/`compile_source`, `JavaLowerError`, and lowering for
literals (`42`, `3.14`, `true`/`false`, `null`, `"str"`) plus a
synthesized `main` wrapper (mirrors every other frontend's own M1 —
Java's `public static void main(String[] args)` entry point synthesizes
the same way Ruby/Python/JS's top-level-statement wrapper does).

**M1 — variable references / operators.** Local variable declarations
(`int x = 1;`, `var x = 1;` — Java 10+ type inference, itself scoped to
`SUPPORTED_VERSIONS` entries `>= "10"`), re-assignment, arithmetic/
comparison/logical operators, string concatenation (`+` on `String`).

**M2 — control flow.** `if`/`else`, `while`, classic `for`, enhanced
`for` (`for (T x : xs)` → `ForEach`), `do`/`while`. Split into two PRs
once implementation revealed the real size of the scope-stack
infrastructure `if`/`while`/`do`-`while` alone need (every `Block` is
its own lexical scope, mirroring the SIR validator's own `LocalEnv`
mark/rewind discipline): **M2a** — `if`/`else`, `while`, `do`/`while`,
plus compound-assignment/increment/decrement as bare statements (a real
practical necessity for the classic `for`'s own update clause, pulled
forward from M2b since it's needed there too). **M2b** — classic `for`,
desugared to `Stmt::While` (mirroring `c-to-semantic-ir`'s own precedent
for C's identically-general three-clause `for`, chosen over
`javascript-to-semantic-ir`'s stricter canonical-`ForRange`-only
approach since Java's classic `for` is highly variable in shape); and
enhanced `for`, which lowers directly to `Stmt::ForEach` with no
desugaring needed (SIR already has exactly this shape).
`switch` (statement form; `switch` *expressions*, Java 14+, are a
later-milestone stretch goal regardless) turned out to need its own
spec-level design decision, not a mechanical translation: `semantic-ir`
has **no** `Switch`/`Match`/`Case` IR node at all (confirmed by a
repo-wide grep during M2a's implementation, not assumed from reading
the spec), and Java's `switch` fall-through semantics don't map onto
existing IR surface the way `if`/`while` did — tracked as its own
backlog item, not M2-blocking. `break`/`continue` have the identical
gap (no IR primitive) and the identical "own design decision" status.

**M3 — methods / calls / lambdas.** Static and instance top-level-
function-shaped methods (still not class-nested — that's M6), calls,
Java lambdas (`(a, b) -> body`) → `MakeClosure`, matching how every other
frontend's own arrow/lambda syntax lowers. Split into two PRs, mirroring
M2's own split, once research showed M3's combined scope (multi-function
tables, typed params, tail-position return, *and* lambda capture
analysis) was comparably large to M2's combined scope: **M3a** — every
`method_declaration` in the class body (static or instance; both lower
identically to a flat top-level `Function` since there's no real
object/receiver model until M6) with typed parameters, resolved via a
two-pass scheme (register every method's name + call signature before
any body is lowered, mirroring `python-to-semantic-ir`'s/`javascript-to-
semantic-ir`'s own precedent) so forward references and mutual
recursion between methods work regardless of textual order; bare
unqualified calls (`foo(a, b)`, confirmed via direct CST inspection to
be `primary_expression(primary=NAME, primary_suffix=LPAREN
[argument_list] RPAREN)`) → `Expr::DirectCall`; `return`, accepted only
as the literal last top-level statement of a method body (SIR has no
`Stmt::Return` primitive at all — a function's value is always its own
body `Block`'s trailing value, confirmed by an exhaustive grep of the
`Stmt` enum — so an early or branched return is a clean, disclosed
rejection, not a mis-lowering; real branching-return support needs the
same kind of design work `switch`/`break`/`continue` already need, and
is deferred alongside them). Qualified calls (`x.foo(...)`, which the
grammar distinguishes from a bare call by chaining *two* `primary_suffix`
nodes rather than one) and method overloading (this frontend has no
type-based overload resolution — only one method per name is supported)
are explicitly out of scope. **M3b** — Java lambdas
(`lambda_expression`, both `lambda_body` shapes — expression and block)
→ `Expr::MakeClosure`, reusing `javascript-to-semantic-ir`'s on-resolve
capture-discovery approach (captures fall out of ordinary name
resolution walking the scope-frame stack outward, no separate free-
variable pre-scan needed) rather than `python-to-semantic-ir`'s separate
AST pre-scan, since this crate's existing M1/M2 scope-stack machinery
(`Lowerer.locals`) is architecturally closer to JS's per-frame model
(adapted from JS's one-frame-per-*function* design to this crate's own
one-frame-per-*block* stack via a `closure_stack` of scope-index marks);
the lambda body hoists to a synthesized top-level `Function`
(`__lambda_N`, mirroring how `main` itself is already synthesized).
**Scope narrowed during implementation** from the original three-shape
`lambda_parameters` plan to explicitly-typed parameters only: an
untyped or `var`-inferred lambda parameter's type is inferred by real
Java from the lambda's own *target functional-interface type* (the
abstract method it implements), and this frontend has no visibility
into that at all — no functional-interface declarations exist anywhere
yet (a later SIR29 milestone) — so the bare-name and untyped-
parenthesized `lambda_parameters` shapes are rejected rather than
guessed at, the same "reject rather than mis-lower" discipline this
crate uses everywhere else. Also out of scope: *invoking* a lowered
closure value (`Expr::IndirectCall` is not wired up — a lambda can be
created and passed around, e.g. as a `var`-typed local's initializer,
but never called), so no execution-proof test exists for this milestone
(see `tests/e2e_python.rs`'s own doc comment) — only structural
verification against `semantic_ir::validate()`.

**M4 — arrays / collections / strings / indexing.** Java arrays (`int[]
xs = {1, 2, 3};`, `xs[0]`, `xs.length`), `String` method surface
(`.length()`, `.charAt()`, `.substring()`, etc. — the built-in method
catalog `sir-method-dispatch.md`/`sir-collection-methods.md` already
define, reused rather than redefined), `List`/`Map` collection literals
where a fixed-shape lowering is unambiguous (`List.of(...)`, `new
ArrayList<>()` + `.add`).

**M5 — statics/breadth parity groundwork.** Pulled forward from the
original M9 slot: static field/method access patterns
(`ClassName.field`, `ClassName.staticMethod()`) that M6/M7 need as
building blocks, so class-body lowering (next) isn't blocked re-deriving
static-access shape mid-milestone.

**M6 — classes.** `class Name { fields; constructors; methods; this }` →
`Stmt::NominalClassDef` with nested `Stmt::MethodDef`s (SIR29's own
nest-don't-hoist convention — see that spec's own rationale). One
milestone per OOP feature (fields, then constructors, then instance
methods, then `this`), mirroring `semantic-ir-to-ruby`'s own finer-
grained rollout (v0.11.0 classes+constants → v0.12.0 instance methods →
v0.13.0 ivars/`self`, etc. — see that crate's `CHANGELOG.md`) rather than
one undifferentiated "M6: classes" PR.

**M7 — inheritance / interfaces.** `extends`/`implements` →
`NominalClassDef.superclass`/`.interfaces`; overridden-method call sites
→ `Expr::VirtualCall` with a frontend-computed, per-class-hierarchy-
stable `slot` (SIR29's own dispatch contract — the frontend, not the IR,
owns slot assignment, so this milestone is where that assignment
algorithm actually gets written and tested: a base class's method claims
a slot number, every override in every subclass reuses it). `interface
Name { signatures }` → `Stmt::InterfaceDef`.

**M8 — exceptions.** `try`/`catch`/`finally` → `Stmt::TryCatch`, reusing
the existing SIR17 node exactly as SIR29 itself specifies (no new IR
needed — see that spec's "Explicitly deferred: a checked-vs-unchecked
exception distinction").

**M9 — generics / statics / breadth parity.** Type-erasure-level generics
(`class Box<T> { ... }` → `NominalClassDef.type_params`, `SirType::
TypeParam`), remaining static surface, and breadth parity against
whatever construct coverage gaps M0–M8's own tests surfaced along the
way (mirrors every other frontend's own "final milestone closes gaps its
predecessors didn't anticipate" pattern).

~14–15 PRs total (M0's hardening + CI work is itself 1–2 PRs; M1–M9 are
9 milestones, several split into 2 PRs per the class/OOP finer-grained
precedent above).

## CI toolchain-detection gap

**The problem.** CI's `needs_java` flag (gating `.github/workflows/
ci.yml`'s `actions/setup-java@v4` JDK 21 setup step) is computed by
`code/programs/go/build-tool`'s generic path-bucket language inference
(`internal/discovery/discovery.go`'s `inferLanguage`/`packageBoundary`):
a package's language is whichever directory segment immediately follows
`packages/`/`programs/` in its path. `java-to-semantic-ir` lives at
`code/packages/rust/java-to-semantic-ir` — its bucket is `rust`, so
touching it flips `needs_rust`, never `needs_java`, regardless of how
many of its own tests shell out to `javac`/`java`. Without a fix, this
frontend's own execution-proof tests (M1 onward — real Java source
compiled through `parse_java` → `compile_source` → a backend → run)
either silently skip (if written with the established graceful-skip-when-
toolchain-absent pattern every other cross-language test in this repo
already uses) or run against whatever JDK version the CI runner image
happens to ship by default — not the pinned JDK 21 this repo's own CI
setup step exists specifically to guarantee.

**Why this wasn't a problem for the SIR29 core PR.** That PR touched only
`semantic-ir` and downstream crates whose own tests don't invoke a real
`javac`/`java` process — compile-exhaustiveness fixes need no toolchain
beyond `rustc` itself, so the gap was real but silent (nothing failed,
nothing ran either). `java-to-semantic-ir`'s own M1 execution-proof tests
are the first thing in this initiative that actually needs the fix.

**Shape of the fix** (implementation work, tracked for an M0 PR — not
designed to completion here): `discovery.Package` needs a way to declare
"I need toolchain X" beyond its own inferred bucket language, analogous
to how `gitdiff`'s existing `ciToolchains` mechanism already answers a
different question ("did the CI workflow file itself change, and for
which toolchain's setup step") for a different case. The minimal shape:
a package-level opt-in — e.g. a marker recognized by `discovery`'s own
package-scanning pass (a `// build-tool:needs-toolchain=java` comment
convention, or a small `BUILD`-file directive, matching how this repo's
own `BUILD` files already declare `DeclaredDeps`) — that
`computeLanguagesNeeded` (`main.go`) consults alongside
`toolchainForPackageLanguage(pkg.Language)` rather than instead of it.
Cross-language backend crates that already exist today
(`semantic-ir-to-{go,python,ruby,javascript}`, whose own execution-proof
tests already need `go`/`python3`/`ruby`/`node` on PATH despite living
under `packages/rust`) have the identical latent gap — this fix, once
built, closes it for all of them, not just Java. Whether those crates'
existing tests already tolerate an absent toolchain via graceful skip (as
this session's own established convention for new cross-language tests
already does) determines whether fixing this is urgent for them today;
it is not optional for `java-to-semantic-ir`'s own M1 onward, since a
silently-skipped `javac` proof defeats the entire purpose of an
execution-proof test.

## Verification

Per milestone: `compile_source` unit tests for every new construct
(construction + span handling, mirroring every existing frontend's own
test shape); a `tests/e2e_*.rs` proving that milestone's full construct
set lowers to a `semantic_ir::Module` that a backend can actually run
(gracefully skipping when the relevant toolchain is absent, per this
repo's established cross-language test convention, until the CI gap
above is closed). M7 in particular needs a dedicated test proving the
`VirtualCall.slot` assignment algorithm is correct across a 3+-level
inheritance chain with at least one override — the exact scenario SIR29's
own `Expr::VirtualCall` doc comment calls out as the reason `slot`
(not `class_of(recv)`-based dispatch) is the right primitive.

## References

Internal: [SIR29](SIR29-nominal-static-oop-profile.md) (the IR surface
this frontend targets), [JV01](JV01-java-grammars.md) (the versioned
grammar design `java-lexer`/`java-parser` implement),
[SIR17](SIR17-object-oriented-frontends.md) (`Stmt::TryCatch`, reused
as-is for M8), [SIR19](SIR19-javascript-to-semantic-ir.md) (the frontend
spec template this document's shape follows),
`code/packages/rust/semantic-ir-to-ruby`'s `CHANGELOG.md` (the
finer-grained per-OOP-feature milestone precedent M6 follows).
