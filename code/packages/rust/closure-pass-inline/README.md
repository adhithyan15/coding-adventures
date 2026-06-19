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
   an expression statement or a `var` / `let` / `const` declaration, plus an
   optional `return` as the *final* statement; nothing else (no *early*
   return).
4. **No `this` / `arguments`** — frame-bound, would rebind on a splice.
5. **Callee locals alpha-renamed to program-fresh names** before
   splicing — a spliced `let e` can never collide with the call-site scope.
   This is what makes admitting a `var` local sound (CLOC15 Open Q3): a `var`
   hoists to the caller-function top on a flat splice, but a name that appears
   nowhere else in the program is observationally inert wherever it hoists.
6. **Free identifiers must be true globals, uniquely-declared top-level
   names, or top-level names spliced only at a top-level site.** A free ident
   declared *nowhere* is a true global — unshadowable everywhere, spliceable
   anywhere. A free ident resolving to a **top-level declaration** (a sibling
   `function`, a top-level `const`/`let`/`var`) is admitted by **CLOC16**:
   - if it is declared **exactly once** program-wide (`decl_counts == 1`), no
     other binding of the name exists, so it is unshadowable everywhere and
     splices anywhere (**Slice B1**);
   - if it is **also** declared elsewhere (`decl_counts > 1`), a local could
     shadow it at a nested site, so the candidate is top-level-only — it
     splices **only when its single call is a direct `program.body` member**
     (**Slice A**); at any nested call site it is declined.

   A free ident declared only *inside another function* is still rejected.
   (Slice B2 will widen the multiply-declared nested case via an
   in-scope-binding walk / `closure-scope-analyzer`.)
7. **Side-effect-free arguments** — the same `is_simple_arg` gate.
8. **Reassigned parameters are materialized** (CLOC18) — a parameter the body
   assigns to (`x = …`, `x += …`) cannot be substituted by its argument
   expression (you cannot reassign a literal; a captured value would read the
   pre-assignment argument). Such a parameter is instead **materialised** into a
   fresh mutable local seeded from the argument (`let <fresh> = <arg>;`) and
   routed through the rename map — exactly a real call's binding semantics. A
   *member-target* write through a parameter (`x.k = …`) mutates a property of
   the argument, not the binding, so it stays substituted. (Reachable only since
   assignment statements parse, CLOC17.)

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
expression), and a body with no tail-return value are all declined. The
`return`-argument value position is admitted by PR-5, and the
assignment-target position (`g = f(x)`) by PR-6 (both below).

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

### Result returned — `return f(x)` (CLOC15 PR-5)

The other airtight value position is a call that is the **entire argument of a
`return`** — the everyday "tail-call a helper" shape. Here no temp is needed:
the helper's tail value becomes the **caller's own return value**, because
`return` is a terminator (nothing after it on that path runs):

```js
function helper(p) { log(p); return p + 1; }
function main()    { return helper(3); }
main(); main();
// SIMPLE  ⇒  function main(){ log(3); return 4 }   (helper declaration removed)
```

Replacing `return f(x)` with `body…; return E` runs the body's effects exactly
as they ran inside the callee, then returns the same value. As with PR-3 the
call must be the *whole* argument: `return cond && f(x)` (right operand of
`&&`), `return c ? f(x) : y` (a conditional branch), and a void helper used as
`return f(x)` (no value to return) are all declined. Composes with local
alpha-renaming and PR-4a per-argument temps.

### Result assigned — `g = f(x)` (CLOC15 PR-6)

The third airtight value position is a call that is the **entire right-hand
side of a simple assignment to a bare identifier** — the everyday "store a
helper's result" shape. As with PR-5 no temp is needed: the helper's tail value
becomes the **assignment's right-hand side**:

```js
function f(x) { side(); return x * 2; }
var g; g = f(7); use(g);
// SIMPLE  ⇒  var g; side(); g = 14; use(g);
```

`g = f(x)` evaluates the (trivial) reference to the bare target `g`, then the
call, then assigns — so splicing `body…; g = E` preserves that order exactly,
and the assignment statement's result value is discarded anyway. The gate is
deliberately narrow: **compound** assignment (`g += f(x)`, which reads the old
`g` *before* the call), a **member** target (`o.k = f(x)`, whose base `o` is
evaluated *before* the call), a call that is not the whole RHS (`g = f(x) + 1`),
and a void helper (no value) are all declined — each would reorder an
observable effect. Reachable only since the CLOC17 grammar fix made
assignment-expression statements parse. Composes with local alpha-renaming and
PR-4a per-argument temps.

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
