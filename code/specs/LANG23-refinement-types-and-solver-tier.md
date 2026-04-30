# LANG23 — refinement types + the solver tier
## (gradual, opt-in, slot-into-the-existing-type-checker)

## Overview

Today's type checkers — even the strongest ones — answer one
question per binding: *what kind of value is this?*  An `int`
parameter accepts every integer.  An `i64` accepts every 64-bit
signed integer.  An `array<u8, 256>` accepts every 256-byte
buffer.

But the kind of value is rarely the **whole** constraint.  Most
real APIs have hidden contracts the type system doesn't carry:
"this index must be ≤ length"; "this port must be ≥ 1024"; "this
ASCII codepoint must be < 128".  When the contract is broken,
the type system has nothing to say, and the bug ships.

LANG23 closes that gap by adding a **second tier** to the type
checker: **refinement types**, where the annotation carries not
just a kind but a predicate, and a **theorem/constraint solver**
discharges the predicate as part of compilation.  It's the same
pattern types already use — declare a constraint, compiler
enforces it, runtime gets the optimisation — applied one level
deeper.

The design choice that makes this practical (and that no
production language has shipped) is that **refinement is opt-in
per variable**.  An engineer writes:

```scheme
(define count : int 25)                       ; just a type — same as today
(define index : (Int 0 256) 25)               ; refinement: solver fires
```

The first line costs nothing.  The second buys: a compile error
if a caller could ever supply 500, an inferred narrower type
flowing through every downstream use, and stripped runtime
checks where the solver proves them redundant.  Programs without
any refinement annotations compile at exactly today's speed.
Programs with one annotation pay for one obligation.  The dial
runs from "zero solver work" through "every public API surface
is refinement-typed" without forced migration.

The solver also becomes a **library** the program can use
directly via a `solve` builtin — same engine, two doors.

## Motivating bug

This spec exists because of a real ASCII-indexing bug.  A function
of shape:

```scheme
(define (ascii-info i)
  (vector-ref ASCII_TABLE i))   ; ASCII_TABLE has 128 entries
```

was called from somewhere that didn't bound `i` to `[0, 128)`.
Type system: ✅.  Test suite: ✅ (it never tested `ascii-info(200)`).
Production: 💥.  The bug existed because *the function signature
didn't say what valid inputs were* — `i: int` is the type, but the
contract is `i: int ∧ 0 ≤ i ∧ i < 128`.

With LANG23:

```scheme
(define ASCII_TABLE : (Vector 128 _) (vector ...))
(define (ascii-info (i : (Int 0 128)))
  (vector-ref ASCII_TABLE i))     ; provably safe — bounds check stripped
```

Every caller is forced into one of three:

1. **Pass a value already in range** — silently accepted (callee
   can prove from caller's annotations).
2. **Insert a guard** — `(if (< x 128) (ascii-info x) ...)` — solver
   sees the guard, accepts (TypeScript-style flow narrowing).
3. **Pass an unconstrained value** — compile error.  The bug
   becomes literally inexpressible.

That story is the user-facing pitch for refinement types.  The
rest of the spec is the engineering detail.

---

## Why this spec is needed now

LANG22 specified compilation across the typing spectrum (Tier A
fully typed, Tier B partial, Tier C untyped + inference).  The
spectrum stops at "what kind of value" — `int`, `f64`, `bool`,
`(Pair int int)`.  LANG23 extends it one tier further: **Tier D —
refinement-typed** — where the annotation also carries a predicate.

Three reasons to land it now:

1. **The architecture is ready.**  LANG22's IIR `type_hint` field
   already has the right shape; widening it to carry a predicate
   is a syntactic change.  LANG22 specified the AOT codegen path
   that *consumes* narrowed types ("typed instr → native
   specialised code"); LANG23 just gives the codegen sharper
   inputs.  Doing this after the codegen lands and again after
   the JIT lands would mean two big migrations instead of one
   well-placed extension.

2. **The bug class it catches is the most expensive one in
   production.**  Off-by-one indexing, integer overflow, division
   by zero, state-machine misuse, port/byte/percentage range
   violations — these are *the* main source of runtime panics in
   safer languages and *the* main source of CVEs in C/C++.  A
   gradual, opt-in mechanism that catches these at compile time
   ships orders-of-magnitude more value than the next available
   compiler feature.

3. **The product gap is real.**  No production-deployed language
   ships gradual refinement types as a "just an annotation"
   feature.  Liquid Haskell (separate tool, never adopted), F\*
   (research), Whiley (research), Dafny (research), TypeScript's
   control-flow narrowing (limited to existing types) — every
   prior attempt was either too narrow (single-predicate, e.g.
   Rust's `NonZero`) or too all-or-nothing (whole-program proof
   obligation, target audience is the formal-verification
   community).  TypeScript proved gradual *typing* works at
   scale; nobody has done the same for *refinement*.

---

## Relationship to existing specs

| Spec | What it gives | LANG23 extends with |
|------|---------------|---------------------|
| LANG01 | IIR with `type_hint` field per instruction | Field semantics: `type_hint` is now a structured value (kind + optional predicate), not a string. |
| LANG02 | Interpreter (vm-core) | When a runtime check is emitted (third outcome — see below), the interpreter reads it and traps on violation. |
| LANG03 | jit-core | JIT speculates on profile-observed *value ranges*, not just types.  Reuses LANG23's refinement lattice for the speculation. |
| LANG04 | aot-core (typed languages) | AOT codegen consumes refinement-narrowed types for unboxing, overflow elimination, dead-branch removal. |
| LANG12 | vm-type-suggestions | Suggestions extend from "annotate the type" to "tighten the refinement" (e.g. "profile observed `[0..127]`; declare `(Int 0 128)` to skip overflow checks"). |
| LANG20 | LangBinding trait, value rep | `LangBinding::class_of` returns a richer kind that carries refinement; bindings can declare per-language refinement vocabularies. |
| LANG22 | Typing spectrum (Tiers A–C) + AOT/JIT/PGO + `.ldp` profile format | LANG23 adds **Tier D**.  The `.ldp` profile artefact records observed value ranges per instruction; LANG23 specifies how the solver consumes them. |

LANG23 is **strictly additive**.  Nothing in any prior spec needs
to change for refinement-free code; the refinement path activates
only where a frontend emits an annotation with a predicate.

---

## The optional-typing protocol extension

The contract is small.  An IIR `type_hint` today is a string.
Under LANG23 it becomes a structured type:

```rust
/// LANG23: an IIR type_hint is now a kind plus an optional
/// refinement predicate.  Frontends that don't emit refinements
/// produce `RefinedType { kind, predicate: None }` and the
/// type checker behaves exactly as before.
pub struct RefinedType {
    /// The base kind (i64, f64, bool, class-id, "any").
    pub kind: Kind,
    /// Optional predicate restricting the value set within `kind`.
    /// `None` means "any value of `kind`" — unchanged from today.
    pub predicate: Option<Predicate>,
}

pub enum Predicate {
    /// `lo ≤ x` and/or `x ≤ hi`.  The 80%-of-real-bugs case.
    Range { lo: Option<i128>, hi: Option<i128>, inclusive_hi: bool },
    /// `x = a ∨ x = b ∨ ... ∨ x = z`.  For enum-like states.
    Membership { values: Vec<i128> },
    /// Conjunction / disjunction / negation of inner predicates.
    /// Allows arbitrary boolean combinations of the above.
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
    /// Linear arithmetic predicate `Σ aᵢ·xᵢ ⊙ c` where `⊙ ∈ {<, ≤, =, ≥, >}`.
    /// Lets a refinement reference *other* variables in scope.
    LinearCmp { coefs: Vec<(VarId, i128)>, op: CmpOp, rhs: i128 },
    /// Escape hatch: an opaque user-supplied predicate carrying a
    /// language-specific AST.  Frontends use this for predicates
    /// the solver can't yet handle (quantifiers, recursive
    /// definitions, theory of arrays).  Such predicates degrade to
    /// the "unknown" outcome: emit a runtime check.
    Opaque { display: String },
}
```

The protocol is **the same lattice the type checker already
walks** with one new step: when a `RefinedType` carries
`predicate = Some(p)`, run the solver on the proof obligation.
Nothing else changes.  Type-only annotations continue to flow
through the existing checker unchanged.

### Three outcomes per proof obligation

Every refinement-relevant operation produces a proof obligation
the solver must discharge.  Three possible outcomes:

```
        ┌─────────────────────────────────────────────┐
        │  proof obligation                           │
        │  e.g. "callee requires `i: (Int 0 128)`,    │
        │   call site passed `x: int`"                │
        └────────────────────┬────────────────────────┘
                             │
                  ┌──────────┴──────────┐
                  │   solver fires      │
                  └──────────┬──────────┘
                             │
            ┌────────────────┼────────────────┐
            ▼                ▼                ▼
      proven safe       proven unsafe    unknown
            │                │                │
            ▼                ▼                ▼
    strip runtime    compile error    emit runtime
    check; narrow    with concrete    check; log
    downstream       counter-example  "couldn't prove"
    type             ("could pass     warning;
    accordingly      500; out of      narrow as type;
                     [0, 128)")       proceed
```

The **unknown** outcome is the load-bearing one for adoption.
Without it, refinement types are like Liquid Haskell — all-or-
nothing, hard to retrofit.  With it, you can refinement-annotate
one function in a million-line codebase and the rest just keeps
working.  The solver gracefully concedes; the runtime check
catches what the static analysis couldn't.

### Configurability of the unknown path

Three modes, configurable per crate / per file / globally:

| Mode | Unknown outcome |
|------|-----------------|
| `--refinement-mode=lenient` (default) | emit runtime check; warn |
| `--refinement-mode=strict`            | compile error; force user to add a guard or relax the annotation |
| `--refinement-mode=off`               | ignore predicates entirely; type-only checking |

`lenient` is the gradual-adoption-friendly default.  `strict` is
for libraries that want zero runtime checks (the F\* / Liquid
Haskell experience).  `off` is the escape hatch for crates that
opt out entirely.

---

## The roll-up scope hierarchy

> "If we marry that with a CFG, we should be able to allow for
> opt in at a variable level and then roll up to a function
> level and then to a module and then finally to a program
> level."

This is the spec's organising principle.  Refinement adoption
isn't all-or-nothing; it climbs an explicit ladder.  Each rung
extends the solver's reach without forcing the lower rungs to
re-engage.

### Rung 1 — variable-scope

Annotate any single binding in any function, anywhere in the
program:

```scheme
(define (compute n)
  ;; Most of the function is type-only; one local has a
  ;; refinement.  Solver fires only for `bucket`.
  (let* ((scaled : int (* n 2))
         (bucket : (Int 0 16) (modulo scaled 16)))
    (vector-ref BUCKET_TABLE bucket)))
```

Solver scope: just this binding's defining expression.  CFG: the
single statement `bucket := scaled mod 16`.  Either the predicate
follows from the expression (provable: `mod 16 ∈ [0, 16)` —
solver discharges trivially) or it doesn't (emit runtime check
or compile error per mode).  No other binding's checking changes.

### Rung 2 — function-scope (params + return type)

Annotate the function signature.  CFG-based predicate propagation
across the body becomes the differentiator from rung 1:

```scheme
(define (clamp-byte (x : int) -> (Int 0 256))
  (cond ((< x 0)   0)
        ((> x 255) 255)
        (else      x)))
```

The solver sees:
- branch 1: `x < 0` → `result = 0` → `0 ∈ [0, 256)` ✓
- branch 2: `x > 255` → `result = 255` → `255 ∈ [0, 256)` ✓
- branch 3: `¬(x < 0) ∧ ¬(x > 255)` → `0 ≤ x ≤ 255` → `result = x ∈ [0, 256)` ✓

All three discharge.  The function's declared return type is
proven.  Callers can rely on it without runtime checks.

This is **path-sensitive analysis**, the same machine TypeScript
uses for control-flow narrowing — extended to arbitrary numeric
predicates.  Liquid Haskell calls this "abstract interpretation
over the predicate lattice"; we just call it walking the CFG with
the solver as oracle.

### Rung 3 — module-scope

Every public binding in a module has refinements (or marks
itself `: any` to opt out per-symbol).  The solver can now reason
*across* function boundaries within the module:

```scheme
;; module: text/ascii.twig
(define (decode (codepoint : (Int 0 128))) ...)

(define (latin1-decode (cp : (Int 0 256)))
  (if (< cp 128)
      (decode cp)              ; solver: cp narrowed to [0, 128) by guard ✓
      (latin1-fallback cp)))
```

The call from `latin1-decode` to `decode` is provably safe
because the `if (< cp 128)` guard narrows `cp` inside the then-
branch.  Without module-scope refinements, `latin1-decode`
couldn't be sure `decode` wouldn't reject — there'd be a runtime
check at the boundary or an opacity wall.  With them, the
solver flows the constraint across.

Module-scope annotations are the practical sweet spot — they
give libraries strong contracts at their public surface without
forcing internal implementations to be refinement-typed.

### Rung 4 — program-scope (whole-program closed-world)

Every public binding from every module the program transitively
consumes has refinements.  The solver runs across the closed-
world call graph at link time:

```scheme
(declare-program-mode strict-refinement)

(define (main) ...)
;; The compiler now refuses to link any module
;; whose public surface includes type-only `: any`
;; bindings.  Every cross-module call is proven
;; refinement-safe at build time.
```

This is the "AOT-no-profile maximum specialisation" mode —
LANG22's Tier A treatment, applied across the whole program.
Equivalent to GraalVM Native Image's closed-world model but
with refinement rather than just types.  Compile-time cost is
high; the result is a binary with zero refinement-related
runtime checks anywhere.

### Why the rung hierarchy matters

Each rung is **the same compiler running with a different scope
of obligations**.  No new pipeline per rung; no separate tool;
no migration command.  An engineer adds one annotation (rung 1).
If they like the result, they refinement-annotate the rest of
the function (rung 2).  If a library author wants stronger
guarantees, they refinement-type the public surface (rung 3).
If a deployment wants maximum specialisation, they enable
program-scope mode at link time (rung 4).

The rung is a property of the **annotated code**, not the
toolchain.  A program can mix all four:

```
program
├── auth_module: rung 3 (every public binding refinement-typed)
├── handlers_module: rung 2 (a few hot functions refinement-typed)
├── utils_module: rung 1 (one binding here, one binding there)
└── 3rd-party deps: untyped (rung 0 — current LANG22 behaviour)
```

The solver only fires where annotations exist.  This is the
critical design property: **opt-in refinement does not penalise
non-refined code**.

---

## CFG-based predicate propagation

Every refinement-related question is a question about a path
through the program's control-flow graph:

- "Is `i` in `[0, 128)` at this `vector-ref`?"
- "Is the result of this branch in `[0, 256)` for the function's
  return type?"
- "Does this `(if (< x 128) ...)` narrow the then-branch's
  knowledge of `x`?"

These all reduce to: walk the CFG, gather predicates from every
guard reaching the point of interest, and ask the solver:
*given everything I've gathered, does the obligation hold?*

### Gathering predicates

Walking the CFG, three sources of predicates accumulate:

1. **Annotation-declared**: a parameter's refinement, a `let`
   binding's annotation, a function return type.  These enter
   the predicate set when the binding's scope opens.
2. **Guard-narrowed**: an `if`/`cond`/`while` condition
   contributes a predicate to its then-branch (and the negation
   to its else-branch).  Same machine as TypeScript's
   `if (typeof x === "string")` narrowing — just over arbitrary
   linear-arithmetic predicates.
3. **Call-returned**: when a function with a refinement-typed
   return is called, the call's destination register inherits
   the predicate.

```scheme
(define (f (n : (Int 0 100)))
  (if (< n 50)
      ;; Predicate set here: { n ∈ [0, 100), n < 50 }  →  n ∈ [0, 50)
      ;; A subsequent (vector-ref BUCKETS_50 n) is provably safe.
      (vector-ref BUCKETS_50 n)
      ;; Predicate set here: { n ∈ [0, 100), n ≥ 50 }  →  n ∈ [50, 100)
      ;; (vector-ref BUCKETS_50 n) here would FAIL the obligation.
      ...))
```

### How narrowing flows through arithmetic

If `a: (Int 0 10)` and `b: (Int 0 10)`, the solver propagates:

| Expression | Inferred type |
|------------|---------------|
| `(+ a b)` | `(Int 0 20)` |
| `(* a b)` | `(Int 0 100)` |
| `(- a b)` | `(Int -10 10)` |
| `(modulo a b)` | `(Int 0 10)` (assuming `b > 0`; solver picks up the guard if present) |
| `(if c a b)` | `(Int 0 10)` (union of arms, both `[0, 10)`) |

This is **interval arithmetic** but driven by the same SMT solver
that handles the path-sensitive logic.  No separate pass.

### Implementation note

The CFG already exists in the compiler — `ir-optimizer` walks it
for dead-code elimination, register allocation, etc.  LANG23
adds a **predicate-tagged extension** to the existing CFG node
visit:

```rust
struct CfgNodeContext<'a> {
    // existing fields (block, instructions, etc.)
    predicates_in_scope: &'a PredicateSet,  // NEW
}

trait CfgVisitor {
    fn visit_node(&mut self, node: &CfgNode, cx: &mut CfgNodeContext<'_>);
}
```

The refinement checker is then a `CfgVisitor` that:
1. On block entry: extend `predicates_in_scope` with annotations
   active at this point, plus guard predicates from incoming edges.
2. For each instruction: emit proof obligations the operation
   produces (e.g., a `vector-ref` with bounded-int index in scope
   needs to prove `0 ≤ i < len`).
3. On block exit: trim predicates that go out of scope.

Same pass that already runs; one new visitor.  No second IR.

---

## Solver architecture

Two questions: (a) what theory does the solver support and
(b) what's its implementation.

### Theory

For LANG23 v1, the solver supports:

| Theory | Decidable? | Used for |
|--------|-----------|----------|
| Boolean logic (∧, ∨, ¬) | Yes (SAT) | Combining predicates |
| Linear arithmetic over integers (LIA) | Yes | All `Range`, `Membership`, `LinearCmp` predicates |
| Linear arithmetic over reals (LRA) | Yes | Float refinements (later PRs; LANG23 v1 is integer-only) |
| Theory of arrays | Yes (decidable subset) | Array-bounds reasoning (PR 23-D) |
| Quantifiers | No (in general) | Out of scope for v1; degrade to `Opaque` |
| Uninterpreted functions (EUF) | Yes | Out of scope for v1 |

LIA is the load-bearing theory.  All of `Range` /
`Membership` / `LinearCmp` reduce to LIA.  The solver is
**complete** for the v1 vocabulary — every well-formed
obligation gets a yes-or-no answer.

### Implementation: roll our own vs Z3 vs hybrid

**Three options, evaluated:**

| Option | Pros | Cons |
|--------|------|------|
| Hand-rolled (Rust crate `lang-smt`) | No external dep; matches the LANG project ethos of "build from scratch"; can specialise for our vocabulary; runs as a Lispy program (eats own dogfood) for tooling | Real engineering work — a complete LIA solver is ~5–10K lines |
| Z3 via FFI (`z3-rs`) | Industry standard; complete; handles theories we'll want later (arrays, quantifiers) | C++ dep; ~50MB binary; supply-chain weight; doesn't run on every target |
| Hybrid: hand-rolled fast path, Z3 fallback for rare hard cases | Best of both | Two solvers to maintain |

**Recommendation: hand-rolled.**  v1's vocabulary is bounded
(LIA + boolean), the LANG project values self-containment, and
a baby SMT solver is a great learning artefact in its own right.
The Cooper algorithm for LIA is ~500 lines plus boolean
preprocessing; Omega test plus simplex is the standard heavy-
artillery approach.  We start with Cooper for clean integers and
upgrade to simplex when we need real-arithmetic mixing.

If a future PR needs theories beyond LIA (uninterpreted
functions, quantifiers), revisit Z3 vs extending `lang-smt`.
At that point Z3 might become the right call — but it shouldn't
gate v1.

### Solver as a Lispy program (eat-own-dogfood option)

The `lang-smt` crate is implemented as a **Lispy program that
runs on the LANG VM** plus a thin Rust shim that calls into it.
This means:

- The solver is itself JIT-promotable — V8-style speedups apply
  to the solver's own hot loops.
- The solver's optimisation is driven by `.ldp` profile data —
  the solver gets faster as users use it.
- The solver code can be inspected, modified, profiled by the
  same tooling as user code.
- The compile-time cost question becomes "how fast is our JIT
  on this Lispy code" — which is the same question we're
  answering for everything else.

This is more ambitious than the alternative (write the solver in
Rust directly) but it pays the LANG project's "build everything
from scratch and have it run on its own VM" dividend.  It also
means a future user-facing `solve` builtin (see below) shares
exactly one implementation with the compile-time checker.

---

## Per-language refinement syntax

Frontends choose syntactic surface; the IIR target is uniform.

### Twig / Lispy / Scheme

Type ascription on parameters and let-bindings, with a structured
predicate form:

```scheme
;; Range
(define (square (x : (Int 1 256))) (* x x))

;; Long form (matches Lispy/Scheme R6RS-ish convention)
(define (lookup
          (i : (refined int (lambda (n) (and (>= n 0) (< n LEN))))))
  (vector-ref TABLE i))

;; Membership
(define (set-state (s : (Member int (1 2 3 4))))
  ...)

;; Function-level ascription with return type
(define (clamp-byte
          (x : int) -> (Int 0 256))
  (cond ((< x 0)   0)
        ((> x 255) 255)
        (else      x)))
```

### TypeScript-style

Generic-parameter shape, fits TypeScript's existing surface:

```typescript
function squarify(x: int<1, 256>): int<1, 65536> {
  return x * x;
}

function asciiInfo(i: int<0, 128>): AsciiInfo { ... }
```

### Sorbet-style (Ruby)

Sigblock with a refinement DSL:

```ruby
sig { params(x: T::Int.in(1..256)).returns(T::Int.in(1..65536)) }
def squarify(x)
  x * x
end
```

### Mypy-style (Python)

`Annotated` from PEP 593 with a refinement-aware annotation:

```python
def squarify(x: Annotated[int, Range(1, 256)]) -> Annotated[int, Range(1, 65536)]:
    return x * x
```

### Mapping to IIR

Every frontend lowers its syntax to the same `RefinedType`
structure described above.  The downstream pipeline doesn't know
or care which frontend produced the IIR — same as today's
type_hint strings.

---

## Solver as a runtime library: the `solve` builtin

Same engine, second door.  Engineers can invoke the solver from
their programs:

```scheme
;; Solve for x such that the constraint holds.
(let ((x (solve '(int) '(and (>= x 1) (<= x 100) (= (modulo x 7) 3)))))
  (printf "first solution: ~a~%" x))
;; → 3

;; Check satisfiability.
(if (solve-sat? '(and (> x 5) (< x 3)))
    "yes there's an x"
    "no, contradiction")
;; → "no, contradiction"

;; Enumerate.
(for-each
  (lambda (x) (printf "~a~%" x))
  (solve-all '(int) '(and (>= x 1) (<= x 5))))
;; → 1 2 3 4 5
```

This is **not constraint-programming-as-Prolog** — `solve` runs
the same SMT engine we use at compile time, with the same
vocabulary.  Useful for: range generation, configuration solving,
dependency-resolution kernels, optimisation problems that fit
LIA, schedulers, anything where "find values satisfying a
predicate" is the natural shape.

### Why expose this at all

1. **Eat-own-dogfood**: the compiler uses the same builtin
   internally; bugs fixed in user-space `solve` automatically
   improve the compiler.
2. **Refinement annotations are themselves expressed in the
   solver vocabulary**.  Users who want custom predicates
   beyond `Range` / `Membership` can write them as `Opaque`
   with a Lispy lambda body — and the runtime check the
   compiler emits is just `(unless (solve-sat? predicate-with-x)
   (raise))`.  One implementation, two consumers.
3. **The library is small** (~200 line surface) once the engine
   exists.

### Scope (v1)

- `(solve vars predicate)` — find any solution
- `(solve-sat? predicate)` — check satisfiability
- `(solve-all vars predicate)` — enumerate solutions (bounded
  by user-supplied limit; otherwise diverges on infinite ranges)
- `(solve-min vars predicate cost)` — optimisation
- `(solve-max vars predicate cost)` — optimisation

Out of scope for v1: arrays, quantifiers, uninterpreted
functions, real arithmetic, MaxSAT, theories beyond LIA.

---

## Educational tooling

The same `lang-perf-suggestions` tool LANG22 spec'd consumes
LANG23 data:

```
$ lang-perf-suggestions --profile myapp.ldp --refinement-mode lenient

myapp/twig/handlers.twig
========================

Function: ascii-info  (called 487,213 times)
  parameter `i`:
    declared:  int
    observed:  always int (487,213 / 487,213 = 100.0%)
    range:     [0, 127] (min 0, max 127, never violated)

  REFINEMENT SUGGESTION (high confidence):
    declare:   (i : (Int 0 128))
    benefits:
      ─ strip 487,213 runtime bounds-check failures (0 occurred)
      ─ enable AOT to drop the bounds check in vector-ref
      ─ promotes this function to refinement-typed; callers'
        flow-narrowing can now satisfy (decode codepoint) on
        line 14 without runtime check

  REFINEMENT SUGGESTION (medium confidence):
    declare:   (return-type : ascii-info-result)
    benefits:
      ─ already returned a non-nil ascii-info-result for all
        observed inputs; encoding this lets callers skip nil-checks

myapp/twig/util.twig
====================

Function: parse-int  (called 1,432 times, currently typed `int -> int`)
  CANNOT REFINE — observed inputs include str, int, nil:
    str:  1,200
    int:    200
    nil:     32
  This function is polymorphic on input type.  Either:
    ─ split into per-type entry points and refinement-type each
    ─ accept the polymorphic surface and skip refinement
```

The educational story: **the tool tells the engineer specifically
what to write**, with a concrete cost in skipped runtime checks.
LANG12 already specs this for type-only suggestions; LANG23 is
the same machinery with a richer suggestion language.

---

## Performance budget

The compile-time cost is the explicit tradeoff.  Numbers below
are projections from prior-art benchmarks (Liquid Haskell, F\*,
Whiley):

| Annotation density | Solver fires per build | Wall-time impact (1M-LOC program) |
|--------------------|------------------------|------------------------------------|
| 0% (no refinements) | 0 obligations | 0 ms (default cost) |
| 1% (one annotation per ~100 lines) | ~10K obligations | +5–15 sec |
| 10% (every public surface) | ~100K obligations | +60–180 sec |
| 100% (every variable) | ~1M obligations | +10–30 min |

These are with cold solver invocations.  **Caching** brings the
incremental case down by 50–90×:

- Per-obligation salsa-style cache.  The cache key is
  `(annotation, predicate-set, expression-tree-hash)`.
- Cache invalidates when any input changes, which is rare for
  most obligations (most annotations and most call sites are
  stable across edits).
- Build N+1 typically discharges only the obligations affected
  by the diff — usually <1% of the total.

Net: a real-world incremental dev loop pays single-digit-second
overhead even on a heavily refinement-typed codebase.  Full
clean builds pay a one-time cost; the user has explicitly
chosen this by adding annotations.

### Modes the user can dial

```bash
# Skip solver entirely (refinements act as documentation only).
lang-build --refinement-mode=off

# Default: solver fires on annotations; unknown → runtime check.
lang-build --refinement-mode=lenient

# Solver fires; unknown → compile error.
lang-build --refinement-mode=strict

# Solver fires only for marked-hot files (e.g. CI checks
# refinement on changed files only).
lang-build --refinement-mode=incremental
```

`incremental` mode is the practical default for CI — solver
runs only on files that changed in the diff, plus their
transitive refinement-callers.

---

## Crate structure

New crates introduced by LANG23, plus extensions to existing
ones:

```
code/packages/rust/
├─ lang-refined-types/      NEW
│   The `RefinedType` / `Predicate` / `Kind` data types and
│   the predicate algebra (and/or/not, simplification, hashing
│   for caching).  No solver — pure data.
│
├─ lang-smt/                NEW
│   The hand-rolled SMT solver.  Implemented as a Lispy program
│   (eats own dogfood) plus a thin Rust shim that owns the FFI
│   surface.  Cooper algorithm for LIA, basic boolean
│   preprocessing, predicate-set abstraction.  ~5K LoC of Lispy
│   plus ~500 LoC of Rust shim.
│
├─ lang-refinement-checker/ NEW
│   The compiler pass.  Walks the CFG (already in ir-optimizer);
│   gathers predicates; emits proof obligations; consults the
│   solver; classifies as PROVEN_SAFE / PROVEN_UNSAFE / UNKNOWN;
│   writes back narrowed types to IIR `type_hint` fields and
│   emits runtime-check instructions for UNKNOWN cases.
│
├─ lang-runtime-core/       EXISTS; LANG23 extends:
│   - LangBinding::class_of returns RefinedType (was kind-only)
│   - new RuntimeError::RefinementViolation for runtime-check trips
│
├─ lispy-runtime/           EXISTS; LANG23 adds:
│   - `solve` / `solve-sat?` / `solve-all` / `solve-min` /
│     `solve-max` builtins routed through `lang-smt`
│   - registers refinement-aware versions of arithmetic
│     builtins so AOT can use narrowed types for unboxed paths
│
├─ codegen-core/            EXISTS; LANG23 adds:
│   - emit_refinement_check(op, type) — runtime check codegen
│     for UNKNOWN obligations
│   - lower_typed_instr now consumes refinement-narrowed types
│     for unboxing decisions
│
├─ lang-perf-suggestions/   EXISTS (LANG22); LANG23 extends:
│   - reads refinement annotations from IIR
│   - produces refinement-suggestion reports (see §"Educational
│     tooling")
│
└─ ldp-format/              EXISTS (LANG22); LANG23 adds one
    field to the per-instr observation record:
    - observed_value_range: { min: i128, max: i128 }
    Already implicitly there as LANG22 carried "types_seen";
    LANG23 names the integer-range observation explicitly so
    refinement-suggestion can read it without re-aggregation.
```

Sharing notes:

- `lang-smt` is the only new "logic-heavy" crate.  Everything
  else is glue.
- `lang-refinement-checker` is a tree visitor; small.
- The `solve` builtin and the compile-time checker share the
  exact same solver instance.

---

## Migration path

LANG23 ships in a sequence of small, independently-reviewable PRs:

| PR | Scope | Acceptance |
|----|-------|-----------|
| 23-A | `lang-refined-types` crate: data types, predicate algebra, simplifier, hashing.  No solver, no checker. | Round-trip every Predicate variant through the algebra; combine/simplify produces canonical form |
| 23-B | `lang-smt` crate, LIA fragment.  Hand-rolled Cooper algorithm in Rust first (defer the eat-own-dogfood Lispy implementation to a follow-up).  Pure-data API: take a predicate, return SAT/UNSAT/UNKNOWN. | Discharges all of 100 standard LIA obligations from the SMT-LIB benchmark suite |
| 23-C | `lang-refinement-checker` MVP: variable-scope only (rung 1).  Reads annotations from IIR `type_hint` (extended in 23-A); emits proof obligations; calls `lang-smt`; classifies outcomes; writes back narrowed types. | A Twig program with `(define x : (Int 1 256) 25)` compiles; with `(define x : (Int 1 256) 500)` rejects with a counter-example; with an unbounded `(define x : (Int 1 256) (read-int))` emits a runtime check |
| 23-D | Extend the checker to function-scope (rung 2): CFG-based predicate propagation, guard narrowing, branch arms, function-return obligations. | The `clamp-byte` example from §"Rung 2" compiles with no runtime checks; modifying it to violate the return type produces a clean error |
| 23-E | Twig syntax: parser + IR compiler emit refinement annotations from `(x : (Int a b))` syntax. | Round-trip: parse → IR → checker → run; refinement metadata preserved |
| 23-F | Module-scope (rung 3): cross-function reasoning within a module; per-symbol opt-out via `: any`. | A two-function module like the `latin1-decode` / `decode` example proves cross-fn calls without runtime checks |
| 23-G | Program-scope (rung 4): closed-world refinement-only mode; link-time check that no `: any` slips through. | Compiling with `--refinement-mode=strict` against a refinement-incomplete dep produces a clear error |
| 23-H | `solve` builtin family in `lispy-runtime`.  Same `lang-smt` engine. | All five `solve*` shapes from §"Solver as runtime library" pass tests |
| 23-I | `lang-perf-suggestions` v3: refinement suggestions from `.ldp` profile data. | The "ascii-info" suggestion from §"Educational tooling" appears against a real profiled run |
| 23-J | Re-implement `lang-smt` solver as a Lispy program running on the LANG VM (eat-own-dogfood).  Rust solver becomes a fallback. | Performance parity within 2× of the Rust impl on the LIA benchmark |
| 23-K | Solver caching layer (salsa-style).  Per-obligation cache invalidating on inputs. | Incremental rebuild of a heavily-refinement-typed codebase under 10× the no-refinement build time |
| 23-L | Configuration plumbing (`--refinement-mode={off,lenient,strict,incremental}`) and CI integration. | All four modes work end-to-end on a sample project |

PRs 23-A through 23-D are the **MVP** — they ship variable-scope
+ function-scope refinement, which is enough to cover the
motivating ASCII-bug story.

PRs 23-E through 23-H are the **practical** layer — frontends
use it, modules use it, programs can target the strict mode,
and the solver is exposed to user code.

PRs 23-I through 23-L are **polish** — educational reports,
self-hosting, caching for incremental builds, configurability.

The MVP unblocks anyone whose pain matches the ASCII bug.  The
later layers compound the value.

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Solver wallclock kills compile times | Medium | Adoption blocker | Caching (PR 23-K) + per-obligation timeouts + `--refinement-mode=incremental` for CI; bench every PR against a refinement-heavy testbed |
| Hand-rolled solver bugs (false positives) | Medium-High | Correctness blocker | SMT-LIB benchmark suite as acceptance gate; differential testing against Z3 in CI (cross-check, not dependency); fuzz the solver |
| Hand-rolled solver bugs (false negatives) | Lower | Annoys users (they get unknown when proof exists) | Same as above; gradual mode means false negatives produce runtime checks rather than crashes |
| Predicate algebra explosion (combinatorics) | Medium | Slow solver invocations | Simplify aggressively in `lang-refined-types` (PR 23-A); reject annotations with >2¹⁶ AST nodes |
| LIA decidability assumed but predicates leak into non-LIA territory | Medium | Solver returns UNKNOWN often | The `Opaque` predicate variant exists for exactly this; degrade to runtime check |
| Cross-language refinement compatibility | Lower | Polyglot programs misalign | Refinement is per-binding-language by default; cross-language calls go through type-only boundaries unless explicitly opted in |
| Solver-as-Lispy bootstrap problem | Lower (PR 23-J only) | Compiler can't bootstrap | Keep the Rust solver as fallback; the Lispy version is an optimisation, not a dependency |
| Profile data drives refinement-suggestion that's not actually safe | Medium | User adds annotation, compiler still rejects | The suggestion is "consider declaring..."; the user owns the decision; compiler then proves or rejects |
| Refinement annotations as documentation rot | Medium | False sense of security | `--refinement-mode=lenient` is the default; `lenient` warns but doesn't fail, so out-of-date annotations show up as warnings rather than silent passes |
| Quantified predicates ("for all x") | Lower | Some real refinement needs them | Out of scope for v1; degrade to `Opaque`; revisit in a follow-up spec |

---

## Open questions

1. **Annotation propagation across function calls in profile mode.**
   When a function is JITted and the profiler observes a stable
   value range for an argument, should the JIT speculate on a
   refinement *the source code didn't declare*?  Trade-off:
   speedup vs. speculation-deopt cost.  **Recommendation:** yes;
   this is just a finer-grained type speculation.  The deopt
   mechanism from LANG20 already handles "type guard fails".

2. **Refinement on function values (closures).**  Can a closure
   itself be refinement-typed?  A function value with a
   refinement-typed parameter and return type is just a function
   type — no new mechanism.  But "this closure refines its result
   based on which closure it is" is dependent typing and is out
   of scope.

3. **Refinement on heap structures.**  Arrays, dicts, structs.
   PR 23-D adds the theory of arrays for `(vector-ref arr i)`
   bounds reasoning; richer structural refinements (e.g.,
   "every element of this list is in [0, 256)") are out of scope
   for v1.  Will need recursive predicates and an inductive
   reasoning step.

4. **Cross-binding refinement vocabularies.**  Should Lispy's
   `(Int 0 256)` and Ruby's `T::Int.in(0..256)` be canonically
   the same predicate?  **Recommendation:** yes for built-in
   predicate vocabulary (Range, Membership, LinearCmp); per-
   language `Opaque` predicates stay per-language.

5. **Negative predicates.**  Are predicates like `(Not (Member
   int (1 2 3)))` first-class?  Yes via `Not(...)`, but the
   solver may simplify them to ranges.  Open: how to surface
   this to the user — let them write `(NotIn ...)` directly?

6. **Effect refinements** (e.g., "this function is pure", "this
   function never panics").  Out of scope for v1; would need a
   different abstract domain than value ranges.  Worth a
   follow-up spec when needed.

7. **Real arithmetic refinements.**  Float refinements (e.g.,
   `(Float 0.0 1.0)` for probabilities) are useful but float
   semantics over LRA are tricky (NaN, ±Inf, precision).  v1 is
   integer-only; PR 23-N would extend.

8. **Compile-time error messages.**  When a refinement obligation
   fails, what counter-example do we surface to the user?
   "Could pass 500" is the simplest.  More elaborate
   formulations (which path through the CFG produced the
   counter-example) are higher-effort but more debuggable.
   **Recommendation:** start simple; iterate based on user
   feedback.

9. **Library publishing.**  When a library publishes with
   refinement-typed public surface, what's the discovery story
   for downstream users?  Embed the refinement metadata in the
   library's `Cargo.toml`-equivalent + IIR module.  Tools (LSP,
   docs) display it.  This is just packaging.

10. **Solver budget per build.**  Should there be a per-build
    timeout that aborts the solver on any single obligation?
    **Recommendation:** yes; default 5 seconds per obligation
    (matches Liquid Haskell's default).  Hitting the timeout
    surfaces as UNKNOWN; user can increase per-file via
    `--refinement-solver-timeout-ms=NN`.

---

## Acceptance criteria

LANG23 is "done" — locked enough to start implementation — when:

1. **The roll-up scope hierarchy is specified** (variable →
   function → module → program) with concrete examples per rung
   (§"The roll-up scope hierarchy").
2. **The three outcomes are specified** (proven safe / proven
   unsafe / unknown) with the runtime-check fallback for
   unknown (§"Three outcomes per proof obligation").
3. **The predicate vocabulary is enumerated** (Range,
   Membership, And/Or/Not, LinearCmp, Opaque) with the LIA
   reduction (§"The optional-typing protocol extension").
4. **CFG-based propagation is specified** as a visitor over the
   existing CFG, accumulating predicates from annotations,
   guards, and call returns (§"CFG-based predicate
   propagation").
5. **Solver architecture decided** — hand-rolled `lang-smt`
   with LIA, optional Lispy self-hosting, Z3 deferred (§"Solver
   architecture").
6. **Per-language syntax mapped** to the uniform IIR
   `RefinedType` (§"Per-language refinement syntax").
7. **`solve` builtin family specified** for runtime use (§"Solver
   as a runtime library").
8. **Educational tooling extension specified** for refinement
   suggestions on profile data (§"Educational tooling").
9. **Performance budget characterised** with caching strategy
   and configurable modes (§"Performance budget").
10. **Crate structure agreed** (§"Crate structure").
11. **Migration path sequenced** into ~12 reviewable PRs
    (§"Migration path").

This document satisfies all eleven.

---

## Out of scope (named for clarity)

- **Quantified predicates** (∀, ∃) — out of scope for v1;
  degrade to `Opaque`.  Liquid Haskell handles them; we defer.
- **Effect refinements** ("this function is pure", "this never
  panics") — different abstract domain; future spec.
- **Real / float refinements** — v1 is integer-only.  Float
  refinement involves NaN, ±Inf, precision rounding; deferred to
  PR 23-N or LANG24.
- **Recursive / inductive predicates** ("every element of this
  list is in [0, 256)") — out of scope; needs separate inductive
  reasoning step.
- **Dependent types in the full sense** (types that depend on
  arbitrary values) — out of scope.  Refinement is **value-
  range-bounded** dependence; full dependence is a research
  language.
- **Refinement-aware refactorings in the IDE** — outside the
  compiler; tooling spec.
- **MaxSAT / weighted CSP** — out of scope; `solve` is
  satisfiability + ranged search only in v1.
- **Cross-process solver caching** — caching is single-build for
  v1.  Distributed caches (turborepo-style) are a follow-up.
- **SMT-LIB import / export** — interesting for interop but not
  required.  Future tooling PR.
