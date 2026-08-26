# Changelog

All notable changes to the `semantic-ir` crate are documented here.

## 0.28.0 — SIR30: switch statement (task #51)

Implements [SIR30](../../../specs/SIR30-switch-statement.md) — the core
IR previously had **zero** `Stmt` for `switch`/`case`, confirmed by an
exhaustive grep during `java-to-semantic-ir`'s own M2a milestone (tracked
as task #51 ever since, referenced across multiple `JV02` spec sections).
Additive only: every existing SIR10/SIR16/SIR17/SIR18/SIR21/SIR22/SIR23/
SIR26/SIR29 module remains valid; no existing `Expr`/`Stmt`/`SirType`/
`Feature` variant, validator rule, or backend behaviour changed. Mirrors
SIR29's own "spec + pure-additive IR diff, zero backend behavior change"
Slice 0 shape, and this crate's own `[0.27.0]` `Feature::LoopControl`
rollout specifically — no backend accepts `Feature::Switch` yet; every
existing backend's exhaustive `Stmt` match (and two frontends' own
internal `Stmt`-exhaustive traversal passes) gained a compile-forced
reject arm instead.

**New `Stmt` variant** (`nodes.rs`): `Switch { discriminant: Box<Expr>,
cases: Vec<SwitchCase>, default: Option<Vec<Stmt>>, span }` — a
C-family-style switch with real fall-through semantics (Java/
JavaScript's own), not the non-fall-through `case`/`match` shape
MATLAB/IDL/Ruby use (a frontend targeting one of those can always emit
an explicit trailing `Stmt::Break` per case to simulate it; the reverse
— representing real fall-through with a non-fall-through primitive —
isn't a narrowing, it's a correctness trap). `default`, if present, is
always the textually-last case for both match-order and fallthrough-
order purposes — a dedicated field, not a `cases` entry, so a non-last
`default` is structurally unrepresentable rather than merely
disallowed by a runtime check.

**New `SwitchCase` struct** (`nodes.rs`): `{ value: Expr, body:
Vec<Stmt>, span }`. No dedicated multi-value-label shape — an
empty-bodied case naturally falls through into the next, so `case 1:
case 2: foo(); break;` is just two `SwitchCase` entries.

**New `Feature` flag** (`manifest.rs`): `Switch` — a module contains a
`Stmt::Switch`. Kept separate from `Feature::LoopControl` (despite
`Break` now being valid inside a switch too — see below) for the
identical reason `LoopControl` was split from `Loops`: a brand-new node
no backend has ever emitted before needs an explicit backend opt-in,
not a silent expansion of what an existing feature flag promises.

**Validator** (`validator.rs`): `LoopKind` (introduced for
`Feature::LoopControl`) gained a third variant, `Switch`, pushed/popped
around a `Switch`'s own body walk exactly like `While`/`ForEach` push
`Safe` — reusing the *existing* `loop_stack` nesting-order logic rather
than a parallel mechanism, since "does `break` target the switch or an
outer loop" is fundamentally the same nesting question `loop_stack`
already answers for loop-inside-loop. `Stmt::Break`'s own check now
accepts `LoopKind::Switch` at the top of the stack, same as `Safe`.
`Stmt::Continue`'s own check changed shape: rather than consulting only
`loop_stack.last()`, it now walks the stack from the innermost entry
**skipping past any `Switch` frames**, to find the nearest *actual*
loop — matching every C-family language's rule that `continue` never
targets a switch, even when a switch is the more deeply nested
construct. The entire switch body (every case plus `default`) shares
ONE flat local-env scope, matching real `javac`'s own (well-known-
gotcha) rule that a local declared in an earlier case is lexically in
scope for every later case regardless of actual fall-through path.

**`walker.rs`/`backend.rs`/`text/printer.rs`**: each gained a
`Stmt::Switch` traversal/printing arm following the exact shape their
own existing `Stmt::TryCatch` arm already uses for a bare `Vec<Stmt>`
body with no trailing value slot.

