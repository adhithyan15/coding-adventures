# algol-iir-compiler

Rust frontend for compiling a conservative ALGOL 60 scalar subset into the
shared LANG VM `interpreter_ir::IIRModule`.

This crate intentionally lives on the Rust LANG VM chain:

```text
ALGOL source -> algol-lexer/parser -> algol-iir-compiler -> IIRModule
  -> vm-core / jit-core / aot-core / iir-to-wasm / iir-to-jvm / iir-to-cil
  -> iir-to-beam / iir-to-llvm
```

The first slice supports scalar `integer`, `real`, and `boolean` programs with
assignments, integer arithmetic (`+`, `-`, `*`, `div`, `mod`), **real (f64)
arithmetic** (`+`, `-`, `*`, `/`), comparisons, `if`/`else`, compound
statements, labels, `goto`, `for i := a step k until b do ...`, **typed
procedures with `value` or direct scalar call-by-name parameters**, **switches** (computed goto), and
literal string output:

```algol
begin
  integer result;
  integer procedure sq(x); value x; integer x;
    sq := x * x;
  switch jump := first, second, third;
  integer i;
  i := 3;
  goto jump[i];               comment selects the 3rd label;
  first:  result := 1; goto done;
  second: result := 2; goto done;
  third:  result := sq(7);    comment result = 49;
  done:
end
```

A typed procedure lowers to a sibling `IIRFunction` (here `sq(x: i64) -> i64`)
and a call becomes an IIR `call`, so procedures run on every backend exactly
like any other function. A zero-argument procedure can be invoked explicitly
as `f()` in either value or statement position; a bare statement name retains
the report-style no-argument form. A `switch` is a named jump table: `goto s[i]` selects
the i-th (1-based) target by a portable `index == k ? jmp Lk` chain; an
out-of-range subscript falls through. Conditional designators
(`goto if b then L1 else L2`) are also supported. Procedures may be called
before they are textually declared and may recurse. A procedure body sees its
own value parameters and, since **LANG-FULL E6**, may also **read and write a
scalar declared in an enclosing block**: such a shared scalar is materialised as
a typed module **global** (`global_load`/`global_store`) so the procedure and
the block share one cell — e.g. `integer procedure add(x); … add := counter :=
counter + x` over an enclosing `integer counter` runs across every backend.
A nested procedure may also capture an enclosing scalar `value` parameter: its
outer procedure publishes the incoming typed value to a compiler-owned global
before the nested sibling runs, so nested reads and assignments share that
invocation's value. A same-named nested formal still shadows the capture.

Since **LANG-FULL AL6** a variable may be declared **`own`** (`own integer n`),
giving it *static lifetime*: it is allocated once and retains its value across
every call of the enclosing block/procedure (ALGOL 60 §5.2.5). It reuses the
same module-global storage — keyed by a per-procedure-unique slot so two
procedures' `own n` stay independent — and is **not** re-zeroed on entry, so it
accumulates: `own integer n; n := n + d` called three times with `d = 1` yields
`1`, `2`, `3`. Runs on all 7 backends.

Since **LANG-FULL AL8** the **standard function `abs`** (ALGOL 60 §3.2.4) is
built in: `abs(E)` is the absolute value of `E`, keeping its type
(`integer`→`integer`, `real`→`real`). It is resolved by name — a program may
still redeclare its own `procedure abs`, which then wins — and lowers inline to
`if E < 0 then -E else E` (a compare against zero, then a negated or
pass-through move into one result slot), so it **runs on all 7 backends**;
`abs(0 - 42)` ⇒ `42`. **`sign`** is the second (algol-iir-compiler 0.9.0):
`sign(E)` is `+1`/`-1`/`0` for a positive/negative/zero operand and, unlike
`abs`, always yields an **`integer`** (`sign(-2.5)` is the integer `-1`). It
lowers the same way — `if E > 0 then 1 else if E < 0 then -1 else 0` — and
also runs on every backend; `43 + sign(0 - 1)` ⇒ `42`.

