# SIR29 — Nominal/static-dispatch OOP profile (Java-family frontends)

## Status

New. Spec + pure-additive `semantic-ir` crate diff (this PR). No backend or
frontend behavior changes: every existing backend continues to reject
modules using SIR29 nodes via the existing SIR10 capability-check
mechanism, exactly as SIR22/SIR23's own introduction PRs did. Real
producer (`java-to-semantic-ir`) and consumer (`semantic-ir-to-java`)
implementations are follow-up work, tracked in the Java Phase B roadmap
this spec unblocks.

## Motivation

[`SIR25`](SIR25-language-agnostic-object-model.md) §2 specifies SIR's one
existing OOP profile: single inheritance, duck-typed dispatch through an
explicit runtime string-keyed method table, methods hoisted to detached
top-level functions with no receiver at lowering time. That design is a
deliberate, well-reasoned choice for a *dynamic*-OOP profile — three of
SIR's seven backends (Python, JS/TS, Ruby) are themselves dynamically
typed and would implement approximately this model regardless, and even
`semantic-ir-to-ruby` (whose host language has real native classes) does
not map onto them for dispatch, precisely because the profile is designed
around string-table lookup rather than native override resolution.

SIR25 §6 explicitly reserves the fork this spec closes: *"a second,
structurally different OOP profile (nominally-typed, interface/
generic-based, static-dispatch — e.g. what a Java/C# frontend would
need)... it should be a new `Feature` set and dispatch primitive alongside
[§2], the same relationship SIR22/23 have to the base leaf features, not a
replacement or a parameterization of [§2]'s dispatch algorithm."* This
spec is that fork, designed ahead of a Java frontend/backend pair (and
reused, without redesign, by planned later C# and Kotlin frontend/backend
pairs) so all three land on one shared static-dispatch primitive rather
than each re-deriving one independently.

**Why not just extend §2?** A nominally-typed, single-vtable-slot,
compile-time-resolved-override model is a *different algorithm* from
§2.2's runtime ancestry walk, not a parameterization of it — Java's
`instance.method()` resolves to exactly one already-known vtable slot at
the call site; Ruby's `obj.method` walks an ancestry chain (including
mixins) at the call site every time. Bolting a `dispatch_kind` flag onto
`BuiltinCall("__method__", ...)` would force every consumer of that
primitive (5 of 7 backends today) to branch on a mode they don't use,
where a wholly separate primitive lets a backend that doesn't support this
profile simply not implement it — the same reasoning that kept SIR22/23
as sibling extensions rather than folding matrix/symbolic operators into
the base arithmetic `Expr` variants.

## Scope

**In scope:**

- Nominally-typed classes with single inheritance and interface
  conformance (`extends`/`implements`)
- Interfaces as a distinct declaration (method signatures, no
  implementation, multiple-inheritance-of-interface via `extends`)
- Abstract classes (`is_abstract` flag on a class declaration — a class
  that may contain methods with no body-producing implementation)
- Index-based virtual dispatch (`Expr::VirtualCall`) — the profile's one
  new dispatch primitive
- Static (class-level, no-receiver) methods — representable without any
  new node, via ordinary `Expr::DirectCall` against a mangled top-level
  identity
- Erased generic type parameters on classes, interfaces, and (implicitly,
  via the params they use) methods

**Explicitly out of scope (deferred):**

- **Reified generics.** `SirType::TypeParam` carries no runtime
  representation (no `sizeof`, no runtime type token). A future addition
  for a reified-generics host (a later C# extension) should add a sibling
  `SirType` variant or a `reified: bool` flag rather than repurposing this
  one — deliberately left open, not designed now.
- **Default interface methods.** `Stmt::InterfaceDef.methods` is
  signatures only (`MethodSig`, no body) for v1. An addendum that adds a
  body-bearing interface method should follow the same "additive addendum
  extends the same node family" shape SIR22's own APL addendum used for
  `Reduce`/`Scan`/etc.
- **Sealed/final class modifiers.** Method-level `is_override`/
  `is_static` are in scope (SIR29 v1); a class-level sealing/finality flag
  is not.
- **Records/data classes.** No auto-generated equals/hashCode/toString
  vocabulary — a frontend lowering a Java `record` would need to lower it
  to an ordinary `NominalClassDef` plus explicit generated methods, or
  wait for a future addendum.
- **Nested/inner classes.** `NominalClassDef` does not nest inside another
  `NominalClassDef`'s `body` today (nothing prevents it structurally, but
  no lowering/validation contract for the enclosing-instance-capture
  semantics inner classes need exists yet).
