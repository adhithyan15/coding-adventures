# Changelog

## 0.198.0 — 2026-08-25 — transitive identity dependencies

Bounded static while analysis now recognizes complete supported integer and
boolean identity writes while checking a transitive value or predicate
dependency. Changing writes remain conservative, and the check does not add
recursive effect inference.

## 0.197.0 — 2026-08-25 — conditional identity selector writes

The terminal bounded selector-dependency proof now recognizes conditional
assignments when every reachable leaf is a complete supported identity of the
same scalar. A changing leaf still fails closed and does not extend the
ten-level selector proof budget.

## 0.196.0 — 2026-08-25 — grouped selector identities

Bounded static while analysis now follows complete boolean, integer, and real
identity expressions through parentheses, including nested grouping. Grouped
changing expressions remain conservative.

## 0.195.0 — 2026-08-25 — signed neutral selector literals

Bounded static while analysis now recognizes exact unary-plus neutral literals
and signed integer zero inside selector identity expressions. Negative one and
other signed changing expressions remain conservative.

## 0.194.0 — 2026-08-25 — parenthesized selector identities

Bounded static while analysis now treats parentheses as transparent around
exact scalar self-references and boolean, integer, or real neutral literals.
Parenthesized changing expressions remain conservative.

## 0.193.0 — 2026-08-25 — unary-plus selector writes

Bounded static while analysis now recognizes leading unary plus as the exact
no-op already implemented by expression lowering. Integer identity tails remain
stable, while unary minus continues to fail closed for both integer and real
selectors.

## 0.192.0 — 2026-08-25 — power-one selector writes

Bounded static while analysis now recognizes integer and real exponentiation
whose complete literal exponent chain evaluates to one. This matches the
existing zero-multiply lowering exactly; dynamic, real-valued, zero, and
otherwise non-unit exponents continue to fail closed.

## 0.191.0 — 2026-08-25 — boolean identity selector chains

Bounded static while analysis now recognizes left-associative boolean identity
chains composed from `and true`, `or false`, or `eqv true`. Implication retains
its directional single-operation rule, unsupported terms fail closed, and the
ten-level selector-dependency cap is unchanged.

## 0.190.0 — 2026-08-25 — real unit selector chains

Bounded static while analysis now recognizes left-associative finite-real
identity chains composed from multiplication and division by exact positive
one. Division remains directional, non-unit terms fail closed, and the
ten-level selector-dependency cap is unchanged.

## 0.189.0 — 2026-08-25 — integer identity selector chains

Bounded static while analysis now recognizes left-associative integer identity
chains composed from additive zero or multiplicative one. Mixed-precedence
chains remain structural, unsupported terms fail closed, and the ten-level
selector-dependency cap is unchanged.

## 0.188.0 — 2026-08-25 — even boolean negation selector writes

Bounded static while analysis now recognizes even chains of unary `not` as
exact boolean identity writes. Odd chains remain changing, binary identities
under an even negation chain retain their existing structural proof, and the
ten-level selector-dependency cap is unchanged.

## 0.187.0 — 2026-08-25 — real unit selector writes

Bounded static while analysis now recognizes real multiplication and division
by exact positive one as structural identity writes. These operations preserve
finite binary64 selector values without widening the ten-level recursion cap;
real additive zero remains conservative because it can change negative zero's
sign bit, and other computed writes still fail closed.

## 0.186.0 — 2026-08-25 — integer identity selector writes

Bounded static while analysis now recognizes exact structural integer identity
writes using `+ 0`, `- 0`, `* 1`, and `div 1`, including valid commutative
forms. These assignments preserve transitive selector dependencies without
widening the ten-level recursion cap; non-identities, real arithmetic, and
unsupported computed writes still fail closed.

## 0.185.0 — 2026-08-24 — boolean identity selector writes

Bounded static while analysis now recognizes `x and true`, `x or false`,
`x eqv true`, and `true impl x` (including valid symmetric forms) as exact
boolean identity writes. These computed assignments preserve transitive
selector dependencies without widening the ten-level recursion cap; boolean
non-identities and unsupported computed writes still fail closed.

## 0.184.0 — 2026-08-20 — ten-level selector dependencies

Bounded static while analysis may now follow ten nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.183.0 — 2026-08-14 — nine-level selector dependencies

Bounded static while analysis may now follow nine nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.182.0 — 2026-08-14 — eight-level selector dependencies

Bounded static while analysis may now follow eight nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.181.0 — 2026-08-14 — seven-level selector dependencies

Bounded static while analysis may now follow seven nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.180.0 — 2026-08-14 — six-level selector dependencies

Bounded static while analysis may now follow six nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.179.0 — 2026-08-14 — five-level selector dependencies

Bounded static while analysis may now follow five nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.178.0 — 2026-08-14 — four-level selector dependencies

Bounded static while analysis may now follow four nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.177.0 — 2026-08-14 — three-level selector dependencies

Bounded static while analysis may now follow three nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.

## 0.176.0 — 2026-08-14 — two-level selector dependencies

Bounded static while analysis may now follow two nested conditional-selector
dependencies when each known predicate is stable and selects the preserving
leaf. The fixed depth still makes cycles and longer chains fail closed without
general recursive effect inference.

## 0.175.0 — 2026-08-14 — stable nested selector dependencies

A statically known predicate over unchanged ordinary local scalars may now
select the preserving leaf of a conditional assignment to a known predicate
dependency that itself selects a preserving conditional selector assignment.
The proof is capped at one nested dependency level; cycles, deeper chains,
written selectors, and unsupported effects remain conservative.

## 0.174.0 — 2026-08-13 — conditional idempotent selector dependencies

A conditional assignment whose leaves are all the same bare scalar may now
leave a known predicate dependency stable while that predicate selects the
preserving leaf of an assignment to a known transitive selector. Differing,
computed, controlled, and unsupported effects remain conservative without
recursive effect inference.

## 0.173.0 — 2026-08-13 — idempotent conditional selector dependencies

An exact bare self-assignment may now leave a known ordinary local predicate
dependency stable while that predicate selects the preserving leaf of a
conditional assignment to a known transitive selector. Computed assignments,
loop controls, and unsupported effects remain conservative without recursive
effect inference.

## 0.172.0 — 2026-08-13 — stable conditional selector assignments

A statically known predicate over unchanged ordinary local boolean, integer,
or real scalars may now select the preserving leaf of a conditional assignment
to a known transitive selector. Written, controlled, nonlocal, array, by-name,
and unsupported predicate dependencies remain conservative without recursive
effect inference.

## 0.171.0 — 2026-08-13 — static conditional selector assignments

A variable-free statically known conditional selector assignment now scans
only its selected leaf when deciding whether a known transitive selector stays
stable. Dynamic predicates, selected changing leaves, and unsupported effects
remain conservative without recursive effect inference.

## 0.170.0 — 2026-08-13 — conditional transitive selector idempotence

A conditional selector assignment whose leaves are all the same bare selector
now leaves that known transitive selector stable even when the conditional
predicate is dynamic. Differing, computed, controlled, and unsupported selector
effects remain conservative without recursive effect inference.

## 0.169.0 — 2026-08-13 — idempotent transitive assignment selectors

An exact bare self-assignment may now leave a known ordinary local selector
stable while it chooses the preserving leaf of a transitive assignment
dependency. Computed assignments, loop controls, and unsupported selector
effects remain conservative without recursive effect inference.

## 0.168.0 — 2026-08-13 — stable transitive assignment selectors

An unchanged ordinary local boolean, integer, or real selector with a known
value may now choose the preserving leaf of a transitive assignment dependency.
Written, controlled, nonlocal, array, by-name, string, and otherwise unsupported
selectors remain conservative without recursive effect inference.

## 0.167.0 — 2026-08-13 — static transitive assignment selectors

A variable-free statically known conditional selector may now choose the
preserving leaf of a transitive assignment dependency. Dynamic selectors with
differing leaves and unsupported effects remain conservative.

## 0.166.0 — 2026-08-13 — conditional transitive idempotence

Transitive assignment dependencies now remain stable when every leaf of a
conditional expression is the same bare scalar, even with a dynamic selector.
Differing leaves and computed or cyclic dependency effects still fail closed.

## 0.165.0 — 2026-08-13 — transitive idempotent assignment dependencies

Checked static idempotent assignments may now depend on an ordinary local
scalar that is itself only exactly self-assigned in the loop body. Computed or
cross-assigned dependency writes, loop controls, and unsupported bindings still
fail closed without recursive effect inference.

## 0.164.0 — 2026-08-13 — stable idempotent assignment dependencies

Checked static idempotent assignments may now reference known ordinary local
numeric or boolean scalars that are never assignment or loop-control targets
in the body. Written, controlled, global, array, by-name, string, unknown, and
non-matching dependencies remain conservative.

## 0.163.0 — 2026-08-13 — checked static idempotent assignments

Capped `while` body-effect analysis now recognizes a self-only static numeric
or boolean expression as idempotent when its checked value exactly matches the
tracked scalar. Expressions with any other dependency, unknown values,
non-finite reals, strings, arrays, globals, and by-name targets fail closed.

## 0.162.0 — 2026-08-13 — stable conditional assignment selectors

Capped `while` body-effect analysis may now select a conditional assignment
leaf using a statically known predicate over local boolean, integer, and real
scalars that the body does not change. Written, controlled, global, array,
by-name, string, and otherwise unknown selector dependencies fail closed.

## 0.161.0 — 2026-08-13 — static conditional assignment effects

Capped `while` body-effect analysis now examines only the selected leaf of a
variable-free statically known conditional assignment. A selected exact scalar
self-assignment preserves the dependency; a selected changing leaf and dynamic
selectors with differing leaves remain conservative.

## 0.160.0 — 2026-08-13 — conditional self-assignment dependencies

Capped `while` body-effect analysis now recognizes a conditional assignment as
idempotent when both leaves are the same bare scalar. Differing or computed
leaves remain conservative.

## 0.159.0 — 2026-08-13 — idempotent while-predicate dependencies

Capped `while` body-effect analysis now treats an exact scalar self-assignment
as preserving a static body-predicate dependency. Computed assignments and
nested controlled variables remain conservative.

## 0.158.0 — 2026-08-13 — stable standard-function while predicates

Capped `while` body-effect analysis now permits supported deterministic
standard functions to wrap stable scalar predicate dependencies. User-shadowed
and unsupported calls remain conservative.

## 0.157.0 — 2026-08-13 — stable scalar while predicates

Capped `while` dependency analysis now selects a body branch controlled by a
statically known predicate over local boolean, integer, and real scalars when
an all-path scan proves the body never writes any dependency. Written, global,
array, by-name, controlled, string, and otherwise unknown dependencies remain
conservative.

## 0.156.0 — 2026-08-13 — composed stable conditional while effects

Capped `while` dependency analysis now selects a body branch controlled by a
statically known boolean composition when every referenced selector is a local
bare boolean that the whole body never writes. Mixed, written, nonlocal, array,
by-name, and otherwise unknown selector sets remain conservative.