`entier(E)` (ALGOL 60 §3.2.5) is the largest **integer** not greater than the
**real** `E` — floor, rounding toward −∞: `entier(2.7)` ⇒ `2`, `entier(-2.7)` ⇒
`-3` (not `-2`). It lowers to a single E8 `real_to_int_floor` IIR op (the floor
and the real→integer narrowing fused into the primitive), so every backend emits
its native floor-then-convert. The other standard mathematical functions
`sqrt`, `sin`, `cos`, `ln`, `exp`, and `arctan` likewise lower through shared
`f64` IIR operations and run on the seven standard backends.

Since **LANG-FULL AL4** the implementation-defined output procedures `print`
and `output` are recognised in statement position when they are not user
declared procedures. String literal actuals lower to shared E4 `str_const` +
`print_str`, so `begin print('HI') end` writes `HI` on all seven LANG backends.
Literal-backed scalar variables now use the same shape: `string s; s := 'HI'`
materialises `s` with `str_const`, and `print(s)` consumes that direct slot.
Literal-backed scalar copies now reuse E4 `str_concat` with an empty suffix:
`string s, t; s := 'OK'; t := s; print(t)` writes `OK`, and `s := 'NO'`
after the copy does not change the copied `t` slot. Multi-argument output over
literal-backed scalar string variables also stays on the same path:
`string s, t; s := 'O'; t := 'K'; output(s, t)` emits ordered `print_str`
calls and writes `OK`. Literal-backed scalar string predicates now lower through
the shared E4 comparison ops too: `s = 'OK'` / `s != 'NO'` use `str_eq` plus a
typed zero comparison, while `s < 'BETA'` / `'BETA' > s` use `str_cmp` plus the
corresponding typed zero comparison before the normal ALGOL conditional branch.
Initialized scalar locals and `string array` elements now also carry runtime string procedure results:
`string s; s := pick(1); if s < 'LO' then ...; print(s)` uses the shared
runtime `str_concat`/`str_cmp`/`print_str` path. Reads before assignment still
fail closed. `string array A[1:2]` uses the same `array<str>` substrate as
other LANG frontends; its elements can be written from literals, initialized
scalar strings, or runtime string procedure results, then read for lexical comparison or output. Procedures can
also pass a runtime string procedure result directly through another `string`
value formal: `matches(pick(1))` preserves the dynamic `str` across both call
boundaries before the callee compares it. Procedures can
also capture an enclosing scalar `string` through typed module globals, and
those captured strings can be reassigned from dynamic procedure results:
`store(0); store(1); matches(shared)` observes the final runtime handle. An
`own string` initializes to the empty string once and retains later assignments
across calls, including replacements from runtime procedure results before the
latest handle is passed onward through a string formal.

`real` values lower to the IIR `f64` type and run across the established LANG
backends. `2.5 * 2.0`, `7.0 / 2.0`, and real comparisons execute as IEEE-754
doubles. When a real is required, an `integer` operand widens through the
shared `int_to_real` IIR conversion: mixed numeric arithmetic and comparisons,
`/`, real assignments/array elements/formals, and the real standard functions
all accept integer inputs. `div` and `mod` remain integer-only.

Typed procedures retain their result type at the call boundary. In particular,
`boolean procedure neg(p); value p; boolean p; neg := not p` can return directly
into `if neg(false) and not neg(true) then ...`, preserving the shared `bool`
value through negation, conjunction, and the conditional branch on all seven
standard backends. Integer procedure values compose as value actuals too:
`combine(scale(3), scale(4))` preserves two `i64` returns as the arguments to
another typed call. Boolean procedure values likewise compose, so
`both(neg(false), not neg(true))` preserves each typed argument and the final
boolean return across both procedure-call boundaries.
Real procedure values compose through the same typed call path:
`entier(combine(scale(3.0), scale(4.0)))` preserves two `f64` returns as
arguments to `combine`, then converts its `f64` result only at the boundary.

