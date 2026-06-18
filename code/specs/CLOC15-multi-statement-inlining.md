# CLOC15 — Multi-Statement Function Inlining

## What this spec locks down

`closure-pass-inline` today inlines only the **single-return-expression**
function shape — a body of exactly `{ return EXPR; }` — by replacing the
matching `CallExpression` node in place with the parameter-substituted return
expression. This spec defines how to extend it to inline **multi-statement
function bodies** (`function f(a){ const t = a + 1; g(t); return t * 2; }`)
soundly, and — critically — pins the **soundness conditions** that make
statement-level inlining safe so we never ship a transform that miscompiles
valid input.

This is a **design + staging** spec, not a single-PR change. Statement-level
inlining requires an architectural change to the pass and several independent
soundness obligations; this document exists so the work is sliced into sound,
reviewable increments rather than wired in unsafely in one pass.

## Why the current pass cannot do this (the architecture blocker)

The inline pass is **expression-only**. Its rewrite happens inside
`inline_in_expr`: when it finds a `CallExpression` matching a single-use
candidate, it builds a `param → arg` map, clones the callee's single return
expression, substitutes, and does `*expr = replacement` — an **in-place
expression swap**. The enclosing `ExpressionStatement` (or whatever statement
holds the call) is never touched; only its inner `expression` field mutates.

A call written as a statement,

```js
f(5);
```

is represented as:

```rust
Statement::Tagged(TaggedStatement::ExpressionStatement(ExpressionStatement {
    expression: Expression::CallExpression(/* f(5) */),
}))
```

To inline a multi-statement body we must replace **one** statement (the call's
expression statement) with **many** statements (the spliced body). That is a
**1 → N statement splice**, and it requires access to the `Vec<Statement>`
(inside a `BlockStatement`) or `Vec<ProgramItem>` (at the top level) that
*contains* the call statement, plus its index. The current walker threads
`&mut Statement` / `&mut Expression` — it never sees the enclosing statement
list — so it structurally cannot expand one statement into several.

Consequently the existing guard set is built for the expression-only world and
hard-rejects everything else:

| Guard (`candidate_from_function`) | Rejects | Why it exists today |
|---|---|---|
| `fd.generator \|\| fd.is_async` | generators / async | not expressible as a value swap |
| name declared ≠ 1 time | shadowed names | can't resolve uses by name alone |
| duplicate param names | ambiguous substitution map | — |
| `body.body.len() != 1` | **any multi-statement body** | **the limitation this spec removes** |
| not `return EXPR` (with arg) | non-return / bare return | only the single-expression shape |
| any free identifier in body | globals / `this` / `arguments` / recursion | no-capture: every ident must be a param |

The last guard is the subtle one: today the body may reference **only**
parameters. A real multi-statement body references **local bindings it
declares** (`const t = …`) and **free globals** (`g`, `console`, …). Both must
be permitted — and handled — for statement inlining, which is exactly where the
soundness obligations live.

## The transform, precisely

Given a single-use function

```js
function f(p1, …, pk) { S1; S2; …; Sn }   // n ≥ 1 statements
```

and a call site `f(a1, …, ak)`, replace the call with the statement sequence
`S1'; …; Sn'` where each `Si'` is `Si` with:

1. every parameter `pj` substituted by argument `aj`, and
2. every **callee-local binding** alpha-renamed to a program-fresh name.

Two call-site flavors:

- **Result discarded** — the call is an `ExpressionStatement`: `f(5);`. The
  spliced statements replace it directly. A trailing `return E` becomes either
  a dropped statement (if `E` is pure) or `E;` (kept for its side effects).
- **Result used** — the call is a sub-expression: `x = f(5) + 1;`,
  `log(f(5));`. The returned value must be **captured into a fresh temporary**
  declared before the call statement, and the call expression replaced by a
  reference to that temporary. This is materially harder (see Open Questions)
  and is deferred to a later slice.

## Soundness conditions (the contract)

A multi-statement body is inlinable **only if all** of the following hold. Each
is a hard reject; when in doubt, do not inline (forgoing an inline is never a
miscompile, miscompiling is).

1. **Single-use, single-declaration.** As today: the function name is declared
   exactly once and used exactly once (statement inlining of multi-use bodies
   is a separate, budgeted concern — out of scope here).

2. **No `this` / `arguments`.** Their meaning is bound by the *callee's* call
   frame; splicing into the caller silently rebinds them. Reject any body that
   references either. (Today's free-identifier guard catches this incidentally;
   the new guard must catch it explicitly since other free idents become legal.)

3. **No generators / async.** Unchanged.

