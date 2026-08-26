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
both lambda-body shapes), M4a (single-dimensional array
types with a bare `{ ... }` literal initializer → `Expr::SeqLit`,
indexing reads → `Expr::SeqIndex`, `.length` → `Expr::SeqLen` — SIR16's
`Sequences` primitives, not SIR22's `NDArrays`/matrix family; see this
section's own M4a entry for why), M4b (plain indexed assignment,
`xs[i] = v;` → `Stmt::SeqSet`, distinguished from a bare-name assignment
target by a new check run ahead of the existing bare-name-only
resolution — compound-assignment/increment-decrement on an indexed
target remains deferred; see this section's own M4b entry for why), and
M4c (`new`-based array-creation expressions — `new int[]{1,2,3}`
delegating to M4a's own array-literal lowering, and `new int[N]`
zero-filled sized creation only for a compile-time-constant,
non-negative, capped-size numeric/boolean `N`; see this section's own
M4c entry for why), and M4d (real multi-dimensional arrays — array
types and explicitly-typed literal declarations, capped at a small
dimension limit, plus chained index reads via a generalized suffix-chain
dispatch; a mixed index-then-`.length` chain and a chained indexed-
assignment target both remain deferred; see this section's own M4d
entry for why), and task #54 (wiring `Expr::IndirectCall` — a
`Closure`-kinded local can now be *invoked*, `f(5)`, not just created and
passed around; `lower_call_expression` checks local-variable resolution
ahead of the top-level-method lookup, mirroring real Java's own name-
resolution priority; `Kind::Closure` gained a `u32` index into a new
interned-signature side table so an indirect call can type-check its
own arguments and result — this finally makes a real execution-proof
test possible for M3b's own lambdas, the first this crate has ever had)
are merged — see `code/packages/rust/java-to-semantic-ir`'s own
`CHANGELOG.md` for the exact per-milestone construct list and the real
correctness bugs each milestone's own test suite caught before shipping.
M2's own scope split into two PRs (M2a;
M2b) once implementation revealed how much scope-stack infrastructure
`if`/`while`/`do`-`while` alone already needed; M3 similarly split into
M3a and M3b (this section) once research showed M3's combined scope —
multi-function tables, typed params, tail-position return, *and* lambda
capture analysis — was comparably large to M2's own combined scope; M4
was likewise narrowed into M4a (merged) and M4b (merged) once design
research (grammar probing + a direct read of `semantic-ir`'s own node/
validator/backend source) showed the original undifferentiated M4 scope
— arrays, `new`-based array creation, indexed assignment, multi-
dimensional arrays, `String` methods, and `List`/`Map` literals, all at
once — was comparably large to M2's and M3's own combined scopes; M4b
was narrowed *again* during its own implementation (indexed assignment
alone turning out comparably sized to M4a) into M4b (plain indexed
assignment only, merged) plus M4c (`new`-based array creation, merged —
its own original bundling with compound-assignment/increment-decrement
on an indexed target was narrowed *yet again* once implementation
revealed those were two structurally unrelated pieces of work; that
piece is now its own standalone follow-up task, not a lettered
sub-milestone) and M4d (this section, multi-dimensional arrays, merged
— during its own implementation, a mixed index-then-`.length` suffix
chain was similarly split off into its own standalone follow-up task
rather than folded into this milestone's own scope) — see those entries
below. `switch` was also discovered, during M2a, to have no
corresponding SIR IR node at all (confirmed by a repo-wide grep, not
assumed) — it needs its own spec-level design decision (Java's
fall-through semantics in particular) before any frontend can target it,
tracked as a separate backlog item rather than folded into "M2"/"M3"
implicitly; `break`/`continue` had the identical gap. **Update**: fully
resolved as of task #64. `Stmt::Break`/`Stmt::Continue`/`Feature::
LoopControl` landed at the core-IR level first (see
[SIR16](SIR16-ir-extensions-for-python-and-javascript.md)'s "Loop control
(addendum)" section), `semantic-ir-to-javascript` became the first
backend to accept the feature (task #62), and this frontend now lowers a
bare `break`/`continue` inside any of `while`/`do`-`while`/classic-`for`/
enhanced-`for` (task #64) — including fixing two genuine, `/security-
review`-caught non-termination bugs task #64 found already latent in this
crate's own `do`-`while` and classic-`for` desugarings (each had
appended a synthetic bookkeeping statement — a guard-flag clear, the
update clause — to the very end of the lowered body, which a `continue`
anywhere earlier would skip entirely; both are fixed by moving that
bookkeeping into the loop's own condition expression instead, the one
position a `continue` can never skip — see `java-to-semantic-ir`'s own
`CHANGELOG.md` `[0.12.0]` entry for the full shapes). A labeled `break`/
`continue` remains rejected (SIR has no loop-label vocabulary at all).
`switch` itself remains fully unaddressed — tracked as task #51. M5
onward, plus the standalone follow-up tasks split off from M4c and M4d,
are pending.

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
backlog item, not M2-blocking. `break`/`continue` had the identical gap
at M2a time; fully resolved as of task #64 — see the "Implementation
progress" note near the top of this spec for the full story (core-IR
primitive, first backend, this frontend's own consumption plus two
non-termination bugs fixed along the way). `switch` itself remains its
own, separately-tracked (task #51) unresolved backlog item.

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
rejection, not a mis-lowering; real branching-return support needs its
own comparable core-IR design work, same as `switch` still does — see the
note on `break`/`continue` above, whose own IR primitive has since landed
even though this frontend doesn't consume it yet). Qualified calls
(`x.foo(...)`, which the
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
crate uses everywhere else. *Invoking* a lowered closure value was
originally out of scope for this milestone (`Expr::IndirectCall` wasn't
wired up — a lambda could be created and passed around, e.g. as a
`var`-typed local's initializer, but never called, so no execution-proof
test existed here — only structural verification against
`semantic_ir::validate()`); see task #54 (below) for where invocation was
actually wired up.

**M4a — array declarations, indexing reads, `.length`.** Single-
dimensional Java arrays of primitive/`String` element type: `int[] xs =
{1, 2, 3};` (bare `{ ... }` array-initializer literal syntax only —
`new int[5]`/`new int[]{...}` array-creation-*expression* forms are
deferred to M4b; confirmed via empirical probing, not assumed, that they
genuinely fall through to this frontend's existing "unsupported primary
expression" rejection rather than being silently mis-lowered), indexing
reads (`xs[0]`), and `.length`. Lowers to SIR16's `Sequences` primitives
(`Expr::SeqLit`/`SeqIndex`/`SeqLen`, `Feature::Sequences`) rather than
SIR22's row-major-matrix-shaped `NDArrays`/`ArrayLit`/`IndexGet` family —
a direct read of `semantic-ir`'s own node/validator/Python-backend source
confirmed SIR16 Sequences is the better structural fit for Java's flat
1-D arrays, and the only one `semantic-ir-to-python` already fully
supports without a separate `sir-runtime-array` dependency, which is what
makes this the first milestone since M3a able to add a real execution-
proof test (unlike M3b's lambdas, creatable but not yet invocable). A new
`Kind::Array(ArrayElemKind)` variant (`ArrayElemKind` a small flat `Copy`
enum, kept separate from `Kind` so `Kind` itself doesn't need a recursive
`Box` and lose its own `Copy` derive) tracks an array-typed local's/
parameter's/return's element kind — since every array-typed declaration,
parameter, and return type already routes through the one shared
`kind_of_type_node`, array-typed method parameters and call-argument kind
checking fall out for free as a side effect of this milestone, without
needing their own dedicated work.

**M4b — plain indexed array assignment.** Deferred from M4a during scope
narrowing, then narrowed *again* once its own implementation began.
`xs[i] = v;` lowers to `Stmt::SeqSet` via a new `indexed_assign_target`
check in `lower_expr_statement`, run ahead of the existing bare-name-only
assignment-target resolution (`extract_bare_name`) — it recognizes the
one new target shape (`primary_expression` with exactly one `[...]`
suffix) and falls through to the unchanged bare-name path for everything
else. `Stmt::SeqSet` needs only `Feature::Sequences` (already declared
since M4a); its `seq` field is an arbitrary expression, not a bound name,
so unlike `Stmt::Assign` no `check_varref` applies. **Compound assignment
and increment/decrement on an indexed target (`xs[i] += v;`, `xs[i]++;`)
remain deferred to M4c**, not attempted here: naively lowering either
would evaluate the index expression twice (once to read the current
element, once to write the new one), silently double-evaluating any side
effect a non-constant index expression carries (e.g. a method call used
as the index) — the same class of correctness bug this frontend's own
`/security-review` history has caught before in the do-while and
for-update desugarings (see `code/packages/rust/java-to-semantic-ir`'s
own `CHANGELOG.md`, `[0.3.0]`/`[0.4.0]`).

**M4c — `new`-based array-creation expressions.** Deferred from M4b
during its own scope narrowing (see that entry above); narrowed *again*
during its own implementation once compound-assignment/increment-
decrement on an indexed target turned out to be a structurally unrelated
piece of work (split off into its own standalone follow-up task rather
than a lettered sub-milestone — see below). `new int[]{1,2,3}` (explicit
array-creation-type + initializer, semantically identical to the bare
`{1,2,3}` form M4a already supports, just `new`-prefixed) delegates
directly to M4a's own `lower_array_initializer`. `new int[5]` (sized,
uninitialized) lowers to a zero-filled `Expr::SeqLit`, but only when the
size is a compile-time-constant, non-negative integer literal under a
`MAX_SIZED_ARRAY_LEN` element-count cap (a CWE-400/770-style resource-
exhaustion guard) and the element kind is numeric or boolean — SIR16 has
no repeat/fill primitive (confirmed by an exhaustive grep of every
`Seq*` node), so a non-constant size genuinely cannot be represented
without a new SIR primitive that doesn't exist yet, and a reference-
typed sized array would need real Java's own `null`-fill semantics,
which this frontend's exact element-kind-match invariant doesn't
cleanly represent yet — both rejected rather than attempted. Both new
`primary`-expression shapes were confirmed via empirical CST probing,
not assumed from the grammar text alone.

**Also tracked as its own follow-up task (not a lettered sub-
milestone), split off from M4c during its own scope narrowing**:
compound-assignment/increment-decrement on an indexed target (`xs[i] +=
v;`, `xs[i]++;`) — it needs either a temp-variable-hoisting design (to
evaluate the index expression exactly once) or a determination that this
frontend's currently-reachable index expressions are narrow enough in
practice to skip that safely; naively lowering either without one of
those would evaluate the index expression twice (once to read, once to
write), silently double-evaluating any side effect a non-constant index
expression carries (e.g. a method call used as the index) — the same
class of correctness bug this frontend's own `/security-review` history
has caught before in the do-while and for-update desugarings (see
`code/packages/rust/java-to-semantic-ir`'s own `CHANGELOG.md`,
`[0.3.0]`/`[0.4.0]`).

**M4d — multi-dimensional arrays.** Deferred from M4b during its own
scope narrowing (see that entry above). Resolved the design question
that entry left open (a recursive `Kind::Array(Box<Kind>)`, accepting
the `Copy`-loss ripple M4a deliberately avoided, vs. some other
non-recursive-but-nested representation) in favor of the latter:
`Kind::Array` gained a plain `u8` dimension count alongside its existing
`ArrayElemKind`, capped at `MAX_ARRAY_DIMS = 8` — a multi-dimensional
Java array is representationally just a nested sequence of sequences (a
`SeqLit` of `SeqLit`s), so a flat dimension count is enough; `Kind`
itself never needs to nest. `int[][] grid` (and deeper nesting) is now
supported as a real array *type*; explicitly-typed literal declarations
recurse one dimension at a time in `lower_array_initializer`, including
genuinely ragged rows (`{{1,2,3},{4}}`); and chained index reads
(`grid[i][j]`) reach a new `lower_chained_index` via a generalized
`lower_primary_expression` dispatch, requiring every suffix in the chain
be `[...]`-shaped. **Narrowed further during implementation**: a mixed
index-then-`.length` chain (`grid[i].length`) is *not* supported —
`lower_chained_index`'s own all-bracket requirement means such a chain
still falls through to the pre-existing multi-suffix rejection (the
sub-array's own `.length` remains reachable via an intermediate local) —
split off into its own standalone follow-up task rather than folded into
this milestone (fixing it needs the suffix-chain fold generalized
further, to accept a *trailing* `.length` after any number of leading
`[...]` suffixes, a real design question about how far to generalize
before it starts overlapping with method-call dispatch). A *chained*
indexed-assignment target (`grid[i][j] = v;`) also remains unreachable —
`indexed_assign_target`'s own fixed single-suffix match arm doesn't
recognize a multi-suffix lvalue — deferred alongside compound-assignment/
increment-decrement on an indexed target (the task split off from M4c).
Multi-dimensional `new`-based array creation (`new int[2][3]`, `new
int[][]{{1,2}}`) remains out of scope too — M4c's own two shapes stay
single-dimension only by construction, unaffected by this milestone.

Also still open from the original undifferentiated M4 scope, not yet
assigned to a lettered sub-milestone: `String` method-dispatch surface
(`.length()`, `.charAt()`, `.substring()`, etc. — the built-in method
catalog `sir-method-dispatch.md`/`sir-collection-methods.md` already
define, reused rather than redefined; needs qualified-call support,
itself still out of scope everywhere in this frontend), and `List`/`Map`
collection literals where a fixed-shape lowering is unambiguous
(`List.of(...)`, `new ArrayList<>()` + `.add`).

**Task #54 — wire `Expr::IndirectCall` for invoking a lambda-valued
local.** Not a lettered milestone (a standalone follow-up, picked up
once the M4a–M4d array arc closed out). Closes M3b's own disclosed gap:
a `Closure`-kinded local or parameter can now actually be *called*
(`f(5)`), not just created and passed around. `lower_call_expression`
checks `resolve_name` on the bare callee before falling back to
`method_signatures` — mirrors real Java's own name-resolution priority
(a functional-interface-typed local in scope is invoked directly through
that binding; a same-named top-level method is not reachable through
this call syntax while such a local exists). `Kind::Closure` changed
from a flat unit-like variant to `Kind::Closure(u32)`, an index into a
new `Lowerer::closure_signatures` side table interning each lambda's own
param kinds and return kind — needed so an indirect call can type-check
its arguments and pick the right result kind, kept as a small `Copy`
index rather than embedding the signature inline on `Kind` itself (the
same non-recursive-representation concern M4d's own `Kind::Array`
navigated). This finally makes a real execution-proof test possible for
M3b's own lambdas — the first this crate has ever had. Calling a
lambda-valued *method parameter* remains out of scope: this frontend has
no way to declare one at all (no functional-interface parameter type
exists), so it's a boundary of what's expressible, not a gap in
invocation itself. **Caught by `/security-review` before push (MEDIUM,
CWE-704 stale-type-tracking)**: reassigning a `Closure`-kinded local is
now rejected outright — this crate only tracks a local's `Kind` at
declaration time, and `Kind::Closure(idx)`'s own `idx` is load-bearing
for a later call site's type-checking, so an unrejected reassignment
would leave that index silently stale.

**Task #64 — `break`/`continue` support.** Not a lettered milestone (a
standalone follow-up, picked up once `semantic-ir-to-javascript` became
the first backend to accept `Feature::LoopControl`, task #62). Closes
M2's own disclosed gap: a bare `break_statement`/`continue_statement`
inside a `while`/`do`-`while`/classic-`for`/enhanced-`for` body now
lowers to `Stmt::Break`/`Stmt::Continue`. A new `Lowerer::loop_depth`
counter rejects one outside any loop with a Java-flavored diagnostic
(the shared `semantic-ir` validator's own `loop_stack` independently
enforces the same rule, but this gives a clearer error first), and is
explicitly reset to `0` around a lambda body's or a method body's own
lowering — real Java forbids a `break`/`continue` written directly
inside either from targeting a loop the *declaration* merely happens to
be lexically nested in. A labeled `break foo;`/`continue foo;` is
rejected cleanly (SIR has no loop-label vocabulary at all); `switch`
itself remains its own separately-tracked (task #51) unresolved gap.
**Found while wiring `continue` support, not by inspection beforehand —
two real, `/security-review`-caught non-termination bugs**: both
`lower_do_while_statement` and `lower_for_statement_inner` appended a
synthetic "bookkeeping" statement (a guard-flag clear; the update
clause) to the very end of the lowered loop body — a `continue` anywhere
earlier in that body (SIR's own `Stmt::Continue` jumps straight to
re-evaluating the loop's `cond`) skipped it entirely, making the loop
run forever on the very first `continue` reached (for the classic-`for`
case, the very first iteration, since `i == 0` is even in the
regression test's own source). Both fixed by moving the bookkeeping
*into* the loop's own condition expression instead — the one position a
`continue` can never skip — mirroring the same flag-guard idiom
`do`-`while`'s own desugaring already used, just relocated. A useful
side effect of the classic-`for` fix: `update` no longer shares scope
with the loop body at all (it lives in a separate wrapped-condition
`Expr::Block` now), so the update-target/body-local shadowing collision
this crate previously had to reject (`for (int i = 0; ...; i++) { int i
= 999; ... }`) is now structurally impossible and no longer rejected —
more faithful to real Java scoping than the old "append to body" shape
was. See `java-to-semantic-ir`'s own `CHANGELOG.md` `[0.12.0]` entry for
the full before/after shapes, and its `tests/loop_control_java_
execution.rs` for `node`-execution-proof tests (two of which are direct
termination-regression tests for the bugs above — a reintroduction of
either would hang the affected test, not fail it cleanly).

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