Direct calls may also use **call-by-name** `integer`, `real`, and `boolean`
formals. A name formal is compiled as a call-site-specialised sibling function:
each read re-emits its caller expression in the caller's captured environment,
and an assignment writes through a bare variable or subscripted array-element
actual. This makes Jensen-style loops work without a dynamic closure ABI:
`sum(i, 3, i * i)` re-evaluates `i * i` after each name-bound loop-variable
update. Forwarding a scalar name formal to another direct name call preserves
the original caller expression through compiler-generated lexical aliases, even
when the nested callee reuses the same parameter spelling. Array and string
**formals** remain value-only, and recursive name-formal dispatch is rejected
explicitly.

**Arrays** lower and run on **all seven standard backends** (LANG-FULL E5 /
AL2). `integer array A[1:10]` (and `real`, `boolean`, or `string` arrays) becomes an
`alloc_array` sized at run time from the bounds (`upper - lower + 1`, so dynamic
bounds `A[lo:hi]` work); `A[i]` reads/writes become bounds-checked `array_get`/
`array_set` with the index translated to the IIR's 0-based form `i - lower`.
N-dimensional arrays use the same flat storage with row-major
`(subscript - lower) * stride` indexing. An out-of-range subscript traps at run
time. Procedures can capture an enclosing array: its handle and declared
lower-bound/stride metadata are stored in typed module globals, so a captured
subscript keeps the declaration's index space. `own integer array A[lo:hi]`
has static lifetime too: its bounds and backing storage are initialized on the
first procedure call and persist across later calls.

Array `value` parameters pass the caller's storage handle plus the complete
rank-specific descriptor. The formal's rank is inferred from indexed uses in
its procedure body: a 2-D formal `a[i,j]` receives its two lower bounds and
outer row-major stride beside the `array<T>` handle, and writes remain visible
to the actual array. This works for `integer`, `real`, `boolean`, and `string` elements;
the formal and actual must have matching ranks. A never-indexed formal retains
the compatible 1-D descriptor shape. A nested procedure can capture an outer
array formal too: the outer procedure publishes the handle and complete
descriptor to compiler-owned globals, so the nested sibling function preserves
the caller's declared index space while sharing the same element storage.

Scalar `string` captures use typed module globals, so a procedure can write
and read a string declared by its enclosing block. `own string` initializes to
the empty string on its first procedure call and retains subsequent assignments
across calls. A captured string still requires assignment before its first
read, just like a local string.