## 0.155.0 — 2026-08-13 — stable conditional while effects

Capped `while` dependency analysis now selects a body branch controlled by a
statically known bare boolean local when an all-path scan proves the body never
writes that selector. Dynamic, written, nonlocal, and compound selectors remain
conservative.

## 0.154.0 — 2026-08-13 — static conditional while effects

Capped `while` dependency analysis now scans only the selected branch of a
variable-free statically decidable body conditional. Dynamic and
variable-dependent conditions continue to scan both branches conservatively.

## 0.153.0 — 2026-08-13 — idempotent while dependencies

Capped `while` control analysis now permits an exact scalar self-assignment to
a stable local dependency in the loop body. Computed assignments, array targets,
nested controls, globals, and by-name values remain conservative.

## 0.152.0 — 2026-08-13 — real while-control snapshots

Capped `while` control analysis now has an explicit real-valued conformance
proof. Finite binary64 updates retain the first control value whose predicate
is false, while rounded-away progress remains conservative.

## 0.151.0 — 2026-08-13 — standard-function while controls

Capped `while` control analysis now recognizes dependency names beneath exact
static standard-function calls without misclassifying the callee as a scalar.
User-declared procedures with the same spelling continue to shadow the built-in
and remain conservative.

## 0.150.0 — 2026-08-13 — read-only while dependencies

Stable local dependencies used by capped `while` analysis may now be read by
the loop body. An AST write scan still rejects assignment targets and nested
controlled variables; calls and existing control-flow barriers continue to
invalidate static tracking.

## 0.149.0 — 2026-08-13 — stable local while dependencies

Capped abstract execution of a `while` control may now use statically known
ordinary local scalar dependencies when the loop body does not reference them.
Globals, arrays, by-name values, unknown snapshots, and body-referenced
dependencies remain conservative.

## 0.148.0 — 2026-08-13 — static while-control exits

A self-contained `while` for-list element whose value and predicate reference
only its controlled scalar now retains the first control value whose predicate
is false. Abstract execution is capped at 4,096 iterations; external
dependencies, unknown expressions, and nonterminating or longer loops remain
conservative.

## 0.147.0 — 2026-08-13 — static while-body snapshots

A `while` for-list element whose initial predicate is statically true now
retains path-independent integer, real, and boolean constants established by a
body that does not reference the controlled scalar. The final controlled value,
control-dependent bodies, and unknown initial predicates remain conservative.

## 0.146.0 — 2026-08-13 — finite loop-body snapshots

Finite static `step`/`until` loops now retain path-independent integer, real,
and boolean constants established by a body that does not reference the
controlled scalar. Control-dependent bodies and every existing dynamic or
tracking barrier remain conservative.

## 0.145.0 — 2026-08-13 — finite real step-loop control exits

Finite static real `step`/`until` elements whose bodies avoid the controlled
scalar now retain its first post-limit value by simulating the emitted binary64
additions. Simulation is capped at 4,096 iterations and rejects non-finite or
non-progressing additions; larger, dynamic, and target-referencing loops remain
conservative.

## 0.144.0 — 2026-08-13 — finite step-loop control exits

Finite static integer `step`/`until` elements whose bodies avoid the controlled
scalar now retain its first post-limit value using checked widened arithmetic.
Exactly-one elements with a direct static controlled assignment retain the
post-increment exit value rather than the pre-increment body value. Floating
multi-iteration progress, overflow, zero steps, arrays, globals, by-name targets,
dynamic writes, and target-referencing multi-iteration bodies remain conservative.

## 0.143.0 — 2026-08-13 — single-step controlled snapshots

Exactly-one-iteration `step`/`until` elements now retain a finite static value
assigned directly to their controlled scalar by the body. Dynamic assignments,
compound bodies, and potentially repeating loops remain conservative.

## 0.142.0 — 2026-08-13 — zero-trip while-loop snapshots

Statically false `while` elements now preserve entry scalar snapshots and
record the initial value assigned to their controlled scalar. Unknown and
potentially repeating `while` elements remain conservative.

## 0.141.0 — 2026-08-13 — single-iteration step-loop snapshots

Statically proven single-iteration `step`/`until` elements now retain scalar
constants established by a body that does not reference the controlled
variable. Multi-iteration loops, calls, and other barriers remain conservative.

## 0.140.0 — 2026-08-13 — zero-trip controlled-variable snapshots

A statically empty `step`/`until` element now records the initial value assigned
to its controlled scalar after restoring unrelated entry snapshots. This keeps
formatter-free output aligned with the runtime loop preheader assignment.

## 0.139.0 — 2026-08-13 — zero-trip loop snapshot preservation

Statically proven empty `step`/`until` elements now preserve their entry
integer, real, and boolean snapshots. Unknown, `while`, and potentially
repeating elements remain conservative.

## 0.138.0 — 2026-08-13 — static conditional real output selection

Formatter-free real output now evaluates only the selected branch when a
conditional expression's selector is statically known. Dynamic selectors still
require both branches to be statically printable.

## 0.137.0 — 2026-08-13 — static conditional expression selection

Numeric conditional assignments with a statically known selector now retain
the reachable branch's integer or real snapshot. Unknown selectors continue to
require equal branch values before preserving metadata.

## 0.136.0 — 2026-08-13 — static statement branch selection

Conditional statements with a statically known condition now carry only the
reachable branch's initialization and scalar snapshot state through the join.
Unknown conditions continue to intersect both exits conservatively.

## 0.135.0 — 2026-08-13 — static boolean snapshots

Local boolean scalars now retain path-independent compile-time values across
straight-line assignments, copies, and equal statement branches. Calls,
labels, jumps, loops, globals, and dynamic assignments still invalidate them.

## 0.134.0 — 2026-08-13 — path-independent conditional predicates

An initial `while` conditional with an unknown selector now has a static result
when both branches independently prove the same boolean. Differing or unknown
branches remain conservative.

## 0.133.0 — 2026-08-12 — right-dominating while predicates

The initial `while` proof now recognizes the symmetric truth-dominating forms
`x and false`, `x or true`, and `x impl true` when `x` is unknown. The
non-dominating right-literal forms remain conservative.

## 0.132.0 — 2026-08-12 — truth-dominating while predicates

The initial `while` proof now applies the sound truth-dominating identities
`false and x`, `true or x`, and `false impl x` without requiring `x` to be
statically known. Non-dominating forms and `eqv` remain conservative.

## 0.131.0 — 2026-08-12 — literal while predicates

The initial `while` proof now recognizes direct boolean literals. A `true`
predicate establishes the first body execution, while `false` remains a
zero-trip path that cannot establish definite string initialization.

## 0.130.0 — 2026-08-12 — static conditional while predicates

The initial `while` proof now evaluates a conditional boolean predicate when
its selector is statically known, inspecting only the selected branch. Dynamic
selectors and unknown selected predicates remain fail-closed.

## 0.129.0 — 2026-08-12 — composed static while conditions

The initial `while` condition proof now composes already-known finite numeric
comparisons through `not`, `and`, `or`, `impl`, and `eqv`. Every required leaf
must remain statically known; bare boolean literals and dynamic operands still
fail closed.

## 0.128.0 — 2026-08-12 — static initial while conditions

Definite string initialization now flows out of a `while` for-list element
when a bounded static numeric comparison proves its initial condition true
after abstractly assigning the controlled variable's first value. The abstract
maps are restored before normal lowering; false, dynamic, call-bearing, array,
by-name, and captured-global cases remain fail-closed.

## 0.127.0 — 2026-08-12 — tracked step-loop bounds

The statically nonempty `step`/`until` proof now consumes straight-line integer
or real snapshots from the loop header before dynamic-loop lowering invalidates
numeric metadata. Unknown and statically zero-trip bounds remain fail-closed,
and snapshots are still disabled before the loop body is analyzed.

## 0.126.0 — 2026-08-12 — statically nonempty step-loop initialization

Definite string initialization now flows out of `step`/`until` elements when
finite static start, step, and limit values prove the loop body executes at
least once. Descending loops use the negative-step bound rule; zero-trip and
dynamic cases continue to restore the entry initialization set.
Local string slots receive an empty runtime seed so typed backends can verify
the syntactic zero-trip predecessor; the compiler's separate initialization
set still rejects every source-level read before assignment.

## 0.125.0 — 2026-08-11 — controlled-variable snapshots

Plain single-value `for` elements now update the local controlled variable's
integer or real snapshot alongside the emitted assignment. Body expressions
therefore observe each element's current static value instead of stale entry
metadata.

## 0.124.0 — 2026-08-11 — single-value loop snapshots

`for` lists containing only plain single-value elements now retain integer and
real snapshots through their straight-line body repetitions. Lists containing
`while` or `step`/`until` elements continue to disable snapshot tracking.

## 0.123.0 — 2026-08-11 — single-value loop initialization

Plain single-value `for` list elements now retain definite scalar-string
initialization from their body because each executes exactly once. `while` and
`step`/`until` elements remain conservative, and mixed lists join each element
in sequence.

## 0.122.0 — 2026-08-11 — loop string-initialization joins

`for` statements now restore the definite scalar-string initialization state
from loop entry at exit. This rejects body-only initialization because a
`while` or `step` element may execute zero times, while preserving strings
initialized before the loop.

## 0.121.0 — 2026-08-11 — conditional string initialization joins

Definite initialization for scalar strings now intersects the initialized-slot
sets from both statement-conditional exits. A then-only assignment no longer
makes a later read unsoundly valid, while assignments on both branches and an
already initialized path without `else` remain supported.

## 0.120.0 — 2026-08-11 — conditional-statement snapshot merging

Statement conditionals now compile each branch from the same post-condition
metadata state and retain only identical integer and real snapshots at the
join. Missing `else` branches merge against the unmodified path, while calls
and every existing invalidation boundary continue to poison the result.

## 0.119.0 — 2026-08-11 — path-independent conditional snapshots

Integer and real assignments through runtime conditional expressions now keep
static metadata when both independently proven branches have exactly the same
value. The condition still lowers normally, while differing, dynamic, or
non-finite branches fail closed.

## 0.118.0 — 2026-08-11 — static integer-function snapshots

Integer snapshot tracking now evaluates non-overridden `abs`, `sign`, and
`entier` calls over bounded static operands. Absolute-value overflow and
non-finite, inexact-range, dynamic, or lexically overridden calls fail closed.

## 0.117.0 — 2026-08-11 — static integer power snapshots

Integer snapshot tracking now evaluates nonnegative literal exponent chains
within the existing exponent cap using checked `i64` powers. Overflow,
negative or dynamic exponents, and oversized chains invalidate metadata.

## 0.116.0 — 2026-08-11 — static integer arithmetic snapshots

Straight-line integer snapshot tracking now evaluates checked `+`, `-`, `*`,
`div`, and `mod` expressions. Overflow, zero division, unsupported operators,
and dynamic operands invalidate the metadata rather than introducing host
arithmetic or an unsound runtime formatting shortcut.

