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

9. **Parameters are not reassigned.** Substitution (condition 1) replaces every
   parameter occurrence with its *argument expression*, which is only sound if
   the parameter behaves as an immutable value. If the body reassigns a
   parameter (`x = …`, `x += …`, or a nested `y = (x = 5)` / `f(x = 5)`), the
   model breaks: a non-lvalue argument would become an assignment target
   (`7 = …`), and a captured tail value would read the pre-assignment argument
   rather than the post-assignment parameter — so `function f(x){ x = x+1;
   return x }` inlined at `var g = f(7)` would yield `g = 7` instead of `8`, a
   miscompile. Reject any candidate whose body assigns to a parameter
   (`body_assigns_to_param`, recursing every expression position). A
   member-target whose base is a parameter (`x.k = 5`) mutates a *property* of
   the argument, not the parameter binding, and stays admitted. (Only reachable
   once assignment statements parse — CLOC17; before that such a helper made
   the whole program fall back to whitespace-only.) Materialising a mutated
   parameter into a fresh local seeded from the argument is a future slice.

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

- **PR-4 — widen control flow / arguments.** Split into two independent,
  separately-shippable slices (4a and 4b below). Each is behind its own
  soundness proof and fixture.

A SPEC note (this file) plus PR-1 is the first sound increment; do **not** ship
statement splicing without PR-1's alpha-renaming and free-identifier guards.

### PR-4a — non-simple arguments via per-argument temps

**Status:** PR-1/PR-2/PR-3 are merged. They require every argument to be
*simple* (a literal or a bare identifier, the `is_simple_arg` gate) so a
parameter used 0/1/many times can be substituted in place without dropping or
duplicating a side effect or reordering evaluation. PR-4a lifts that for the
**statement-inlining paths** (the void/discard pass and the value-capture pass
— NOT the expression inliner, which has no statement context to hoist into).

**The transform.** When a call's arguments are not all simple, materialise
**every** argument into a fresh `const` temp, in source order, *before* the
spliced body, and substitute each parameter with its temp identifier:

```js
function f(p, q) { sink(p); use(p, q); }
f(obj.x, compute());
// ⇒
const t0 = obj.x;     // every arg hoisted, left-to-right, once
const t1 = compute();
sink(t0); use(t0, t1);   // params → temps (the body, locals already renamed)
```

This is exactly JS call semantics: all arguments are evaluated left-to-right,
once each, before the callee body runs; the temps capture those values so a
parameter referenced N times in the body reads the captured value, never
re-evaluating the argument.

**Soundness conditions (each a hard reject):**

1. *Uniform temping when any argument is non-simple.* If **all** arguments are
   simple, keep PR-1..PR-3's direct substitution (no temps) — this preserves
   the existing single-pass output, so no fixture churn. If **any** argument is
   non-simple, temp **all** of them (including the simple ones) so left-to-right
   order is preserved with no per-argument case analysis. The redundant temps on
   simple args are removed downstream by `inline-variables` + `constant-fold`.
2. *Temps are program-fresh.* Mint each arg temp from the same `avoid` set used
   for callee-local renaming, adding each to `avoid` as minted, and mint the
   arg temps **before** the callee-local fresh names so the two name spaces
   cannot collide.
3. *Order: arg temps, then body.* The temp declarations are prepended to the
   spliced statement list (for the value-capture path they precede the body and
   the capture temp). The substitution map sends `param_i → Identifier(temp_i)`.
4. *No new argument-shape restriction needed.* Any argument **expression** is
   admissible because temping captures its value once at the splice point — a
   throwing argument still throws at the same point (before the body); a
   side-effecting argument runs once. (Arguments the front-end cannot bridge —
   e.g. assignment expressions, see *Implementation findings* — never reach the
   inliner at all: the program falls back to whitespace-only minification, so
   the pass simply never runs on them.)

