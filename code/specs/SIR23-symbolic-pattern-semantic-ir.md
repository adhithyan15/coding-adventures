# SIR23 — Symbolic expression + pattern/rewrite IR extension (CAS math languages)

## Motivation

[`SIR10`](SIR10-narrow-waist-semantic-ir.md) has no symbolic-expression
vocabulary: no "apply a head to args as data" concept, no pattern variables,
no rewrite rules. This spec adds that vocabulary, additively, following the
same discipline [`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md)
used for Python/JS. Every existing module and backend match arm stays valid;
this only adds new `Expr` variants and `Feature` flags.

This is Stream B of [`HML01`](HML01-math-to-semantic-ir.md) — the substrate
for `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, and
`maxima-to-semantic-ir`, and for every future symbolic-CAS-family frontend
(Reduce, Derive, Maple) per
[`HML00`](HML00-historical-math-languages-roadmap.md).

**The fidelity decision this spec commits to:** the IR carries **full
rewrite semantics** — patterns and rules are first-class data, and
`ReplaceAll`/`ReplaceRepeated` are IR-level operations a backend must
actually execute (via a real pattern matcher in its runtime), not a
frontend-side operation that only ever lowers a precomputed answer. This
was an explicit repo-owner choice over the cheaper "evaluate with the
existing CAS engine, then lower only the final `IRNode` result as inert
data" alternative — the cheaper path can't compile an *uncomputed* Wolfram
function body (one that pattern-matches at call time) into equivalent JS
logic; this spec's path can.

Every new node kind is mapped **1:1** onto `symbolic_ir::IRNode`'s existing
five-variant shape (`Symbol`/`Integer`/`Rational`/`Float`/`Str`/`Apply`), so
lowering a Wolfram or Macsyma parse tree into these nodes is mechanical.

## Scope

**In scope:**

- Symbolic expression application: `head[args…]` / `head(args…)` as data,
  not just as a call — the same expression can appear as a value, a pattern
  target, or a rewrite-rule left-hand side
- Pattern blanks: `_` (`Blank`), `x_` (named `Pattern`), `_h` / `x_h`
  (head-constrained blank)
- Rules: `a -> b` (`Rule`, eager RHS) and `a :> b` (`RuleDelayed`, RHS
  re-evaluated per match)
- Replacement: `expr /. rules` (`ReplaceAll`, one pass) and `expr //. rules`
  (`ReplaceRepeated`, fixed point)
- Exact rational and complex scalars (`SirType::Rational`/`SirType::Complex`
  — shared with [`SIR22`](SIR22-array-matrix-semantic-ir.md), landed once)

**Explicitly out of scope (deferred):**

- Sequence patterns (`__`/`___`, `BlankSequence`/`BlankNullSequence`),
  `Repeated`, `Except`, `Longest`/`Shortest`, `Replace` level-specs — these
  are still open even in Wolfram's *native* pipeline per
  [`MA04`](MA04-wolfram-language.md) (W-20's own deferred list); SIR23
  tracks that scope exactly rather than getting ahead of the native frontend.
- Arbitrary assumption contexts (`assume(x >= 0)`) as IR-level data — the
  frontend resolves what it can at lowering time; an unresolved assumption
  stays an unevaluated `SymApply`, exactly as the native runtimes do today.
- The `cas-*` algorithm surface itself (`Expand`/`Factor`/`Solve`/`D`/
  `Integrate`, …) is **not** new IR — those are ordinary `SymApply` nodes
  with well-known heads (`"Expand"`, `"Factor"`, …); whether a *backend*
  can evaluate them is a runtime-library question (does
  `sir-runtime-symbolic` implement a `Factor` evaluator?), not an IR
  question. First cut ships pattern-matching + rewriting infrastructure;
  wiring the full `cas-*` algorithm library into `sir-runtime-symbolic` is
  separate, later work, matching how the native Wolfram frontend itself
  still has this as an open "Future" item.

## New `SirType` variants

```text
SirType::SymExpr    -- an opaque symbolic-expression handle; carries no
                        static shape (mirrors symbolic_ir::IRNode's own
                        dynamically-shaped tree)
SirType::Rational    -- shared with SIR22
SirType::Complex     -- shared with SIR22
```

## New `Expr` variants

All new variants carry `span`. The five core variants are named to mirror
`symbolic_ir::IRNode` one-for-one:

```text
SymSymbol { name: String, span }                  -- IRNode::Symbol
SymRational { numer: i64, denom: i64, span }      -- IRNode::Rational
                                                      (reduced form; frontend
                                                      normalizes exactly as
                                                      IRNode::rational does)
SymApply {
    head: Box<Expr>,       -- usually a SymSymbol, but a computed head
                              (`f[x][y]`) is legal — head is an Expr, not
                              a bare string, unlike SIR22's simpler shapes
    args: Vec<Expr>,
    span,
}
```

(`IntLit`/`FloatLit`/`StrLit` already exist in SIR10/SIR16 and are reused
directly for `IRNode::Integer`/`Float`/`Str` — no new literal nodes needed
for those three.)

### Patterns

```text
SymPatternBlank { head: Option<Box<Expr>>, span }
    -- Wolfram `_` (head: None) or `_h` (head: Some(SymSymbol("h")))

SymPatternNamed { name: String, pattern: Box<Expr>, span }
    -- Wolfram `x_` desugars to SymPatternNamed { name: "x",
       pattern: SymPatternBlank { head: None } }; `x_h` to
       SymPatternNamed { name: "x", pattern: SymPatternBlank { head: Some(h) } }
```

### Rules and replacement

```text
SymRule { lhs: Box<Expr>, rhs: Box<Expr>, delayed: bool, span }
    -- delayed: false is `->` (Rule); true is `:>` (RuleDelayed)

SymReplaceAll {
    expr:     Box<Expr>,
    rules:    Vec<Expr>,   -- each element is a SymRule (or a SymApply
                              evaluating to a list of rules — the frontend
                              may leave this as a runtime concern)
    repeated: bool,        -- false: `/.` one pass; true: `//.` fixed point
    span,
}
```

## Matcher semantics (binding contract every backend must honor)

A backend implementing `SymReplaceAll` must:

1. Walk `expr`'s tree in the same traversal order `cas-pattern-matching`
   uses today — **bottom-up (post-order)**: a node's `head` and every
   `args` element are visited (and possibly replaced) before the node
   itself is tried against any rule. (An earlier draft of this bullet said
   "top-down, left-to-right over `args`" — that never matched what
   `cas-pattern-matching`'s `rewrite()` actually does; corrected here to
   match the real, tested algorithm this spec's own "port, not a
   reimplementation" framing below commits to.)
2. At each subtree, try each `rules[i].lhs` in order; the first structural
   match wins (no backtracking across rules — matches the reference CAS
   behavior).
3. A `SymPatternBlank { head: None }` matches any subtree; `head: Some(h)`
   matches only a `SymApply` whose `head` structurally equals `h` (or a bare
   `SymSymbol`/literal whose "head" is its own type tag, matching Wolfram's
   `Head[]` convention).
4. A `SymPatternNamed` binds `name` to the matched subtree for the rest of
   that match attempt; a second occurrence of the same `name` elsewhere in
   `lhs` requires structural equality with the first binding (not just any
   match) — this is the one place a matcher needs backtracking-free
   consistency checking, not full unification.
5. On match, substitute bound names into `rhs` and replace the subtree.
   `delayed: false` rules substitute into an already-evaluated `rhs` (built
   once, at rule-construction time, per Wolfram `Rule` semantics); `delayed:
   true` rules re-run substitution fresh per match (`RuleDelayed`).
6. `repeated: true` reruns the whole pass until no rule fires (a fixed
   point) or a backend-defined iteration cap is hit — every backend **must**
   enforce a cap here; an unbounded `//.` is a guaranteed non-terminating
   program for some inputs and every runtime in this repo enforces DoS caps
   on unbounded constructs (see `Range`/list-op caps already used by the
   native Wolfram runtime, `MA04` §10.3/§13.3/§19.5, for the established
   convention this must match).

This contract is deliberately identical to `cas-pattern-matching`'s existing
`match_pattern`/`rewrite` algorithm — `sir-runtime-symbolic` is a **port**,
not a reimplementation from a blank slate.

## New `Feature` flags

```text
Feature::SymbolicExpr
Feature::PatternMatching
Feature::Rationals    -- shared with SIR22
Feature::Complex      -- shared with SIR22
```

## Effects

`SymApply`, `SymRule`, `SymReplaceAll` are all `Pure` — pattern matching and
substitution are deterministic and side-effect-free. (A CAS session-level
side effect, like Macsyma's `assume`/`kill`, stays entirely inside each
frontend's own runtime and is out of scope for the IR — same "session state
isn't SIR" boundary the native runtimes already draw.)

## Backend impact

- **JS/TS**: new `match` arms construct/consume a tagged term-tree value at
  runtime via `sir-runtime-symbolic` (`__SirSym.apply(head, args)`,
  `__SirSym.replaceAll(expr, rules)`, `__SirSym.replaceRepeated(expr, rules,
  cap)`), imported only when `Feature::SymbolicExpr`/`PatternMatching` is in
  the manifest.
- **Rust/Go/Python backends**: not required in this first wave; they reject
  modules declaring `SymbolicExpr`/`PatternMatching` per the existing
  capability-rejection path.

## Versioning

Additive extension, same discipline as SIR16/SIR18/KW1/SIR22. Modules using
SIR23 nodes bump `metadata.sir_version`.

## References

Internal: [`HML01`](HML01-math-to-semantic-ir.md),
[`SIR10`](SIR10-narrow-waist-semantic-ir.md),
[`SIR22`](SIR22-array-matrix-semantic-ir.md) (sibling domain, shared
`Rational`/`Complex`), `symbolic-ir` (the `IRNode` shape this spec mirrors
1:1), `cas-pattern-matching` (the matcher algorithm `sir-runtime-symbolic`
ports), [`MA04`](MA04-wolfram-language.md) (the frontend that will emit
these nodes; also the source of the deferred sequence-pattern scope list).

## Addendum — SIR23 symbolic evaluator + per-language display convention

### Why now

This spec's own "Explicitly out of scope (deferred)" section always said the
quiet part out loud: *"whether a backend can evaluate them is a runtime-library
question ... wiring the full `cas-*` algorithm library into
`sir-runtime-symbolic` is separate, later work."* That was a deliberate,
disclosed deferral, not an oversight. It stayed abstract until
`derive-to-semantic-ir`'s oracle/golden tests (PR #8754,
`derive-to-semantic-ir/tests/oracle.rs`, and its `CHANGELOG.md`'s `[0.1.1]`
entry) gave it a concrete, measured shape: of a 38-case corpus cross-checking
`derive-runtime` (ground truth, `symbolic-vm`-backed) against
`derive_to_semantic_ir::compile_source` → `semantic_ir_to_javascript::compile`
→ real `node` execution, only **4 cases** (bare integer/float/symbol atoms)
agree today. The other 34 are marked `known_bug` and excluded from the
compiled-side assertion, for two confirmed, disjoint reasons, both rooted in
`semantic-ir-to-javascript`, not in any frontend's own lowering:

1. **No evaluator of any kind.** `Expr::SymApply` compiles unconditionally to
   `__Sir.Symbolic.apply(head, [args])` (`emit.rs`'s `Expr::SymApply` arm,
   confirmed unchanged by this addendum's own research) — a pure, inert term
   constructor. `1 + 2*3` stays `Add(1, Mul(2, 3))`; `x := 5` / `x + 1` never
   binds `x`, so the second statement stays `Add(x, 1)`; `DIF(x^2, x)` stays
   `D(Pow(x, 2), x)`; `5 > 3` stays `Greater(5, 3)`; `F(x) := x*x` / `F(5)`
   never dispatches. `Symbolic.replaceAll`/`replaceRepeated` (`runtime.rs`)
   ARE real, working pattern-rewrite engines — but they only fire for an
   explicit `expr /. rules` a frontend's own grammar can emit, and none of
   Derive/Wolfram/Macsyma/Reduce/Maple's *ordinary* arithmetic/assignment/
   function-call/comparison programs ever construct a `SymReplaceAll` node,
   so that machinery is simply never reached by the 34 failing cases.
2. **No per-source-language SIR23 display convention.** `Symbolic.
   toDisplayString` renders every compound term generically as `head(args)`
   — `Add(x, 1)`, `List(1, 2, 3)`, `Neg(x)` — with no infix, no bracket
   convention, no case-bridging back to a source language's own surface
   spelling. Unlike the SIR22 array domain (`ArrayRt.fmtNum`/`display`,
   already gated by `SIR_DISPLAY_APL_HIGH_MINUS`/`SIR_DISPLAY_J_UNDERSCORE`
   — see `emit.rs` lines 118–160 and `runtime.rs` lines 57–114), SIR23 has no
   such mechanism for any source language yet.

`derive-to-semantic-ir`/`CHANGELOG.md` and `HML01-math-to-semantic-ir.md` §5
both note this is **not** a Derive-specific gap: it is the same reason
`wolfram-to-semantic-ir` and `macsyma-to-semantic-ir` — Stream B's other two
shipped, e2e-tested frontends — have never shipped an oracle file of their
own, and it is what blocks `reduce-to-semantic-ir`/`maple-to-semantic-ir`
(shipped, e2e-tested, but likewise oracle-less) from getting one either.

**No new IR surface is needed.** This addendum adds zero `Expr` variants,
zero `SirType` variants, and zero `Feature` flags — every construct it
evaluates or displays already exists as an ordinary `SymApply` with a
well-known head, exactly as the original spec's deferral predicted. This is
purely a `semantic-ir-to-javascript` codegen + runtime-library change (plus,
by the same reasoning, a future `sir-runtime-symbolic`/TS-backend change —
see "Scope boundary" below). `metadata::CURRENT_SIR_VERSION` does **not**
bump for this addendum, unlike SIR22's APL addendum (which added real `Expr`
variants and bumped `"3"` → `"4"`) — there is no new SIR text token here for
a validator to need to recognize.

### Architecture decision: a dedicated head-dispatch evaluator, not rewrite-rules-through-the-existing-engine

**Decision: build `Symbolic.evalTerm(term)` as its own recursive, per-head
dispatch function inside the `Symbolic` IIFE — a direct JS port of
`symbolic-vm`'s `VM`/`Backend` architecture — rather than expressing
evaluation as a set of `SymRule`s fed through the existing `matchPattern`/
`applyRuleTerm`/`replaceAllTerm` machinery.**

`symbolic-vm` (`code/packages/rust/symbolic-vm/src/{vm,backend,backends,
handlers}.rs`) is the ground truth every oracle test's "native side" already
calls (`derive-runtime`, and — confirmed by each crate's `Cargo.toml` — also
`wolfram-runtime`, `macsyma-runtime`, `reduce-runtime`, `maple-runtime`), and
its own architecture already answers this question directly, because it
*itself* implements evaluation and rewriting as two separate mechanisms, not
one:

- **Evaluation** (`vm.rs::VM::eval`/`eval_symbol`/`eval_apply`) is a
  recursive tree-walk dispatching on a plain `head_name → Handler` map
  (`backend.rs`'s `Handler = Arc<dyn Fn(&mut VM, IRApply) -> IRNode>`,
  populated by `handlers.rs::build_handler_table`). A handler for `Add`
  literally computes a sum; it is not expressed as a `Rule`/`RuleDelayed`
  term matched and substituted through `cas-pattern-matching`'s engine.
- **Rewriting** (`SymReplaceAll`/`SymReplaceRepeated`, ported in `runtime.rs`
  as `matchPattern`/`substituteTerm`/`applyRuleTerm`/`replaceAllTerm`/
  `replaceRepeatedTerm`) is `symbolic-vm`'s `Backend::rules()` hook — a much
  smaller, optional, purely-syntactic seam checked *before* handler dispatch,
  used for cheap rewrites, not the primary evaluation path.

Porting the ground truth's actual, tested architecture 1:1 (as this repo's
own "port, not a reimplementation" discipline for `sir-runtime-symbolic`
already commits to for the matcher) is lower-risk than inventing a new
"evaluation via rewrite rules" design, for four concrete reasons:

1. **It is what the reference implementation does.** Every arithmetic/
   comparison/held-form/calculus fold this addendum needs already exists,
   correct, in `handlers.rs`; porting it head-by-head is mechanical and
   directly re-provable against `derive-runtime`'s own behavior (the same
   oracle corpus). Recasting it as rewrite rules would be a fresh design
   with no reference to check against.
2. **Arithmetic folding gains nothing from pattern matching.** `Add(2, 3) →
   5` needs no variable binding, no repeated-name consistency check, no
   bottom-up fixed-point walk — the one thing `matchPattern`/`applyRuleTerm`
   provide that a plain `if (a.kind === "integer" && b.kind === "integer")`
   dispatch doesn't. Dressing every arithmetic identity as a `SymRule` whose
   `rhs`-transform closure just... computes the sum is `Handler` by another,
   more expensive name.
3. **`SymReplaceAll`/`SymReplaceRepeated` are a different IR-level construct
   with different semantics.** They are `Pure` (see this spec's own
   "Effects" section above) and explicit — `expr /. rules` only ever fires
   when a frontend's grammar actually emits that node (Derive's grammar
   never does, MA07 §4). Implicit evaluation of *every* program a frontend
   compiles is a different, always-on semantics; conflating the two would
   blur a distinction SIR23 deliberately keeps (opt-in term rewriting vs.
   ordinary program evaluation).