## 0.115.0 — 2026-08-11 — static integer scalar widening

Straight-line local integer literals and copies now retain exact `i64`
snapshots for bounded static real expressions. Widening remains restricted to
binary64's exact integer range, and the metadata is cleared at the same
control-flow, call, capture, and dynamic-assignment boundaries as static reals.

## 0.114.0 — 2026-08-11 — static mixed numeric output

Integer literals within binary64's exact integer range may now widen into the
bounded static real evaluator, including assignments to tracked real locals and
mixed arithmetic output. Larger integer literals fail closed instead of losing
precision during compile-time conversion.

## 0.113.0 — 2026-08-11 — static real scalar expressions

Tracked straight-line real locals may now participate in the same bounded,
finite arithmetic and exact standard-function evaluator used by direct static
output. Runtime or invalidated operands continue to fail closed rather than
materializing a partial formatter.

## 0.112.0 — 2026-08-11 — static real scalar copy output

Straight-line copies between tracked local real scalars now preserve the
source's canonical static value as an independent snapshot. Reassigning the
source updates only its own tracked value, while all existing control-flow,
call, capture, dynamic-value, and finiteness guards remain unchanged.

## 0.111.0 — 2026-08-11 — straight-line static real scalar output

Local real scalars assigned a finite compile-time expression may now print its
canonical decimal value through the existing portable string path. Tracking is
restricted to straight-line local state and is cleared by labels, branches,
loops, gotos, procedure calls, dynamic reassignment, and captured globals, so
values that can vary at run time still fail closed pending a typed formatter.

## 0.110.0 — 2026-08-11 — integer-function static real composition

Exact `sign` and safely bounded `entier` results may now widen into the
formatter-free literal-only real arithmetic evaluator. `entier` is restricted
to binary64's exact integer range; larger results continue to fail closed
instead of silently losing integer precision.

## 0.109.0 — 2026-08-11 — canonical transcendental output

The formatter-free static real path now recognizes the exact identities
`sin(0)=0`, `cos(0)=1`, `ln(1)=0`, `exp(0)=1`, and `arctan(0)=0` without
invoking host or backend math libraries. Other transcendental inputs continue
to require runtime real formatting and fail closed.

## 0.108.0 — 2026-08-11 — conditional static standard-function output

Runtime conditionals may now select between formatter-free real branches that
contain composed `abs` and exact-`sqrt` calls. Both branches are validated with
lexical override, domain, exact-root, and finiteness checks before the compiler
emits the existing portable string-output control flow.

## 0.107.0 — 2026-08-11 — composed static real standard-function output

Formatter-free `abs` and exact-`sqrt` values may now nest and participate in
finite literal-only real arithmetic. Evaluation remains aware of lexical
standard-function overrides, and runtime, inexact, invalid, or non-finite
subexpressions continue to fail closed.

## 0.106.0 — 2026-08-11 — static real standard-function output

Direct non-overridden `abs` calls over finite literal-only real arithmetic now
use the formatter-free static output path. `sqrt` joins that path when its
finite nonnegative operand has an exactly round-tripping root; inexact roots,
invalid domains, runtime operands, and user overrides continue to fail closed.

## 0.105.0 — 2026-08-11 — static integral exponent-chain output

The bounded real output path now evaluates right-associated exponent chains
whose integer- or real-literal members are all nonnegative and integral. Every
computed exponent must remain within the existing cap of 64; fractional,
negative-chain, oversized, and runtime shapes continue to fail closed.

## 0.104.0 — 2026-08-11 — static integral-real exponent output

Finite real literal exponents whose values are exactly integral and within
`-64.0..=64.0` now share the deterministic repeated-arithmetic output path.
Fractional, oversized, non-finite, chained-real, and runtime exponents continue
to require runtime formatting and therefore fail closed.

## 0.103.0 — 2026-08-11 — static real signed-power output

The bounded real output evaluator now accepts one explicitly signed integer
literal exponent in `-64..=64`. Negative exponents use repeated division, while
zero bases, non-finite intermediates, signed exponent chains, and runtime or
real exponents continue to fail closed without a runtime formatter.

## 0.102.0 — 2026-08-11 — static real integer-power output

Finite literal-only real bases raised to small nonnegative integer-literal
exponent chains now print through the bounded compile-time string path. The
existing exponent cap and repeated-multiplication semantics avoid host `pow`
variation; real, negative, oversized, runtime, and non-finite powers fail closed.

## 0.101.0 — 2026-08-11 — static real division output

Finite literal-only real division now completes the bounded arithmetic output
evaluator. Quotients use the existing shared string path, while zero divisors,
non-finite results, and runtime operands fail closed without introducing a
runtime `f64` formatter ABI.

## 0.100.0 — 2026-08-11 — static real multiplication output

Finite literal-only real multiplication now joins addition and subtraction in
the bounded compile-time output evaluator. Products, including parenthesized
and conditional leaves, print through the shared string path; division,
runtime operands, and non-finite results continue to fail closed.

## 0.99.0 — 2026-08-11 — static additive real output

Literal-only real addition and subtraction now evaluate at compile time and
print their finite result through the shared string path. Direct literals still
preserve source spelling, while multiplication, division, non-finite results,
and any runtime operand remain outside this bounded formatter-free step.

## 0.98.0 — 2026-08-11 — parenthesized real-literal output

The source-spelled real output path now unwraps exact balanced parentheses
around direct literals, including signed forms. Parenthesized arithmetic still
fails closed, so grouping cannot bypass the runtime f64 formatter boundary.

## 0.97.0 — 2026-08-11 — conditional real-literal output

Implementation-defined output now accepts an arithmetic conditional when every
leaf is a direct real literal. The condition branches directly between each
leaf's source-spelled string output, so runtime selection works without a
partial f64 formatter ABI; any computed or runtime real leaf still fails closed.

## 0.96.0 — 2026-08-11 — signed real-literal output

The real-literal output fast path now accepts an exact unary `+` or `-` and
preserves that sign in stdout. Binary arithmetic and runtime real values still
bypass the literal recognizer and fail closed pending a runtime formatter ABI.

## 0.95.0 — 2026-08-11 — real-literal standard output

Implementation-defined `print` and `output` calls now accept a direct real
literal and print its deterministic source spelling through the shared string
path. Runtime `f64` values remain explicit type errors until a portable typed
formatter ABI is available.

## 0.94.0 — 2026-08-11 — boolean standard output

Implementation-defined `print` and `output` procedure calls now render boolean
variables and expressions as `true` or `false`. Typed control flow selects the
corresponding shared string literal, avoiding a boolean-to-integer ABI coercion.

## 0.93.0 — 2026-08-11 — integer standard output

Implementation-defined `print` and `output` procedure calls now accept integer
variables and expressions, lowering them to the shared `print_i64` builtin.
String output retains its existing `print_str` paths; real and boolean output
remain explicit type errors.

## 0.92.0 — 2026-08-11 — fail-closed array extents

Array declarations now require every run-time dimension extent to remain
positive before computing strides or allocating storage. Reversed bounds and
`upper - lower + 1` overflow take a portable bounds-trap path. Row-major stride
and total-length products are divided back and checked, preventing signed `i64`
overflow from becoming an undersized allocation.

## 0.91.0 — 2026-08-11 — multidimensional coordinate bounds

Each array subscript coordinate is now bounds-checked before row-major
flattening. Dimension sizes are recovered from the existing flat array length
and adjacent descriptor strides, so invalid coordinates cannot alias a valid
neighboring row and the procedure ABI does not grow. Invalid reads and writes
reuse the shared backend `array_get`/`array_set` trap path.

## 0.90.0 — 2026-08-11 — array value copies

Array `value` actuals now receive a fresh typed element copy while retaining
the existing handle, lower-bound, and stride call ABI. Callee writes no longer
mutate caller storage; name-mode arrays continue to share their caller's
descriptor and storage. The copy loop uses the shared bounds-checked E5 array
operations and works for every supported element type and rank.

## 0.89.0 — 2026-08-11 — split formal specifications

Complementary report-style specification parts now merge before procedure
lowering: `integer a; array a;` forms an integer-array formal, and `integer p;
procedure p;` forms a typed procedure formal. Duplicate type/kind parts and
conflicting types or kinds still fail closed. The resulting array descriptors
and direct formal-procedure specialization retain their existing typed ABIs.

## 0.88.0 — 2026-08-11 — procedure heading validation

Procedure headings now fail closed before IIR lowering when the formal list,
`value` part, or specification parts are inconsistent. Formal names and value
names must be unique, every value/specification name must belong to the formal
list, and each formal must receive exactly one specification. Valid headings
with multiple specification groups retain the existing typed direct-call ABI.

## 0.87.0 — 2026-08-11 — typed formal procedures

Report-style typed procedure formals such as `integer procedure p` now retain
their expected result type during direct call-site specialization. Declared and
forwarded procedure actuals plus supported standard functions are checked
before lowering; proper procedures and mismatched result types fail closed.
The static direct-call IIR ABI is unchanged.

## 0.86.0 — 2026-08-11 — canonical ALGOL grammar resynchronization

The compiler now consumes the canonical generated parser shapes for scalar
`own` declarations and recursively nested conditional expression and
designational branches. This removes the parser artifact's historical drift
without introducing a dynamic closure or thunk ABI.

## 0.85.0 — 2026-08-11 — nested procedure visibility

Nested procedure signatures and declaration ASTs now leave compiler lookup
tables with their lexical block. Emitted sibling names remain reserved so two
disjoint blocks cannot collide. This restores undeclared and standard-function
lookup after scope exit while preserving calls already emitted inside the block.

## 0.84.0 — 2026-08-11 — block-scoped procedure shadowing

Procedure declarations now receive stable sibling-function identities while a
nearest-binding map follows ALGOL block scope. A nested declaration may shadow
an outer procedure, calls inside the block resolve to the nested sibling, and
leaving the block restores the outer binding. Same-block duplicates remain an
error, and calls retain the existing fully typed direct IIR ABI.

## 0.83.0 — 2026-08-11 — real for controlled variables

ALGOL for-clause controlled variables may now be `real` scalars or real array
elements. Integer initial values, steps, and limits widen to `f64`; loop sign,
bound, and increment operations retain the controlled variable's numeric width.

## 0.82.0 — 2026-08-11 — subscripted for controlled variables

Integer array elements may now serve as ALGOL for-clause controlled variables.
Their designators are re-evaluated for each assignment and read through the
existing bounds-checked typed `array_set`/`array_get` lowering.

## 0.81.0 — 2026-08-11 — dummy statements

ALGOL dummy statements now lower as explicit no-ops. Boundary lookahead in the
parser permits empty blocks, branches, loop bodies, and labeled statements while
preventing arbitrary invalid syntax from disappearing during compilation.

## 0.80.0 — 2026-08-11 — multiple labels per statement