Direct real literals also use the shared string output path, preserving their
source spelling and an exact unary `+` or `-`. An arithmetic conditional whose
leaves are all direct real literals
branches to the selected source-spelled string at run time. Exact parentheses
around a direct signed literal are ignored while preserving that spelling.
Finite literal-only addition, subtraction, multiplication, and division are
evaluated by the frontend and printed through the same string path. Zero
divisors, non-finite results, and conditionals with a non-static leaf remain
explicit type errors. A local real scalar assigned one of these finite static
expressions can also print its canonical decimal value while execution remains
straight-line. Copies between tracked real locals preserve independent value
snapshots even if the source is later reassigned, and tracked locals may feed
the same bounded finite arithmetic and exact standard-function evaluator.
Integer literals also enter that real evaluator when their magnitude is within
binary64's exact integer range; larger widenings remain unsupported.
Labels, branches, loops,
gotos, calls, dynamic reassignment, and captured globals invalidate that shortcut.
Direct real-procedure results with zero parameters, value-mode scalar
parameters, array descriptor formals, direct procedure formals,
integer/boolean/string name formals, or a finitely specialised real scalar name formal bound to a
proven runtime-real actual, and local real scalar variables whose latest
straight-line
assignment is such a result or a bare copy of another
provenance-backed local call the portable six-significant-digit
IIR formatter shared with Dartmouth BASIC; its helper
functions use only typed arithmetic, control flow, calls, conversions, and
`putchar`, so the same path runs on every standard backend. A runtime procedure
result may therefore migrate through local real copies and be printed later.
Conditional statements preserve that runtime provenance only for slots proven
on every reachable exit. Conditional value expressions likewise preserve it
when the selector contains no procedure call and every reachable value branch
is a direct runtime real result or a provenance-backed local. A selector may
call a procedure when both value branches are selector-safe runtime-real
expressions rooted in direct formatter-safe real procedure calls. Those calls
may nest through a real name formal only when its actual has the same proof;
unary signs, arithmetic composition with finite static operands, and
real-valued standard functions preserve it, while a finite static real actual
is safe directly. Variable-free exact `sign` and exact-range `entier` results
may also widen through a real name formal. Selector calls paired with
provenance-backed local or other real-name branches remain conservative.
Built-in `sign` also preserves the proof for a selector-safe runtime-real
operand because its result is always `-1`, `0`, or `1`.
The same bounded result preserves runtime-real formatter provenance when it is
widened through an ordinary real assignment or direct real name formal.
`entier(sign(runtime-real))` retains that proof because `entier` receives an
already integral `-1`, `0`, or `1`; unrestricted runtime `entier` remains gated.
Unary `+` and `-` around the built-in `sign` result preserve the same bound.
Built-in integer `abs` may also map that bounded result to exact `0` or `1`
before `entier`; a user-declared `abs` remains conservative.
Variable-free exact additive zero terms may surround that bounded result before
`entier`; nonzero or dynamic additive terms remain conservative.
Variable-free exact multiplicative unit factors may likewise surround it;
non-unit or variable factors remain conservative.
Exact unit division is also permitted when the bounded result is the numerator;
non-unit or variable divisors and a bounded denominator remain conservative.
An exact variable-free exponent chain evaluating to one may preserve the
bounded result before `entier`; other or dynamic exponents remain conservative.
Additional built-in `entier` calls may wrap that already integral bounded
result; an unrestricted runtime `entier` operand remains conservative.
Built-in `sqrt` may also map an `abs`-normalized bounded result to exact `0`
or `1` before `entier`; an unnormalized or unrestricted operand remains gated.
Additional built-in `sqrt` calls may wrap that nonnegative unit result while
preserving the same bound.
Built-in `entier` may also remain inside `sqrt` when its operand is already
nonnegative and bounded; signed, unrestricted, and overridden forms remain gated.
Built-in `sign` may likewise remain inside `sqrt` when its operand is already
nonnegative and bounded; signed and overridden forms remain gated.
Built-in `cos` over a direct built-in `sign` result may also remain inside
`sqrt`: its input is restricted to `-1`, `0`, or `1`, so the cosine is
nonnegative. Non-sign-rooted, exponential, and overridden forms remain gated.
Unary `+` and `-` may wrap that direct `sign` result without changing the
`[-1, 1]` bound; nonzero additive, non-unit multiplicative, and other computed
wrappers remain gated.
Variable-free exact additive zero terms may also surround that sign result;
nonzero terms, repeated sign operands, and overrides remain gated.
Variable-free exact multiplicative unit factors may likewise surround it, with
unit division allowed only when the sign result is the numerator; non-unit,
dynamic, repeated-sign, denominator-sign, and overridden forms remain gated.
An exact variable-free exponent chain evaluating to one may preserve that sign
result as the power base; other exponents, a sign-rooted exponent, and
overrides remain gated.
Built-in `abs` may normalize that sign-rooted result while retaining the same
bound; non-sign-rooted operands and `abs` or `sign` overrides remain gated.
Built-in `sqrt` may similarly normalize a nonnegative unit-bounded sign-rooted
result before cosine; unrestricted operands and `sqrt` or inner-function
overrides remain gated.
Built-in `entier` may also normalize that nonnegative unit range before cosine;
unrestricted operands and `entier` or inner-function overrides remain gated.
Built-in `sin` and `arctan` preserve the direct sign-rooted unit range before
cosine, including nested combinations; unrestricted operands and overrides
remain gated.
Built-in `ln` may map one built-in exponential over a signed unit-bounded
sign-rooted range before cosine; unrestricted operands, nested exponentials,
and overrides remain gated.
Built-in `exp` may map a unary-negated nonnegative unit-bounded sign-rooted
range into `(0, 1]` before cosine; unrestricted, positive, and overridden forms
remain gated.
Built-in `sin`, `cos`, and `arctan` may map a bounded sign-rooted result before
`entier`, including nested combinations; domain-sensitive or unbounded
standard functions and non-sign-rooted runtime operands remain conservative.
A single built-in `exp` may also map such a bounded chain before `entier`;
nested exponential calls remain conservative to keep the finite bound explicit.
Built-in `ln` may map that one exponential when its operand remains rooted in
the bounded sign chain; standalone `ln` and user-declared overrides stay conservative.
Built-in `sqrt` may likewise map that one positive bounded exponential;
nested exponentials and user-declared overrides remain conservative.
Additional built-in `sqrt` calls may wrap that positive bounded result while
preserving the same finite proof.
Built-in `sqrt` may also map `ln(exp(...))` when the exponential operand is
already nonnegative and bounded; signed and nested-exponential forms stay conservative.
One-sided reassignment remains conservative. `for` loops preserve
runtime-real provenance for caller-frame locals whose values their bodies leave
invariant; controlled, changed, captured, and name-promoted storage remains
conservative. Unary plus, unary minus, additive
composition, multiplication, division, and exponentiation whose operands are
proven runtime-real values or finite static numeric expressions preserve the
same provenance. The real-valued standard functions `abs`, `sqrt`, `sin`,
`cos`, `ln`, `exp`, and `arctan` preserve runtime-real provenance for a proven
runtime-real operand while respecting user-declared overrides. Reads from real
array elements and real value formals, including formals promoted into the
existing nested-procedure capture globals, also carry runtime-real provenance
through assignment and composition. A specialised real name formal also
retains the proof when its actual is a runtime-real expression. This includes
assignable real array elements: reads re-evaluate the stored actual while
writes continue through the existing specialised caller-storage path. Direct
forwarding into another name formal preserves that original actual and its
proof. Ordinary real scalars captured through the existing E6 typed-global
path likewise remain concrete f64 values and retain formatter provenance in
nested sibling procedures. A real procedure specialised for a proven
runtime-real scalar name actual also retains formatter provenance on its result;
the existing finite specialisation supplies the caller expression directly, so
no runtime thunk ABI is introduced. Real procedure calls with value or name
array formals likewise retain result provenance because both supported paths
use the existing concrete typed descriptor ABI. Direct procedure formals retain
it as well: finite specialisation substitutes the statically known target, so
no procedure descriptor crosses the IIR ABI. Broader computed scalar formatting
remains outside this bounded proof. Integer, boolean, and string name formals also
retain real-result provenance because their actual expressions are substituted
by that same finite specialisation; unproven real name actuals remain excluded.
Procedure calls with string-literal actuals are kept distinct from literal
expressions, so their typed result is lowered instead of the argument literal.
Direct value and statement calls also preserve runtime-real provenance for
caller-frame scalar slots that remain ordinary locals after call lowering. A
captured or call-by-name actual promoted to shared storage is not restored by
this rule, so the optimization introduces no new aliasing or thunk ABI.
For definite string initialization, a `step`/`until` element may establish an
initialized local when finite static start, step, and limit values prove that
its body executes at least once; zero-trip and dynamic bounds fail closed.
Those static values may come from straight-line tracked numeric locals; their
metadata is consumed before loop lowering disables snapshot propagation.
For a statically bounded multi-iteration `step` loop, pure local scalar
assignments may depend on the control, their own prior snapshots, or earlier
body assignments: the compiler simulates them in source order and retains the
exact integer, finite real, or boolean results. Statically decidable statement
conditions select one recursively supported assignment branch on each pass.
Writes to the control, dynamic selectors or bounds, and integer or real loops
beyond the 4,096-iteration analysis cap remain conservative.
For a `while` element, a bounded static numeric comparison may likewise prove
the initial body execution after abstractly assigning the controlled value;
such known comparisons compose through ALGOL's boolean operators, while bare
boolean literals, unsupported shapes, and dynamic operands remain conservative.
A bounded control evolution may also simulate pure local scalar assignments in
source order after every true predicate and retain their exact integer, finite
real, or boolean results. Statically decidable statement conditions may select
different recursively supported branches on successive passes. Predicate-
dependencies may remain stable. One directly assigned dependency may also
evolve through a supported recurrence whose expression references itself, the
controlled scalar, and ordinary local scalars that are unchanged or evolve
through a graph of supported recurrences. Capped source-order execution also
handles unconditional cross-assigned dependency cycles when every participating
write is a supported local scalar assignment. Such a cycle may also contain
conditional expressions whose selectors use the exact loop control or exact
ordinary local snapshots, including snapshots that evolve through another
supported recurrence in the graph, recursively through their own supported
recurrence, or through an exact mutually recursive selector cycle. The same
selector recurrence may itself contain a conditional expression selected by
an exact snapshot in that graph, including a direct self-reference in one of
the selected leaves. A selector that changes during capped execution may also
choose among conditional leaves when every leaf retains that direct
self-reference; a dynamic leaf without it remains conservative. The same exact
snapshots may select conditional statement branches containing recurrence-cycle
writes. Those changing selector-cycle assignments may themselves appear in statically selected
conditional statement branches while still resolving dependencies across the
whole loop body. A conditional statement selector is also treated as a control
dependency of writes in its branches, so it may close an exact selector cycle
whose selected boolean assignments contain their own exact conditional
expressions. Changing dependency recurrences may contain conditional
expressions when their selectors are the controlled scalar or other exact
local snapshots in that graph. The
recurrence assignment itself may also appear in one or both branches of a
conditional statement selected by those snapshots; a branch without the
assignment leaves the dependency unchanged for that pass. The controlled
scalar itself may likewise be updated alongside other supported local scalar
recurrences. A dependency may be assigned repeatedly in one body pass; bounded
execution applies every supported write in source order. The next `while`
element expression consumes all resulting exact dependency and control
snapshots, and terminating sibling snapshots remain available after the loop.
An acyclic boolean recurrence may use an exact conditional expression to choose
different boolean values on successive passes and then select recurrence-cycle
statements or expressions. Such an exact evolving selector may also choose
between a directly self-recursive boolean leaf and a non-recursive exact leaf;
a finite chain of distinct exact boolean identity copies may sit between that
selector recurrence and the partial self-recursive assignment. These include
bare variables, neutral boolean operations, and even `not` chains. Copy cycles,
changing expressions, and unknown conditional inputs still fail closed. A
conditional identity copy may have a dynamic selector when both branches
preserve the same unique forwarded selector. A boolean projection of the form
`if selector then true else false` is also an exact selector copy, as is its
complemented form `if not selector then false else true`. Either literal branch
may instead be the selector itself, yielding a one-sided guarded projection.
Literal-only boolean combinations may supply projection constants and neutral
identity operands. Literal integer predicates, including checked literal
arithmetic operands, finite binary64 literal predicates, and literal string
predicates may also supply boolean identity operands. Variable-free literal
string predicates may select statement or expression branches in bounded
recurrence bodies, including preserving leaves of conditional dependency or
selector assignments, bounded controlled-scalar recurrences, conditional value
expressions and predicates of bounded `while` elements, finite step-loop header
expressions, and values sequenced across bounded multi-element `for` lists,
including mixed finite-step and single-value elements in either order and
finite-step exits that seed following bounded `while` elements when the body
only reads the controlled variable. That cross-element exit propagation also
applies to exactly simulated finite binary64 steps. A terminating bounded
`while` element likewise retains its exact exit snapshot when that shared body
only reads the control, allowing a following finite-step or single-value
element to derive its initial value from the exit, including real-controlled
elements with exactly simulated binary64 progress. A finite-step element may
also derive its initial value from an earlier finite-step exit under the same
read-only body rule, including for exactly simulated real controls. Bounded
`while` elements may likewise chain exact terminating snapshots when the shared
body only reads the control.
Unknown selectors, unsupported selector or dependency writes, string targets, overflow,
non-finite values, and loops that do not reach false within 4,096 evaluations
fail closed.
A conditional predicate is also evaluated when its selector is statically
known; only the selected branch participates in the proof.
Local string slots carry an empty verifier seed, but this is not a source-level
initial value: reads remain gated by the compiler's definite-initialization set.
Real literal bases also accept the existing
capped nonnegative integer-literal exponent chains or one explicitly signed
integer literal in `-64..=64`. A single real-literal exponent is also accepted
when its value is exactly integral and within that cap; other exponent shapes
still require runtime real formatting. Nonnegative right-associated exponent
chains may mix integer and exactly integral real literals when every computed
exponent remains within the same cap.
Direct calls to the non-overridden standard `abs` function over finite
literal-only real arithmetic also use the static string path. `sqrt` does so
when its nonnegative operand has an exactly round-tripping root; inexact roots,
invalid domains, runtime operands, and user overrides still require runtime
real formatting.
Those supported static standard-function calls may nest and compose with the
same finite literal-only arithmetic evaluator, while every nested call remains
subject to the same override, domain, exact-root, and finiteness checks.
Runtime arithmetic conditionals may select between those validated static
standard-function expressions, branching directly to each precomputed string.
The exact identities `sin(0)=0`, `cos(0)=1`, `ln(1)=0`, `exp(0)=1`, and
`arctan(0)=0` also use this path without host or backend transcendental math.
Exact `sign` results and `entier` results within binary64's exact integer range
may widen into the same static real arithmetic path; larger floors fail closed.