**Plumbing the gate.** The single-use gate currently calls
`name_use_and_call_counts`, whose `inlinable` figure reuses the expression
inliner's `is_inlinable_call` (which requires *simple* args). PR-4a must add a
statement-path counter — uses of the name plus calls matching **name + arity**
regardless of argument simplicity — and the splice-matching predicate
(`is_void_target_call`) must likewise drop its `is_simple_arg` requirement
(keeping name + arity). The expression inliner's `is_inlinable_call` is left
unchanged.

**Shared helper.** Factor a `materialize_args(cand, args, avoid) -> (Vec<Statement>
prelude, HashMap<param, Expression> map)`: all-simple → `(vec![], direct map)`;
any-non-simple → temp every arg into the prelude and map each param to its temp.
Both `build_spliced_body` (void) and `build_captured_body` (value-capture) call
it and prepend the prelude.

### PR-4b — `if` without an early exit in the body

Admit an `IfStatement` (and later other straight-line-preserving control) in
the body **only** when neither branch contains `return` / `break` / `continue`
(which would change control flow when spliced). This requires (i) recursing the
callee-local collection and alpha-rename into the nested block(s) — a nested
`let`/`const` is already block-scoped, so collision-freedom needs the same
fresh-rename treatment — and (ii) extending the body-shape walk to accept the
admitted control constructs while still rejecting early exits. Defer until 4a
ships; it is independent.

### PR-5 — value capture in `return`-argument position (**merged**)

**Status:** merged. PR-3 captured a used result only when the call was the
entire initializer of a single-declarator `var`/`let`/`const`. PR-5 admits the
other airtight value position from *Open question 2*: a call that is the
**entire argument of a `return`** (`return f(x)`), the everyday "tail-call a
helper" shape.

**The transform.** Replace `return f(args);` with the hoisted body followed by
the callee's tail return re-emitted as **this** function's return — *no temp*,
because the value flows straight out:

```js
function helper(p) { log(p); return p + 1; }
function main()    { return helper(3); }
// ⇒
function main()    { log(3); return 4; }   // (after fold; helper decl removed)
```

**Soundness.** `return` is a terminator: the single `return f(x)` is the last
reachable statement on its path, so splicing `body…; return E` in its place
runs the body's effects exactly as they ran inside the callee before its own
return, then returns the same value. Any statement textually after
`return f(x)` was dead before and remains dead after. The call must be the
*entire* return argument — `return cond && f(x)` (a `LogicalExpression`),
`return c ? f(x) : y` (a `ConditionalExpression`), and a void helper used as
`return f(x)` (no tail-return value to surface) are declined. Local
alpha-renaming and PR-4a per-argument temps compose unchanged.

**Implementation.** `build_captured_body` gained a `CaptureTail` parameter —
`IntoTemp(&temp)` (PR-3's `const <temp> = E;`) or `AsReturn` (PR-5's
`return E;`) — so the rename / substitute / arg-materialisation logic is shared
and only the final statement varies. `try_capture_in_stmt` matches a
`ReturnStatement` whose argument is exactly the target call
(`capture_splice_for_return`); because that helper is invoked from
`splice_valued_in_stmt_vec`, it fires inside any nested function body.

## Implementation findings (after PR-1..PR-3, verified against `closurec`)

Empirical behaviour of the merged slices, confirmed by running the real
`closurec --compilation_level SIMPLE` binary — these pin what PR-4 may rely on:

- **Assignment expressions in a body are unreachable, not mishandled.** A body
  containing an `AssignmentExpression` (e.g. `sink(p = 5)`, `q(r = 1)`) causes
  the **typed bridge** (`grammar_to_program`) to bail, so the program falls
  back to whitespace-only minification and the inline pass never runs on it.
  This is why a body that reassigns a parameter is never inlined — it is
  *declined upstream of the pass*, not by a candidate guard. Consequence: the
  shared `substitute` helper's choice to leave a bare-identifier assignment
  target unsubstituted is not currently a miscompilation risk, because such
  bodies never reach it. **PR-4b must keep this property in view**: if the
  bridge ever gains assignment-expression support, a body that reassigns a
  parameter MUST be rejected explicitly (the substitution would write to the
  caller's variable), so add that guard *before* relying on assignment bodies.
- **Computed/member reads in a body inline fine.** `function f(p){ sink(p);
  use(arr[p]); } f(y)` → `sink(y);use(arr[y]);` — non-assignment member and
  computed-member expressions bridge and inline correctly today.
- **Non-simple arguments are the live limiter.** `f(obj.x)` is declined purely
  by the `is_simple_arg` gate (the call otherwise qualifies). This is the
  single highest-leverage widening still open, hence PR-4a's priority.

## Open questions

1. **Free-variable capture without a scope analyzer.** Condition 6's
   "global-and-unshadowed" rule is conservative but needs a cheap way to know
   the set of bindings live at the splice site. `closure-scope-analyzer`
   (CLOC13) exists but the inline pass does not currently consume per-node
   CvIds; the simplest sound bootstrap is a local walk that collects every
   binding name declared in any scope enclosing the call statement and rejects
   if a free body identifier is in that set. Decide: bootstrap walk vs. wire the
   scope analyzer.
   **Design pinned:** see [CLOC16](CLOC16-inline-free-identifier-widening.md),
   which splits free-ident classification (definition-site) from the shadowing
   check (splice-site), and stages it as Slice A (top-level splice site —
   trivially sound, no scope walk) then Slice B (nested site — in-scope-binding
   walk). The current "declared nowhere" rule is the safe baseline until A
   ships.

2. **Result capture in non-statement positions.** `a && f(x)`, `c ? f(x) : y`,
   `for (…; f(x); …)`, default-parameter and object-literal positions — hoisting
   a temp before these changes semantics or is illegal. PR-3 must enumerate and
   reject every such position; the safe default is "only inline a value-used
   call when its enclosing statement is a plain `ExpressionStatement`,
   `VariableDeclaration` init, or `return` argument, and the call is not under a
   short-circuit/conditional operator."
   **Resolved:** PR-3 implemented the `VariableDeclaration`-init position, PR-5
   the `return`-argument position, and PR-6 the assignment-target position
   (`g = f(x)`, bare-identifier simple assignment only); all reject the
   short-circuit/conditional sub-positions. PR-6 became buildable once the
   typed bridge parsed assignment-expression statements (CLOC17).

3. **`var` hoisting vs. let/const.** First slice should restrict callee locals
   to `let`/`const` to sidestep hoisting reasoning, then admit `var` once the
   fresh-rename argument (condition 5) is reviewed as sufficient.
   **Resolved:** `var` locals are now admitted. The fresh-rename argument *is*
   sufficient: condition 5 renames every callee local — `var` included — to a
   name that appears in no declaration or use program-wide, so wherever the
   `var` hoists to (the caller-function top) is unobservable; nothing but the
   spliced body, in source order, touches the name. The collision case (a
   caller binding of the same spelling) is the crux and is covered by a test.

4. **Interaction with the fixed-point pipeline.** Statement inlining can expose
   new single-use candidates and new constant-folding opportunities; confirm the
   `PassPipeline` fixed-point loop converges (it is capped at `MAX_SWEEPS`) and
   that inlining never oscillates with `remove-unused-vars` / `rename`.

5. **Size budget.** Single-use statement inlining is a near-strict win (the
   callee declaration is later removed by `remove-unused-vars`/treeshake). With
   PR-4a's per-argument temps a non-simple argument is materialised **once**
   into a temp regardless of how many times its parameter is used, so the
   substituted body cannot duplicate a large argument expression — the temp
   itself bounds growth. (A simple argument is still substituted directly, as
   today.) The `multiuse_budget_ok` node-count heuristic therefore does not need
   extending for PR-4a's single-use path; revisit only if statement splicing is
   ever extended to multi-**use** callees (a separate non-goal).

## Non-goals

- Inlining multi-**use** functions by statement splicing (size-explosive;
  governed by a budget, separate work).
- Cross-module / cross-file inlining.
- Inlining recursive functions (rejected by the single-declaration +
  no-self-reference guards).