The compiler now pre-registers and emits every label prefix attached to one
statement. Each stable IIR label points at the same instruction position, so
forward and backward gotos may use any of the source labels while existing
block shadowing and duplicate checks remain intact.

## 0.79.0 — 2026-08-11 — declaration-scoped nested switches

Switch declarations now receive stable lexical identities in a block pre-pass.
Stored switch-list references bind to the declaration-time identity, so a later
nested declaration with the same spelling cannot retarget an outer switch graph.
The pre-pass also preserves forward switch references within one block.

## 0.78.0 — 2026-08-11 — block-scoped label shadowing

Labels now receive stable internal names within their lexical block. Nested
blocks may shadow an outer label, nearest-binding lookup applies to forward and
backward gotos, and leaving the block restores the outer binding. Switch-list
label designators retain the declaration block's binding when expanded later.
Duplicate labels in one block remain an error.

## 0.77.0 — 2026-08-11 — block-scoped switch shadowing

Switch declarations now follow lexical block scope. A nested block may shadow
an outer switch, nearest-binding lookup applies while the block is active, and
the outer binding is restored on exit. Duplicate switches in one block remain
an error, and existing deferred designator and cycle checks are unchanged.

## 0.76.0 — 2026-08-09 — mutual recursive direct procedure formals

Regression coverage now proves that two mutually recursive direct `procedure`
formals reuse their active specialised sibling pair. The direct target survives
the finite recursion graph without a function-pointer, closure, or descriptor
value in the IIR ABI.

## 0.75.0 — 2026-08-09 — recursive direct procedure formals

Regression coverage now proves that a recursive direct `procedure` formal
reuses its active specialised sibling at each recursive call. Its static target
survives to the base case without a function-pointer, closure, or descriptor
value in the IIR ABI.

## 0.74.0 — 2026-08-09 — nested capturing direct procedure actuals

Regression coverage now proves that a direct nested procedure may capture an
enclosing value formal and still travel through a `procedure` formal. The
specialised wrapper calls the nested sibling directly while the existing
capture substrate supplies its outer value, without a closure descriptor.

## 0.73.0 — 2026-08-09 — direct value-mode formal procedures

`value procedure p` now accepts the same direct, statically resolvable actuals
as a name-mode formal. Specialisation retains the direct target rather than
materialising a procedure value in the IIR ABI, including when a value-mode
formal is forwarded into another direct wrapper. Dynamic procedure values
remain unsupported.

## 0.72.0 — 2026-08-09 — standard output as direct formal procedures

A direct `procedure` actual may now be the implementation-defined standard
output procedure `print` or `output`. Specialised wrappers retain that direct
target and reuse the established statement-only `print_str` lowering at each
formal call site, including nested forwarding, without inventing an IIR
procedure signature or runtime function-pointer ABI. Dynamic procedure values
remain unsupported.

## 0.71.0 — 2026-08-09 — standard functions as direct formal procedures

A direct `procedure` actual may now be any implemented ALGOL standard function:
`abs`, `sign`, `entier`, `sqrt`, `sin`, `cos`, `ln`, `exp`, or `arctan`. A
specialised wrapper retains the function target and reuses its existing inline
lowering at the formal call site, preserving normal arity/type validation and
avoiding a function-pointer or closure ABI. Dynamic procedure values remain
unsupported.

## 0.70.0 — 2026-08-09 — direct formal procedures

`procedure` formals now accept a direct declared procedure actual. The compiler
specialises the enclosing call, retains the target in a formal-procedure binding,
and invokes it with its existing checked signature; no function-pointer or
closure ABI crosses IIR. A formal may be forwarded through a nested direct
wrapper, while expressions, scalar variables, standard procedures, and `value`
procedure formals remain rejected because they need a runtime descriptor ABI.

## 0.69.0 — 2026-08-09 — recursive scalar call-by-name remapping (AL7)

Direct and mutually recursive call-by-name procedures can now remap each
scalar formal to any active same-typed scalar formal, such as a recursive
`flip(n, y, x)`. The compiler tracks in-flight specialised siblings by their
canonical captured binding map, so a finite remapping cycle reuses the right
sibling while preserving each original caller expression and assignable write
target. Changed recursive scalar expressions still need a dynamic thunk frame
and remain rejected.

## 0.68.0 — 2026-08-09 — recursive scalar call-by-name forwarding (AL7)

A direct specialised sibling can now re-enter itself or another in-flight
sibling when every scalar name formal is forwarded unchanged under its original
formal name. This preserves the initial caller expression and assignable write
target, so direct and mutual recursion retain scalar read/write semantics
without erasing name passing to a value. Changed recursive scalar actuals still
need a dynamic thunk frame and remain rejected.

## 0.67.0 — 2026-08-09 — recursive array call-by-name (AL7)

Procedures whose name formals are arrays can now re-enter the direct specialised
sibling currently being lowered. Array actuals already use the ordinary typed
descriptor ABI, so direct and mutual recursion preserve the caller's backing
storage, bounds, and strides without a scalar thunk. Regression coverage fills
the same array through direct and mutually recursive procedures; scalar name
recursion remains explicitly rejected pending a runtime thunk ABI.

## 0.66.0 — 2026-08-09 — direct array call-by-name (AL7)

Direct array name formals now retain the existing typed array descriptor ABI:
bare array actuals pass their handle, every lower bound, and every row-major
stride to a specialised direct sibling. The descriptor aliases caller storage,
so element writes remain visible and a name array can be forwarded through a
nested direct call. Coverage includes nested two-dimensional forwarding with
non-unit lower bounds.

## 0.65.0 — 2026-08-09 — direct string call-by-name (AL7)

Direct `string` name formals now use the existing specialised sibling path,
including nested forwarding through lexical aliases. Reads re-evaluate the
caller string binding and literal or variable writes preserve normal
string-copy semantics before storing through that binding. Coverage includes a
forwarded formal whose inner parameter deliberately shadows the outer spelling
and rejection of a written literal actual.

## 0.64.0 — 2026-08-09 — direct scalar call-by-name forwarding (AL7)

Direct scalar name formals can now be forwarded to another direct name call.
The compiler substitutes the stored caller actual and captures each dependency
under a generated global alias before lowering the next specialised sibling, so
the forwarded expression retains its lexical binding even when the callee
reuses the same parameter name. Coverage includes write-through forwarding,
expression forwarding, and nested parameter shadowing.

## 0.63.0 — 2026-08-08 — direct scalar call-by-name (AL7)

Direct calls can now use `integer`, `real`, and `boolean` name formals. Each
call emits a specialised typed sibling function whose formal reads re-evaluate
the caller expression and whose writes target an assignable caller variable or
array element. Regression coverage includes state-sensitive re-evaluation, a
Jensen-style sum, real and boolean formals, and rejection of a written literal
actual.

## 0.62.0 — 2026-08-02 — dynamic `own string` reassignment

Regression coverage now proves that an `own string` can retain its typed global
identity while repeated calls replace its runtime handle and forward the newest
value through a string formal.

## 0.61.0 — 2026-08-02 — dynamic captured-string reassignment

Fixes bare-variable detection so a one-argument procedure call is not mistaken
for its actual. Regression coverage now proves that a captured string can be
overwritten by branch-selected string procedure results and passed onward as a
typed string formal.

## 0.60.0 — 2026-08-02 — composed integer procedure calls

Regression coverage now proves that two integer procedure results remain typed
`i64` values when passed to another integer procedure before reaching the block result.

## 0.59.0 — 2026-08-02 — composed real procedure calls

Regression coverage now proves that two real procedure results remain typed
`f64` values when passed to another real procedure before `entier` observes its result.

## 0.58.0 — 2026-08-02 — composed string procedure calls

Regression coverage now proves that a branch-selected `string procedure` result
remains a typed value while crossing another procedure's string value formal.

## 0.57.0 — 2026-08-02 — composed boolean procedure calls

Regression coverage now proves that boolean procedure results remain typed when
they become value actuals for another boolean procedure before branching.

## 0.56.0 — 2026-08-02 — boolean procedure control-flow results

Boolean procedure values now keep a `bool` comparison type through negation,
conjunction, and a conditional branch; regression coverage proves the path.

## 0.55.0 — 2026-08-02 — runtime string procedure results in string arrays

Regression coverage now proves that a runtime `string procedure` result can
populate `array<str>` storage, be read back for lexical ordering, and be printed
through the shared string runtime path.

## 0.54.0 — 2026-08-02 — right-associative exponentiation

ALGOL exponentiation chains now lower from the right, so `2 ^ 3 ^ 2` evaluates
as `2 ^ (3 ^ 2)` = 512 rather than `(2 ^ 3) ^ 2` = 64. Literal-only chains still
use the bounded typed multiplication fast path.

## 0.53.0 — 2026-08-01 — forwarding captured four-dimensional boolean arrays

Regression coverage now proves that a nested procedure can forward a captured
four-dimensional `boolean array` value formal to a sibling array formal. The forwarding
call reloads the `array<bool>` handle, four non-unit lower bounds, and all three row-major
strides before the callee writes caller-visible checkerboard cells.

## 0.52.0 — 2026-08-01 — runtime integer exponents

ALGOL exponentiation now lowers every numeric pair that is not a nonnegative
integer-literal fast path through `f64_pow`. Integer bases therefore support runtime and
negative integer exponents by widening to scalar `f64` results, preserving reciprocal
powers without compile-time multiplication expansion.

## 0.51.0 — 2026-08-01 — forwarding captured four-dimensional real arrays

Regression coverage now proves that a nested procedure can forward a captured
four-dimensional `real array` value formal to a sibling array formal. The forwarding
call reloads the `array<f64>` handle, four non-unit lower bounds, and all three row-major
strides before the callee writes caller-visible floating-point cells.

## 0.50.0 — 2026-08-01 — forwarding captured four-dimensional string arrays

Regression coverage now proves that a nested procedure can forward a captured
four-dimensional `string array` value formal to a sibling array formal. The forwarding
call reloads the `array<str>` handle, four non-unit lower bounds, and all three row-major
strides before the callee writes the caller-visible dynamic string cells.

## 0.49.0 — 2026-08-01 — captured four-dimensional string-array formals

Regression coverage now proves that a nested procedure can write a four-dimensional
`string array` value formal. The captured descriptor retains its `array<str>` handle,
four non-unit lower bounds, and all three row-major strides, while lexical ordering and
equality checks confirm the intended dynamic string cells survive.

## 0.48.0 — 2026-08-01 — captured four-dimensional integer-array formals

Regression coverage now proves that a nested procedure can write a four-dimensional
`integer array` value formal. The captured descriptor retains its `array<i64>` handle,
four non-unit lower bounds, and all three row-major strides, while the caller sums four
dynamic corner cells to confirm that the intended values survive.

## 0.47.0 — 2026-08-01 — captured three-dimensional real-array formals