4. **Control flow is "straight-line to a single tail return".** The body must
   have **no early return in the middle** and no control construct that a naive
   splice would mis-scope. The sound starting subset:
   - statements are `ExpressionStatement` and `VariableDeclaration` only, plus
   - an **optional single `return` as the final statement**.

   `return` anywhere but the tail changes control flow when spliced (the
   caller's following statements would still run); `break` / `continue` /
   labeled statements / loops / `if` / `switch` / `throw` / nested blocks are
   excluded from the first slice and admitted later only with explicit control
   analysis. A bare `return;` (no argument) in tail position is fine (it is a
   no-op once the value is discarded).

5. **Callee locals are alpha-renamed.** Every `var` / `let` / `const` binding
   the body introduces is renamed to a **program-fresh** identifier (via the
   shared `FreshNames` generator, avoiding every identifier anywhere in the
   program) before splicing, so a callee local can never shadow or collide with
   a caller-scope binding visible at the splice point. Renaming rewrites the
   binding *and* every in-scope use of it within the body.

   `var` hoisting subtlety: a callee `var` is function-scoped to the callee, so
   after fresh-renaming it is collision-free; but splicing it into the caller
   hoists it to the caller's function scope. Because the name is fresh this is
   observationally inert. (If the first slice restricts callee locals to
   `let`/`const`, this subtlety disappears entirely — a reasonable conservative
   start.)

6. **Free identifiers resolve to the same binding at the splice site.** A body
   identifier that is neither a parameter nor a callee-local is a **free
   reference** (`g`, `console`, a top-level `const`). Splicing is sound only if
   that name resolves to the *same* declaration at the call site as it did at
   the definition site. The conservative, checkable rule for the first slice:
   the free name must be a **program-global** (declared at top level or
   undeclared) **and** must **not** be shadowed by any binding in scope at the
   splice site. Capturing a caller local of the same name would be a miscompile,
   so absent a real scope analysis we reject when we cannot prove non-shadowing.

7. **Argument evaluation order and arity preserved.** Arguments are evaluated
   left-to-right exactly once, before the body runs. Today's `is_simple_arg`
   guard (arguments are literals or bare identifiers — no side effects, freely
   duplicable) keeps this trivially true and lets a parameter used `m` times be
   substituted `m` times. Keep that guard for the first slice; lifting it
   requires hoisting each argument into a fresh temp to preserve once-only,
   in-order evaluation (deferred).

8. **No name capture through substitution.** A parameter substituted by a bare
   identifier argument must not be captured by a callee-local of the same name —
   prevented by condition 5 (locals are fresh) plus 7 (args are simple).

If any condition is unprovable from the local AST, **reject**.

## Staged implementation plan

Each stage is an independent, sound, shippable PR. Earlier stages are strictly
correct on their own; later stages only *widen* what is inlinable.

- **PR-1 — statement-list walker + discard-result, no-return slice.**
  Refactor the call-site rewrite so it walks `Vec<Statement>` /
  `Vec<ProgramItem>` and can do 1 → N splicing. Admit bodies that are
  `ExpressionStatement` + `VariableDeclaration` (let/const) only, **no
  return**, called as an `ExpressionStatement` (result discarded). Alpha-rename
  callee locals (condition 5). Enforce conditions 1–8, with the conservative
  global-and-unshadowed rule for free idents (condition 6). Fixture: a
  side-effecting void helper called once is replaced by its (renamed) body.

- **PR-2 — tail `return` with discarded result.** Admit an optional trailing
  `return E`; when the call is a statement, drop a pure `E` or keep `E;` for
  effects. (Still no value capture.)

- **PR-3 — result-used via hoisted temp.** When the call is a sub-expression,
  declare `const <fresh> = <tail-return-expr>;` after the body statements and
  replace the call expression with `<fresh>`. Requires the same statement-list
  context at the call's *enclosing statement* (handled by PR-1's refactor) and
  care that the call is not inside a position where statement hoisting changes
  evaluation order (e.g. inside `&&` / `?:` short-circuit, or a `for` header) —
  reject those positions.

- **PR-4 — widen control flow / arguments.** Admit `if` without `return`
  inside, non-simple arguments via per-argument temps, etc. Each behind its own
  soundness proof and fixture.

A SPEC note (this file) plus PR-1 is the first sound increment; do **not** ship
statement splicing without PR-1's alpha-renaming and free-identifier guards.

## Open questions

1. **Free-variable capture without a scope analyzer.** Condition 6's
   "global-and-unshadowed" rule is conservative but needs a cheap way to know
   the set of bindings live at the splice site. `closure-scope-analyzer`
   (CLOC13) exists but the inline pass does not currently consume per-node
   CvIds; the simplest sound bootstrap is a local walk that collects every
   binding name declared in any scope enclosing the call statement and rejects
   if a free body identifier is in that set. Decide: bootstrap walk vs. wire the
   scope analyzer.

2. **Result capture in non-statement positions.** `a && f(x)`, `c ? f(x) : y`,
   `for (…; f(x); …)`, default-parameter and object-literal positions — hoisting
   a temp before these changes semantics or is illegal. PR-3 must enumerate and
   reject every such position; the safe default is "only inline a value-used
   call when its enclosing statement is a plain `ExpressionStatement`,
   `VariableDeclaration` init, or `return` argument, and the call is not under a
   short-circuit/conditional operator."

3. **`var` hoisting vs. let/const.** First slice should restrict callee locals
   to `let`/`const` to sidestep hoisting reasoning, then admit `var` once the
   fresh-rename argument (condition 5) is reviewed as sufficient.

4. **Interaction with the fixed-point pipeline.** Statement inlining can expose
   new single-use candidates and new constant-folding opportunities; confirm the
   `PassPipeline` fixed-point loop converges (it is capped at `MAX_SWEEPS`) and
   that inlining never oscillates with `remove-unused-vars` / `rename`.

5. **Size budget.** Single-use statement inlining is a near-strict win (the
   callee declaration is later removed by `remove-unused-vars`/treeshake), but a
   body whose params are used many times with non-trivial (post-simple-arg)
   substitutions could grow. Reuse / extend `multiuse_budget_ok`'s node-count
   heuristic before admitting non-simple arguments (PR-4).

## Non-goals

- Inlining multi-**use** functions by statement splicing (size-explosive;
  governed by a budget, separate work).
- Cross-module / cross-file inlining.
- Inlining recursive functions (rejected by the single-declaration +
  no-self-reference guards).