- **Annotations.** No IR representation for `@Override`/`@Deprecated`/
  custom annotations; `is_override` covers the one piece of `@Override`'s
  *meaning* the profile needs (a backend may still choose to *emit*
  `@Override` when `is_override` is true — that's an emission choice, not
  an IR requirement).
- **A checked-vs-unchecked exception distinction.** SIR29 reuses
  `Stmt::TryCatch` exactly as SIR17 already defined it. SIR does not need
  to enforce Java's checked-exception rule at the IR level — like overload
  resolution (below), that is a frontend-only, already-resolved-before-
  lowering concern.
- **Overload resolution.** The IR never represents an overload *set* —
  only the one already-chosen call target. A frontend lowering `f(int)`
  and `f(String)` as two overloads has already picked, at each call site,
  which `MethodDef`/mangled identity is being invoked before it emits an
  `Expr::Call`/`Expr::VirtualCall` at all.

## New `Feature` flags

```text
Feature::NominalClasses    -- a module contains a Stmt::NominalClassDef
Feature::Interfaces        -- a Stmt::InterfaceDef, or a NominalClassDef
                               with a non-empty `interfaces` list
Feature::VirtualDispatch   -- a module contains an Expr::VirtualCall
Feature::ErasedGenerics    -- a module uses a SirType::TypeParam anywhere
```

`Interfaces` is split out from `NominalClasses` the same way SIR22 split
`MatrixOps` from `NDArrays`: a backend could in principle support nominal
classes without interface-conformance checks. `VirtualDispatch` is its own
flag (not folded into `NominalClasses`) because a class hierarchy with no
overridden methods never needs the dispatch primitive at all — a
`NominalClassDef` with only static methods and non-virtual instance calls
declares `NominalClasses` alone.

A backend that doesn't declare the relevant flag(s) in `accepts_features()`
cleanly rejects any module using these nodes — SIR10's existing
capability-check mechanism, no new mechanism needed.

## New `SirType` variants

```text
SirType::Nominal { name: String }
    -- an advisory, unresolved reference to a declared class/interface —
       same discipline as Stmt::ClassDef.superclass under the §2 profile:
       SIR v0 has no class/interface symbol table, so the validator does
       not resolve `name` against a declared NominalClassDef/InterfaceDef.

SirType::TypeParam { name: String, bound: Option<Box<SirType>> }
    -- an ERASED generic type parameter (Java/C#/TypeScript `<T extends
       Bound>`). Carries no runtime representation by design — see
       "Explicitly out of scope: reified generics" above. `bound` is
       `None` for an unbounded parameter (`<T>`), `Some(t)` for a bounded
       one; like `Nominal.name`, a bound is advisory and unchecked by the
       validator.
```

## New `Stmt` variants

All new variants carry `span` like every existing node.

```text
NominalClassDef {
    name:        String,
    type_params: Vec<SirType>,       -- SirType::TypeParam entries
    superclass:  Option<String>,     -- advisory, unresolved (like ClassDef)
    interfaces:  Vec<String>,        -- advisory, unresolved
    is_abstract: bool,
    fields:      Vec<FieldDef>,
    body:        Vec<Stmt>,          -- methods NEST here (see below)
    span,
}

FieldDef { name: String, sir_type: SirType, span }
    -- a declared field. Unlike §2's Scope::Instance (any name legal at
       first write, no declaration), a nominal class declares its field
       surface up front, matching a statically-typed source language.

InterfaceDef {
    name:        String,
    type_params: Vec<SirType>,
    extends:     Vec<String>,        -- an interface may extend SEVERAL
                                         (unlike NominalClassDef.superclass,
                                         which is single)
    methods:     Vec<MethodSig>,     -- signatures only, no body (v1)
    span,
}

MethodSig { name: String, params: Vec<SirType>, ret: SirType }
    -- a bodyless method contract, used only by InterfaceDef.

MethodDef {
    name:         String,
    params:       Vec<Param>,        -- same Param shape as top-level Function
    return_type:  Option<SirType>,
    is_static:    bool,
    is_override:  bool,
    vtable_slot:  Option<u32>,       -- see "Dispatch primitive" below
    body:         Block,
    span,
}
```

**Methods nest directly in `NominalClassDef.body`.** This is a deliberate
departure from `Stmt::ClassDef`'s convention (SIR25 §2, Phase 14a/14b):
that convention hoists every `def` to a detached top-level `Function`
with no receiver, existing *only* to support §2's string-table dispatch —
a frontend-side transformation that has nothing to do with the profile's
actual meaning. Since SIR29 uses a structurally different dispatch
primitive that never looks a method up by name at runtime, there is no
reason to detach methods from their owning class, and doing so would only
make a `VirtualCall`'s `slot` harder to reason about at the declaration
site. `body` may also contain other `Stmt`s (a field initializer sequence,
a static initializer block) alongside `MethodDef`s, exactly as
`ClassDef.body` already permits non-`def` statements alongside hoisted
methods.