Regression coverage now proves that a nested procedure can write a three-dimensional
`real array` value formal. The captured descriptor retains its `array<f64>` handle,
three non-unit lower bounds, and both row-major strides, while the caller sums four
distinct dynamic cells through `entier` to confirm the intended values survive.

## 0.46.0 — 2026-08-01 — captured three-dimensional boolean-array formals

Regression coverage now proves that a nested procedure can write a three-dimensional
`boolean array` value formal. The captured descriptor retains its `array<bool>` handle,
three non-unit lower bounds, and both row-major strides, while checkerboard reads confirm
the caller observes the intended boolean cells.

## 0.45.0 — 2026-08-01 — captured three-dimensional string-array formals

Regression coverage now proves that a nested procedure can write a three-dimensional
`string array` value formal. The captured descriptor retains its `array<str>` handle,
three non-unit lower bounds, and both row-major strides, while lexical and equality
checks confirm the caller observes the correct dynamic string cells.

## 0.44.0 — 2026-08-01 — captured multidimensional string-array formals

Regression coverage now proves that a nested procedure can write a two-dimensional
`string array` value formal. The captured descriptor retains its `array<str>`
handle, two non-unit lower bounds, and row-major stride, while lexical and equality
checks confirm the caller observes the correct dynamic string cells.

## 0.43.0 — 2026-08-01 — multidimensional boolean array formals

Rank-aware boolean array value formals now have direct regression coverage:
the descriptor preserves two non-unit lower bounds and the outer row-major
stride, so writes in the procedure select the same cells as the caller.

## 0.42.0 — 2026-07-31 — boolean arrays

`boolean array` declarations and boolean array `value` formals now lower to
the shared bounds-checked `array<bool>` descriptor path. Reads retain their
boolean element type for `not` and boolean operations, and
formal writes continue to alias the caller's storage across declared bounds.

## 0.41.0 — 2026-07-31 — nested scalar-formal capture

A nested procedure may now read and assign an enclosing scalar `value`
parameter. The outer procedure copies the incoming typed value into a
compiler-owned global before entering the nested sibling function, which then
reloads and updates that same value. Shadowing formals remain local rather than
capturing an enclosing block scalar.

## 0.40.0 — 2026-07-31 — conditional and nested switch designators

Switch-list elements now retain their complete designational expression until
the corresponding `goto s[index]` runs. An element may choose a label with
`if` or dispatch through another switch, so conditions observe their current
values and nested indices resolve at the computed-goto site. Cyclic switch
elements receive a deterministic compiler error instead of recursively growing
the generated dispatch chain.

## 0.39.0 — 2026-07-31 — nested array-formal capture

A procedure nested inside another procedure may now read and write an outer
`integer`, `real`, or `string` array `value` parameter. The outer frame copies
the incoming typed handle, every lower bound, and each non-final row-major
stride into compiler-owned globals; the nested sibling IIR function reloads the
same descriptor before each access. This preserves multidimensional declared
index spaces and aliases the caller's storage.

## 0.38.0 — 2026-07-31 — explicit zero-argument procedure calls

Typed and proper procedures with no formal parameters may now be called with
explicit empty parentheses. `answer()` works in an expression and `reset()` in
statement position, each lowering to an ordinary zero-argument IIR `call`.
Bare names retain their existing parsing: a statement may use a report-style
bare procedure name, while an expression bare name remains a variable read.

## 0.37.0 — 2026-07-31 — rank-aware array value parameters

`integer`, `real`, and `string` array `value` parameters now accept
multidimensional actuals. The compiler infers a formal's rank from its indexed
uses in the procedure body, then lowers the call ABI to the typed array handle
plus every lower bound and each non-final row-major stride. This keeps the
callee's `a[i,j,...]` accesses in the caller's declared index space, including
nonzero and negative lower bounds; writes still alias the actual storage.

One-dimensional descriptors retain their existing handle-plus-lower-bound ABI.
The compiler rejects rank-mismatched actuals and formals used with inconsistent
subscript counts. Regressions cover 2-D descriptor layout, 3-D VM execution,
and the diagnostics.

## 0.36.0 — 2026-07-31 — AL1 integer-to-real promotion

`integer` values now widen through the shared `int_to_real` IIR conversion
when a `real` is required. This covers mixed integer/real arithmetic and
comparisons, real division, real scalar and array-element assignments, real
value parameters, real standard-function arguments, and a real exponent for
`^`. `div` and `mod` remain integer-only; nonnumeric mismatches remain type
errors.

The compiler regressions cover each conversion site, and the LANG matrix runs
an ALGOL program through Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

## 0.35.0 — 2026-07-31 — AL8 array value parameters

One-dimensional `integer`, `real`, and `string` array `value` parameters now
cross a procedure call as a typed IIR descriptor pair: the backing-storage
handle and the actual array's declared lower bound. The callee rebinds that
descriptor in its fresh scope, so ordinary subscript lowering preserves the
actual array's index space and writes remain visible to the caller. Captured
and `own` array actuals reload both descriptor fields from typed globals.

Array actuals must be bare, element-type-compatible, one-dimensional variables;
multidimensional and by-name array parameters remain unsupported. The frontend
regressions cover integer, real, string, and captured integer descriptors plus
the multi-dimensional rejection case.

## 0.34.0 — 2026-07-30 — AL7 static scalar strings

Procedures can now assign and read scalar `string` variables declared by their
enclosing ALGOL block. These captures lower to typed module globals, allowing
the shared LANG backends to use their native string-handle representation.
`own string` has ALGOL static lifetime: a module-global initialization flag
installs its empty-string value on the first procedure call, then subsequent
calls reuse the same stored string.

The regression invokes a procedure that assigns an enclosing string and a
second procedure that writes and rereads an `own string` across calls. It
executes to 3 on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

## 0.33.0 — 2026-07-30 — AL6 own arrays

`own [type] array` declarations now have real ALGOL static lifetime. The
compiler guards their first allocation with a scalar module-global flag, then
stores the typed array handle and declared lower-bound/stride metadata in
module globals. Later procedure invocations skip allocation and recover that
same state, so an `own integer array memo[4:5]` retains element values and its
nonzero index space across calls.

The parser's checked-in Rust grammar artifact now recognizes `own_array_decl`.
The execution regression calls a procedure three times and observes `memo[4]`
as 1, 2, and 3 on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

## 0.32.0 — 2026-07-30 — ALGOL captured arrays

Procedures can now read and write arrays declared by their enclosing ALGOL
block. The frontend globalizes the array handle together with each declared
lower bound and row-major stride, so a procedure uses the declaration's real
subscript space rather than assuming a zero-based one. This covers integer,
real, and string arrays; array value parameters remain a separate call-ABI
follow-up.

The regression declares `integer array values[4:5]`, has a proper procedure
write both elements, and reads them in the enclosing block. It executes to 42
across Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

## 0.31.0 — 2026-07-30 — E4d-AL: string arrays

ALGOL `string array` declarations now reuse the shared E5 `array<str>`
substrate already exercised by Dartmouth BASIC. Literal and initialized-scalar
string writes lower to `str_const`/`array_set`; subscript reads yield `str` and
can feed lexical ordering or `print_str`. The regression program stores `HI`
and `LO`, compares the elements, and prints the selected element across native
AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

`own`/captured string storage and array parameters remain separate follow-ups.

## 0.30.0 — 2026-07-30 — E4d-AL: runtime scalar string ordering

Initialized scalar strings carrying a branch-selected `string procedure`
result now participate in lexical ordering. `s := pick(1); if s < 'LO' then ...`
lowers through the shared `str_cmp` operation and its signed comparison against
zero, instead of rejecting the string because its contents are not known at
compile time. The direct backend, generic JIT, AOT snapshot, WASM runtime, real
BEAM runtime, and seven-standard-backend matrix all execute `HI < LO`.

The old literal-provenance set is gone: definite source-order initialization is
the relevant safety invariant for scalar string reads, regardless of how the
value was produced.

## 0.29.0 — 2026-07-30 — E4d-AL: runtime scalar string locals

Initialized local scalar strings now carry runtime procedure results instead of
being restricted to direct literals. For example, `s := pick(1); if s = 'HI'
then ...; print(s)` lowers through the portable `str_concat`, `str_eq`, and
`print_str` operations.

- Local string reads require definite source-order initialization. Assigning a
  string procedure result copies it with `str_concat(result, "")`, preserving
  immutable snapshot semantics.
- Equality and output accept initialized runtime values; lexical ordering stays
  literal-backed because it still needs the broader runtime comparison slice.
- The cross-backend regression lowers the program through WASM, JVM, textual
  CIL/CLR, BEAM, and LLVM; AOT and generic-JIT tests cover the shared VM chain.

Captured/`own` strings, string arrays, and Unicode-aware BEAM string operations
remain outside this slice.

## 0.28.0 — 2026-07-03 — E4d-AL: string procedures (E4-dyn frontend payoff)

The first E4-dyn *frontend* payoff, now that all seven backends carry a runtime
string: **`string procedure`s**. A typed procedure whose return type is `string`
was previously rejected (`Unsupported("string procedures")`); it now compiles.

- **`procedure_parts`**: dropped the `ret == String` early rejection.
- **`compile_procedure`**: a string result slot (the procedure name) is seeded
  with `str_const ""` (a real empty-string handle) instead of `const 0`, and
  marked literal-backed so an unassigned path still returns a printable value.
  The body assigns the result like any string variable; a branch-selected
  assignment (`if n > 0 then p := 'HI' else p := 'LO'`) makes the result a
  genuinely runtime, control-flow-selected string.
- **`try_emit_standard_output_stmt`**: added a general string-expression path so
  `print(pick(1))` evaluates a string-procedure *call result* to a runtime handle
  and prints it via `print_str`. `print(42)` is now rejected by type (a clearer
  message) rather than by the old literal-only shape check.

Unit test `string_procedure_returns_runtime_string_and_prints` asserts the IIR:
`pick` is a function returning `str` whose result is assigned by `str_const` in
both branches and returned as `str`, and `main` calls it and prints the runtime
result. A `lang-aot` matrix cell proves it end-to-end on NativeAot + VM/JIT (the
columns that already carry a runtime string arriving as a call result). The
LLVM/WASM/JVM/CLR columns take their runtime-string path only for promoted-slot
operands today, so a string *return value* on those backends is the E4d-2b/3b +
JVM/CLR follow-up that will extend the cell to all seven.

## 0.27.0 — 2026-07-02 — AL-pow: the `↑` exponentiation operator (LANG-FULL)

ALGOL 60's exponentiation operator `↑` (§3.3.4; spelled `^` / `**` in our
grammar) is now lowered instead of rejected.  The `factor` / `expr_pow` node is
folded left-to-right, and each `base ↑ exp` uses one of two shapes — both
reusing IIR the code-gen backends already run, so **no new op and no backend
change**:

| exponent | lowering | result type |
|----------|----------|-------------|
| nonnegative integer literal `k` (≤ 64) | `k−1` repeated `mul`s (`x*x*…`); `x↑0 = 1`, `x↑1 = x` | **the base's type** — `integer↑k` stays `integer`, `real↑k` stays `real` |
| `real` exponent (with a `real` base) | the `f64_pow` op (libm `pow`) — the same op BASIC's BA-pow proved on all 7 backends | `real` |

The integer-literal path keeps ALGOL's typing: `2 ↑ 10` is the *integer* 1024
(BASIC always widens to `real`).  A non-literal exponent on an `integer` base, or
a negative literal, is a clean `Unsupported` — those need int→real coercion or
reciprocals not in this slice.

New:
- `emit_pow` / `emit_power_step` / `emit_pow_unroll` methods.
- `literal_nonneg_integer_exponent` helper + `MAX_POW_UNROLL_EXPONENT` (64).
- 9 new unit tests (121 total): integer power, square, `↑0`/`↑1`, precedence vs
  `*`, real-base integer-literal exponent, `real↑real` via pow, and the
  `integer↑real` rejection.

The 7-backend proof (`10 + 2 ^ 5` = 42, integer unroll) lives in `lang-aot`
`lang_matrix.rs`.

## 0.26.0 — 2026-07-02 — AL-multidim-bounds: arbitrary/negative lower bounds (LANG-FULL)

Proves that ALGOL's **arbitrary per-dimension lower bounds** (`[lo:hi]` with
`lo ≠ 1`, including `0` and negative) compose correctly with the multidim
row-major strides.  Each subscript is translated to `sub − lower` *before* the
stride is applied: `flat = Σ_d (sub[d] − lower[d]) * stride[d]`.  The 1:N cells
shipped so far never exercised `lower ≠ 1`, so this closes a real correctness
gap in the multidim index math.

No compiler code changed — `ArrayDim` already records a per-dimension
`lower_slot`, and `resolve_array_index` already emits the `sub − lower`
subtraction; this is a coverage-gap closure.

Two new unit tests (113 total):
- `two_d_array_arbitrary_lower_bounds` — `M[0:1, 2:4]`, flat index `(i)*3 + (j−2)`
- `two_d_array_negative_lower_bounds` — `M[−2:−1, 0:1]`, flat `(i+2)*2 + j`

The 7-backend proof lives in `lang-aot` `lang_matrix.rs` (`M[−1:1, 2:3]`, a
negative and a non-zero lower bound, summing to 42).

## 0.25.0 — 2026-07-02 — AL-multidim-3D: 3-D integer arrays (LANG-FULL)

Proves the multidim array machinery is genuinely **N-dimensional**, not
hardcoded to 2-D.  For `integer array M[1:2, 1:2, 1:2]` the strides are computed
right-to-left at declaration: `stride[2] = 1` (elided), `stride[1] = size[2] =
2`, `stride[0] = size[1] * stride[1] = 4` — so subscript `M[i,j,k]` lowers to
the flat 0-based index `(i−1)*4 + (j−1)*2 + (k−1)` over a single `alloc_array`
of length 8.

No compiler code changed — `emit_array_decl`'s right-to-left stride loop and
`resolve_array_index`'s accumulation already handle any dimensionality; the
3-D case just walks the loop one more iteration.  This is a coverage-gap
closure.

Three new unit tests (111 total):
- `three_d_array_store_and_load` — `M[2,2,2]` (flat index 7, last of 8 cells)
- `three_d_array_all_eight_cells` — triple-nested loop fills all 8, reads three
  corners (flat 0/4/7)
- `three_d_array_non_cubic` — `M[1:2, 1:3, 1:4]` (24 elements) proves the general
  stride product (stride[0]=12, stride[1]=4)

The 7-backend proof lives in `lang-aot` `lang_matrix.rs`.

## 0.24.0 — 2026-07-02 — AL-multidim-real: f64 multidim array elements (LANG-FULL)

Proves that the N-dimensional array machinery added in 0.23.0 carries **`real`
(f64) elements**, not just integers.  When AL-multidim first landed, f64
multidim elements were flagged as a follow-up; this closes that gap.

No compiler code changed — `emit_array_decl` already threads the declared
`elem_ty` (here `ScalarType::Real`) into `declare_array`, and `array_get`/
`array_set` ride the same 8-byte slots for f64 as for i64.  Only the flat-index
computation is multidim; the element storage is identical to a 1-D `real array`.

Two new unit tests (108 total):
- `two_d_real_array_roundtrips` — store four doubles into `M[1:2, 1:2]`, read
  `M[2,2]` back = 4.5
- `two_d_real_array_sum` — sum all four cells (1.5+2.5+3.5+4.5) = 12.0 via the
  f64 `add` path over multidim reads

The 7-backend proof lives in `lang-aot` `lang_matrix.rs` (fractional cells
summing to 42.0, floored via `entier`).

## 0.23.0 — 2026-07-01 — AL-multidim: N-dimensional integer arrays (LANG-FULL)

**`ArrayDim` struct** (new): per-dimension record holding `lower_slot: String`
(the IIR variable that holds the declared lower bound) and `stride_slot:
Option<String>` (the IIR variable that holds `product(size[d+1..N-1])`; `None`
for the last dimension where the stride is 1 and the multiply is elided).

**`ArrayInfo` struct** (changed): `lower_slot: String` → `dims: Vec<ArrayDim>`
so multi-dimensional arrays carry one `ArrayDim` per subscript position.

**`emit_array_decl`** (rewritten): For each dimension, evaluates the declared
lower and upper expressions, computes `size_d = upper - lower + 1` via
`sub_i64` + `add_i64`.  Strides are computed right-to-left: the last dimension
has `stride_slot = None` (multiply elided); each outer dimension `d` emits
`stride_d = size_{d+1} * running_stride` via `mul_i64`.  Total allocation
length = `size_0 * stride_0` for N ≥ 2 (single `mul_i64`), or `size_0` for N
= 1 (unchanged 1D path).

**`resolve_array_index`** (rewritten): Validates that `subs.len() ==
info.dims.len()` (compile-time arity check; panics on mismatch).  For each
dimension `d`: computes `diff_d = sub_d - lower_d` via `sub_i64`; contributes
`diff_d * stride_d` via `mul_i64` when `stride_slot` is Some, else `diff_d`
directly.  Accumulates contributions via `add_i64` into a single flat 0-based
index.  No backend change required — the emitted IIR uses only
`alloc_array`/`array_set`/`array_get` with flat indices, identical to 1D.

**5 new unit tests** (106 total):
- `two_d_array_store_and_load` — 2×2 matrix, single cell round-trip
- `two_d_array_all_four_cells` — all four cells produce the correct flat indices
- `two_d_array_non_square` — 3×2 (6-element flat array)
- `two_d_array_filled_with_loops` — nested ALGOL `for` loops fill + sum a 2D array
- `rejects_wrong_subscript_count_for_2d_array` — compile-time arity check panics

## 0.22.0 — 2026-06-30 — Fix `emit_not_value` WASM boolean type mismatch

`emit_not_value` implements `not b` as `cmp_eq(b, false_const)`.  The
`false_const` slot was emitted with `ScalarType::Integer` and
`Operand::Int(0)`, which caused `infer_local_type_hints` in the WASM
backend to assign the local a type of "i64".  When `b` is a boolean
variable (local type "bool", i.e. i32), the subsequent `cmp_eq`
instruction selected `I32_EQ` but one operand was i64 → WASM validation
failure.

Fix: change `false_const` to use `ScalarType::Boolean` + `Operand::Bool(false)`.
The WASM backend then assigns type "bool" (i32) to the false_const slot,
matching the boolean variable's i32 local.  LLVM is unaffected — both
`Operand::Int(0)` and `Operand::Bool(false)` lower to the literal `0` in
LLVM IR.

## 0.21.0 — 2026-06-29 — Real procedure unit test (LANG-FULL AL13-real-proc)

Added `real_procedure_runs` unit test covering the `real procedure` code path that
was already implemented but untested:

- `scale(x) = x * 6.0` (real procedure, single real parameter)
- Caller: `result := entier(scale(7.0))` — asserts 42 (42.0 floored)

This exercises `ScalarType::Real` as a procedure return type end-to-end through
the VM: `declare_var` seeds the `scale` slot as f64, the body assigns to it, `ret`
returns the f64, and the caller's `emit_entier` floors it to i64.

No compiler code changed — this is a coverage gap closure only.

## 0.20.0 — 2026-06-29 — `arctan` standard function (LANG-FULL AL8-arctan)

The ALGOL 60 §3.2.4 `arctan` standard function is now recognised and lowered to the
`f64_atan` IIR op via the existing `emit_f64_unary` helper:

- `arctan(E)` → `f64_atan` (libm `atan` on native/LLVM; `env.__atan` on WASM;
  `Math.atan` on JVM; `System.Math.Atan` on CLR; `f64::atan` on VM/JIT)

The inverse tangent maps −∞..+∞ → −π/2..π/2.  `arctan(0.0)` = 0.0 exactly in
IEEE-754 double, used as the matrix proof value.  Completes the transcendental standard
functions listed in ALGOL 60 §3.2.4.

## 0.19.0 — 2026-06-29 — `sin`/`cos`/`ln`/`exp` transcendentals (LANG-FULL AL8-trig)

The four ALGOL 60 §3.2.4 transcendental standard functions are now recognised and
lowered to the new `f64_sin`, `f64_cos`, `f64_ln`, `f64_exp` IIR ops:

- `sin(E)` → `f64_sin` (dispatch to libm `sin` on native, `env.__sin` on WASM,
  `@llvm.sin.f64` on LLVM, `Math.sin` on JVM, `System.Math.Sin` on CLR, `f64::sin` on VM/JIT)
- `cos(E)` → `f64_cos` (same pattern with `cos`)
- `ln(E)`  → `f64_ln`  (ALGOL uses `ln` for natural log; backends use `log`/`log.f64`)
- `exp(E)` → `f64_exp` (same pattern with `exp`)

Each op is implemented by the new `emit_f64_unary` helper (mirrors `emit_sqrt`).
All require exactly one `real`-typed argument; wrong arity or type → `CompileError::Type`.
Proof programs exit 42 on all 7 backends.

## 0.18.0 — 2026-06-28 — String-typed value parameters (LANG-FULL AL4-str-params)

ALGOL 60 typed procedures can now accept `string`-typed value parameters.
Previously `specifier_scalar_type` rejected `"string"` with an `Unsupported`
error; now it returns `Ok(ScalarType::String)`, unblocking the full
`value s; string s` spec syntax for string formals.

When `compile_procedure` binds a string parameter, its slot is immediately added
to `literal_string_slots` — the same set that makes a locally-assigned variable
printable.  This means `print(s)` inside the body lowers to `print_str s` with
no special string-parameter handling: the parameter is pre-seeded by the call
site's `str_const` (literal) or slot (named variable), which the E4 type system
already ensures is a known string value.