4. **Held forms need real side effects, not tree rewriting.** `Assign`
   mutates an environment that the *next* statement reads — a stateful,
   ordered effect fundamentally incompatible with `ReplaceAll`'s declared
   `Pure`, order-independent-within-a-pass contract.

The `Backend` trait's layering (`backends.rs`) surfaces one more finding
that changes this addendum's scope in a good way: `StrictBackend`/
`SymbolicBackend` are the **core**, undecorated evaluator (arithmetic,
comparison, logic, held forms, `D`/`Integrate`, `Factor`/`Apart`,
`Assume`/`Forget`). `WolframBackend` (`wolfram-runtime/src/backend.rs`) and
`MacsymaBackend` (`macsyma-runtime/src/lib.rs`) *decorate* a
`SymbolicBackend`/`StrictBackend` with their own large, language-specific
extension tables (`wolfram-runtime/src/builtins.rs`: `Length`/`First`/
`Last`/`Part`/`Append`/`Range`/`Map`/`Apply`/`Table`/`Do`/`Sum`/`Product`/
`With`/`Module`/`Block`/`Sort`/`Reverse`/`Join`/`Flatten`/`Select`/`Count`/
`Total`/`EvenQ`/`OddQ`/`Nest`/`NestList`/`Fold`/`FoldList`/`Mod`/
`StringJoin`/… ; `macsyma-runtime/src/lib.rs`: `Display`/`Ev`/`Float`/
`Solve`/`Simplify`/`Expand`/`Subst`/`TrigSimplify`/`TrigExpand`/
`TrigReduce`/list ops/…), falling back to the core handler only when their
own table misses (`handler_for` = "check mine, `.or_else` the inner
backend's"). By contrast — confirmed directly in each crate's `src/lib.rs`
— **`derive-runtime`, `reduce-runtime`, and `maple-runtime` all construct
`SymbolicBackend::new()` completely unchanged**, no decorator at all. That
means the *core* evaluator this addendum scopes is not just "the shared
substrate" in the abstract — it is **the entire evaluation semantics** three
of the five Stream B frontends need, full stop. Wolfram's and Macsyma's own
decorator-layer builtins are real, substantial, but explicitly a **separate,
later, per-frontend task** (matching how their native runtimes already
structure it as an independent extension), not part of this rollout.

### Environment / held-form execution model

`BaseBackend` (`symbolic-vm/src/backend.rs`) is the port template: one flat
`env: HashMap<String, IRNode>` and one `held: HashSet<String>` (`{"Assign",
"Define", "If", "Assume", "Forget"}`), shared by both reference backends.
The JS port mirrors this with a plain `Map` living inside the `Symbolic`
IIFE's closure, constructed once per compiled program's execution (a Derive/
Wolfram/etc. program is one flat top-level script/session, exactly matching
one `BaseBackend` instance per `VM`):

- `HELD_HEADS = new Set(["Assign", "Define", "If"])` for this rollout.
  `Assume`/`Forget` are excluded from the first cut: they exist to mutate
  `symbolic-vm::VM::assumptions` (`cas_simplify::AssumptionContext`), a
  sign/equality-fact store no Stream B oracle case exercises yet — deferred
  alongside `Factor`/`Apart` below, not silently dropped.
- `Symbolic.evalTerm(term)`: a `Symbol` looks itself up in the env (self-loop
  guard: if the binding is reference-equal/structurally-equal to the
  original term, return it unchanged, exactly mirroring `eval_symbol`'s "`x
  := x` would recurse forever without this" comment) and re-evaluates the
  binding; an `Apply` evaluates its args first *unless* its head is in
  `HELD_HEADS` (mirrors `eval_apply`'s applicative-order-except-held step),
  then dispatches to a per-head handler, then — if no handler matches and
  the head symbol is itself bound to a stored `Define(...)` term — performs
  user-function dispatch (below); anything else returns the (arg-evaluated)
  term unchanged, matching `SymbolicBackend::on_unknown_head`'s pass-through
  policy (the correct policy for every one of these 5 frontends —
  `StrictBackend`'s panic-on-unknown policy is not used by any of them).
- `assign_handler`'s port: evaluate the RHS, bind `name → value` in the env,
  return `value`. `define_handler`'s port: store the *whole* `Define(name,
  params, body)` term under `name`, return `Symbol(name)` (this is why a
  correctly-evaluated `F(x) := x*x` displays as the bare name `F`, never as
  `Define(...)` — see "Display convention" below). `if_handler`'s port:
  evaluate the condition; branch on the resulting `True`/`False` symbol (a
  `2`- or `3`-arg form, matching `symbolic-vm`'s own arity check); if the
  condition doesn't resolve to a boolean symbol, rebuild the unevaluated
  `If(...)` term (free-variable-safe, matches `maple-runtime`'s and
  `reduce-runtime`'s printers' own documented expectation that `If` can
  legitimately still reach a display path).
- **This is not the same environment as `runtime.rs`'s existing OOP/closure
  machinery** (`SirInstance`/`classVarBag`/`currentSelf`, used for SIR's
  class-instance-variable domain) — that machinery models a completely
  different value space (host JS object identity) and reusing it would be a
  type-confusion hazard, not a simplification. A brand-new binding table is
  correct here, but it is a ~10-line `Map`, not a new lexical-scoping/scope-
  chain mechanism: every one of these 5 frontends' currently-implemented
  grammars is single-session/flat (no nested `With`/`Module`/`Block`-style
  local scope reaches `SymbolicBackend` today — those are exactly the
  decorator-layer extensions named out of scope above), so one flat `Map`
  is a faithful, not a simplified, port of `BaseBackend.env`.
- **Genuine, one-place reuse**: user-function dispatch (below) needs
  "substitute these parameter names for these argument terms in this body,"
  which is *exactly* what `substituteTerm` (already in the `Symbolic` IIFE,
  built for `SymRule` RHS substitution) already does — the same helper,
  called with a plain `name → term` bindings `Map` instead of a pattern-
  match `Bindings` result. This is the one real piece of code-sharing
  between the rewrite engine and the new evaluator; everything else is
  deliberately separate per the architecture decision above.

### Function dispatch

Ports `vm.rs::VM::apply_user_function` exactly: on `Apply(head, args)` where
`head` is a `Symbol` with no core handler match, look the symbol up in the
env; if the stored value is a `Define(name, List(params...), body)` term,
zip the (already-evaluated, applicative-order) `args` against `params` by
position (arity mismatch → leave the call unevaluated, matching the Rust
port's `None` return), build a `name → arg` bindings map, `substituteTerm`
it into `body`, and recursively `evalTerm` the result — mirroring `apply_user_function`'s "substitute, then the VM's caller re-evaluates" contract exactly (`vm.rs` line ~144: `if let Some(node) = result { return self.eval(node); }`).

Note for implementers: neither `symbolic-vm::apply_user_function` nor this
port has any recursion-depth guard specific to *user-function* calls — a
genuinely self-recursive Derive/Wolfram/etc. function (`F(x) := F(x) + 1`
called anywhere) is already an infinite loop on the Rust ground-truth side
today. This is not a new gap this addendum introduces; the shared
`MAX_EVAL_DEPTH` cap below (which every `evalTerm` recursion passes through,
including this one) is what converts that shared, pre-existing risk from an
uncontrolled native-stack crash into a controlled, thrown `Error` on the JS
side — a strict improvement over the status quo, not full termination
detection (which `symbolic-vm` itself doesn't have either).

### The one required `emit.rs` change

`Expr::SymApply`'s own codegen arm does **not** change — it keeps emitting a
bare, unevaluated `__Sir.Symbolic.apply(head, [args])` construction, exactly
as today. The wrapping happens exactly **once per top-level statement**, at
`emit_stmt`'s `Stmt::ExprStmt` arm (`emit.rs` line 458): when the statement's
`expr` is one of the three SIR23 root shapes these frontends ever produce at
statement level (`SymApply`/`SymSymbol`/`SymRational` — confirmed exhaustive
by `derive-to-semantic-ir/CHANGELOG.md`'s own disclosed scope, "this crate
therefore only ever constructs `Expr::SymSymbol`/`Expr::SymApply`"), wrap the
existing `emit_expr` output in `__Sir.Symbolic.unwrap(__Sir.Symbolic.
evalTerm(...))` instead of emitting it bare. This mirrors `symbolic_vm::
VM::eval_program`'s own driver exactly — one `eval()` call per top-level
statement, sharing one environment across the whole program — and, because
`evalTerm` recurses into `head`/`args` itself (mirroring `eval_apply`'s own
"evaluate args first" step), a deeply nested expression is evaluated bottom-
up by *one* top-level `evalTerm` call, not once per nested `SymApply` —
avoiding the redundant, potentially-exponential re-evaluation a naive
"wrap every `SymApply` occurrence" approach would cause.

`Symbolic.unwrap` (already implemented and already used by `SymReplaceAll`'s
codegen) needs no change to be reused here: it structurally checks for a
`{kind: "depth-limit" | "rewrite-cycle"}` sentinel regardless of which
function produced it, so `evalTerm`'s own depth-cap failure (below) can
return the same sentinel shape and reuse the existing `unwrap` unchanged.

### Depth/DoS guard

`evalTerm` needs its own recursion-depth cap, distinct from the existing
`MAX_TERM_DEPTH = 512` (which only guards `replaceAllTerm`/
`replaceRepeatedTerm`'s tree walk, a different function with a different
per-frame cost). Per this repo's established CWE-674 methodology (the same
one `derive-to-semantic-ir`'s own `MAX_EXPR_DEPTH = 256` used — "the
`derive-parser`'s own measured bare-stack crash floor, 298 `parse_rule`
frames"), the implementing PR must empirically measure `evalTerm`'s own
per-frame stack cost on a bare default stack before fixing a
`MAX_EVAL_DEPTH` constant, rather than assuming `MAX_TERM_DEPTH`'s existing
512 is safe for a heavier-per-frame function. The same cap incidentally
covers the user-function self-recursion risk noted above, since both
manifest as ordinary deep native-JS recursion through the same function.

### Canonical head → evaluator mapping (shared core, union across all 5 frontends)

Confirmed via `symbolic_ir::lib.rs`'s own head-name constants and
`symbolic-vm::handlers::build_handler_table` — every one of Wolfram/
Macsyma/Derive/Reduce/Maple's `lower.rs` imports these same constants
(`use symbolic_ir::{ADD, SUB, ...}`) for their shared arithmetic/comparison/
logic/held-form/calculus core, confirming the canonicalization this
addendum leans on is real, not assumed:

| Category | Heads | Port notes |
|---|---|---|
| Arithmetic | `Add` `Sub` `Mul` `Div` `Pow` `Neg` `Inv` `Abs` | Needs a small numeric-tower port (`handlers.rs`'s `Numeric::{Int(i64), Rat(i64,i64), Float(f64)}` promotion/GCD-reduction rules) — the exact-rational-result semantics (`1/3` stays `1/3`, not `0.333…`) is not optional, it is directly asserted by oracle case `inexact_division_folds_to_a_rational`. The JS term *constructors* (`Symbolic.rational`/`int`/`numberNode`) already do GCD reduction and safe-integer checks; only the *operations* over them are missing. |
| Comparison | `Equal` `NotEqual` `Less` `Greater` `LessEqual` `GreaterEqual` | Fold to the `True`/`False` **symbol** terms (not a JS boolean, not `1`/`0`) when both sides are numeric literals; stay unevaluated when either side is a free symbol (`equation_with_a_free_variable_stays_symbolic`). |
| Logic | `And` `Or` `Not` | N-ary fold (a flat `And(a, b, c)` chain, matching each frontend's own n-ary chain-fold lowering — not pairwise binary nesting). |
| Held / control | `If` `Assign` `Define` | See "Environment / held-form execution model" above. |
| Elementary functions | `Sin` `Cos` `Tan` `Exp` `Log` `Sqrt` `Atan` `Asin` `Acos` `Sinh` `Cosh` `Tanh` `Coth` `Sech` `Csch` `Asinh` `Acosh` `Atanh` | Identity folds on literal arguments (`Sin(0) → 0`, `Cos(0) → 1`, `Sqrt(4) → 2` exactly, not a float approximation) are what the current oracle corpus exercises; general (non-identity, non-literal) simplification is out of scope. |
| Calculus | `D` `Integrate` | Scoped to what the current corpus needs: `D` on a power/product/`Sin` (sum, power, and chain rules over the elementary functions above); `Integrate` on a bare symbol. **Not** the full `symbolic-vm` calculus/polynomial surface — see "Explicitly out of scope" below. |
| Structural | `List` | **No handler needed at all.** `List` is not held, so applicative-order argument evaluation alone folds `List(Add(1,1), Mul(2,3), Pow(2,3))` into `List(2, 6, 8)` for free — confirmed by `symbolic-vm::list_handler`'s own body being a bare passthrough (`IRNode::Apply(Box::new(expr))`). |

**Explicitly out of scope for this rollout** (confirmed present in
`symbolic-vm`, and materially larger/more advanced than this addendum's own
originating prompt assumed — see "Reality check" below): `Factor`, `Apart`
(and their `hensel.rs`/`n_variate_factor.rs`/`weierstrass_symbolic_
coefficients.rs`/`ibp_tabular.rs` test-covered internals), `Assume`/
`Forget`/`ForgetAll`, and the special-function heads `symbolic_ir` already
reserves constants for (`LegendreP`/`LegendreQ`, `BesselJ`/`BesselY`,
`HermiteH`/`HermiteH2`, `ChebyshevT`/`ChebyshevU`). None of these are
exercised by any Stream B frontend's current or near-future oracle corpus.
Also explicitly out of scope: each frontend's own **decorator-layer**
extension builtins (Wolfram's `Map`/`Table`/`Sort`/… , Macsyma's `Solve`/
`Expand`/`Simplify`/… — see the architecture section above) — separate,
later, per-frontend work, tracked the same way their own native runtimes
already track it.

### Per-language display convention (a separate, parallelizable work item)

Extends the existing, already-proven `SIR_DISPLAY_*` mechanism
(`emit.rs` lines 118–160, `runtime.rs` lines 57–114) rather than inventing a
new one: `emit.rs` already computes mutually-exclusive booleans from
`m.metadata.source_language` and substitutes **hardcoded literal** `"true"`/
`"false"` text into the `RUNTIME` blob before it's emitted — the existing
code's own SECURITY comment is explicit that this must never become
source-derived text, only a boolean-selected fixed literal. This addendum
adds the fourth and fifth (etc.) instance of that exact pattern:
`SIR_DISPLAY_DERIVE` (this rollout), with `SIR_DISPLAY_WOLFRAM`/
`_MACSYMA`/`_REDUCE`/`_MAPLE` following the identical recipe whenever each
of those frontends gets its own oracle file (separate future tasks, per
"Scope boundary" below).

`Symbolic.toDisplayString` gains, gated behind the relevant flag:

- Infix `Add`/`Sub`/`Mul`/`Div`/`Pow`, prefix `Neg`/`Not`, with a precedence
  table — this is a direct JS port of each native `<lang>-runtime::printer::
  print_<lang>`'s existing, already-written precedence ladder (e.g.
  `derive-runtime/src/printer.rs`'s 9-level table; `reduce-runtime`'s/
  `maple-runtime`'s near-identical ones), not new design.
- A per-language `List` bracket convention: Derive's `[a, b, c]` /
  `[a, b; c, d]` (row-separated by `;`, confirmed by `derive-runtime::
  lower::lower_vector`); Wolfram's/Reduce's `{a, b, c}`; Maple's `[a, b, c]`
  for `List` plus a *separate* `Set`-head `{a, b, c}` (`maple-runtime::
  lower::SET`, confirmed distinct from the shared `symbolic_ir::LIST` —
  not currently emitted by `maple-to-semantic-ir`, so out of scope until it
  is).
- A per-language function-call bracket convention: Wolfram's `f[…]`
  (confirmed in `wolfram-runtime/src/printer.rs`) vs. every other language's
  `f(…)`.
- Case-bridging back to a language's own builtin surface spelling: Derive's
  UPPERCASE (`Sin` → `"SIN"`, confirmed by `derive-runtime::printer::
  print_derive`'s reverse bridge) is the only one this rollout's own oracle
  corpus needs; other languages' bridges (if any) are deferred with their
  own display-convention item.

**A finding that shrinks this item's real scope**: once the evaluator above
is ported faithfully, `Assign`/`Define` **never reach the display path at
all** — `assign_handler`'s port returns the bound *value*, not an
`Assign(...)` term, and `define_handler`'s port returns `Symbol(name)`, not
a `Define(...)` term — exactly matching every native printer's own
documented invariant ("`Assign`/`Define` never appear in a printed result... this module has no rendering case for either head at all," `reduce-runtime`/
`maple-runtime`'s printers, verbatim). So `toDisplayString` needs **no**
special-casing for those two heads — only infix/prefix/bracket/case-bridging
work is in scope here.

**Scope boundary for this rollout**: only **Derive's** own convention is in
scope — it is the only Stream B frontend with an oracle corpus proving it
today. Wolfram/Macsyma/Reduce/Maple's own conventions are deferred to
whenever each of those frontends gets its own `tests/oracle.rs` (see
"Verification strategy" below) — separate future tasks, not part of this
rollout's own PR list, though the mechanism (one more boolean flag +
printer port, following this section's exact recipe) is now proven three
times over (Ruby, APL, J) before this addendum and does not need
re-justifying each time.

Also out of scope for this rollout, noted for completeness: the published
`@coding-adventures/sir-runtime-symbolic` TS package
(`code/packages/typescript/sir-runtime-symbolic/src/index.ts`) has the
*identical* gap — confirmed by reading its `CHANGELOG.md`: it only re-exports
`cas-pattern-matching`'s matcher and `symbolic-ir`'s leaf-term constructors,
no evaluator. `semantic-ir-to-typescript`'s SIR23 codegen (which imports that
package rather than inlining it, per this backend's own established
import-vs-inline split) would need a mirrored addition for TS-backend
parity, but no Stream B frontend's oracle testing runs through the TS
backend today, so it is out of scope here — a follow-up once one does.

### Crate layout and rollout (one item = one PR)

All items touch only `code/packages/rust/semantic-ir-to-javascript/src/
{runtime.rs, emit.rs}` — a single crate, so sequence to avoid contending
with any other in-flight PR in that crate (mirrors `sir-display-convention.md`'s
own "sequenced to avoid contended crates" rollout wisdom). No other crate
changes.

1. **`Symbolic.evalTerm` scaffold + arithmetic/comparison/logic folding.**
   The foundational PR: the recursive `evalTerm` dispatcher, the `emit.rs`
   `Stmt::ExprStmt` wrapping change, the `MAX_EVAL_DEPTH` guard (with its own
   empirical stack-depth measurement test), and handlers for `Add`/`Sub`/
   `Mul`/`Div`/`Pow`/`Neg`/`Inv`/`Abs`/comparisons/`And`/`Or`/`Not`. No
   environment, no held forms yet — `Assign`/`Define`/`If` are declared held
   but have no handler, so (matching `on_unknown_head`'s pass-through
   policy) they safely stay inert data, identical to today's behavior, until
   item 2 lands. **No dependency** — this is the foundation every other item
   builds on. Flips roughly 14 of the 34 `known_bug` oracle cases to fully
   green on its own (e.g. `multiplication_binds_tighter_than_addition`,
   `power_is_right_associative`, `inexact_division_folds_to_a_rational`,
   `comparison_true`, `three_term_and_chain_folds_n_ary` — the existing
   `toDisplayString` already renders a bare integer/rational/`True`/`False`
   symbol correctly, so no display work is needed for these).
2. **Held-form execution: environment + `Assign`/`Define`/`If` + user-
   function dispatch.** Adds the `Map`-backed environment, the three held-
   form handlers, the self-loop guard, and `apply_user_function`'s port
   (reusing `substituteTerm`). **Depends on item 1's scaffold** (same
   dispatch table/`HELD_HEADS` set), independent of item 3. Flips 5 more
   cumulative cases (`variable_assignment_and_later_reference`,
   `single_param_function_definition_and_call`,
   `multi_param_function_definition_and_call`, `if_true_branch`,
   `if_false_branch`).
3. **Calculus/elementary-function handlers**, scoped exactly to the table
   above (not the full `symbolic-vm` polynomial/special-function surface).
   **Depends on item 1's scaffold**; independent of item 2 except for one
   shared oracle case. Flips 3 more cases outright
   (`sin_of_zero`/`cos_of_zero`/`sqrt_of_a_perfect_square`, whose folded
   results are plain integers needing no display work); the `DIF`/`INT`
   cases whose folded results are still compound terms (`Mul(2, x)`, etc.)
   wait on item 4.
4. **Derive's own SIR23 display convention** (infix/prefix/bracket/case-
   bridging, scoped to Derive only, per "Scope boundary" above). **No code
   dependency on items 1–3** (touches only `toDisplayString`, a different
   function in the same file) — can be developed and reviewed in parallel,
   but land it sequenced with the others to avoid a same-file merge
   collision. Flips the remaining 12 cases, including the ones needing both
   evaluation *and* display (e.g. `dif_differentiates_a_power` needs item 3
   for the `D` fold and item 4 for the infix `2*x` rendering) and the 5 pure
   display-only cases that need no evaluation at all (`flat_vector_literal`,
   `two_by_two_matrix_literal`, `equation_with_a_free_variable_stays_
   symbolic`, etc.).

Each item: `/security-review` before push (even though this is IR-codegen/
runtime work, not IR-surface work — the existing `SIR_DISPLAY_*` SECURITY
comment's "never source-derived text" invariant must hold for every new
flag), full `cargo test --workspace` (this crate is shared by every existing
frontend/backend per `HML01` §7), and a run of `derive-to-semantic-ir`'s
`tests/oracle.rs` to confirm exactly which `known_bug` markers flip (see
below) before moving to the next item.

### Verification strategy

**The acceptance signal is `derive-to-semantic-ir/tests/oracle.rs`'s
`known_bug` markers flipping to `known_bug: None`.** This is a significant
practical win: the existing 38-case corpus becomes a real, evaluation-aware
regression suite for Derive **for free** — no new test-writing is needed for
Derive specifically as each rollout item lands; the corpus was written to
already assert the *correct*, fully-evaluated `expected` value on the
ground-truth side, deliberately anticipating this fix (see `oracle.rs`'s own
`Case::known_bug` doc comment: `Some(reason)` names "which documented
shared-crate gap ... is responsible," implying the natural next step is
removing entries from this list, not writing new ones). After all four
rollout items land, all 38 cases should read `known_bug: None`; if any case
still disagrees, that is either a genuinely new bug (fix it) or evidence
this addendum's own scoping was wrong somewhere above (revisit the relevant
section, don't force the corpus to match a broken assumption).

Beyond Derive: this rollout, once landed, is what makes it *possible* for
each of the other four Stream B frontends to get their own `tests/
oracle.rs` — a **separate future task each**, not part of this rollout —
using the same harness pattern (`node_available` skip-guard, `Case`/
`CORPUS`, `ground_truth`/`compiled` pair). Per the architecture finding
above: `reduce-to-semantic-ir`'s and `maple-to-semantic-ir`'s future oracle
files should be able to reach **full** parity from this rollout alone (their
native runtimes use `SymbolicBackend` unchanged, no decorator);
`wolfram-to-semantic-ir`'s and `macsyma-to-semantic-ir`'s future oracle
files will still find real gaps in their own decorator-layer builtins
(`Map`/`Table`/`Sort`/… ; `Solve`/`Expand`/`Simplify`/… ) — expected,
already-scoped-out gaps, not a sign this rollout under-delivered.

### Reality check: what this research found that the originating scope assumed differently

- **The shared canonical-head vocabulary claim holds, and holds more
  strongly than assumed**: not just "the same heads," but three of five
  frontends (`derive-runtime`, `reduce-runtime`, `maple-runtime`) use
  `symbolic-vm::SymbolicBackend` **completely unmodified** — confirmed by
  reading each crate's `src/lib.rs` directly. The shared core this addendum
  scopes is not a lowest-common-denominator subset; it is the *entire*
  evaluation semantics for 3 of the 5 frontends.
- **`symbolic-vm` is a substantially larger, more mature CAS than "arithmetic
  + comparison + a few elementary functions"** — it has working partial-
  fraction decomposition (`Apart`, `tests/apart.rs`), Hensel lifting
  (`tests/hensel.rs`), n-variate polynomial factoring (`tests/
  n_variate_factor.rs`), Weierstrass symbolic-coefficient integration
  (`tests/weierstrass_symbolic_coefficients.rs`), tabular integration by
  parts (`tests/ibp_tabular.rs`), an assumption-context store
  (`cas_simplify::AssumptionContext`), and reserved special-function heads
  (Legendre/Bessel/Hermite/Chebyshev) with no handlers registered yet. None
  of this is needed by any current or near-future oracle corpus, so this
  addendum draws an explicit, deliberate line well short of all of it — see
  "Explicitly out of scope" above — rather than treating "port `symbolic-vm`"
  as one undifferentiated task.
- **Oracle-test correctness only requires matching the *displayed string*,
  not the internal term-tree shape.** `symbolic-vm`'s `add_handler` does
  nested-`Add` flattening/canonicalization (`Numeric` accumulation across a
  flattened chain) to support a Macsyma-specific summation-telescope
  detector — a niceity this port can skip entirely, since oracle tests
  compare `derive-runtime`'s printed output against `node`'s printed
  `stdout`, never the two sides' internal `IRNode`/term-tree shapes. This
  measurably de-scopes the arithmetic-folding port: correct final values,
  not bit-for-bit-identical canonical trees.
- **`sir-runtime-symbolic` (the published TS package this JS code is a port
  of) has the identical gap**, confirmed by reading its own `CHANGELOG.md`
  — it is not a case of "the JS backend regressed relative to its own
  reference package"; the reference package itself never had an evaluator
  either. Noted above as an explicit, separate follow-up rather than
  silently expanded into this rollout.