**Static methods need no new node.** `is_static: bool` on `MethodDef`
marks a class-level method with no receiver; a call to one compiles to an
ordinary `Expr::DirectCall` against a mangled top-level identity (e.g.
`ClassName$$methodName`) — the same representation an ordinary top-level
function call already uses. This mirrors how SIR22's `IndexSet` needed a
new `Stmt` (a genuinely new mutation shape) while most of that spec's
nodes were pure `Expr` reuse of an existing shape: a static call is not a
new *kind* of call, only a new *naming convention* for an existing one, so
introducing a dedicated node for it would duplicate `DirectCall` for no
semantic gain.

## New `Expr` variant: the dispatch primitive

```text
VirtualCall {
    receiver: Box<Expr>,
    method:   String,     -- DISPLAY/DEBUG ONLY. Never a codegen dispatch
                              key — see the anti-injection note below.
    slot:     u32,
    args:     Vec<Expr>,
    effects:  EffectSet,
    span,
}
```

`slot` is a **frontend-assigned, per-class-hierarchy-stable vtable
index** — an overriding method reuses its parent's slot number, so the
same virtual call site compiles correctly regardless of which concrete
subclass `receiver` turns out to be at runtime. This is the *index-based*
sibling of SIR25 §2.2's *string-based* table lookup
(`BuiltinCall("__method__", [recv, StrLit(method_name), ...])`): both
primitives resolve "the right method for this receiver," but SIR29's
resolves it through a position a frontend has already computed, while
§2's resolves it through a runtime ancestry walk keyed by name.

**Anti-injection invariant (mirrors SIR25 §2.5).** `method` rides along
for debug/display purposes only — a backend's generated comment, or a
text-format pretty-printer. It is **never** a codegen dispatch key. Unlike
§2.2, which achieves its "never reflection on a source-derived string"
invariant through an explicit runtime table a backend must route every
lookup through, SIR29 achieves the same invariant **by construction**:
there is no string anywhere in this node a backend *could* route through
a generic reflection facility even if it wanted to, since `slot` — not
`method` — is the only field a correct backend implementation consults
for dispatch.

**Static calls need no new node** (see above) — this node exists only for
the instance/virtual case. **Overload resolution is entirely a frontend
concern**, already resolved before lowering (see "Explicitly out of
scope" above): the IR never represents an overload *set*, only the one
already-chosen call target, so `args` here is exactly the resolved
argument list for that one target.

## Feasibility sketch

Confirmed at both ends of this repo's host-language spectrum before any
backend implementation started, to validate the primitive is realizable
rather than merely designed on paper:

- **TypeScript** (closest existing precedent — real `class`/`extends`/
  `implements`, erased generics already): `NominalClassDef` maps directly
  onto a native `class ... extends ... implements ...`. `VirtualCall`
  **discards `slot` entirely** and emits an ordinary
  `receiver.method(args)` — TS's own prototype-chain override resolution
  already *is* a vtable, so the index carried on the IR node is redundant
  information for this backend (present because other backends need it,
  not because every backend must consult it).
- **C** (zero native OOP — the hardest target in this repo): compiles
  `NominalClassDef` to a plain `struct C { ...fields; struct C_vtable*
  vtable; }` plus one `struct C_vtable` of function pointers per concrete
  class, built from consecutive slot numbers and wired in by the
  constructor. `VirtualCall` becomes
  `receiver->vtable->slot_N(receiver, args...)` — a direct, mechanical
  translation of `slot` into a function-pointer-table index. Confirming
  this end of the spectrum is realizable, with zero native language
  support to lean on, is the strongest evidence the primitive is
  well-formed rather than accidentally JVM-shaped.

Both sketches are backend-implementation follow-up work (not part of this
PR) — recorded here so the Java Phase B backend slice starts from a
confirmed design rather than re-deriving feasibility from scratch.

## Effects

`Expr::VirtualCall` carries an explicit `effects: EffectSet` field, the
same shape as `DirectCall`/`IndirectCall`/`BuiltinCall` — a virtual call
may do anything the invoked method's body does, so (unlike SIR22/23's
uniformly-`Pure` new nodes) this profile's one new `Expr` variant is
call-shaped and effect-bearing by design, not `Pure`. `Stmt::MethodDef`
carries its own `body: Block`, whose statements determine the method's
actual effects the same way any `Function.body` does — SIR29 adds no new
`Effect` tag, reusing the five already defined in `effects.rs`.

