# coding-adventures-closure-pass-inline

Function-inlining pass for the Closure Compiler clone. Substitutes
a callee's body at the call site when doing so is cheaper than the
call. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## What it does

```js
// before
function double(x) { return x * 2; }
log(double(7));

// after inline (the call is replaced; the now-dead declaration is
// removed by the later remove-unused-vars / treeshake passes)
log(7 * 2);

// after the next constant-fold iteration
log(14);
```

That cascade — inline exposes new constant-folding opportunities —
is exactly why `IterationPolicy::FixedPoint` is the right policy.

## The two questions every inliner answers

1. **Is it safe?** Substituting the body must not change
   semantics. `this`, `arguments`, captured variables, recursion,
   and side-effecting argument expressions used multiple times all
   need careful handling. The sidecar's `no_side_effects` / `pure`
   attributes plus a per-parameter use-count analysis answer most
   of it.
2. **Is it worth it?** Inlining a 1000-line function at 50 call
   sites bloats output. Inlining a 3-line single-use helper
   shrinks it. CLOC06 leaves the exact heuristic open.

## What's here

- `InlinePass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "inline"`
  - `depends_on = ["constant-fold"]` — folded arguments plug
    into parameters cleanly.
  - `iteration_policy = FixedPoint` — inlined bodies expose new
    inlineable calls.
  - `cost = 4` — shadow/use analysis + clone-and-substitute per
    inlined call site.
- `Pass::run` is a **real transform** over the Phase-1 AST. It is
  self-contained — its own name-based shadow and use-count walk,
  in the spirit of the `rename` pass — and does not depend on
  `closure-scope-analyzer` (whose candidate scan keyed on optional
  per-node CvIds the bridge does not populate).

## The provably-safe slice

Rather than answer the hard inlining questions with heuristics,
this slice inlines only the subset where each hazard is
*structurally impossible*. A call `f(a₁, …, aₙ)` is inlined when:

1. **`f` is a top-level plain `function`** (not generator / not
   `async`) — no enclosing scope to capture, no resumable state.
2. **`f`'s body is exactly `{ return EXPR; }`** — a pure
   expression-for-expression swap; nothing to splice.
3. **Every identifier in `EXPR` is a parameter** — the capture
   guard. No free identifiers ⇒ no global capture, no
   `this`/`arguments`, recursion excluded for free.
4. **`f`'s name is declared exactly once in the program** — no
   shadowing, so uses resolve to this function by name alone.
5. **Every use of `f` is an inlinable call** with matching arity —
   no value use (`g(f)`), no wrong-arity / side-effecting call. We
   inline *all* the calls so `f` ends up unreferenced; if any use
   isn't an inlinable call we decline the whole function (partial
   inlining duplicates the body *and* keeps the declaration).
6. **Every argument is side-effect-free** (literal or bare
   identifier) — substitution can neither drop nor duplicate a
   side effect.

Everything outside this subset is left untouched (`changed` stays
`false`). The now-dead callee declaration is **not** removed here —
the later `remove-unused-vars` / `treeshake` passes delete it.

### Single-use vs. multi-use — the only "is it worth it?" knob

Rules 1–6 are all about *soundness*. The remaining question — is it
worth it? — splits on call-site count:

- **One site** → always inline (a strict win).
- **N > 1 sites** → inline only when the body fits the budget
  `expr_node_count(body) <= 2 + params.len()`, so the substituted
  body is no larger than the call it replaces and duplicating it
  across the sites never grows the output. A larger body is left
  alone.

Substitution happens on the typed AST, so the precedence-aware
`closure-emitter` adds any parentheses the new tree shape needs.

## Where this pass sits

CLOC06 §"Canonical pass set" pins:

```text
constant-fold → fold-control-flow → dce → inline → rename → ...
```

Inline runs **after DCE** so it doesn't bother inlining callees
that are about to be deleted, and **before rename** so the
inliner's heuristic sees meaningful names. Both relationships
are *preferences*, not *correctness*, so they're not in
`depends_on` — only the constant-fold dependency is.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output and
  the typed AST the transform walks.

(The transform path no longer uses `closure-scope-analyzer`,
`type-sidecar`, `correlation-vector`, or `serde_json`; those remain
declared for parity with the sibling pass crates.)

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-constant-fold` for the
  two-pass ordering integration test.
- `coding-adventures-javascript-parser` + `coding-adventures-closure-emitter`
  for the source → bridge → inline → emit roundtrip tests.
