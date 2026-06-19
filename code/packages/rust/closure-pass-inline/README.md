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

## Beyond the expression shape — void multi-statement helpers (CLOC15)

A real helper is usually several statements, not a single `return`. A
second, statement-level path (the first slice of
[CLOC15](../../../specs/CLOC15-multi-statement-inlining.md)) splices a
**single-use void multi-statement helper** at its call site:

```js
function track(n, v) { const e = n + v; metrics.push(e); }
track(a, b);
// SIMPLE  ⇒  const c = a + b; metrics.push(c);
```

Replacing one call statement with several body statements is a 1 → N
splice the expression walker (which only swaps an expression in place)
structurally cannot do. It is admitted only under a tight, sound subset
— each condition a hard reject:

1. **Single-use, single-declaration** — name declared once, used once.
2. **The use is a discarded statement call** (`track(…);`), not a value
   position (`x = track(…)`). No result to capture (value capture is a
   later slice).
3. **Straight-line body to an optional tail `return`** — each statement is
   an expression statement or a `let`/`const` declaration, plus an optional
   `return` as the *final* statement; nothing else (no *early* return).
4. **No `this` / `arguments`** — frame-bound, would rebind on a splice.
5. **Callee locals alpha-renamed to program-fresh names** before
   splicing — a spliced `let e` can never collide with the call-site
   scope.
6. **Free identifiers must be true globals** — declared nowhere, so
   unshadowable at the splice site (a conservative bootstrap; a later
   slice widens it via `closure-scope-analyzer`).
7. **Side-effect-free arguments** — the same `is_simple_arg` gate.

Since the call site discards the result (CLOC15 PR-2), a **tail `return E`**
is normalized: dropped when `E` is provably inert (a literal or a bare
parameter/local read), else kept as `E;` for its side effects. A bare
*global* read is kept (it can throw `ReferenceError`). This also reaches a
shape the expression inliner cannot — a single `return g()` with a free
global `g` becomes `g();`.

```js
function init(n) { setup(n); return ready(); }
init(cfg);
// SIMPLE  ⇒  setup(cfg); ready();
```

An unbraced single-statement slot (`if (c) f();`) gets the spliced body
wrapped in a block; a real statement list is spliced flat. As with the
expression path, the now-dead declaration is left for `remove-unused-vars` /
`treeshake`.

### Result used — capture into a hoisted temp (CLOC15 PR-3)

When the result *is* used, the body is hoisted before the enclosing
statement and the tail-return value captured into a fresh temp:

```js
function compute(a) { const t = a + 1; return t * 2; }
var x = compute(5);
// SIMPLE  ⇒  var x = 12;   (hoist + capture, then fold + propagate + treeshake)
```

The soundness crux is **evaluation order**: hoisting the body before the
statement runs it before anything else that statement evaluates, so the only
airtight subset is when the call is the **entire initializer of a
single-declarator** `var`/`let`/`const` — nothing is evaluated before it and
an initializer is never short-circuited. `var x = a + f()` (reorders `a`),
`var x = f(), y = …` (multi-declarator), `var x = h(f())` (not the top
expression), and a body with no tail-return value are all declined. Broader
value positions (assignment targets, `return` arguments) are later slices.

### Non-simple arguments — per-argument temps (CLOC15 PR-4a)

The statement-inlining paths no longer require arguments to be simple. When
any argument is non-simple, every argument is hoisted into a fresh `const`
temp, in source order, before the spliced body, and each parameter is
substituted by its temp:

```js
function log2(x) { trace(x); record(x); }
log2(compute());
// SIMPLE  ⇒  const a = compute(); trace(a); record(a);
```

This is exact JS call semantics — all arguments evaluated left-to-right,
once each, before the body — so a side-effecting argument runs once even
when its parameter is used many times (never duplicated to
`trace(compute()); record(compute())`). When **all** arguments are simple the
old direct-substitution path is kept (no temps), so existing output is
unchanged. Composes with PR-3 for value-position calls.

### Conditional bodies — `if` without an early exit (CLOC15 PR-4b)

A helper body may contain an `if`, as long as it is control-flow-inert: each
branch is an expression statement or a block of expression statements — no
`return`/`break`/`continue` (an early exit a flat splice would mis-scope) and
no nested declaration (a block-scoped local the name-based renamer cannot
shadow-correctly):

```js
function guard(x) { if (x > 0) accept(x); else reject(x); }
guard(value);
// SIMPLE  ⇒  value > 0 ? accept(value) : reject(value);
//   (inlined, then fold-control-flow makes the ternary, treeshake drops the decl)
```

The `if`'s test is unrestricted (its identifiers are vetted normally).
Nested `if` / loops are kept for a later slice.

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
