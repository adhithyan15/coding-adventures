# CLOC16 — inline free-identifier widening (top-level declarations)

**Status:** Slice A **implemented** (`closure-pass-inline` 0.11.0); Slice B1
(global-uniqueness gate) **implemented** (0.12.0); Slice B2 (scope walk)
deferred. Resolves [CLOC15](CLOC15-multi-statement-inlining.md) **Open
question 1**.

## The limiter

The statement inliner (`closure-pass-inline`) splices a single-use
multi-statement helper at its call site. A **free identifier** in the body —
one that is neither a parameter nor a callee-local — is admitted today **only
if it is declared nowhere in the program** (`decl_counts[name] == 0`, i.e. a
true global like `console`, `Math`, or an undeclared name). The guard lives in
`void_candidate_from_function`:

```rust
// (6) a free identifier: sound only if it is a true global, i.e.
// never declared as a binding anywhere (so unshadowable everywhere).
if decl_counts.get(name).copied().unwrap_or(0) != 0 {
    return None;
}
```

This is correct but **over-conservative**: it rejects any helper whose body
references **another top-level declaration** — a sibling `function`, a
top-level `const`/`let`. That is the *common* case, so a large class of
helpers never inline:

```js
function dep(x) { return x * 2; }
function f(p)   { log(p); use(dep(p)); }   // `dep` is a free ident → declined today
f(5);
dep(1);                                      // (dep kept alive)
```

Empirically (SIMPLE, verified against the `closurec` binary on 2026-06-19):
`f` above is **not** inlined (`function f(p){log(p);use(dep(p))};f(5);…`),
whereas the identical helper referencing an *undeclared* global **is**
(`log(5);use(GLOB);`). The only difference is that `dep` is declared at top
level. Widening the guard to admit top-level-declared free idents is the
single highest-leverage inline win remaining.

## Why it is soundness-critical

A free identifier is sound to splice **only if it resolves to the same binding
at the call site as it did at the definition site** (CLOC15 condition 6). For
a *true global* that holds everywhere — there is no binding of that name to
shadow it, at any splice location. That location-independence is exactly why
today's guard can live entirely in the candidate gate (which does not know
where the single call is).

A *top-level declaration* does **not** have that property. If the helper's one
call site sits inside a scope that re-binds the name, splicing captures the
**wrong** binding — a miscompile:

```js
function dep(x) { return x * 2; }
function f(p)   { use(dep); }               // `dep` = the top-level function
function g()    { let dep = 0; return f(1); }  // f's only call, under a local `dep`
g();
// UNSOUND naive splice ⇒ function g(){ let dep = 0; use(dep); }
//   now `use(dep)` reads the local 0, not the top-level function. WRONG.
```

So admitting top-level free idents requires a **splice-site shadowing check**
that the candidate gate alone cannot perform: it depends on *where* the call
is, not just on the helper definition.

## Design

Split the free-ident classification (definition-site, location-independent)
from the shadowing check (splice-site, location-dependent).

### 1. Candidate gate — classify, don't reject

Pass the set of **top-level program-scope declaration names** into
`void_candidate_from_function` (functions + top-level `var`/`let`/`const` +
… see *Open questions* on what counts as program-scope). Replace the binary
reject with a three-way classification of each free ident:

- **param / callee-local** → handled by splicing (unchanged).
- **true global** (`decl_counts == 0`) → admit, no splice-site obligation
  (unchanged behaviour).
- **top-level declaration** → admit **conditionally**; record the name in a
  new candidate field `free_top_level: HashSet<String>`.
- **anything else** (declared, but *not* at top level — e.g. a name bound only
  inside some other function) → **reject** (as today). A non-top-level
  declaration cannot be the binding a top-level helper's free ident resolves
  to, so this would be a definition-site resolution we cannot prove.

`this` / `arguments` rejection is unchanged.

When `free_top_level` is empty the candidate behaves exactly as today (no new
obligation) — **zero behaviour change** for every currently-inlined helper.

### 2. Splice site — enforce non-shadowing

A candidate with **non-empty `free_top_level`** may be spliced **only at a
call site where none of those names is shadowed** by a binding in scope. Two
sound enforcement levels:

- **Slice A (top-level splice site — trivially sound, ship first).** Splice a
  `free_top_level` candidate **only when its single call is a direct member of
  `program.body`** (the program/global statement list — not nested in any
  block or function). At a direct top-level site the scope is exactly the
  program scope, so a top-level-declared name resolves to its top-level
  binding identically in the helper and at the splice — **no intervening scope
  can shadow it**. In every nested splice path (`splice_*_in_stmt` /
  `_in_stmt_vec`, block / `if` / loop / switch / function body) a
  `free_top_level` candidate is **declined** (the call is left intact; the
  helper declaration stays — declining is never a miscompile). This covers
  top-level single-use helpers (module-init style) with full soundness and no
  scope walk.

  > Note: a call inside a top-level **block** (`{ … f(); }`) or a top-level
  > `if`/loop is *nested*, not a direct `program.body` member, so Slice A
  > declines it — correctly, since a block-scoped `let`/`const` there could
  > shadow.

- **Slice B1 (nested splice site — global-uniqueness gate, IMPLEMENTED).** A
  much simpler sound mechanism than a scope walk handles the *common* nested
  case. `count_decl_names_*` counts **every** binding declaration at every
  depth program-wide (its catch-all arm is deliberately exhaustive), so
  `decl_counts[name]` is exact. If a top-level free ident has
  `decl_counts[name] == 1`, it is declared **exactly once** in the whole
  program — **no other binding of the name exists anywhere**, so it cannot be
  shadowed at *any* splice site. Such a name carries **no** top-level-only
  obligation (it is treated like a true global for splice-location): the
  candidate inlines even at nested sites, with no scope walk. A top-level name
  with `decl_counts > 1` keeps the Slice A top-level-only obligation. This is
  sound on the pass's own terms — the gate reads `decl_counts` of the program
  the pass actually receives, so `== 1` genuinely means unshadowable there.
  Implemented in `void_candidate_from_function` as a single classification
  branch; the splice-site guards are unchanged.