The call site in `emit_call_common` type-checks string actuals the same way as
integer/real parameters (`value.ty != *expected`); no new infrastructure is needed
because `str`-typed function parameters are already proven on all 7 backends via
Twig (TW4).

New unit tests verify: compilation succeeds, the body emits `print_str`, the IIR
parameter carries type `str`, the call site emits `call echo …`, a named string
variable can be passed, and an integer actual to a string parameter is a
`CompileError::Type`.

## 0.17.0 — 2026-06-28 — `sqrt` standard function (LANG-FULL AL8-sqrt)

`sqrt(E)` — the ALGOL 60 §3.2.4 hardware square root — is now recognised by
`algol-iir-compiler` and lowered to the new `f64_sqrt` IIR op.  The op carries
a `ScalarType::Real` result type, exactly like `real_to_int_floor` carries its
result type, so every backend's typed-dispatch path fires without change.

The lowering is gated on the presence of exactly one `real` argument; a non-real
or wrong-arity call is a compile-time `CompileError::Type`.  The proof program
`begin real r; integer result; r := sqrt(49.0); result := entier(r) end` exits
7 on all 7 backends.

## 0.16.0 — 2026-06-28 — Literal-backed string predicates (LANG-FULL AL4 on E4)

ALGOL 60 string comparisons now lower through the shared E4 string ops when both
operands are string literals or literal-backed scalar string variables:

```algol
begin string s; s := 'ALPHA';
  if (s = 'ALPHA' and s != 'OMEGA') and
     (s < 'BETA' and 'BETA' > s) then print('OK') else print('BAD')
end
```

`=`/`!=` use `str_eq` plus a typed zero comparison so the expression result is a
normal boolean. Ordering operators use `str_cmp` plus the corresponding typed
zero comparison. The slice remains intentionally fail-closed for unassigned
strings, captured/`own` strings, arrays, parameters, and dynamic string storage.

## 0.15.0 — 2026-06-27 — Multi-argument string output proof (LANG-FULL AL4 on E4)

ALGOL 60 `output` now has an explicit proof for multiple literal-backed scalar
string variables in one statement:

```algol
begin string s, t; s := 'O'; t := 'K'; output(s, t) end
```

The compiler preserves actual order by emitting two `print_str` calls over the
literal-backed E4 string slots. The matrix observes `OK` on every LANG backend
without adding an ALGOL-specific output hook or dynamic procedure call.

## 0.14.0 — 2026-06-27 — Scalar string copy snapshot proof (LANG-FULL AL4 on E4)

ALGOL 60 scalar string copy now has an explicit snapshot proof:

```algol
begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end
```

The compiler still lowers `t := s` through E4 `str_concat` with an empty
suffix. The new regression proves the copied target slot remains independently
printable after the source slot is rematerialized with a later `str_const`.

## 0.13.0 — 2026-06-27 — Literal-backed scalar string copies (LANG-FULL AL4 on E4)

ALGOL 60 scalar string assignment now accepts a literal-backed string variable
RHS:

```algol
begin string s, t; s := 'OK'; t := s; print(t) end
```

The compiler lowers `t := s` as E4 `str_concat t, s, ""`, materializing the
empty suffix as `str_const`. The target is marked literal-backed so `print(t)`
can consume it through `print_str`. Unassigned string sources still fail closed.

## 0.12.0 — 2026-06-27 — Literal-backed string variables (LANG-FULL AL4 on E4)

ALGOL 60 now supports the first scalar string-variable foothold:

```algol
begin string s; s := 'HI'; print(s) end
```

The `string` declaration records a `str` slot, assignment from a string literal
emits `str_const` directly to that slot, and `print(s)` is accepted only when
the slot is known to be literal-backed. That shape is important for the static
backends: WASM/LLVM/native/JVM/CLR can all consume the same direct E4 producer
metadata already proven by literal output.

This is still intentionally fail-closed. Unassigned string variables,
string-to-string copies, captured/`own` string globals, string procedures, and
string arrays remain outside the dynamic string model and produce explicit
errors.

## 0.11.0 — 2026-06-27 — Literal string output (LANG-FULL AL4 on E4)

ALGOL 60 now has an executable literal-output foothold for strings. Undeclared
statement-position calls named `print` or `output` lower each string literal
actual to the shared E4 pair:

```text
str_const <temp>, "HI" : str
print_str <temp> : void
```

That keeps the backend story language-neutral: no ALGOL-specific code-gen hook,
just the same `str_const` + `print_str` path Dartmouth BASIC already proved on
all seven LANG backends. A program may still declare its own procedure named
`print`/`output`; user procedures win over the standard output fallback.

This slice is intentionally narrow and fail-closed. Non-literal arguments such
as `print(42)` or `print(s)` still produce an explicit unsupported-feature error
until full ALGOL string declarations/expressions land.

## 0.10.0 — 2026-06-23 — `entier` standard function (LANG-FULL E8 PR-7)

The ALGOL 60 standard function **`entier`** (§3.2.5) — the largest integer not
greater than a real (floor, toward −∞):

```algol
entier(2.7)   ⇒ 2
entier(-2.7)  ⇒ -3      ; NOT -2 — floor, not truncate-toward-zero
entier(42.0)  ⇒ 42
```

This is the first **frontend consumer** of the E8 numeric-conversion ops. Unlike
`abs`/`sign` (which synthesise a conditional), `entier` lowers to a **single**
`real_to_int_floor` IIR op — the floor and the real→integer narrowing fused into
the primitive — so each backend emits its native floor-then-convert
(`llvm.floor`+`fptosi`, `f64.floor`+`i64.trunc_sat`, `Math.floor`+`d2l`,
`Math::Floor`+`conv.ovf.i4`, `frintm`+`fcvtzs`, `roundsd`+`cvttsd2si`). The
floor-vs-truncate distinction is exactly why E8 provides a distinct
`real_to_int_floor` alongside `real_to_int_trunc`.

The operand must be `real` (`entier` is specifically the real→integer floor; an
`integer` argument is a type error). A user `integer procedure entier` still
overrides the builtin (`proc_sigs` is consulted first in `emit_call_common`).
Resolved like `abs`/`sign` — no grammar change (`entier(x)` already parses as a
`proc_call`). Eight frontend unit tests (floor, toward-−∞, exact-integer,
single-op lowering, real-required, arity, user-override, composition with `abs`),
executed on the VM. The 7-backend executed matrix proof lands alongside.

## 0.9.0 — 2026-06-22 — `sign` standard function (LANG-FULL AL8, PR-2)

The second ALGOL 60 standard function (§3.2.4), **`sign`**, building on the
`abs` machinery from 0.8.0.

- `sign(E)` is the *signum*: `+1` if `E > 0`, `-1` if `E < 0`, `0` if `E = 0`.
  Unlike `abs`, the **result is always `integer`** regardless of the operand's
  type — `sign(-2.5)` is the integer `-1` (no real→integer coercion needed at
  the use site).  The operand may be `integer` or `real`.
- It lowers to the nested conditional `if E > 0 then 1 else if E < 0 then -1
  else 0`: a `cmp_gt` then a `cmp_lt` against a typed zero (compared at the
  operand width), with three `i64` constants moved into one result slot — the
  same store-per-branch shape (no SSA phi) `abs` uses, so it **runs on all seven
  backends** (native-AOT/LLVM/WASM/JVM/CLR/VM/JIT).  `E` is evaluated once.
- Same name-based, overridable resolution as `abs`: a user `procedure sign`
  wins over the built-in.
- **Verified by RUNNING:** a `lang_matrix.rs` cell — `43 + sign(0 - 1)` ⇒ exit
  **42** (the negative branch) — executes on every backend; plus 8 inline tests
  (positive / negative / zero integer `sign`, positive / negative real `sign`
  yielding an integer, composition with `abs`, the user-override case, and the
  wrong-arity rejection).

`entier` (floor of a real → integer) needs a float-floor+convert that is not a
portable IIR op, and `sqrt`/`sin`/`cos`/… need a runtime math library on every
backend; those are later AL8 slices.

## 0.8.0 — 2026-06-22 — `abs` standard function (LANG-FULL AL8, PR-1)

ALGOL 60 *standard functions* (§3.2.4) are built into the language rather than
user-declared procedures.  This release adds the first one, **`abs`**.

- `abs(E)` yields the absolute value of `E`, preserving its numeric type
  (`integer`→`integer`, `real`→`real`).  It lowers inline to the value of
  `if E < 0 then -E else E`: a `cmp_lt` against a typed zero, then a
  `jmp_if_false` choosing between a negated (`0 - E`, i.e. `sub`/`fsub`) and a
  pass-through `mov` into a single result slot.  This is the same store-per-branch
  shape the conditional-expression lowering already runs on **all seven backends**
  (native-AOT/LLVM/WASM/JVM/CLR/VM/JIT) — no backend learns anything about `abs`;
  it is compare + branch + subtract in the shared IIR.  `E` is evaluated once.
- **Resolution is name-based and overridable.** A standard function has no
  `proc_sigs` entry, so a call resolves to the built-in only when the name is
  *not* a user-declared procedure — a program that redeclares `procedure abs`
  gets its own version, exactly as the Report permits.
- **No grammar change.** `abs(x)` already parses as a `proc_call`; only the
  IIR-compiler's call lowering changed.
- **Verified by RUNNING:** a new `lang_matrix.rs` cell — `result := abs(0 - 42)`
  ⇒ exit **42** — executes on every backend; plus 9 inline tests (negative /
  positive / zero / composed integer `abs`, negative / positive real `abs`, the
  lowers-to-branches-not-a-call structural check, the user-override case, and the
  wrong-arity rejection).

`sign`/`entier`/`sqrt`/`sin`/`cos`/… follow in later AL8 slices (the
transcendentals need a runtime math library on every backend; the pure-IIR
`abs`/`sign`/`entier` come first).

## 0.7.0 — 2026-06-22 — `own` variables: static lifetime (LANG-FULL AL6)

ALGOL 60's `own` declarations (`own integer n`) now lower to **module globals**,
reusing the E6 global substrate (`global_load`/`global_store`). An `own` variable
is allocated once and retains its value across every call of its enclosing
block/procedure (ALGOL 60 §5.2.5), which is exactly the semantics a module
global gives — it zero-inits at module load and persists.

- `declare_var` gained an `is_own` flag; a declaration is materialised as a
  global when `is_own || captured` (E6). The slot is already unique per scope
  (`__algol_s<N>_<name>`, the per-procedure `scope_counter` differs), so two
  procedures' `own n` map to **distinct** globals — they don't alias.
- **A global is no longer given a per-declaration `const` zero-init.** For an
  `own` variable inside a procedure that init would re-zero it on every call,
  destroying persistence; for an E6-captured block scalar it was a dead
  register write shadowing the global. Globals zero-init once at module load,
  so the `const` is both unnecessary and wrong for them. Plain (register)
  scalars keep their zero-init.
