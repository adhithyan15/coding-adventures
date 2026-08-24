# SIR25 — SIR's object model is SIR's own, not "whatever Ruby does"

## Status

New. Spec-only PR (specs-first). No code change. This spec does not alter any
existing behavior — every backend and frontend continues to do exactly what it
does today. What changes is the *authority* a reader points to when asking "is
this correct?": today the honest answer for the OOP/dispatch/collection-method
surface is "run it in a real Ruby interpreter and compare" (see
[`sir-conformance`](../packages/rust/sir-conformance/), whose `Program.ruby` /
`.expected` fields encode exactly that). After this spec, the answer is "this
document, §2–§5" — a real Ruby interpreter is no longer the oracle, even though
today it happens to agree with the oracle on every case that's tested.

## Motivation

SIR's frontend side already proves the narrow-waist promise: 19 frontends
(`ruby-, python-, javascript-, c-, twig-, apl-, axiom-, derive-, idl-, j-,
macsyma-, maple-, matlab-, maxima-, octave-, q-, reduce-, scilab-,
wolfram-to-semantic-ir`) lower into one `semantic_ir::Module`. For the SIR22
(array/matrix) and SIR23 (symbolic/pattern) domains, this is fully realized:
those features are specified in their own docs, oracle-tested against multiple
independent reference runtimes (Wolfram, Macsyma, MATLAB, Octave — see those
specs' §Verification), and reachable from most of the 19 frontends above.

The general-purpose object/dispatch/collection-method surface — `Feature::
Classes/InstanceVars/ClassVars/Constants/Exceptions/Modules` and the
`BuiltinCall("__method__"/"__new__"/"__def_method__"/"__super__"/"__self__",
...)` family — did not get the same treatment. It was built by porting Ruby's
object model and Ruby's stdlib method catalog one slice at a time
(`sir-classes-oop.md`, `sir-method-dispatch.md`, `sir-collection-methods.md`,
`sir-mixins.md`, `sir-exception-hierarchy.md` — each explicitly framed as
"toward the north star: any Ruby script → semantically-correct X output"), and
the one test harness that exercises it end-to-end
([`sir-conformance`](../packages/rust/sir-conformance/)) hardcodes a Ruby
source string as its only input and a real Ruby interpreter's output as its
only reference answer. Backend implementations followed suit: doc-comments in
`semantic-ir-to-go` cite "Ruby's MRO", `semantic-ir-to-c`/`-rust` cite "matching
Ruby" / "true Ruby" / "Ruby's `ensure`" as the correctness bar.

None of this means the *design* is wrong. A duck-typed, string-keyed dispatch
model with MRO-based mixin resolution is a reasonable, well-understood choice
for a first dynamic-OOP profile — arguably the natural one, since 3 of SIR's 7
backends (Python, JS/TS, Ruby) are themselves dynamically typed and would
implement approximately this model regardless. The problem is purely one of
*authority*: "matches Ruby" is not a specification, it's a pointer to an
external, unversioned, unspecified oracle that happens to be convenient because
one frontend for it already exists in this repo. A future frontend or backend
author has no way to know which parts of "what Ruby does" are load-bearing SIR
semantics and which are Ruby implementation trivia irrelevant to SIR.

## What this spec is and isn't

**Is:** the authoritative statement that SIR has exactly one dynamic-OOP
profile today (§2), that its dispatch algorithm and `Feature` surface are
defined in this document and the per-feature specs it supersedes-in-authority
(§3), and that any frontend or backend implementing that surface is measured
against this document, not against a Ruby interpreter.

**Isn't:** a semantics *change*. Every rule below is chosen to match exactly
what `ruby-to-semantic-ir` + the mature backends already implement, because
that implementation is the only one that exists and there's no reason to
diverge from working, tested code just to prove independence. "Independent of
Ruby" means *specified* independently and *permitted* to diverge with a
documented reason — not different today.

**Isn't (yet):** a second OOP profile for a structurally different family
(nominally-typed, interface-based, static-dispatch languages like Java/C#).
Adding one is a real, separate arc — it would likely need a new `Feature` set
and a different dispatch primitive alongside (not replacing) the one below,
the same way SIR22/23 sit alongside the base leaf features rather than
replacing them. Out of scope here; noted in §6 as the next fork if pursued.

## §2 — The dynamic-OOP profile

SIR defines one object model, available via `Feature::Classes` and its
dependents:

- **Single inheritance.** `ClassDef{name: String, superclass: Option<String>,
  body: Vec<Stmt>}` — one optional superclass, no interfaces, no traits/mixins
  in the class header itself (mixins are a separate runtime operation, §2.4).
- **Instance state** (`Feature::InstanceVars`): named, per-instance, untyped
  slots (`Scope::Instance`), read/written by a `(receiver, name) -> value`
  table walk, not by struct-offset layout. No declared field list — any name
  is legal at first write.
- **Class state** (`Feature::ClassVars`): named slots owned by a class,
  visible to that class and its subclasses through the same instance walking
  the ancestry chain that method dispatch uses (§2.2) — a subclass reads and
  writes its ancestor's class variable, it does not get its own copy.
- **Module namespacing / constants** (`Feature::Constants`, `Feature::
  Modules`): a flat, string-keyed global constant table; a `ModuleDef` is a
  named, non-instantiable namespace that participates in method resolution
  (§2.4) but not in the class hierarchy.
- **Exceptions** (`Feature::Exceptions`): a class hierarchy rooted at a
  built-in base exception class; `raise`/`rescue`/`ensure` map to `Throw`/
  `TryCatch{handlers, ensure}`; a rescued value is a first-class object with
  `.class`/`.message`/`.is_a?` — never a plain string.

### §2.1 — Object construction

`BuiltinCall("__new__", [StrLit(class_name), ...args])`. Allocates a new
instance tagged with `class_name`, binds it as the current receiver, invokes
an `initialize` method if one is registered anywhere in the class's ancestry
(§2.2), unbinds, returns the instance. A class with no `initialize` simply
skips that step — construction never fails for that reason alone.

### §2.2 — Method dispatch

`BuiltinCall("__method__", [recv, StrLit(method_name), ...args])`. Resolution
order, **explicit table lookup only — never reflection on a source-derived
string** (this is a security invariant, not a style preference: dispatch must
never let an IR-carried string reach a general-purpose "call anything by this
name" primitive in the target language):

1. A method registered for `class_of(recv)` directly.
2. Walking `class_of(recv)`'s ancestry (superclass chain), most-derived first,
   consulting each ancestor's directly-registered methods, and at each step
   also consulting that ancestor's included modules (most-recently-included
   first) before moving to the next ancestor. This is the same resolution
   order Ruby's method resolution order (MRO) uses; SIR adopts it as its own
   defined order, not as a reference to Ruby's implementation.
3. The built-in method catalog for `recv`'s primitive kind (`sir-method-
   dispatch.md`, `sir-collection-methods.md` — Array/Hash/String/Numeric/
   Symbol/Object tables), consulted only after every user-defined possibility
   above is exhausted.
4. Unknown → a raised `NoMethodError`-family exception (not a silent `nil`,
   except where an individual built-in method is documented to return `nil`
   as its own defined behavior, e.g. `Hash#[]` on a missing key).

Method registration (`__def_method__("Class", "name", closure)`) and class-method
registration (`__def_class_method__`) populate the tables this walk consults;
they are themselves `BuiltinCall`s emitted by a frontend, not IR structure.

### §2.3 — `self` and `super`

`BuiltinCall("__self__", [])` reads the current receiver from a single
dispatch-scoped binding (pushed before a method body runs, popped after —
correct for the single-threaded transpiled programs SIR targets; true
per-object/per-thread receiver binding is out of scope, same as documented in
`sir-classes-oop.md`). `BuiltinCall("__super__", [StrLit(method_name),
StrLit(defining_class), ...args])` resolves from the superclass of
`defining_class` (not of `class_of(recv)` — this is what makes a 3-level
inheritance chain call the *next* ancestor's override rather than re-entering
the same one) and dispatches with the *same* current receiver, unchanged.

### §2.4 — Mixins

`include`/`extend` register a module against a class in a separate table
(`sir-mixins.md`); §2.2's ancestry walk consults a class's included modules at
each ancestor step, most-recently-included first. `extend` does the same for
class-method resolution. This is additive to §2.2, not a separate algorithm.

### §2.5 — Anti-injection invariant

Every string that reaches a dispatch table lookup, a class/method/ivar name,
or a generated identifier is validated at its emission site (constant-path
syntax, identifier syntax, or symbol-quoting depending on target language) and
routed through an explicit table — never through the target language's own
generic reflection/eval facility (Ruby's `public_send`, a hypothetical Python
`getattr`-by-string, etc. — where used, e.g. `sir-um_`-prefixed dispatch in
`semantic-ir-to-ruby`, the prefix is chosen so no built-in of the target
language can collide with it). This holds regardless of source or target
language and is non-negotiable for any new backend or frontend implementing
this surface.

## §3 — Built-in method catalog

The specific method names, arities, and per-method semantics (what `gsub`,
`each_slice`, `<<`, floor-vs-truncating integer division, etc. each do) are
defined in `sir-method-dispatch.md` and `sir-collection-methods.md` — this
spec does not re-list them. Those documents remain the detailed reference;
read this spec's §2 for the dispatch *mechanism* they plug into, and treat
their remaining "Ruby" framing as *naming provenance* ("this catalog happens
to be Ruby's stdlib surface, chosen because it's well-understood and one
frontend already emits it") rather than as the source of correctness. A method
whose behavior is genuinely Ruby-idiosyncratic and not worth generalizing may
stay exactly as Ruby defines it — that's a legitimate per-method choice, not a
structural coupling, as long as it's a deliberate choice recorded in the
method's own doc-comment rather than an unstated assumption.

## §4 — Numeric semantics: parameterized, one default

SIR's numeric type (`semantic-ir/src/types.rs`, `IntSpec{width, signed,
overflow}`, `Overflow::{Wrap,Trap,Saturate,Checked,Undefined,Arbitrary}`) is
already a language-agnostic lattice, not a Ruby-specific one — see
`SIR21-type-system-and-integer-semantics.md`. Its *default*, when a frontend
supplies no static type (`IntSpec::arbitrary()`, unbounded, floor-dividing), is
chosen because it's the faithful behavior for SIR's most dynamically-typed
frontends (Ruby, Python, JS) and is a safe default for any frontend that
hasn't been taught to supply better information — not because SIR's numeric
model *is* Ruby's. A frontend for a statically-typed source language should
supply `IntSpec::sized(width, signed, overflow)` and get native-width lowering
once `op_select::resolve_binary`/`resolve_numeric` are consulted by every
backend (SIR21 T3c-3 — tracked, not yet wired as of this spec; see that spec's
milestone table for current status). This spec does not change the numeric
model; it clarifies that the *default* is a default, not the definition.

## §5 — Conformance discipline

Per `SIR22`/`SIR23`'s already-established pattern (oracle-testing against
Wolfram *and* Macsyma *and* MATLAB *and* Octave — multiple independent
references, not one privileged interpreter), the OOP/Collections surface's
conformance suite (`sir-conformance`) must accept a program from **any**
registered frontend, not only Ruby, with the same literal `expected` string
serving as the reference regardless of which frontend produced the `Module`
that generated it. A `Program`'s `expected` field is the SIR-level answer;
today it is written by running the Ruby-sourced version through a real Ruby
interpreter once (a convenient way to get a correct answer, not a structural
dependency), but nothing about the harness should *require* a Ruby source to
exist for a given case once a second frontend can reach the same feature. See
`code/packages/rust/sir-conformance/` for the harness this section governs;
generalizing its `Program`/`Frontend` types to reflect this is tracked as
implementation work following this spec, not part of the spec itself.

## §6 — Out of scope / future forks

- **A second, structurally different OOP profile** (nominally-typed,
  interface/generic-based, static-dispatch — e.g. what a Java/C# frontend
  would need). **Designed** — see
  [SIR29](SIR29-nominal-static-oop-profile.md), a new `Feature` set
  (`NominalClasses`/`Interfaces`/`VirtualDispatch`/`ErasedGenerics`) and
  dispatch primitive (`Expr::VirtualCall`, an index-based sibling of this
  section's string-based table lookup) alongside §2, the same relationship
  SIR22/23 have to the base leaf features — not a replacement or a
  parameterization of §2's dispatch algorithm. As of SIR29's own PR this is
  a pure-additive IR diff only; no frontend produces it and no backend
  implements it yet (both are tracked, separate follow-up work).
- **True per-object/per-thread `self` binding**, metaprogramming
  (`method_missing`, runtime `define_method`, `send`), singleton methods on
  arbitrary objects, reopening built-in classes — all already out of scope
  per `sir-classes-oop.md` and unchanged here.
- **Backend doc-comment/citation cleanup** (replacing "matches Ruby" / "Ruby's
  MRO" / "true Ruby" citations with references to this spec) is a separate,
  purely mechanical follow-up — tracked, not part of this spec's diff.

## Cross-reference index

| Concern | Authoritative doc |
|---|---|
| Dispatch mechanism, `self`/`super`, construction | this spec, §2 |
| Class/method/ivar/exception design rationale, milestones, history | `sir-classes-oop.md` |
| Built-in method catalog (Array/Hash/String/Numeric/Symbol) | `sir-method-dispatch.md`, `sir-collection-methods.md` |
| Mixins / `include`/`extend` / MRO detail | `sir-mixins.md` |
| Exception class hierarchy | `sir-exception-hierarchy.md` |
| `to_s`/`inspect` display convention | `sir-display-convention.md`, `sir-display-inspect-split.md` |
| Numeric type lattice, sized-int lowering roadmap | `SIR21-type-system-and-integer-semantics.md` |
| Array/matrix domain (separate profile, already multi-frontend) | `SIR22-array-matrix-semantic-ir.md` |
| Symbolic/pattern domain (separate profile, already multi-frontend) | `SIR23-symbolic-pattern-semantic-ir.md` |