- **Slice B2 (nested splice site — in-scope-binding set, FUTURE).** The
  remaining case is a top-level name that is *also* declared elsewhere
  (`decl_counts > 1`) but is **not actually shadowed at the specific call
  site** (the other declaration is in an unrelated scope). Admitting it needs a
  real shadowing check: thread the set of **binding names in scope at the
  splice point** through the splice walkers — on entering a function body add
  its params, its hoisted `var`s, its nested `function` declaration names, and
  the function's own name; on entering a block add that block's
  `let`/`const`/`function` names; etc. Before splicing a `free_top_level`
  candidate, reject if **any** `free_top_level` name is in the current in-scope
  set. CLOC13's `closure-scope-analyzer` already computes per-node scope
  information; Slice B2 may consume it (preferred long-term) or use a
  self-contained enclosing-binding walk (simpler bootstrap, matching CLOC15
  OQ1's suggestion). Lower priority than B1 since the multiply-declared name is
  the rarer case and B1 already covers the bulk.

### 3. Both inline paths share the gate

`void_candidate_from_function` feeds **both** the void path and the valued
(PR-3 `const r = f(x)` / PR-5 `return f(x)`) paths. The `free_top_level`
obligation therefore must be enforced in **every** splice entry point, not
just the void one:

- `return f(x)` is only valid inside a function, so a `free_top_level`
  candidate captured in return position is **always nested** → declined by
  Slice A (sound, zero value there until Slice B).
- `const r = f(x)` **can** be a direct `program.body` item → Slice A may splice
  it at top level.

Implementation must make the "splice only where unshadowed" check a single
shared helper invoked by `splice_void_*`, `capture_splice_for_vardecl`, and
`capture_splice_for_return`, so no path can splice a `free_top_level` candidate
without discharging the obligation.

## Soundness proof obligations (must hold before merge)

1. **Zero regression.** A helper with empty `free_top_level` is spliced exactly
   as today, at exactly the same sites. (Test: the full existing suite is
   unchanged; no fixture churn.)
2. **Slice A safety.** A `free_top_level` candidate is spliced **iff** its call
   is a direct `program.body` element; at such a site the spliced free idents
   resolve to the same top-level bindings. (Tests: top-level call with a
   top-level-function free ident → inlined; the *same* helper called inside a
   function → declined; called inside a top-level block → declined.)
3. **No shadow capture.** The `g(){ let dep = 0; return f(1); }` miscompile
   case above is declined under Slice A (nested) and, under Slice B, declined
   by the in-scope check. (Test: both.)
4. **Definition-site resolution.** A free ident that is declared *only inside
   some other function* (not top level) is still rejected — we cannot prove
   what it resolves to inside the helper. (Needs a **new** test: a helper whose
   free ident matches a name bound only in an unrelated function's scope.)

> **Existing-test impact.** `does_not_inline_void_helper_with_free_declared_name`
> today asserts that `const K = 5; function f() { sink(K); } f();` is *not*
> inlined (it documents the conservative baseline). `K` is a **top-level**
> const and the call `f()` is a **direct `program.body` member**, so this is
> exactly a Slice A **positive** case: when Slice A lands its expected output
> flips to the inlined form (`const K=5;sink(5);` after fold + propagate +
> treeshake, or `const K=5;sink(K);`'s spliced equivalent before downstream
> passes). The Slice A PR must **rename/repurpose** this test to assert the new
> (inlined) behaviour and add a *separate* negative test for the genuinely
> non-top-level case (obligation 4). This is intended behaviour change, not a
> regression — call it out explicitly in the Slice A commit message.

## Test plan

Extend `closure-pass-inline` unit tests (the harness already has
`does_not_inline_body_with_free_global`, `does_not_inline_shadowed_name`,
`does_not_inline_void_helper_with_free_declared_name` to build on):

- `inlines_top_level_helper_referencing_sibling_function` (Slice A positive).
- `does_not_inline_free_top_level_when_call_is_nested` (Slice A negative —
  call inside a function).
- `does_not_inline_free_top_level_when_call_in_top_level_block` (Slice A
  negative — call inside a `{ … }`).
- `does_not_inline_free_top_level_shadowed_at_nested_site` (Slice B negative).
- `inlines_free_top_level_at_unshadowed_nested_site` (Slice B positive).
- An end-to-end `closurec --compilation_level SIMPLE` fixture proving the
  `dep`/`f` example collapses, plus a fixture proving the shadowed case does
  **not**.

## Open questions

1. **What counts as "program-scope"?** Top-level `function`, `var`, `let`,
   `const`. A name declared at top level inside a top-level *block* with
   `let`/`const` is block-scoped, not program-scope — exclude it from the
   top-level set (it is not unshadowable). Confirm `decl_counts` /
   `count_decl_names_*` distinguishes program-scope from block-scope, or add a
   dedicated `collect_top_level_decl_names` walk (only `program.body`'s direct
   `FunctionDeclaration` and non-block `var`/`let`/`const`).
2. **`var` hoisting into the in-scope set (Slice B).** A `var` anywhere in a
   function hoists to that function's top — the in-scope walk must add hoisted
   `var` names from the *whole* enclosing function body, not just statements
   lexically before the call.
3. **Function-name self-reference.** A `free_top_level` name equal to an
   enclosing function's own name (named function expression / declaration) must
   count as shadowing in Slice B.
4. **Slice A vs. Slice B sequencing.** Ship Slice A first (small, airtight, no
   scope walk); Slice B in a follow-up once the in-scope-binding walk (or
   CLOC13 consumption) is reviewed. Each is independently sound.
5. **Interaction with `rename-globals`.** ADVANCED renames top-level private
   names *after* inline. A `free_top_level` ident spliced at a top-level site
   is still a normal reference to the (possibly-renamed) top-level binding, so
   ordering is unaffected — but add a combined-pipeline test to confirm.

## Non-goals

- Consuming CLOC13 per-node `CvId` scope data is **not** required for Slice A
  and is optional for Slice B (a local enclosing-binding walk suffices as a
  bootstrap). Wiring the analyzer is a separate concern.
- Free idents that are block-scoped (non-top-level) declarations remain
  rejected — proving their resolution needs full scope analysis and is out of
  scope here.