**Backend/frontend compile-exhaustiveness fixes** (mechanical,
zero-behavior-change): `semantic-ir-to-{javascript,typescript,python,
ruby,go,c,rust}` and `{python,ruby}-to-semantic-ir`'s own internal
`Stmt`-exhaustive traversal passes each gained the identical
"capability check should have rejected it" panic arm (or an existing
file's own equivalent no-op convention for a frontend-internal pass)
every SIR29 node already carries at its own initial landing.

**New tests** (`validator.rs`): 11 new tests covering a simple switch
with `default`, `break` inside a switch (valid), `continue` inside a
switch with no enclosing loop (rejected), `continue` correctly skipping
a switch frame to reach an enclosing `while`, the reverse pairing
(`break`/`continue` inside a switch inside a `for-range`, confirming
`break` still targets the switch while `continue` still gets the
existing `for-range` rejection), cross-case local-variable visibility
(the `javac` scoping gotcha, both the positive case and the
does-not-leak-past-the-switch negative case), discriminant scope-
timing, and `default`'s own body being validated (not skipped).

## 0.27.0 — SIR16 addendum: bare loop-control statements (`break`/`continue`)

Implements the "Loop control" addendum to
[SIR16](../../../specs/SIR16-ir-extensions-for-python-and-javascript.md) —
the core IR previously had **zero** `Stmt` for `break`/`continue`, confirmed
by an exhaustive grep during `java-to-semantic-ir`'s own M2a milestone
(tracked as a standing backlog item). Additive only: every existing
SIR10/SIR16/SIR17/SIR18/SIR21/SIR22/SIR23/SIR26/SIR29 module remains valid;
no existing `Expr`/`Stmt`/`SirType`/`Feature` variant, validator rule, or
backend behaviour changed. Mirrors SIR29's own "spec + pure-additive IR
diff, zero backend behavior change" Slice 0 shape — no backend accepts
`Feature::LoopControl` yet; every existing backend's exhaustive `Stmt`
match gained a compile-forced reject arm instead.

**New `Stmt` variants** (`nodes.rs`): `Break { span }`, `Continue { span }`
— bare (unlabeled) only; SIR v0 has no loop-label vocabulary, so a source
language's labeled `break`/`continue` (Java `break outer;`) has no
lowering yet.

**New `Feature` flag** (`manifest.rs`): `LoopControl` — a module contains a
`Stmt::Break`/`Stmt::Continue`. Deliberately split from the existing
`Feature::Loops` (which every backend already declares for plain
`While`/`ForRange`/`ForEach`), so this addition doesn't silently redefine
what `Loops` has always promised.

**Validator** (`validator.rs`): new `ValidatorState::loop_stack:
Vec<LoopKind>` (`LoopKind::Safe` for `While`/`ForEach`, `LoopKind::
ForRange` for `ForRange`), pushed/popped around each loop's body walk
(mirrors the existing `in_call_args` save/restore idiom, generalized to a
stack since loops nest). `Stmt::Break`/`Stmt::Continue` consult only the
innermost entry:
- Empty stack → `"break"`/`"continue" outside a loop` error.
- Innermost loop is a `ForRange` → rejected. Several backends lower
  `ForRange` to a `while` with the step increment appended *after* the
  body (a bare `continue` would skip it), and `semantic-ir-to-ruby`
  hoists the body into a `->(){ }` lambda where a bare `break` doesn't
  propagate to the enclosing loop at all — the two backends' failure
  modes don't even agree on which statement is unsafe, so both are
  rejected uniformly rather than making validator correctness depend on
  a specific backend's lowering strategy.
- Innermost loop is `While`/`ForEach` → accepted.

`loop_stack` is explicitly reset (save/restore) around a top-level
`Function`'s own body walk and around a `Stmt::MethodDef`'s body walk, so
a `break`/`continue` inside a hoisted closure or a nested method can never
resolve against an unrelated enclosing loop the declaration happens to be
lexically nested in (a real, if pathological, IR-legal shape:
`while (...) { class C { method m() { break; } } }`).

**Caught by `/security-review` before push**: the same reset was
initially missing from `Stmt::ClassDef`/`Stmt::ModuleDef`/
`Stmt::SingletonClassDef`/`Stmt::NominalClassDef`'s own body walks — a
`break`/`continue` sitting *directly* in one of those bodies (not nested
inside a `MethodDef`, which already got its own reset) could resolve
against a loop the class/module declaration happens to be nested in
(`while (...) { class C; break; end }`). Inert today (no backend accepts
`Feature::LoopControl` yet, so nothing can reach codegen), but it
contradicted this very changelog's own stated invariant and would have
become a real bug the moment a backend adopts the feature. Fixed by
applying the identical save/restore to all four body walks; regression
tests added for each (`break_directly_in_{nominal_class,class_def,
module_def,singleton_class_def}_body_does_not_see_enclosing_loop`).

**Compile-compat stubs**: every crate with an exhaustive `Stmt` match —
six backends (`semantic-ir-to-{go,javascript,python,ruby,rust,
typescript}`) and three frontends whose internal generic tree-walking
utilities stay exhaustive even though none of them produce these nodes
yet (`{ruby,python}-to-semantic-ir`) — gained the minimal arm needed to
keep `cargo build --workspace` green, mirroring SIR29's own precedent:
panic guards in the backends' real emit paths (unreachable because
`ACCEPTED_FEATURES` never lists `LoopControl`), a defensive
`ScanHit::Unsupported` rejection in `semantic-ir-to-ruby`'s pre-emit scan
(this backend treats "capability check should reject it" as insufficient
defense on its own — same reasoning as its existing SIR29 arm), `None`/
no-op in the handful of pre-capability-check analysis passes that must
stay non-panicking (`semantic-ir-to-go`'s `check_soundness_stmt`,
`semantic-ir-to-rust`'s `const_ref_in_stmt`), and faithful structural
recursion (a no-op, since neither statement carries a child `Expr`/
`Stmt`) in the frontends. `javascript-to-semantic-ir` and `c-to-semantic-
ir`/`semantic-ir-to-c` required no changes (no exhaustive `Stmt` match in
either).

**`CURRENT_SIR_VERSION`**: `"5"` → `"6"` (per this crate's "adding a
feature is a v.bump" policy).

**Named out of scope** (disclosed, not silently dropped): labeled break/
continue; `switch` (still has no corresponding IR node at all — its own,
separately tracked backlog item, since fall-through semantics need their
own spec-level design decision); early/branched `return` (SIR still has
no `Stmt::Return` — a function's value is always its trailing `Block`
value — tracked alongside `switch` as needing comparable design work);
wiring any backend to actually emit `break`/`continue` (this PR is IR +
validator only, mirroring SIR29 Slice 0 — backend rollout is its own
follow-up, per-backend, once a real consumer frontend exists);
`java-to-semantic-ir` itself consuming this (its own follow-up — note its
classic-`for`-loop lowering currently desugars to `Stmt::While` with the
update-clause statement spliced onto the end of the body, the same
"trailing bookkeeping statement a bare `continue` would skip" shape this
addendum's `ForRange` restriction guards against, just at the frontend
rather than the backend layer — that lowering needs to change before this
crate can safely accept `continue` inside a classic `for`).

## 0.26.0 — SIR29: nominal/static-dispatch OOP profile (Java-family frontends)

Implements [SIR29](../../../specs/SIR29-nominal-static-oop-profile.md) — a
second, structurally different OOP profile alongside SIR25 §2's dynamic-
OOP `Feature::Classes`, closing the fork SIR25 §6 explicitly reserved for
"a nominally-typed, interface/generic-based, static-dispatch" family (the
substrate a `java-to-semantic-ir` frontend and `semantic-ir-to-java`
backend will need). Additive only, following the exact discipline SIR22/
SIR23 used for their own extensions: every existing SIR10/SIR16/SIR17/
SIR18/SIR21/SIR22/SIR23/SIR26 module remains valid; no existing `Expr`/
`Stmt`/`SirType`/`Feature` variant, validator rule, or backend behaviour
changed.

**New `SirType` variants** (`types.rs`):
- `Nominal { name: String }` — an advisory, unresolved reference to a
  declared class/interface, mirroring `Stmt::ClassDef.superclass`'s own
  unresolved-name discipline. Prints `(nominal Name)`.
- `TypeParam { name: String, bound: Option<Box<SirType>> }` — an *erased*
  generic type parameter (no runtime representation by design — see the
  spec's "Explicitly out of scope: reified generics"). Prints
  `(type-param T)` or `(type-param T Bound)`.

**New `Stmt` variants** (`nodes.rs`):
- `NominalClassDef { name, type_params, superclass, interfaces,
  is_abstract, fields: Vec<FieldDef>, body: Vec<Stmt>, span }` — unlike
  `ClassDef`, methods nest **directly** in `body` rather than being
  hoisted to detached top-level functions, since this profile's dispatch
  primitive never looks a method up by name at runtime.
- `InterfaceDef { name, type_params, extends: Vec<String>, methods:
  Vec<MethodSig>, span }` — signatures only (no default methods) for v1.
- `MethodDef { name, params: Vec<Param>, return_type, is_static,
  is_override, vtable_slot: Option<u32>, body: Block, span }` — nested
  inside a `NominalClassDef.body`. A static method needs no new node: it
  compiles to an ordinary `Expr::DirectCall` against a mangled top-level
  identity.
- New support structs: `FieldDef { name, sir_type, span }`, `MethodSig {
  name, params: Vec<SirType>, ret: SirType }` (bodyless — used only by
  `InterfaceDef`).

**New `Expr` variant, the dispatch primitive** (`nodes.rs`):
- `VirtualCall { receiver: Box<Expr>, method: String, slot: u32, args:
  Vec<Expr>, effects: EffectSet, span }` — an *index*-based sibling of
  SIR25 §2.2's *string*-based method-table lookup. `slot` is a frontend-
  assigned, per-class-hierarchy-stable vtable index (an override reuses
  its parent's slot); `method` rides along for debug/display only and is
  never a codegen dispatch key — the anti-injection invariant SIR25 §2.5
  requires is achieved *by construction* here (no string a backend could
  route through reflection), not by validation. Carries `effects`, unlike
  every SIR22/23 node: a virtual call may do anything its target method's
  body does.

**New `Feature` flags** (`manifest.rs`): `NominalClasses` (a
`NominalClassDef`), `Interfaces` (an `InterfaceDef`, or a
`NominalClassDef` with a non-empty `interfaces` list — split from
`NominalClasses` the same way SIR22 splits `MatrixOps` from `NDArrays`),
`VirtualDispatch` (a `VirtualCall`), `ErasedGenerics` (a `TypeParam`
anywhere).

**Validator** (`validator.rs`): `NominalClassDef` observes its features,
depth-guards, and recurses into `body` in a fresh local-env scope (like
`ClassDef`, since methods nest here rather than being hoisted);
`InterfaceDef` rejects a duplicate method-signature name; `MethodDef` gets
its own `check_method_def` — validated like a top-level `Function`
(duplicate-parameter detection, `OptionalTypeAnnotations`/
`DynamicTyping`/`DefaultParams`/`KeywordParams` observation, default-value
checking), but without `check_function`'s Ruby-specific `*rest`/`**opts`
ordering rules, since SIR29 v1 methods are a Java/C#-shaped positional-
plus-optional-default surface, not Ruby's full variadic/keyword calling
convention. `VirtualCall` observes `Feature::VirtualDispatch` and recurses
into `receiver` + every arg.

**Walker, backend intrinsics-scanner, printer** (`walker.rs`,
`backend.rs`, `text/printer.rs`): all three recurse into every new node's
children the same way the validator does; the printer gained new
S-expression forms (`nominal-class-def`, `interface-def`, `method-def`,
`method-sig`, `virtual-call`, `type-param`, `nominal`), none colliding
with an existing head keyword.

**Feasibility sketch** (spec-only, no code): confirmed the primitive is
realizable at both ends of this repo's host-language spectrum before any
backend implementation started — TypeScript (`VirtualCall` discards
`slot`, emits ordinary `receiver.method(args)`, since TS's own prototype
chain already is a vtable) and C (`NominalClassDef` → `struct C {
...fields; struct C_vtable* vtable; }`, `VirtualCall` →
`receiver->vtable->slot_N(receiver, args...)`).

**Backend capability rejection**: no backend changes are required for
this spec to land safely — a backend that doesn't declare
`NominalClasses`/`Interfaces`/`VirtualDispatch`/`ErasedGenerics` in
`accepts_features()` cleanly rejects any module using these nodes via the
existing SIR10 capability-check mechanism, proven with new `backend.rs`
tests (one per new feature, mirroring the SIR22/SIR23 capability-
rejection test blocks). All Rust-workspace crates with an exhaustive
`match` over `Stmt`/`Expr` — five backends
(`semantic-ir-to-{go,javascript,python,rust,typescript}`), one whose
`Stmt` match (not `Expr`, which already wildcards) is exhaustive
(`semantic-ir-to-ruby`), and three frontends whose internal generic
tree-walking utilities must stay exhaustive even though none of them
produce these nodes today (`{ruby,javascript,python}-to-semantic-ir`) —
gained the minimal compile-compat arm needed to keep `cargo build
--workspace` green, following the exact precedent SIR22/SIR23 set for
their own equivalent rollouts: panic guards in the backends (unreachable
because `ACCEPTED_FEATURES` never lists the new features) and faithful
structural recursion in the frontends. `semantic-ir-to-c` and
`c-to-semantic-ir` required no changes (no exhaustive `Stmt`/`Expr`
match in either crate).

**Versioning** (`metadata.rs`): `CURRENT_SIR_VERSION` bumped `"4"` →
`"5"`, following the same "adding a feature is a v.bump" policy that
moved `"3"` → `"4"` in the SIR22 addendum. The two `v4`-asserting golden
tests (`lib.rs`, `text/printer.rs`) were updated to `v5`.

Extensive unit tests added for every new node kind (construction, span
handling, printer round-trip, walker traversal, validator
feature-observation, duplicate-name rejection) plus new `backend.rs`
capability-rejection tests proving each of the four new features is
independently rejected by a backend that doesn't declare it — mirroring
the SIR22/SIR23 test blocks' style exactly.

## 0.25.2 — SIR21 T3b-2 Slice 1: division-split spec amendment (doc-only)

No code change. `op_select.rs`'s doc comment on the `/`-exclusion note
(`resolve_binary`) is updated for the SIR21 T3b-2 division-split arc
(`code/specs/SIR21-type-system-and-integer-semantics.md` §E3): the arc adds
a fourth canonical op name (`udiv_trunc`, `div_trunc`'s unsigned twin) and
none of `div_floor`/`div_trunc`/`udiv_trunc`/`div_true` ever reach
`resolve_binary` — every frontend that emits one already knows its exact
rounding mode at emission time, unlike `+`/`-`/`*`, which stay genuinely
polymorphic on operand type. `resolve_binary`'s actual behavior is
unchanged; this is a doc-comment-accuracy update landing ahead of the rest
of the arc (backend rollout, frontend migration, cleanup — tracked
separately per-package).

## 0.25.1 — Doc-comment corrections: `op_select`/`type_env` are now consulted (SIR21 T3c-3)

No code change. `semantic-ir-to-python` (see that crate's own changelog)
is now the first backend to build a `type_env::TypeEnv` and consult
`op_select::resolve_binary`, which made two module doc-comment claims in
this crate stale:

- `type_env`'s module doc said "Not yet consulted by any backend. No
  frontend populates `sir_type` on any node today (every operand is
  `Dynamic`)" — both halves were already inaccurate before this PR
  (`c-to-semantic-ir` has populated `Param`/`Stmt::LetStarBinding`
  `sir_type` for a while) and the "not yet consulted" half is now false
  too. Rewritten to state accurately: `semantic-ir-to-python` is the
  first consulting backend; `c-to-semantic-ir` is the first (and only)
  populating frontend; but its typed output cannot reach
  `semantic-ir-to-python` end-to-end yet because that backend does not
  declare `Feature::Conversions` in `ACCEPTED_FEATURES` (only
  `semantic-ir-to-c`/`semantic-ir-to-ruby` do) — the validator rejects
  any such module before emission. That gap is tracked separately, not
  fixed here.
- `op_select`'s module doc had the matching stale claim ("as *every*
  operand is in the current fully-dynamic pipeline") — corrected the
  same way, consistent with the `type_env` fix.

## 0.25.0 — `Feature::ConsoleIO` + `__sys_write__` structural validation (SIR28)

Core-IR slice of the SIR28 syscall-primitive arc (see
`code/specs/SIR28-syscall-primitives.md`). Adds `Feature::ConsoleIO`
(`manifest.rs`, new SIR28 comment group) — declared by any module
containing a `BuiltinCall("__sys_write__", [StrLit(stream),
StrLit(terminator), BoolLit(unpack_arrays), ...values])` node, the reserved
console-output primitive `"print"`/`"puts"`/JS `console.log` will migrate
to in follow-up PRs.

`validator.rs`'s `Expr::BuiltinCall` arm gains `check_sys_write_args`: a
structural check that `__sys_write__`'s `stream`/`terminator` args are
`StrLit`s (never a computed expression — the anti-injection invariant SIR28
§2.2 states) whose value is one of the closed set SIR28 §2.1 defines, and
that `unpack_arrays` is a `BoolLit`. This is a small improvement over the
existing precedent for reserved `BuiltinCall` envelopes (`__new__`/
`__method__`/etc., SIR25), which have zero structural validation today —
malformed args are only ever caught late, per-backend, by whatever a given
`emit.rs` match arm happens to assume.

No behavior change to any existing frontend or backend: nothing emits
`__sys_write__` yet. Verified: `cargo test -p semantic-ir` green (9 new
tests); every `*-to-semantic-ir`/`semantic-ir-to-*`/`sir-conformance`
downstream consumer builds and clippy-checks clean against the new
`Feature` variant.

## 0.24.0 — `type_env`: the SIR21 T3c-3 prerequisite

Adds `type_env::TypeEnv` — a shared, reusable name→type lookup that gives a
backend emitter a way to answer "what is this operand's statically-known
type?" so it can consult `op_select::resolve_binary`/`resolve_numeric`
(T3c-1/T3c-2, already done). This was the missing piece: `sir_type:
Option<SirType>` lives on *declaration* sites only (`Param`, `Capture`,
`Stmt::LetBinding`/`LetStarBinding`) — a bare `Expr::VarRef` carries no type
of its own, and a literal carries no type field at all — so resolving an
operand's type requires a name→declaration lookup, not a field read on the
expression. `TypeEnv::from_function` seeds params/captures; `observe_stmt`
updates it as a caller walks a block's statements in lexical order (matching
`Local` scope's ordinary shadowing); `expr_type` resolves a `VarRef` in
`Local`/`Param`/`Capture` scope or returns `None` (Dynamic) for everything
else — no inference, matching `op_select`'s own discipline. Deliberately
does **not** walk control flow itself (each backend already has its own
block-walking loop with its own scoping rules); a caller updates the
environment at the point it already visits each statement.

**Not yet consulted by any backend** — no frontend populates `sir_type` on
any node today, so wiring an emitter to build and consult a `TypeEnv` is
currently inert (would resolve to `RuntimeDispatch` everywhere, identically
to today). This slice exists so that wiring, when it lands per backend
(SIR21 T3c-3 proper), has one correct, unit-tested primitive to call rather
than six independent reimplementations. 9 new unit tests; zero behavior
change; `cargo test`/`clippy` clean across `semantic-ir` and unaffected in
every downstream crate (purely additive module + two new `pub` re-exports).

## 0.23.0 — SIR22 addendum: APL primitive Expr variants

Extends [SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md) with the
array-family primitive *operators* the base spec's first cut didn't cover.
SIR22's own Motivation section always claimed to be "the substrate for ...
every future array-family frontend (APL, J, K/Q, Scilab, IDL)," but the
first cut only populated the `Expr` variants MATLAB's own frontend needed
(`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`/
`IndexSet`) — MATLAB has no first-class *operator* syntax for reduce/scan/
outer-product/reshape/iota/ravel/catenate (it would call `sum(x)`/
`reshape(x, ...)` etc. as ordinary function calls, which this repo's MATLAB
frontend subset doesn't yet support at all). APL (`apl-runtime`/
`apl-parser`, already shipped) exposes all of these as first-class bare
glyphs, so this is a genuine substrate gap — the exact same kind of
prerequisite gap `array-runtime`'s own AR-2 task was for `apl-runtime`
before that runtime crate could be built. This PR closes the gap at the IR
level; it does **not** add the `apl-to-semantic-ir` frontend that will
consume it — that is a follow-up task. Additive only, following the exact
discipline SIR23 used for its own extension: every existing SIR10/SIR16/
SIR17/SIR18/SIR21/SIR22/SIR23/SIR26 module remains valid; no existing
`Expr`/`Stmt`/`SirType`/`Feature` variant, validator rule, or backend
behaviour changed.

**New `ElementwiseOpKind` variants** (`nodes.rs`): `Max, Min, Eq, Ne, Lt,
Le, Ge, Gt` added alongside the five SIR22 shipped (`Add, Sub, Mul, Div,
Pow`), mirroring `array_runtime::BinOp`'s own identical extension (added
for `apl-runtime`'s `⌈`/`⌊` max/min glyphs and its six comparison glyphs
`= ≠ < ≤ ≥ >` in PR #8072). Kebab-case `.name()`/`.from_name()` round-trip
extended to all thirteen variants.

**New `Expr` variants** (`nodes.rs`), each mapped 1:1 onto an existing
`array_runtime::ops`/`apl-runtime::builtins` op shape (the same "map 1:1
onto an existing op shape" discipline the SIR22 spec itself states as its
design principle, extended here to a second source op-shape since APL's
`⍴`/`⍳`/`,` don't fit the `BinOp` shape any more than they did for
`apl-runtime` itself):
- `Reduce { op: ElementwiseOpKind, target: Box<Expr>, span }` — `+/A`,
  maps 1:1 onto `array_runtime::ops::reduce(op, a)`.
- `Scan { op: ElementwiseOpKind, target: Box<Expr>, span }` — `+\A`, maps
  1:1 onto `array_runtime::ops::scan(op, a)`.
- `OuterProduct { op: ElementwiseOpKind, lhs: Box<Expr>, rhs: Box<Expr>,
  span }` — `A∘.×B`, maps 1:1 onto `array_runtime::ops::outer(op, a, b)`.
- `Shape { target: Box<Expr>, span }` — monadic `⍴A`, mirrors
  `apl-runtime::builtins::shape`.
- `Reshape { shape: Box<Expr>, target: Box<Expr>, span }` — dyadic `A⍴B`,
  mirrors `apl-runtime::builtins::reshape(a, b)`; field names spell out
  the shape-vector-vs-data role instead of reusing `lhs`/`rhs`, since
  (unlike `MatMul`/`ElementwiseOp`) the two operands aren't interchangeable
  in kind.
- `IndexGenerator { count: Box<Expr>, span }` — monadic `⍳N` (iota), mirrors
  `apl-runtime::builtins::index_generator`.
- `IndexOf { haystack: Box<Expr>, needle: Box<Expr>, span }` — dyadic `A⍳B`
  (index-of/search), mirrors `apl-runtime::builtins::index_of(a, b)`.
- `Ravel { target: Box<Expr>, span }` — monadic `,A` (flatten), mirrors
  `apl-runtime::builtins::ravel`.
- `Catenate { lhs: Box<Expr>, rhs: Box<Expr>, span }` — dyadic `A,B`
  (concatenate), mirrors `apl-runtime::builtins::catenate`.

**No new `Stmt` or `SirType` variant** — every new node is `Expr`-shaped and
`Pure`, same as the rest of SIR22.

**No new `Feature` flag** — every new node reuses SIR22's existing
`MatrixOps`/`NDArrays`/`ArrayColumnMajor` flags rather than adding new ones
(`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`Ravel`/`Catenate` observe
both `MatrixOps` and `ArrayColumnMajor` — same class of operation as
`MatMul`/`Transpose`; `IndexGenerator`/`IndexOf` observe only `NDArrays` —
they construct/query arrays without inherently being a "matrix op" any more
than `Range`/`IndexGet` are).

**Validator** (`validator.rs`): one match arm per new variant following the
exact feature-observation split above, each recursing into every child
`Expr`. Extensive new tests (one "rejected undeclared" + one "accepted
declared" pair per variant) mirror the existing SIR22 validator tests.

**Walker** (`walker.rs`) and **printer** (`text/printer.rs`): one arm per
new variant — the walker recurses into every child `Expr` with no `Feature`
bookkeeping (purely structural traversal); the printer renders each as a
parenthesized s-expression (`(reduce <op> <target>)`, `(scan <op>
<target>)`, `(outer-product <op> <lhs> <rhs>)`, `(shape <target>)`,
`(reshape <shape> <target>)`, `(index-generator <count>)`, `(index-of
<haystack> <needle>)`, `(ravel <target>)`, `(catenate <lhs> <rhs>)`),
reusing `op.name()` for the op-kind token exactly like the existing
`ElementwiseOp` arm does. New tests mirror the existing SIR22 walker/
printer tests one-for-one.

**Compile-compat ripple**: adding `Expr` variants to a shared narrow-waist
IR is not confined to `semantic-ir` itself — every crate with an exhaustive
`match` over `Expr` needs a new arm to keep compiling, independent of
whether it does anything semantically interesting with the new node. Eight
downstream crates had such exhaustive matches (carried over from SIR23's
own rollout): the five backends `semantic-ir-to-{javascript,typescript,
rust,go,python}` (panic-guard "deferred, not accepted" stubs in their
codegen `match`, gated the same way as the existing SIR22/SIR23 stubs —
`MatrixOps`/`NDArrays`/`ArrayColumnMajor` aren't in any of their
`ACCEPTED_FEATURES`, so a validated module can never reach these arms) and
the three frontends `{javascript,ruby,python}-to-semantic-ir` (faithful
structural recursion in internal analysis passes — effect inference,
call-graph collection, yield-rewriting, swap-safety checks — since none of
these frontends emit the new nodes today). `semantic-ir`'s own
`backend.rs::walk_intrinsics_in_expr` needed the same treatment. No actual
codegen was added anywhere — per this task's scope, real JS/TS backend
support for these ops (or any SIR22 node) remains a separate, not-yet-
started rollout item. `wolfram-to-semantic-ir`, `matlab-to-semantic-ir`,
`octave-to-semantic-ir`, `c-to-semantic-ir`, `twig-to-semantic-ir`,
`semantic-ir-to-c`, `semantic-ir-to-ruby`, and `sir-conformance` required no
changes (no exhaustive `Expr` match in any of them — their internal helpers
already use a wildcard `_ => ...` arm).

**Versioning** (`metadata.rs`): `CURRENT_SIR_VERSION` bumped `"3"` → `"4"`,
following the same "adding a feature is a v.bump" policy that moved `"2"`
→ `"3"` in SIR23 and `"1"` → `"2"` in SIR22 — this addendum introduces new
SIR text tokens (new printer forms, new op-kind names) even though it
isn't a new numbered spec. The `v3`-asserting golden tests (`lib.rs`,
`metadata.rs`, `text/printer.rs`) were updated to `v4`. No frontend crate
sets `metadata.sir_version` to `"4"` yet — `apl-to-semantic-ir` will, once
it exists.

No frontend crate consumes any of these nine variants yet — this addendum
is IR-substrate-only, mirroring how `array-runtime`'s AR-2 task preceded
`apl-runtime` as its own separate step. `apl-to-semantic-ir` is the
follow-up task that will actually emit these nodes from parsed APL source.

## 0.22.0 — SIR23: symbolic expression + pattern/rewrite IR extension

Implements [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md) —
the narrow-waist vocabulary for symbolic-CAS math languages (the substrate
for `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`,
`maxima-to-semantic-ir`). Additive only, following the exact discipline
SIR22 used for the array/matrix extension: every existing SIR10/SIR16/
SIR17/SIR18/SIR21/SIR22/SIR26 module remains valid; no existing `Expr`/
`Stmt`/`SirType`/`Feature` variant, validator rule, or backend behaviour
changed. Every new node kind maps 1:1 onto `symbolic_ir::IRNode`'s existing
five-variant shape (`Symbol`/`Integer`/`Rational`/`Float`/`Str`/`Apply`);
`IntLit`/`FloatLit`/`StrLit` already cover `Integer`/`Float`/`Str` (SIR10/
SIR16), so no new literal nodes were needed for those three.

**New `SirType` variant** (`types.rs`):
- `SymExpr` — an opaque symbolic-expression handle; carries no static shape
  (the shape lives in the `Expr` tree, not the type carrier). Prints
  `sym-expr`.

**New `Expr` variants** (`nodes.rs`):
- `SymSymbol { name: String, span }` — a bare symbolic-expression symbol
  (Wolfram `x`, `Plus`, `f`) used as *data*, distinct from `VarRef` (a
  host-language variable lookup) and `SymLit` (a Ruby-style interned
  `:symbol`).
- `SymRational { numer: i64, denom: i64, span }` — an exact rational scalar
  in reduced form; the IR carries but does not itself reduce the fraction.
- `SymApply { head: Box<Expr>, args: Vec<Expr>, span }` — `head[args…]` /
  `head(args…)` as data. `head` is a full `Expr` (not a bare `String`)
  because a *computed* head is legal Wolfram (`f[x][y]`) — the one place
  this spec's node shapes deliberately diverge from SIR22's simpler
  bare-name shapes.
- `SymPatternBlank { head: Option<Box<Expr>>, span }` — Wolfram `_`
  (`head: None`) or `_h` (head-constrained, `head: Some(SymSymbol("h"))`).
- `SymPatternNamed { name: String, pattern: Box<Expr>, span }` — a named
  pattern variable, Wolfram `x_` / `x_h`.
- `SymRule { lhs: Box<Expr>, rhs: Box<Expr>, delayed: bool, span }` — a
  rewrite rule; `delayed: false` is `->` (`Rule`), `true` is `:>`
  (`RuleDelayed`).
- `SymReplaceAll { expr: Box<Expr>, rules: Vec<Expr>, repeated: bool, span }`
  — rule application; `repeated: false` is `/.` (one pass), `true` is `//.`
  (fixed point). The full binding/traversal/iteration-cap contract every
  backend implementing this node must honour lives in the spec's "Matcher
  semantics" section, not in the IR shape itself.

**No new `Stmt` variant** — unlike SIR22's `IndexSet`, every SIR23 node is
`Expr`-shaped and `Pure` (see "Effects" below); `effects.rs` needed zero
changes.

**New `Feature` flags** (`manifest.rs`): `SymbolicExpr` (`SymSymbol`/
`SymApply`) and `PatternMatching` (`SymPatternBlank`/`SymPatternNamed`/
`SymRule`/`SymReplaceAll`). `SymRational` reuses SIR22's existing
`Rationals` feature rather than adding a new one, per the spec's explicit
"share, don't duplicate" instruction for `Rational`/`Complex`.

**Effects** (`validator.rs`): every new `Expr` variant is `Pure` — building,
matching, and substituting a symbolic term has no observable side effect
distinct from the value it computes. The validator's `check_expr` observes
the matching `Feature`(s) for each new node and recurses into every child
`Expr` (`SymApply`'s `head`+`args`, `SymPatternBlank`'s optional `head`,
`SymPatternNamed`'s `pattern`, `SymRule`'s `lhs`+`rhs`, `SymReplaceAll`'s
`expr`+`rules`), so a module using these nodes without declaring the
feature is a validator error, exactly like every other feature-observation
rule in this crate.

**Walker** (`walker.rs`, `backend.rs`): both the public `Visitor` traversal
and `backend.rs`'s internal `walk_intrinsics_in_expr` recurse into every new
node's children, including the computed-head case (`SymApply`'s `head` may
itself be a `SymApply`, e.g. `f[x][y]`).

**Printer** (`text/printer.rs`): new S-expression forms, each `sym-`
prefixed so none collides with an existing head keyword (notably the
pre-existing `(sym name)` form is `Expr::SymLit`, a different concept) —
`(sym-symbol name)`, `(sym-rational numer denom)`, `(sym-apply head arg...)`,
`(sym-pattern-blank [head])`, `(sym-pattern-named name pattern)`,
`(sym-rule lhs rhs eager|delayed)`, `(sym-replace-all expr (rules rule...)
once|repeated)`.

**Backend capability rejection**: no backend changes are required for this
spec to land safely, per the SIR23 spec's "Backend impact" — a backend that
doesn't declare `SymbolicExpr`/`PatternMatching` in `accepts_features()`
cleanly rejects any module using these nodes via the existing SIR10
capability-check mechanism (`Backend::check_module`), proven with new
`backend.rs` tests constructing a real module whose body nests
`SymReplaceAll` around a `SymApply` and a `SymRule` with a
`SymPatternNamed`/`SymPatternBlank` left-hand side — the full symbolic +
pattern-matching vocabulary in one tree. All five existing Rust-workspace
backends (`semantic-ir-to-{javascript,typescript,rust,go,python}`) and
three frontends (`{javascript,ruby,python}-to-semantic-ir`) had exhaustive
`match` statements over `Expr` that would otherwise fail to compile; each
gained the minimal compile-compat arm needed to keep `cargo build
--workspace` green — panic guards in the five backends (unreachable because
`ACCEPTED_FEATURES` never lists `SymbolicExpr`/`PatternMatching`), and
faithful structural recursion in the three frontends (none of which emit
these nodes today), following the exact precedent SIR22 set for its own
five-backend/three-frontend rollout. Per the spec's own rollout, JS/TS real
codegen against a `sir-runtime-symbolic` runtime package is explicitly
future work — that package does not exist yet, so even the JS/TS backends
get only the compile-compat panic guard in this PR, mirroring how SIR22's
core PR didn't ship `sir-runtime-array` codegen either. `c-to-semantic-ir`
and `semantic-ir-to-c` required no changes (no exhaustive `Expr` match).

**Versioning** (`metadata.rs`): `CURRENT_SIR_VERSION` bumped `"2"` → `"3"`,
following the same "adding a feature is a v.bump" policy that moved `"1"`
→ `"2"` in SIR22. A frontend lowering a Wolfram/Macsyma/Maxima-family CAS
language sets `metadata.sir_version` to `"3"` when its module uses any
SIR23 node. The two `v2`-asserting golden tests (`lib.rs`,
`text/printer.rs`) were updated to `v3`.

Extensive unit tests added for every new node kind (construction, span
handling, printer round-trip, walker traversal, validator
feature-observation) plus new `backend.rs`/`validator.rs` tests proving the
capability-rejection path against real `SymApply`/`SymPatternBlank`/
`SymRule`/`SymReplaceAll` node usage — both a validator-level test per node
kind (rejected undeclared, accepted declared) and an end-to-end test
through the actual `Backend::check_module` path — not just a hand-set
manifest flag or a unit assertion on the validator's internal state.

## 0.21.0 — SIR22: array/matrix IR extension

Implements [SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md) — the
narrow-waist vocabulary for dense numeric-array languages (MATLAB/Octave today;
APL/J/K/Scilab/IDL per HML00 tomorrow). Additive only, following the exact
discipline SIR16 used to add loops/sequences: every existing SIR10/SIR16/SIR17/
SIR18/SIR21 module remains valid; no existing `Expr`/`Stmt`/`SirType`/`Feature`
variant, validator rule, or backend behaviour changed.

**New `SirType` variants** (`types.rs`):
- `NDArray { elem: Box<SirType>, rank: Option<usize> }` — dense N-D numeric
  array; `rank: None` means unknown/dynamic rank (a frontend that can't prove
  rank statically leaves it absent — backends handle the absence, they never
  infer it). Prints `(ndarray T)` / `(ndarray T n)`.
- `Rational` — exact rational scalar (arbitrary-precision numerator/
  denominator left to the backend runtime). Prints `rational`.
- `Complex` — complex scalar (`{ re: f64, im: f64 }`). Prints `complex`.
- `Rational`/`Complex` are type-level carriers only — no numerator/denominator
  or re/im storage at the type level (a runtime concern) — shared with the
  future SIR23 symbolic-math extension, landed once here.

**New `Expr` variants** (`nodes.rs`), every one mapped 1:1 onto an existing
`array_runtime::execute()` op shape so a MATLAB/Octave frontend's job is
picking the right node, not inventing new semantics:
- `ArrayLit { rows: Vec<Vec<Expr>>, span }` — matrix literal `[1 2; 3 4]`.
- `Range { start, step: Option<Box<Expr>>, stop, span }` — `1:5`, `0:2:10`;
  `step: None` means step = 1.
- `MatMul { lhs, rhs, span }` — matrix multiplication (MATLAB `*`), distinct
  from `ElementwiseOp(Mul, ...)` (MATLAB `.*`).
- `ElementwiseOp { op: ElementwiseOpKind, lhs, rhs, span }` — the five dotted
  operators `.+ .- .* ./ .^` via the new `ElementwiseOpKind` enum
  (`Add`/`Sub`/`Mul`/`Div`/`Pow`).
- `Transpose { target, conjugate: bool, span }` — MATLAB `'` (conjugate,
  `true`) vs `.'` (plain, `false`); both map to the same runtime op.
- `IndexGet { target, indices: Vec<IndexArg>, span }` — indexed *read*
  (`A(2, :)`, `A(1:3)`). `IndexArg` is `Scalar(Box<Expr>)` (already 0-based —
  the frontend resolves `end`-relative subscripts before emitting this; the
  IR never sees `end`) / `Whole` (`:`) / `Range(Box<Expr>)`.

**New `Stmt` variant** (`nodes.rs`):
- `IndexSet { target, indices: Vec<IndexArg>, value, span }` — indexed
  *write* (`A(2, :) = v`). Per the spec, this is lowered as a `Stmt`-position
  mutation (like `Assign`), **not** a value-producing `Expr` — the one
  exception to "every new SIR22 node is Pure." Verified structurally: there
  is no `Expr::IndexSet` variant for the type system to construct.

**New `Feature` flags** (`manifest.rs`): `NDArrays`, `MatrixOps`, `Rationals`,
`Complex`, `ArrayColumnMajor` (the last states explicitly, at the manifest
level, that array storage is column-major/Fortran order — matching
`array_runtime::Array`'s internal layout — since a JS backend owns its own
buffer representation and needs that convention stated rather than left
implicit).

**Effects** (`validator.rs`): every new `Expr` variant observes `Pure`
semantics implicitly (no `EffectSet` field — same as `SeqLit`/`MapLit`/other
pure SIR16 literals). The validator's `check_expr`/`check_stmt_seq` observe
the matching `Feature`(s) for each new node (`ArrayLit`/`Range`/`IndexGet`/
`IndexSet` → `NDArrays`; `MatMul`/`ElementwiseOp`/`Transpose` → `MatrixOps`;
`ArrayLit`/`MatMul`/`ElementwiseOp`/`Transpose` → `ArrayColumnMajor`), so a
module using these nodes without declaring the feature is a validator error,
exactly like every other SIR16/17/18 feature-observation rule.

**Walker** (`walker.rs`, `backend.rs`): both the public `Visitor` traversal and
`backend.rs`'s internal `walk_intrinsics_in_{stmt,expr}` (used for the
intrinsic-whitelist check) recurse into every new node's children, including a
shared `walk_index_args`/`walk_intrinsics_in_index_args` helper for the three
`IndexArg` shapes.

**Printer** (`text/printer.rs`): new S-expression forms — `(array (row ...) ...)`,
`(range start [step] stop)`, `(matmul lhs rhs)`, `(elementwise-op <kind> lhs rhs)`,
`(transpose target conjugate|plain)`, `(index-get target (idx-scalar e) (idx-whole)
(idx-range e) ...)`, `(index-set target ...indices... value)`.

**Backend capability rejection**: no backend changes are required for this
spec to land safely, per the SIR22 spec's "Backend impact" — a backend that
doesn't declare `NDArrays`/`MatrixOps` in `accepts_features()` cleanly rejects
any module using these nodes via the existing SIR10 capability-check
mechanism (`Backend::check_module`), proven with new `backend.rs` tests
constructing a real `ArrayLit`/`MatMul`-using module. Verified this claim
against the actual mechanism rather than just asserting it: all five existing
Rust-workspace backends (`semantic-ir-to-{javascript,typescript,rust,go,python}`)
and three frontends (`{javascript,ruby,python}-to-semantic-ir`) had exhaustive
`match` statements over `Expr`/`Stmt` that would otherwise fail to compile;
each gained the minimal panic-guard arm (never real codegen) needed to keep
`cargo build --workspace` green, following the exact precedent SIR16 set for
its own four-backend rollout (their `ACCEPTED_FEATURES` lists are unchanged,
so the capability check rejects a SIR22-using module before any panic arm is
ever reached — the panic exists only to catch a future capability-check
regression). `twig-to-semantic-ir` required no changes.

**Versioning** (`metadata.rs`): `CURRENT_SIR_VERSION` bumped `"1"` → `"2"`,
following the same "adding a feature is a v.bump" / "new text tokens" policy
that moved `"0"` → `"1"` in SIR21 T1b — SIR22 introduces new `SirType`/`Expr`/
`Stmt` surface with new printed text tokens. A frontend lowering MATLAB/Octave
sets `metadata.sir_version` to `"2"` when its module uses any SIR22 node. The
two `v1`-asserting golden tests (`lib.rs`, `text/printer.rs`) were updated to
`v2`.

Extensive unit tests added for every new node kind (construction, span
handling, printer round-trip, walker traversal, validator feature-observation,
and the `IndexSet`-is-a-`Stmt`-not-an-`Expr` design rule) plus new
`backend.rs` tests proving the capability-rejection path against real
`ArrayLit`/`MatMul` node usage, not just a hand-set manifest flag.

## 0.20.0 — SIR21 T3c-2: complete the op-selection rule (concat + comparison)

Completes the `op_select` rule begun in T3c-1. Where `resolve_numeric` decides
only the `+`/`-`/`*` *numeric* case, the new op-aware `resolve_binary(op, lhs,
rhs)` folds in the two cases a bare numeric resolver can't see, so a backend has
one entry point for every binary operator it lowers.

- New `resolve_binary(op, lhs, rhs) -> BinaryLowering`, with `BinaryLowering` =
  `IntArith(IntSpec)` · `FloatArith` · `StrConcat` · `TypedCompare` ·
  `RuntimeDispatch`. Re-exported from the crate root.
  - `+` is polymorphic: `(Str, Str)` → `StrConcat`, else the numeric decision.
  - `-`/`*` → the numeric decision (via `resolve_numeric`).
  - `<` `>` `<=` `>=` `==` `!=` → `TypedCompare` only when both operands are the
    *same* concrete comparable type (same int spec, both `Float`, both `Str`, or
    both `Bool`); mixed / `Dynamic` → dispatch (no silent promotion).
  - `/` is deliberately **not** modelled — division is split into
    `div_floor`/`div_trunc` with its own rounding semantics (§E3); it and any
    unknown operator resolve to `RuntimeDispatch`.
- No inference, no mutation, no behaviour change: `resolve_numeric` is unchanged
  and every operand is `Dynamic` in the current pipeline, so `resolve_binary`
  returns `RuntimeDispatch` everywhere until a typed frontend (or the per-backend
  sized-int lowering) supplies types. 5 new tests cover arithmetic parity with
  `resolve_numeric`, `+`-concat vs `-`/`*`, comparison specialise/dispatch, and
  the `/`/unknown-operator fallthrough.

Purely additive (extends one module + two more re-exports; no existing code
changed); every downstream consumer still builds and passes.

## 0.19.0 — SIR21 T3c (T3c-1): type-directed op-selection rule

First slice of milestone **T3c** — "semantic neutrality made mechanical". Adds
the pure decision function each backend consults to choose *when* to specialise
a binary numeric op (`+`/`-`/`*`) from its operands' static types, vs. fall back
to runtime dispatch (SIR21 §"Type-directed operation selection").

- New `op_select` module: `resolve_numeric(lhs, rhs) -> NumericLowering` where
  `NumericLowering` is `Int(IntSpec)` (both operands the same concrete integer
  type — specialise; `Arbitrary` = the bignum path), `Float` (a float meets
  another number — promote), or `RuntimeDispatch` (any operand `Dynamic`/absent,
  non-numeric, or two integers of *different* specs — today's `_sir_*` path).
  Re-exported from the crate root.
- **No inference, no mutation, no behaviour change.** SIR carries types, it does
  not synthesise them; mismatched widths dispatch rather than silently promote.
  An untyped program (every operand `Dynamic`, as in the current pipeline)
  resolves to `RuntimeDispatch` on every node, so emitters keep doing exactly
  what they do now. It is the shared rule the per-backend sized-integer lowering
  (T4–T8) will consult so they agree on when to specialise.
- 8 unit tests walk the spec table row by row: matching i32/arbitrary/sized
  widths specialise; float promotes against any number (but not against
  `Dynamic`); mismatched width / signedness / overflow-mode and non-numeric
  types dispatch.

Purely additive (a new module + two re-exports; no existing code changed); every
downstream consumer still builds and passes. Scoped to the numeric keystone —
string concat on `+` and comparison operators are a later slice.

## 0.18.0 — SIR21 T3a: integer-reflection const-intrinsics (`int.max/min/width`)

First slice of milestone **T3**. Adds the canonical evaluator for the three
integer-reflection const-intrinsics (SIR21 §"Min / max / limits are derived"):
a program that reads `INT_MAX` / `i32::MAX` / a type's bit-width goes through
these, and they **const-fold** to a literal from the `(width, signed)` spec —
the value is target-independent, so a backend emits `2147483647`, never a
runtime call.

- New `int_const` module: `IntConst { Max, Min, Width }` with `name()` /
  `from_name()` (canonical boundary names `int.max` / `int.min` / `int.width`),
  `IntConst::ALL`, and `IntConst::eval(spec) -> Option<i128>`, plus the
  convenience `eval_int_const_named(name, spec)`. Re-exported from the crate
  root (`IntConst`, `eval_int_const_named`).
- Values are derived from T1a's `IntSpec::max()` / `min()` / `width().bits()`,
  so they inherit the audited (and panic-free) `i128`-corner handling. An
  `Arbitrary`-width integer (Ruby's `Integer`) has no `Max`/`Min`/fixed `Width`
  → `None`, matching a language whose integers grow without bound.
- 9 unit tests pin the spec examples (`int.max(i32) = 2147483647`,
  `int.min(u8) = 0`, `int.width(i32) = 32`, `arbitrary → None`), the name
  round-trip, and the W128 corner.

Behaviour-preserving: purely additive (a new module + two re-exports; no
existing code changed), so no frontend emits these yet and every downstream
consumer (frontends, backends, conformance) still builds and passes. The module
establishes the one canonical meaning each backend will const-fold to.

## 0.17.0 — SIR21 T1b: source-fidelity types + feature flags + version bump

Milestone **T1b** of the [SIR21 cascade](../../../specs/SIR21-type-system-and-integer-semantics.md).
Adds the additive type surface for typed frontends and the feature flags a
backend uses to fast-reject what it can't express, and bumps the SIR version
to `"1"` — the first milestone whose surface introduces **new text tokens**.

- **Three new `SirType` variants** (additive; no existing consumer matched
  `SirType` exhaustively, so only the `Display` impl needed new arms):
  - `Ptr { pointee, nullable }` — C/C++ pointer/reference (source fidelity;
    `nullable` distinguishes a possibly-null pointer from a non-null ref).
    Prints `(ptr T)` / `(ptr? T)`.
  - `Struct { name, fields }` — nominal record with **ordered** `(field, type)`
    members (order matters for C layout). Prints `(struct Name (f T) …)`.
  - `Optional { inner }` — nullable `T`-or-nil, wraps any type. Prints
    `(optional T)`.
  - Constructors `SirType::ptr` / `struct_type` / `optional`.
- **Seven new `Feature` flags** (manifest.rs) so a backend rejects in O(1)
  what it cannot honour: `SizedIntegers`, `Unsigned`, `WrappingArithmetic`,
  `FixedArrays`, `Pointers`, `Structs`, `Bignum`. Each added to the enum,
  `ALL`, `name()` (kebab-case), and the doc table; the existing
  `name_round_trips` / `all_features_have_unique_names` tests cover them, plus
  a focused `sir21_t1b_features_present_and_named`.
- **`CURRENT_SIR_VERSION` bumped `"0"` → `"1"`.** Per the crate's own policy
  ("adding a feature is a v.bump") and because the type surface grew. All
  frontends set the version symbolically via `.with_sir_version(...)`, and no
  backend gates on the literal, so the bump ripples cleanly; the two printed-
  header golden tests (`v0` → `v1`) were updated. The printer's version
  fallback now reads `CURRENT_SIR_VERSION` instead of a hard-coded `"0"` that
  silently drifted.
- Serialisation of *existing* modules is otherwise unchanged (no frontend
  emits the new types yet). All 10 downstream consumers + `sir-conformance`
  re-run green.

## 0.16.0 — SIR21 T1a: `SirType` v2 core (Dynamic + parameterised Int)

Milestone **T1a** of the [SIR21 type-system cascade](../../../specs/SIR21-type-system-and-integer-semantics.md)
— the Phase-0 mechanical remap. Behaviour-preserving: every existing module
lowers **identically** and serialises **byte-for-byte** the same.

- **`SirType::Any → SirType::Dynamic`.** The top type is renamed to make room
  for a vocabulary of more-specific types. The five in-tree construction sites
  (backend intrinsic scaffolding, validator test, JS/TS `return_type`) were
  updated. The method `is_any()` → `is_dynamic()`.
- **`SirType::Int` is now `SirType::Int(IntSpec)`.** An integer carries its
  `(width, signed, overflow)` semantics — the keystone of SIR21 ("the type is
  the semantics"). New public types:
  - `IntWidth = W8 | W16 | W32 | W64 | W128 | Arbitrary` (with `bits()`).
  - `Overflow = Wrap | Trap | Saturate | Checked | Undefined | Arbitrary`.
  - `IntSpec { width, signed, overflow }` with derived (never-stored)
    `min()` / `max()` / `modulus()` bounds and constructors `arbitrary()` /
    `sized(..)`.
  - `SirType::int_default()` (== `Int(IntSpec::arbitrary())`) and
    `SirType::int(width, signed, overflow)`.
- **The v0 flat `Int` maps to arbitrary-precision, not 64-bit.** *Divergence
  from the spec's tentative `I64_WRAPPING_OR_ARBITRARY` default:* the historical
  dynamic pipeline never masked integers (Ruby → Python both grow), so the
  *faithful* behaviour-preserving default is `Arbitrary`. A frontend that means
  a machine `i64` must now say so explicitly. The SIR21 spec §"The extended type
  lattice" was updated to record this.
- **Text surface unchanged.** The enum variant is `Dynamic` but the S-expr
  keyword for the top type stays `any`, and the default (arbitrary) `Int` still
  prints as bare `int` — so all printed modules and golden tests round-trip
  unchanged. Only a *sized* spec prints its full shape, e.g. `(int u32 wrap)`.
- **No SIR version bump yet.** T1a's serialisation is byte-identical (nothing
  emits sized ints), so `CURRENT_SIR_VERSION` stays `"0"`; the bump lands with
  T1b when new text tokens (`Ptr`/`Struct`/`Optional`, sized ints) first appear.
- New unit tests for `IntSpec` bounds, width bits, overflow/spec Display, and
  the arbitrary-default remap. All 10 downstream consumers (frontends,
  backends, conformance) re-run green.

## 0.15.0 — Reject keyword arguments on indirect/closure calls (KW hardening)

Closes a **soundness gap** left by KW1. The validator's shared keyword check
(`check_kwargs_common`) enforced ordering and duplicate rules on **every** call
kind, but for an `IndirectCall` (callee not statically known — a closure/
function value, validated with `callee == None`) it stopped there and did
**not** reject the mere *presence* of a keyword argument.

Per the design spec (`code/specs/sir-keyword-params.md`, "Out of scope"),
**indirect/closure keyword calls are out of scope for v0** — no backend can
emit them. Every backend's `emit_args` for an `IndirectCall` routes each
argument through `emit_expr`, whose `KeywordArg` arm is a hard `panic!`
(resolving a keyword needs the callee's parameter names/order, which an
indirect call does not have statically). So a validator-accepted module such
as `main(g) { g(x: 1) }` would **panic the backend at lowering time** — a
denial-of-service on validator-accepted input.

The validator now rejects any `KeywordArg` in the argument list of an
`IndirectCall` with:

> keyword argument `NAME` is not allowed on an indirect/closure call (only
> direct calls support keyword arguments in v0)

This change is **purely subtractive**: it forbids more (previously
ill-formed-but-accepted) programs, adds **no** new enum variant or field, and
does **not** alter any accepted `DirectCall` behavior (the direct path passes
`Some(callee)` and never reaches this branch). Downstream crates that only
*construct* IR are therefore unaffected; only ill-formed IR is now caught
earlier, at validation, instead of at backend emission.

### Tests

Added: an `IndirectCall` carrying a `KeywordArg` is rejected with the new
message; the same keyword argument to a matching-signature `DirectCall` still
validates; an `IndirectCall` with only positional args still validates. The
former `indirect_call_skips_keyword_name_resolution` test (which asserted the
now-unsound acceptance) is replaced accordingly.

## 0.14.0 — Keyword parameters & arguments (KW1; core IR)

Adds **named keyword parameters** (`def f(x:)` / `def f(x: 1)`) and
**keyword arguments** (`f(x: 1)`, Python `f(x=1)`) to the core IR. This is
**milestone KW1 of the keyword-params cascade**: it lands the
representation, validation, walker, and printer support only. No frontend
lowers to keyword params yet and no backend accepts the new
`Feature::KeywordParams`, so a keyword-using module is correctly rejected by
the capability check until each backend gains support — **all existing
behavior is unchanged**.

### Model — required vs. optional rides on `Param.default`

A keyword parameter is a `Param` with `kind == ParamKind::Keyword`. Whether
it is *required* or *optional* is carried by the **existing** `default`
field, exactly as a positional optional already works — there is no separate
"is-required" flag:

| `kind`    | `default` | meaning                          | source        |
|-----------|-----------|----------------------------------|---------------|
| `Keyword` | `None`    | **required** keyword parameter   | `def f(x:)`   |
| `Keyword` | `Some(e)` | **optional** keyword parameter   | `def f(x: 1)` |

`ParamKind` remains `Copy` with `#[default] = Required`, so no existing
`Param { .. }` construction changes.

A keyword argument at a call site is the new `Expr::KeywordArg { name,
value, span }`. It lives **inside the existing `args` vec** of a call, after
all positional args (`f(1, a: 2)` → `args: [IntLit(1), KeywordArg{…}]`),
rather than a parallel `kwargs` field on every call node — keeping the
walker/printer/backend surface area unchanged for positional callers.

### Added

- `ParamKind::Keyword` — a named keyword parameter variant.
- `Expr::KeywordArg { name, value, span }` — a call-site keyword argument;
  covered by `Expr::span()` and `Expr::kind_name()` (`"keyword-arg"`).
- `Feature::KeywordParams` (name string `"keyword-params"`) — observed when
  a `Keyword` param or a `KeywordArg` appears; wired into `Feature::ALL`,
  `name`/`from_name`, and `Display`.
- `Function::keyword_params(&self) -> Vec<&Param>` — the params with
  `kind == Keyword`.
- `Function::missing_keywords(&self, supplied: &[&str]) -> Vec<&Param>` —
  the keyword params a caller omitted. For a validator-accepted call every
  returned param carries a `default` (required keywords can never be
  omitted), so a backend may emit each default unconditionally.
- Validator rules:
  - **Def-side ordering** — the canonical param list is
    `Required* Rest? Keyword* KwRest?`. A `Keyword` before a positional/rest,
    or after the `KwRest`, is rejected (and a positional/rest after a
    `Keyword` symmetrically).
  - **Call-side ordering** — every `KeywordArg` must follow all positional
    args; a positional after a keyword is rejected.
  - **No duplicate keyword names** within one call's args.
  - **Known-callee name resolution** (DirectCall only) — each keyword must
    match a `Keyword` param OR the callee declares a `**kwrest`; every
    **required** keyword param must be supplied. IndirectCall/closure calls
    skip resolution (signature not statically known) but still enforce
    ordering + duplicates.
  - **KeywordArg only in call position** — a `KeywordArg` anywhere other
    than directly inside a call's `args` vec is rejected.
  - **Feature gating** — using a `Keyword` param or a `KeywordArg` requires
    the manifest to declare `Feature::KeywordParams` (same contract as
    `DefaultParams`). A keyword param's default triggers `KeywordParams`,
    not `DefaultParams` (that feature is specifically *positional* trailing
    defaults).
- Walker: `walk_expr_default` and the backend intrinsic walker recurse into
  `KeywordArg.value` (depth-bounded like every other child).
- Printer: a `Keyword` param renders `(x: any)` (required) /
  `(x: any (default (int 1)))` (optional); a `KeywordArg` renders
  `(keyword-arg name <value>)` inline in a call's arg list.
- Unit tests (28): ParamKind::Keyword required/optional distinction;
  `Expr::KeywordArg` span/kind-name; the two `Function` helpers; feature
  name/round-trip; def-side ordering (valid + each rejection); call-side
  ordering + duplicate rejection; known-callee name resolution (unknown
  keyword with/without kwrest, missing/optional/supplied required keyword);
  indirect-call skips resolution but keeps ordering; KeywordArg-out-of-call
  rejection (block value, builtin arg, nested in a keyword value); keyword
  value expression is validated; printer output; walker traversal.

### Deferred / scoped

- **Backends** do not yet accept `Feature::KeywordParams`; a keyword-using
  module is rejected at the capability check until per-backend emission
  lands (later milestones).
- **Frontends** do not yet lower to keyword params/args (later milestones).
- **IndirectCall / closure** keyword-name resolution stays deferred (the
  target signature is not known statically).

## 0.13.0 — Default-param call-arity semantics (P2a; behavior-neutral)

Defines the **call-arity rule for default parameters** so a `DirectCall`
to a known function may omit trailing arguments whose params carry
defaults. This is **PR 2a of the default-params sequence**: P1 added the
`Param.default` representation; this PR adds the *semantics layer* (query
API + validation) that the per-backend default-filling emission (follow-up
PRs) builds on. No frontend emits a defaulted call yet and no backend fills
a default yet, so **all existing behavior is unchanged** — every existing
valid module uses exact arity, which the new rule always accepts.

### Evaluation model (documented in SIR10)

Call-time, parameter-scope (Ruby/JS semantics): a default expression is
conceptually evaluated when the call runs, in the callee's parameter scope,
and may reference *earlier* params (validated as before). It is **not** a
caller-side or definition-time constant.

### Added

- `Function::required_param_count(&self) -> usize` — the call-arity floor:
  the length of the leading run of plain positional params that have no
  default. The first defaulted param (or a `*rest`/`**opts`/synthetic block
  param) ends the run.
- `Function::missing_defaults(&self, n_args: usize) -> &[Param]` — the
  trailing params at positions `n_args..len` that a caller omitted. For a
  call the validator accepted, every returned param carries a default, so a
  backend can emit each one's default unconditionally. Clamps on
  over-supply (never panics).
- Validator: `DirectCall` to a **known** function is now arity-checked. With
  R = `required_param_count()` and M = total param count, the call is valid
  iff `R <= args.len() <= M`. Omitting a trailing defaulted arg is OK;
  omitting a required arg (`args.len() < R`) or over-supplying
  (`args.len() > M`) is an error.
- Validator: **defaults must be trailing** — a no-default `Required` param
  may not follow a defaulted `Required` param (a "hole" like
  `def f(a = 1, b)`). This makes the `missing_defaults` guarantee true by
  construction: every param it returns carries a default, so a backend that
  unwraps `param.default` cannot panic. The synthetic `__sir_block__` param
  is exempt. Error message: "required parameter `b` may not follow a
  defaulted parameter (defaults must be trailing)".
- Unit tests (13): exact arity valid; omitting a trailing default valid;
  omitting all defaults valid; omitting a required arg errors; too many args
  errors; default-less callee keeps exact arity; variadic callee skips the
  check; block-passing call (trailing `MakeClosure`) skips the check; splat
  call skips the check; the `required_param_count` / `missing_defaults`
  helpers return the right params; a hole fails validation with the
  trailing-defaults message; trailing-defaults functions validate; a block
  param after a defaulted param is exempt.

### Deferred / scoped

- **IndirectCall** (calling a closure value) keeps its prior behavior — the
  target's params are not known statically, so default-arity resolution is
  deferred. Documented in SIR10.
- **Ruby's required-after-optional** form (`def f(a = 1, b)`, a "hole") is a
  deferred v0 limitation — rejected by the trailing-defaults rule above.
  Documented in SIR10.
- The strict arity check is **skipped** when the callee is variadic
  (`*rest`/`**opts`) or carries the synthetic trailing block param, or when
  any argument is not statically one positional value — a splat
  (`splat`/`double_splat`), argument forwarding (`forward_args`), block-pass
  (`block_pass`), or an implicit Ruby block handle (`MakeClosure`) appended
  to the arg list. These call-position lowerings (produced by the Ruby
  frontend) have no statically meaningful `args.len()`; checking them is
  deferred. This is what keeps the change behavior-neutral for the existing
  frontends.

### Changed

- `Cargo.toml`: minor version bump `0.2.0` → `0.3.0`.

## 0.12.0 — Param default values (core IR representation; behavior-neutral)

Adds the IR representation for **default parameter values** — the `1` in
Ruby `def f(a = 1)` and the Python / JavaScript equivalents. Previously a
parameter had no place to carry its default, so frontends silently dropped
it. This is **PR 1 of a sequence**: the IR can now *represent* a default,
the whole workspace still compiles, and **all existing behavior is
unchanged** (no frontend produces a default yet, and no backend emits one
yet). Backend emission and frontend lowering land in follow-up PRs.

### Added

- `Param.default: Option<Box<Expr>>` — `None` for an ordinary parameter;
  `Some(expr)` for `name = expr`. Boxed to keep `Param` a fixed size despite
  the recursive `Param → Expr → Function → Param` cycle (a default
  expression may contain a closure whose own params have defaults).
- `Feature::DefaultParams` (text name `default-params`) — observed by the
  validator whenever any param carries a default. **Not yet accepted by any
  backend**, so a default-using module is correctly rejected by the
  capability check until each backend gains support (intended).
- Validator: when a param has a default it observes `Feature::DefaultParams`
  and recursively validates the default `Expr` against the parameters
  declared *so far* (a default may reference an earlier param but not a
  later one).
- Walker: `walk_function_default` now visits each param's default
  expression before the body, so passes that walk the IR see them.
- Text printer: a param with a default renders an extra `(default <expr>)`
  clause — `(a any (default (int 1)))` — while defaultless params keep the
  original `(name type)` shape, so existing modules print unchanged.
- Unit tests: default validates + observes the feature, default-expr
  features are observed, a default may reference an earlier param (and may
  not reference a later one), the walker visits the default expr, and the
  printer renders the default clause.

### Changed

- `Param` now derives only `PartialEq` (not `Eq`): it holds an `Expr`, which
  contains an `f64` (`FloatLit`) and so cannot be `Eq` — consistent with
  `Expr` / `Block`.
- Every literal `Param { … }` construction across the SIR backends
  (typescript/rust/python/go/javascript) and the twig/ruby frontends now
  sets `default: None`. This is a mechanical addition; backends read params
  by field access, so the new field does not affect their behavior.

## 0.11.0 — SIR19: variadic parameter kinds (`Param.kind` / `ParamKind`) (M3)

Closes the def-side variadic limitation: previously a splat parameter
(`def f(*rest)` / `def g(**opts)`) lost its splat-ness at the SIR level and
lowered to an ordinary positional `Param`, so the emitted Python/TypeScript
declared a fixed positional parameter and a variadic call (`f(1, 2, 3)`) broke.

### Added

- `ParamKind` enum — `Required` (default), `Rest` (`*rest`), `KwRest`
  (`**opts`) — re-exported from the crate root.
- `Param.kind: ParamKind` — a new field on `Param`. Every in-tree construction
  sets it explicitly.
- Validator rules (`validate`): at most one `Rest` and at most one `KwRest`
  per parameter list, and ordering — required positionals precede the lone
  `Rest`, which precedes the lone `KwRest`. The reserved trailing block
  parameter `__sir_block__` (Q9e) is exempt (always `Required`, always last).
- Text printer renders `*name` / `**name` for the two variadic kinds so a
  round-tripped module preserves splat-ness.

### Changed

- Every literal `Param { … }` construction (smoke tests, printer tests) now
  sets `kind`. Backends read params by field access, so the added field does
  not affect their reads — only constructions.

## 0.10.0 — SIR18: string interpolation (`Expr::StrConcat`)

Introduced by the Ruby frontend's Phase 20b (`"a#{x}b"` interpolation).

### Added

- `Expr::StrConcat { parts, span }` — a first-class string-concatenation
  node that replaces the v0 `BuiltinCall("string_concat", parts)` marker
  (the same marker→node move `Stmt::TryCatch` made for
  `__rescue_marker__`).  A dedicated node lets backends emit native
  string building (`format!` / template literals / f-strings) instead of
  routing through a runtime helper.  Invariant: `parts.len() >= 2`.
- `Feature::StringInterpolation` — observed whenever a module contains a
  `StrConcat` node.  Distinct from `Feature::Strings` (a plain `StrLit`):
  a backend may support string literals without yet knowing how to build
  a concatenation, so the two capabilities are tracked separately.

### Validator

- `StrConcat` observes `Feature::StringInterpolation` and recursively
  checks every part.  A concat with fewer than two parts is a hard error
  (a frontend should emit the bare part instead).

### Text format

- `print_expr` renders `StrConcat` as `(str-concat <part…>)` (kind name
  `str-concat`).  Span and visitor (`walker`) coverage extended to the
  new node.

## 0.9.0 — SIR17: structured exception handling (`Stmt::TryCatch`)

Introduced by the Ruby frontend's Phase 16a (`begin/rescue/ensure/end`).

### Added

- `Stmt::TryCatch { body, rescues, ensure_body, span }` — a first-class
  exception-handling statement that replaces the earlier
  `__rescue_marker__` / `__ensure_marker__` inline `BuiltinCall`
  placeholders.  `body` and `ensure_body` are bare statement lists (like
  `ClassDef.body`); `ensure_body` is `Option`al.
- `RescueClause { exception_types, binding, body, span }` — one `rescue`
  clause.  `exception_types` is a list of advisory class names (empty =
  bare catch-all, not resolved by the validator); `binding` is the
  optional `=> e` exception variable, in scope as a `Scope::Local`
  within that clause's `body` only.
- `Feature::Exceptions` (kebab `exceptions`) — declared by any module
  containing a `TryCatch`.  Backends that don't accept it reject the
  module at the capability check before emit.

### Changed

- `Stmt::span()`, the walker, the validator (`check_stmt_seq`), the text
  printer (`(try-catch … (rescue (types …) (bind …) …) (ensure …))`),
  `backend::walk_intrinsics_in_stmt`, and all four reference backends'
  statement-emit match gain a `Stmt::TryCatch` arm.  The validator walks
  the body, each rescue body (with the binding introduced as a local),
  and the ensure body in fresh local-env scopes; the backend arms are
  unreachable `panic!`s (rejected pre-emit by the capability check).

New tests (4): `print_try_catch_with_rescue_and_ensure`,
`try_catch_validates_and_binding_is_in_scope`,
`try_catch_without_manifest_feature_is_error`,
`try_catch_binding_does_not_leak_past_rescue`.  Test count: 102 → 106.

This is a **breaking enum change** for any exhaustive `match` on `Stmt`
without a `_` rest arm.

## 0.8.0 — SIR17: constant scope (`Scope::Const`)

Introduced by the Ruby frontend's Phase 15c (`FOO` / `MyClass`).

### Added

- `Scope::Const` — a constant (Ruby `FOO`, `MyClass` — any
  uppercase-initial name).  Like `Scope::Instance` / `Scope::ClassVar`,
  it needs **no prior declaration**: `check_varref` performs no
  scope-existence check (a constant resolves against the constant scope,
  not a `let` binding).  `Scope::name()` / `from_name()` gain the
  `"const"` tag.
- `Feature::Constants` (kebab `constants`) — declared by any module that
  references a `Scope::Const`.  The validator observes it from each
  Const-scoped `VarRef`; backends that don't list it in their accepted
  set reject such modules at the capability check, before emit.

### Changed

- `check_varref` gains a `Scope::Const` arm (observe-only, no
  resolution).  The text printer renders `(var-ref FOO const)` via the
  existing `scope.name()` path.  The four reference backends'
  `emit_var_ref` gain an unreachable `panic!` arm for `Scope::Const`
  (rejected pre-emit by the capability check).

New tests (3): `print_var_ref_const_scope`,
`const_ref_needs_no_declaration`,
`const_ref_without_manifest_feature_is_error`.  Test count: 99 → 102.

This is a **breaking enum change** for any exhaustive `match` on
`Scope` without a `_` rest arm.

## 0.7.0 — SIR17: class-variable scope (`Scope::ClassVar`)

Introduced by the Ruby frontend's Phase 15b (`@@x`).

### Added

- `Scope::ClassVar` — a class variable (Ruby `@@x`).  Like
  `Scope::Instance`, a class var needs **no prior declaration**:
  `check_varref` performs no scope-existence check for it (reading an
  unset `@@x` yields nil in Ruby).  `Scope::name()` / `from_name()` gain
  the `"class-var"` tag.
- `Feature::ClassVars` (kebab `class-vars`) — declared by any module
  that references a `Scope::ClassVar` var.  The validator observes it
  from each ClassVar-scoped `VarRef`; backends that don't list it in
  their accepted set reject such modules at the capability check,
  before emit.

### Changed

- `check_varref` gains a `Scope::ClassVar` arm (no resolution; observes
  `Feature::ClassVars`).  The text printer renders
  `(var-ref @@x class-var)` via the existing `scope.name()` path.  The
  four reference backends' `emit_var_ref` gain an unreachable `panic!`
  arm for `Scope::ClassVar` (rejected pre-emit by the capability check).

New tests (3): `print_var_ref_class_var_scope`,
`class_var_ref_needs_no_declaration`,
`class_var_ref_without_manifest_feature_is_error`.  Test count:
96 → 99.

This is a **breaking enum change** for any exhaustive `match` on
`Scope` without a `_` rest arm.

## 0.6.0 — SIR17: instance-variable scope (`Scope::Instance`)

Introduced by the Ruby frontend's Phase 15a (`@x`).

### Added

- `Scope::Instance` — an object instance variable (Ruby `@x`).  Unlike
  `Scope::Local`, an instance var needs **no prior declaration**:
  `check_varref` performs no scope-existence check for it (reading an
  unset `@x` yields nil in Ruby).  `Scope::name()` / `from_name()` gain
  the `"instance"` tag.
- `Feature::InstanceVars` (kebab `instance-vars`) — declared by any
  module that references a `Scope::Instance` var.  The validator
  observes it from each Instance-scoped `VarRef`; backends that don't
  list it in their accepted set reject such modules at the capability
  check, before emit.

### Changed

- `check_varref` gains a `Scope::Instance` arm (no resolution; observes
  `Feature::InstanceVars`).  The text printer renders
  `(var-ref @x instance)` via the existing `scope.name()` path.  The
  four reference backends' `emit_var_ref` gain an unreachable `panic!`
  arm for `Scope::Instance` (rejected pre-emit by the capability check).

New tests (3): `print_var_ref_instance_scope`,
`instance_var_ref_needs_no_declaration`,
`instance_var_ref_without_manifest_feature_is_error`.  Test count:
93 → 96.

This is a **breaking enum change** for any exhaustive `match` on
`Scope` without a `_` rest arm.

## 0.5.0 — SIR17: singleton-class declarations (`Stmt::SingletonClassDef`)

Introduced by the Ruby frontend's Phase 14e (`class << self … end`).

### Added

- `Stmt::SingletonClassDef { target: String, body: Vec<Stmt>, span: Span }`
  — a singleton-class (metaclass) declaration.  `target` is the
  receiver whose singleton class is opened (`"self"` for the dominant
  `class << self` idiom, or a bare object name).  Like
  `ClassDef`/`ModuleDef`, method `def`s in the body are hoisted to
  top-level `Function`s by the Ruby lowerer; `body` carries the
  non-`def` statements.  Reuses `Feature::Classes` (a singleton class
  is a class-opening construct, not a new feature) — no manifest
  change.

### Changed

- `Stmt::span()`, the walker, the validator (marks `Feature::Classes`,
  walks the body via `check_stmt_seq` in a scoped env mark/rewind with
  the `MAX_IR_DEPTH` guard — same shape as `ClassDef`), the text
  printer (`(singleton-class-def << Target …)`), and the
  intrinsic-walk backend helper gain a `SingletonClassDef` arm.  The
  four reference backends gain an unreachable `panic!` arm
  (`Feature::Classes` absent from their accepted sets → rejected at the
  capability check before emit).

New tests (3): `print_singleton_class_def`,
`singleton_class_def_body_with_let_binding_validates`,
`singleton_class_def_body_undefined_varref_is_error`.  Test count:
90 → 93.

This is a **breaking enum change** for any exhaustive `match` on `Stmt`
without a `_` / `..` rest arm.

## 0.4.0 — SIR17: module declarations (`Stmt::ModuleDef`)

Introduced by the Ruby frontend's Phase 14d (`module M … end`).

### Added

- `Stmt::ModuleDef { name: String, body: Vec<Stmt>, span: Span }` — a
  module (namespace / mixin) declaration.  Structurally a `ClassDef`
  without inheritance: a named declaration whose `body` is a list of
  statements.  Like `ClassDef`, method `def`s inside the body are
  hoisted to top-level `Function`s by the Ruby lowerer; the `body`
  carries the module's non-`def` statements.
- `Feature::Modules` (kebab name `modules`) — declared by any module
  that contains a `Stmt::ModuleDef`.  Distinct from `Classes`: a Ruby
  `module` is a namespace/mixin, not an instantiable class.  Backends
  that do not list it in their accepted-feature set reject such modules
  at the capability check, before emit.

### Changed

- `Stmt::span()`, the walker, the validator (marks `Feature::Modules`,
  walks the body via `check_stmt_seq` in a scoped env mark/rewind with
  the `MAX_IR_DEPTH` guard — same shape as the `ClassDef` arm), the
  text printer (`(module-def Name …)` s-expression), and the
  intrinsic-walk backend helper all gain a `ModuleDef` arm.  The four
  reference backends (TypeScript, Rust, Python, Go) gain an unreachable
  `panic!` arm; `Feature::Modules` is absent from their accepted sets,
  so module-using modules are rejected at the capability check before
  emit.

New tests (5): `print_empty_module_def`, `print_module_def_with_body_stmt`,
`module_def_body_with_let_binding_validates`,
`module_def_body_undefined_varref_is_error`,
`module_def_without_manifest_feature_is_error`.  Test count: 85 → 90.

This is a **breaking enum change** for any exhaustive `match` on `Stmt`
that does not use a `_` / `..` rest arm.

## 0.3.0 — SIR17: class inheritance (`ClassDef.superclass`)

Introduced by the Ruby frontend's Phase 14c (`class Foo < Bar`).

### Added

- `Stmt::ClassDef` gains a `superclass: Option<String>` field — the
  parent class name (`Some("Bar")` for `class Foo < Bar`, `None` for a
  base class `class Foo`).  It is an advisory name only: SIR v0 has no
  class symbol table, so the validator does not resolve it (mirroring
  how the class's own `name` is not bound as a local).

### Changed

- The text printer emits a `(< Super)` clause right after the class
  name when `superclass` is set: `(class-def Foo (< Bar))`.  Base
  classes are unchanged (`(class-def Foo)`).
- The walker, validator, intrinsic-walk backend helper, and the four
  reference backends' `ClassDef` arms are unaffected by the new field
  (it carries no sub-expressions to traverse and no capability impact);
  class-using modules are still rejected at the capability check before
  emit.

New test: `print_class_def_with_superclass`.  This is a **breaking
struct change** for any code constructing `Stmt::ClassDef` literally —
all in-tree constructors updated to pass `superclass`.

## 0.2.1 — SIR17 validator: walk populated `ClassDef` bodies

No node-shape change.  The Ruby frontend's Phase 14b begins emitting
`Stmt::ClassDef` nodes with a *populated* `body` (Phase 14a always
emitted an empty body), so the validator now actually walks it.

### Changed

- Factored the statement-sequence walk out of `check_block` into a
  new private `check_stmt_seq(&[Stmt], env, depth)` helper.
  `check_block` now calls it for `block.stmts` (then checks the
  trailing `block.value`), preserving the exact prior behaviour —
  parallel-`let` grouping, sequential `let*`, mutable `Assign`,
  loop/scope handling.
- `Stmt::ClassDef`'s validator arm now calls `check_stmt_seq` on the
  body inside a fresh `env.mark()`/`env.rewind()` scope (Phase 14a
  left this loop a documented no-op).  Class-body locals therefore
  do **not** leak into the surrounding statement stream, and a
  bad reference inside a class body is now reported instead of
  silently accepted.  An explicit `MAX_IR_DEPTH` guard bounds
  recursion for pathologically nested `class … class …` bodies.

New tests (3): `class_def_body_with_let_binding_validates`,
`class_def_body_undefined_varref_is_error` (proves the body is
walked, not no-op'd), `class_def_body_local_does_not_leak_to_sibling`.
Test count: 81 → 84 (+3).

## 0.2.0 — SIR17: class declarations

Adds the first object-oriented IR node, introduced by the Ruby
frontend's Phase 14a (empty `class Foo; end`).

### Added

- `Stmt::ClassDef { name: String, body: Vec<Stmt>, span: Span }` — a
  class declaration whose body is a list of statements.  The Ruby
  frontend's Phase 14a lands the *empty-body* case (`body: vec![]`);
  the variant is shaped to carry a populated body in later phases.
  `body` is a `Vec<Stmt>` rather than a `Block` because a class body
  is a declaration, not a value-producing expression.
- `Feature::Classes` (kebab name `classes`) — declared by any module
  that contains a `Stmt::ClassDef`.  Backends that do not list it in
  their accepted-feature set reject such modules at the capability
  check, before emit.

### Changed

- `Stmt::span()`, the validator, the text printer (`(class-def
  Name ...)` s-expression), the walker, and the intrinsic-walk
  backend helper all gained a `ClassDef` arm.  The four reference
  backends (TypeScript, Rust, Python, Go) reject class-using modules
  via their unchanged capability declarations, so their emit paths
  treat the new arm as unreachable.

## 0.1.0 — initial release (SIR10 v0)

First cut of the narrow-waist Semantic IR.  Implements the v0
surface defined in
[SIR10-narrow-waist-semantic-ir.md](../../../specs/SIR10-narrow-waist-semantic-ir.md).

### Added

- `Module`, `Function`, `Block`, `Stmt`, `Expr`, `Scope`, and all
  supporting node types per SIR10 §"Module structure" through
  §"Expressions".
- `SirType` carrier — `Any`, `Int`, `Bool`, `Nil`, `Symbol`, `Str`,
  `Pair`, `Closure`, parametric `Fn`.
- `EffectSet` bitset with effect tags `MayThrow`, `MayPrint`,
  `MayAllocate`, `MayBlock`, `Divergent`; pure is the empty set.
- `FeatureManifest` with the v0 feature list (`Closures`, `Pairs`,
  `Symbols`, `Strings`, `DynamicTyping`,
  `OptionalTypeAnnotations`, `MutualRecursion`, `TailCalls`,
  `Globals`, `Intrinsics`).
- `Metadata` carrier with source-language/version and SIR-version
  fields; advisory only — IR correctness must not depend on it.
- `Span` source-position carrier (1-indexed line and column).
- `validate(module)` — structural and semantic checks:
  - Manifest covers every feature actually used
  - No `VarRef` references an undefined name in its scope
  - `Intrinsic.targets` is non-empty
  - Function / global name uniqueness
  - Parallel `let` does not leak bindings into sibling RHS
  - Sequential `let*` allows prior bindings on subsequent RHS
- `Visitor` trait + free `walk_*_default` functions for read-only
  traversal.
- Canonical S-expression text printer (`print_module`, `print_expr`,
  `print_block`, `print_function`).  Output is deterministic and
  byte-stable.
- `Backend` trait and `BackendRegistry` with built-in capability
  enforcement (manifest features + intrinsic whitelist + target-tag
  matching).
- Test coverage for every public surface, including parallel/let*
  semantics distinguishing tests.

### Deferred to future versions

- Text-format parser (round-trip currently relies on printer
  determinism).
- Ownership / borrow markers (Move / Copy / Borrow).
- Async / await / coroutines.
- Exception handling (Raise / Try / Catch).
- Pattern matching (Match) and record / union / type-alias forms.
- Effect inference (manual annotation only in v0).
- Sequence / Map / Set / Option / Result primitives.