Proper procedures now lower as side-effecting IIR `void` functions when called
in statement position. They can write enclosing scalar or array globals and use
the same string, integer, or boolean output paths as typed procedures. Using a
proper procedure in value position is a clean type error because it has no
return value.

Statically bounded integer- or real-controlled `while` and finite `step`/`until`
elements can retain the
final snapshots of pure local integer, real, or boolean recurrences. The
analysis must prove a terminating false predicate within 4,096 evaluations and
rejects predicate-dependency writes, dynamic compound bodies, globals, arrays, by-name
targets, overflow, and non-finite numeric results.
Boolean snapshot evaluation distinguishes exact bare variables from unary
wrappers, so direct `not` recurrences preserve their negated value rather than
being treated as identity assignments.
The recurrence assignment in either loop form may be wrapped in a compound
body and may additionally have proven integer, real, or boolean identity
assignments of ordinary local scalars or unlabeled dummy statements as inert
siblings. Unlabeled nested compound statements may group those same recurrence,
identity, and dummy statements. A conditional statement may participate when
its selector is statically decidable from the current bounded snapshot and its
selected branch recursively contains only those supported actions. Labels,
declarations, array/global/string siblings, calls, dynamic selectors, and other
effectful statements remain outside this bounded analysis.
Finite `step`/`until` and terminating static `while` loops may instead carry
several pure local integer, real, or boolean recurrence assignments. Bounded
abstract execution evaluates them in source order on every pass, so later
assignments may consume snapshots written earlier in the same body. Arrays,
globals, strings, by-name targets, labels, calls, and unknown expressions still
fail closed.
An exact controlled-scalar assignment in a single-iteration `step`/`until`
loop may use the same wrapper while retaining its checked post-body exit value.
The assignment may also derive that value from the known entry control; unknown
dependencies and assignments whose increment remains in range fail closed.
Exact integer or real control assignments may be followed across multiple
passes until their checked or finite binary64 increment exits, subject to the
same 4,096-pass analysis cap. Real simulation also rejects rounded-away
progress. A statically decidable statement conditional may choose a different
pure controlled-scalar assignment on each pass as the exact control snapshot
evolves. Dynamic selectors and branches that write another changing scalar
remain conservative. A controlled-scalar assignment may share its compound body with
proven integer, real, or boolean identity assignments of ordinary local
scalars or unlabeled dummy statements, including through unlabeled nested
compound grouping; changing siblings and all effectful or dynamic shapes remain
conservative.

Switch-list elements may use every supported designational expression: a
conditional element selects its branch when `goto s[i]` runs, and a nested
element such as `other[j]` performs that second lookup in the same computed-goto
chain. Cyclic switch elements are rejected explicitly because they cannot be
finitely expanded into portable IIR control flow.

Unsupported ALGOL 60 features — arrays outside the supported integer/real/
boolean/string element set, dynamic string variables, call-by-name array or
string formals, and recursive name-formal dispatch — return explicit compiler
errors.