## Validator behavior

- `Stmt::NominalClassDef` observes `Feature::NominalClasses` (and
  `Feature::Interfaces`/`Feature::ErasedGenerics` when `interfaces`/
  `type_params` are non-empty, or a field's type is a `TypeParam`),
  depth-guards, and recurses into `body` in a fresh local-env scope —
  the same discipline `Stmt::ClassDef` already applies, since (unlike
  `ClassDef`) methods nest here directly rather than being hoisted.
- `Stmt::InterfaceDef` observes `Feature::Interfaces` (and
  `ErasedGenerics` when applicable) and rejects a duplicate method-
  signature name; it has no nested `Stmt`/`Expr` to recurse into.
- `Stmt::MethodDef` is validated like a top-level `Function` — its own
  fresh parameter scope (not the enclosing class body's `env`, since a
  method's parameters and locals are not visible to sibling statements or
  sibling methods), duplicate-parameter detection, and `OptionalType
  Annotations`/`DynamicTyping`/`DefaultParams`/`KeywordParams` observation
  matching `check_function`'s existing rules. `vtable_slot: Some(_)`
  observes `Feature::VirtualDispatch`.
- `Expr::VirtualCall` observes `Feature::VirtualDispatch` and recurses
  into `receiver` and every element of `args`.

## Backend impact

This PR (Slice 0 of the Java Phase B roadmap) is pure-additive: no
backend declares any of the four new `Feature` flags in
`ACCEPTED_FEATURES`, so every module using a SIR29 node continues to be
rejected at the existing SIR10 capability-check boundary, before any
backend-specific code path is reached. `cargo build --workspace`
correctness for this PR extends to every crate that matches
`semantic-ir::nodes::{Stmt, Expr}` exhaustively (five backends —
`semantic-ir-to-{go,javascript,python,rust,typescript}` — plus one whose
`Stmt` match is exhaustive, `semantic-ir-to-ruby` — and three frontends —
`{ruby,javascript,python}-to-semantic-ir`, whose own internal generic
tree-walking utilities must stay exhaustive even though none of them
produce these nodes yet); each gained the minimal compile-compat arm
needed to keep the workspace green, following the exact precedent SIR22/
SIR23 set for their own equivalent rollouts. `semantic-ir-to-c` and
`c-to-semantic-ir` required no changes (no exhaustive `Stmt`/`Expr` match
in either crate).

Real producer/consumer implementations are follow-up work:

- **`java-to-semantic-ir`** (not yet started) — the first planned SIR29
  producer. Lowers Java `class`/`interface`/`extends`/`implements` source
  directly onto `NominalClassDef`/`InterfaceDef`/`MethodDef`, and an
  overridden-method call site onto `VirtualCall` with a frontend-computed
  `slot`.
- **`semantic-ir-to-java`** (not yet started) — the first planned SIR29
  consumer. `NominalClassDef`/`InterfaceDef`/`MethodDef` map onto native
  `class`/`interface`/method declarations; `VirtualCall` discards `slot`
  and emits an ordinary `receiver.method(args)` call, since `javac`'s own
  vtable does the real dispatch work (mirrors the TypeScript feasibility
  sketch above).
- Later C# and Kotlin frontend/backend pairs are expected to reuse this
  same profile without redesigning it — the explicit payoff of designing
  SIR29 as its own spec now rather than deferring it to each language's
  own arc.

## Versioning

This is an additive extension within the SIR line (same discipline as
SIR22/SIR23/the SIR22 APL addendum before it) — no existing module or
backend match arm needs to change; backends simply gain new arms or
explicitly decline the new features. `metadata::CURRENT_SIR_VERSION` bumps
`"4"` → `"5"`. A frontend lowering a nominally-typed static-dispatch
source language (Java, and later C#/Kotlin) sets `metadata.sir_version` to
`"5"` when its module uses any SIR29 node.

## References

Internal: [`SIR10`](SIR10-narrow-waist-semantic-ir.md) (capability-check
mechanism this spec relies on unchanged), [`SIR25`](SIR25-language-agnostic-object-model.md)
§2/§6 (the sibling dynamic-OOP profile, and the explicit fork this spec
closes), [`SIR17`](SIR17-object-oriented-frontends.md) (`Stmt::TryCatch`,
reused as-is), [`SIR22`](SIR22-array-matrix-semantic-ir.md) /
[`SIR23`](SIR23-symbolic-pattern-semantic-ir.md) (the "additive sibling
extension, additive addendum, split-feature-flag" conventions this spec
follows throughout).