- Proven by **running** on all 7 backends (`lang_matrix.rs`): `bump(d)` adds `d`
  to its `own integer n`; `bump(1) + bump(1) + bump(1)` accumulates `1 + 2 + 3
  = 6` (a non-`own` local would give `1 + 1 + 1 = 3`). Plus unit tests: lowering
  to `global_load`/`global_store` with no re-init `const`, VM-run persistence,
  and two procedures' `own` staying independent.
- Requires `coding-adventures-algol-parser` 0.2.0 (the `[ "own" ] type
  ident_list` grammar rule).

## 0.6.0 — 2026-06-22 — procedures share enclosing-block scalars as globals (LANG-FULL E6 layer 1)

A procedure body may now read and write a scalar declared in an **enclosing
block** — the canonical typed module global.  Previously a procedure could touch
only its own value parameters; an enclosing-scope reference was out of reach
(`compile_procedure` installs a fresh, isolated scope).

### Added
- **E6 capture analysis.**  At each block, before any scalar is declared, a
  pre-pass (`collect_block_captures`) scans every procedure body for the names it
  references (minus that procedure's own parameters / result name).  A block
  scalar whose name lands in this set is materialised as a module **global**
  (`VarBinding::is_global`) instead of a register — its slot doubles as the
  global's name.
- **Typed global ops at the access sites.**  A read of a captured scalar lowers
  to `global_load "name"` (via the new `read_scalar` helper); a write lowers to
  `global_store "name", v` — in **both** the procedure and the enclosing block,
  so they share one cell.  These are the same IIR ops every backend now runs
  (VM/JIT/LLVM/JVM/CLR + BEAM/WASM/native).
- Declaration order inside a block is now: non-procedure declarations →
  procedure bodies → statements, so a captured global is declared before any
  procedure that injects it.  `compile_procedure` re-injects the visible global
  bindings into the procedure's fresh scope so its body resolves them.

### Verified
- A procedure sharing an enclosing `counter` with its block (`integer procedure
  add(x); … add := counter := counter + x; counter := 40; result := add(2)`)
  lowers `counter` to `global_load`/`global_store` in both functions and **runs
  on the VM ⇒ 42**.  72 tests; the existing suite is unchanged (plain scalars
  stay registers).

## 0.5.1 — Fix: `for`-loop guard compares at operand width (ALGOL `for` runs on LLVM)

The `for … step … until` loop-guard comparisons (the step-sign check and the
ascending/descending bound checks) were emitted with `type_hint = "bool"` — the
boolean *result* type — instead of the integer *operand* width. A code-gen
backend reads a comparison's `type_hint` as its operand type, so on **LLVM**
(`iir-to-llvm`'s `lower_cmp`) this produced the invalid `icmp i1 <i64>, <i64>`
that `clang` rejects, leaving ALGOL `for` loops un-runnable on LLVM (they worked
on VM/JIT/JVM/CLR, which infer operand types differently). The three guards now
carry `"i64"`, matching the regular relational path (which already tags the cmp
with `lhs.ty.iir()`).

Effect: ALGOL `for`-loop programs now compile to valid LLVM IR and **run via
`clang`**. The E5 sum-of-squares array `Prog` (two `for` loops over an array,
exit 55) — previously LLVM-deferred — now runs on the LLVM matrix column. New
regression test asserts the guards compare at `i64`, not `bool`.

## 0.5.0 — One-dimensional arrays (LANG-FULL enabler E5 / AL2)

Array declarations and subscripts were rejected ("array variables/subscripts").
They now lower to the IIR's E5 array primitive (`interpreter-ir` 0.7.0), which
`vm-core` 0.7.0 executes on a bounds-checked heap:

- **`integer array A[1:10]`** (and `real array`) → an `alloc_array` whose length
  is the **run-time** span `upper - lower + 1`. ALGOL's *dynamic* bounds
  (`array A[lo:hi]` with expression bounds) work because the bounds are emitted
  as ordinary integer expressions, not folded constants. The binding records the
  lower bound so subscripts can be translated.
- **`A[i]` in an expression** → `array_get`, with the index translated to the
  IIR's **0-based** form `i - lower` (ALGOL arrays are declared with an explicit,
  often 1-based, lower bound).
- **`A[i] := e`** → `array_set`, same index translation, with `e`'s type checked
  against the element type.
- **Bounds-checked by construction**: an out-of-range subscript traps at run time
  (`vm-core` returns a `VMError`, surfaced as `CompileError::Runtime`).
- A segment with several names (`integer array A, B[1:2]`) declares **distinct,
  non-aliasing** arrays sharing one set of bounds.

Scope (this slice): **1-D**, with `integer`/`real` element types. Multidimensional
arrays (`M[i, j]`), non-numeric element types, and arrays as procedure parameters
produce a clear "unsupported" message and are tracked as follow-up. Verified end
to end by 9 new unit tests (store/load round-trip, 1-based and non-unit lower
bounds, fill-and-sum in `for` loops, distinct-array segments, out-of-bounds trap,
scalar-subscript and 2-D rejections, real arrays) plus a `lang-aot` matrix `Prog`
that runs a sum-of-squares array program on **VM + JIT** (exit 55). The code-gen
backends lower the array ops in E5 PR-3 (managed) and PR-4 (static).

## 0.4.0 — ALGOL 60 `real` arithmetic (LANG-FULL AL1 / enabler E3, phase 1)

`real` was rejected everywhere ("real scalars/parameters/literals on the common
slice"). It now lowers to the IIR `f64` type:

- **`real` type** (scalars, procedure parameters, procedure return types) →
  `ScalarType::Real` → IIR `f64`; a `real` slot is seeded to `0.0`.
- **`REAL_LIT`** (`3.14`, `1.0E-3`, `100E2`) parses via `f64::from_str` into an
  `Operand::Float`.
- **Arithmetic** `+` `-` `*` and **unary minus** accept `real` operands and emit
  the op with an `f64` `type_hint` (so the runtime computes in double); **`/`**
  is real division (also `f64`). `div`/`mod` remain integer-only (ALGOL's
  integer operators). **Ordered + equality comparisons** of reals compare at
  `f64` width (the operand-width hint, as for integers).
- **No implicit integer→real coercion** in this slice: mixing `integer` and
  `real` in one operator, or using `/` on integers, is a clean `Type` error
  (coercion needs an IIR int→float convert op the code-gen backends don't carry
  yet).

**Verified by RUNNING** on the VM and JIT (`lang-aot` `lang_matrix.rs`): real
multiply + equality fold → exit 42, real division + ordered comparison → exit 1.
10 new unit tests; the former `rejects_real_declarations_cleanly` test is
updated (`real_declarations_compile_to_f64`).

**Scope (E3 phase 1).** Reals run on the VM and JIT, which carry a tagged float
value model. The five code-gen backends don't execute f64 yet — `iir-to-{llvm,
wasm,jvm}` model every variable slot as a uniform `i64` (E3-codegen-slots) and
`iir-to-cil-bytecode` / the native backends reject `Operand::Float`
(E3-clr / E3-native). Those are tracked in `LANG-FULL-IMPLEMENTATION.md`.

## 0.3.0 — ALGOL 60 switches + conditional designators (LANG-FULL AL5)

- Lower **switch declarations** (`switch s := a1, a2, a3`) and the **computed
  goto** that uses them (`goto s[i]`). A switch records an ordered list of
  target labels; `goto s[i]` selects the i-th (1-based) target via a linear
  `index == k ? jmp Lk` chain. An out-of-range subscript matches no arm and
  falls through to the next statement (ALGOL leaves this undefined; treated as
  a no-op, the conventional implementation choice).
- Lower **conditional designational expressions** in `goto`
  (`goto if b then L1 else L2`), including nested/parenthesised designators —
  the branch is emitted with the portable `jmp_if_false` / `jmp` / `label`
  subset, recursing on the else-designator.
- **Fixed comparison lowering** — `cmp_*` now carries the **i64 operand width**,
  not the `bool` result width. Emitting `bool` made the LLVM backend compare two
  `i64` operands at 1-bit `i1` (`3 == 1` truncates both to `1` → wrongly equal)
  and produced invalid IR that `clang` rejected outright, so every ALGOL program
  with a comparison (`if`, `for … while`, switch index) was latently broken on
  the code-gen backends — it had simply never been exercised there (no ALGOL
  matrix program used a comparison until the switch's index test). This is the
  same width fix the BASIC BA0 work applied.
- Proven by **running**: `lang-aot`'s `lang_matrix.rs` executes a 3-element
  switch (`goto s[3]` ⇒ exit 49) across native / LLVM / WASM / JVM / CLR / VM /
  JIT — `s[3]` chosen because an i1-truncated compare would mis-select the first
  arm, so the cell guards the cmp fix. Unit tests cover each switch index, the
  out-of-range fall-through, both conditional-designator branches, the rejection
  paths (undeclared switch, non-integer index), and the cmp operand width.
- **Limits (follow-ups):** switch-list elements must be plain labels
  (conditional / nested-subscript elements rejected); switch declarations are
  not block-scope-shadowable (a flat per-compilation map, save/restored across
  procedure boundaries).

## 0.2.0 — ALGOL 60 typed procedures with value parameters (LANG-FULL AL3)

- Lower **typed (function) procedures with `value` parameters** to sibling
  `IIRFunction`s in the module. A heading like
  `integer procedure sq(x); value x; integer x; sq := x*x` becomes a function
  `sq(x: i64) -> i64`, and a call `sq(7)` (in expression or statement position)
  becomes an IIR `call` whose `srcs[0]` names the callee. Procedure signatures
  are registered in a pre-pass over each block, so a procedure may be called
  before it is textually declared and may call itself (recursion).
- Proven by **running**: `tests/lang_matrix.rs` in `lang-aot` executes
  `result := sq(7)` ⇒ exit `49` across native-AOT / LLVM / WASM / JVM / CLR /
  VM / JIT. Unit tests cover multi-parameter procedures, boolean procedures,
  recursion (factorial via an if-statement body), statement-position calls, and
  the rejection paths (void procedures, call-by-name parameters, arity and type
  mismatches).
- **Scope and limitations** (tracked as follow-ups): only typed procedures with
  `value` parameters are modelled. Proper (void) procedures are rejected — they
  have no observable effect on the current executable slice (no output op, no
  by-reference or enclosing-scope mutation), so admitting one would lower code
  no test could witness. Procedure bodies are lexically flat: they see their
  own value parameters but not enclosing-block variables (call-by-name /
  Jensen's device and non-local access are future work).

## 0.1.0

- Add an ALGOL 60 scalar frontend for the LANG VM Rust chain.
- Lower integer and boolean declarations, scalar assignments, integer arithmetic including `div`/`mod`, comparisons, if/else, compound statements, goto labels, and simple `for step until` loops to `interpreter_ir::IIRModule`.
- Prove the emitted IIR runs through `vm-core`, `jit-core`, `aot-core`, WebAssembly, JVM, CLR, BEAM, and LLVM backend paths.
