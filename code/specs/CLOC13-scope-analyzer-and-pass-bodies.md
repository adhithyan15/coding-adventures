# CLOC13 — Scope analyzer + Phase 1 pass-body activation

**Status:** Implementation in flight. Scaffold + 5 sibling pass bodies landed; analyzer body is the activation step.
**Layer:** Above CLOC06 (canonical pass set) and CLOC09 (AST taxonomy), below CLOC11 (drop-in CLI compat).
**Depends on:** CLOC06, CLOC09, CLOC10.A (PassRegistry).
**Unblocks:** CLOC13.0 (analyzer body), CLOC13.A.1..E.1 (per-pass apply steps).

---

## 1. Why CLOC13 exists

Five Phase-1 optimisation passes (`closure-pass-rename`,
`closure-pass-inline`, `closure-pass-treeshake`,
`closure-pass-collapse-properties`, `closure-pass-remove-unused-vars`)
all need the same input: a lexical scope tree, a binding table, and
a reference table. Each pass building its own ad-hoc symbol table
costs five tree walks per program, five maintenance burdens, and
five chances to disagree about scoping edge cases.

CLOC13 hoists that work into one shared crate
(`closure-scope-analyzer`) and feeds it to all five passes via a
frozen, serialisable API.

The strategic value is *parallel work-stream activation*: once the
analyzer's public surface is stable, the five pass bodies become
independently shippable. A 5-PR fan-out costs about the same
wall-clock as one PR because the rate-limiter is review, not work.

---

## 2. Architecture — the unblocker pattern

```
                        ┌───────────────────────────────┐
                        │   closure-scope-analyzer       │
                        │   pub fn analyze(&Program)     │
                        │       -> ScopeAnalysis         │
                        └───────────────┬───────────────┘
                                        │ consumed via Cargo path dep
        ┌───────────────┬───────────────┼───────────────┬───────────────┐
        ▼               ▼               ▼               ▼               ▼
  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
  │  rename  │    │  inline  │    │ treeshake│    │ collapse-│    │  remove- │
  │ (13.A)   │    │  (13.B)  │    │  (13.C)  │    │properties│    │ unused-  │
  │          │    │          │    │          │    │  (13.D)  │    │vars (13.E)│
  └──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
       PR #4777        PR #4778       PR #4775       PR #4773         PR #4766
```

The analyzer scaffold ships an **identity-style empty `analyze`**
that returns the global scope and nothing else. The five consumer
passes start their real-body work in parallel against that frozen
API. When the analyzer's actual traversal-and-resolution body
lands (CLOC13.0), every wired pass lights up simultaneously with
zero PR-side churn — no rebases, no cascade.

This is the **wire-then-activate** pattern. Encoded as a recipe:

1. Ship the shared crate's types + a stub that returns empty
   collections of those types.
2. Open N PRs in parallel, each wiring one consumer to call the
   stub and walk the empty collections. Each PR is observably
   identity (program passes through unchanged because the analysis
   returned nothing to act on).
3. Land a single follow-up PR that replaces the stub with the
   real body. All N consumers activate simultaneously.

---

## 3. Public contract — `ScopeAnalysis`

```rust
pub fn analyze(program: &Program) -> ScopeAnalysis;

pub struct ScopeAnalysis {
    pub scopes:     Vec<Scope>,        // tree rooted at ScopeId::GLOBAL
    pub bindings:   Vec<Binding>,
    pub references: Vec<Reference>,
}
```

### Identifier handles

- `ScopeId(pub u32)`, `BindingId(pub u32)` — dense indices into
  `scopes` / `bindings`. Newtype-wrapped for type safety, `u32` so
  the analysis is cheap to serialise to the CV sidecar.
- `ScopeId::GLOBAL == ScopeId(0)` — the global scope is reserved
  index 0 and always present.
- We *don't* use pointer identity because (a) we want the analysis
  serialisable to JSON for CV writers, (b) pass crates shouldn't
  need to hold a `&Program` borrow for the whole pass life.

### Scope tree

```rust
pub struct Scope {
    pub kind:     ScopeKind,           // Global | Function | Block
    pub parent:   Option<ScopeId>,     // None only for GLOBAL
    pub bindings: Vec<BindingId>,
}
```

Name resolution walks the parent chain explicitly. Nested scopes
do NOT inherit their parent's bindings — the consumer pass walks
up.

`ScopeKind` is `#[non_exhaustive]`. Reserved future variants:
modules, `with` blocks (if ever), catch-clause scopes.

### Bindings

```rust
pub struct Binding {
    pub name:         String,
    pub kind:         BindingKind,     // Var|Let|Const|Function|Class|Param
    pub scope:        ScopeId,
    pub declared_at:  Option<CvId>,    // populated only when --correlation_vector is on
}
```

`BindingKind` is `#[non_exhaustive]`. Each consumer pass picks
which kinds it accepts:

| Pass         | Accepts kinds                       | Skips                              |
|--------------|-------------------------------------|------------------------------------|
| rename       | ALL (incl. future variants)         | none — admit-by-default            |
| inline       | `Function`, `Class`                 | `Var/Let/Const/Param`, future      |
| treeshake    | `Function`, `Class`                 | `Var/Let/Const/Param`, future      |
| collapse-prop| `Const`                             | `Var/Let/Function/Class/Param`, future |
| remove-unused| ALL (when uses == 0)                | none — admit-by-default            |

Note the split: rename + remove-unused-vars are
*admit-by-default* (a missed admission is wasted bytes, not a
correctness issue). The other three are *skip-by-default* (a
wrong-kind candidate is a correctness bug). Both stances are
defensible; the doc'd-in-code reason for the divergence is what
matters.

### References

```rust
pub struct Reference {
    pub name:    String,
    pub binding: Option<BindingId>,    // None == unresolved (e.g. `console`)
    pub cv:      Option<CvId>,
}
```

`binding: None` is how we detect references to free globals
(`console`, `window`, etc.). The rename pass uses this signal to
*never* rename a free global — it's not in the program's
binding table.

---

## 4. The `changed = false` hard-pin discipline

**Critical constraint codified across all five CLOC13 pass
bodies.** Under `IterationPolicy::FixedPoint`, a pass that reports
`changed = true` while returning an unchanged program causes the
scheduler to re-run forever. The intuitive draft
`changed = !candidates.is_empty()` is wrong: each iteration
re-finds the same candidates, claims a change, returns the same
program.

Fix: until the apply step (step 3, the actual program mutation)
lands, every CLOC13.x pass body returns

```rust
Ok(PassOutput {
    program: ctx.program.clone(),
    changed: false,            // HARD-PINNED literal
    ...
})
```

The discipline applies to `OneShot` passes (rename) too. Pipeline
consumers may key off `changed` for cache invalidation or to skip
downstream serialization; reporting `true` without mutation forces
unnecessary work even when the infinite-loop concern doesn't
literally apply.

See [`lessons.md`](../../lessons.md) — the lesson is codified at
the framework level with the trigger pattern, the defence in
depth (`let _candidates = candidates;` to preserve the
candidate-scan work for the apply-step PR), and the OneShot
variant.

---

## 5. Stage delivery

| PR    | Task                                  | Status |
|-------|---------------------------------------|--------|
| #4763 | CLOC13 unblocker — analyzer scaffold  | merged |
| #4766 | CLOC13.E — remove-unused-vars body    | merged |
| #4773 | CLOC13.D — collapse-properties body   | merged |
| #4775 | CLOC13.C — treeshake body             | merged |
| #4777 | CLOC13.A — rename body                | merged |
| #4778 | CLOC13.B — inline body                | open   |
| (next)| CLOC13.0 — real analyzer body         | queued |
| (later) CLOC13.A.1..E.1 — per-pass apply steps  | queued |

After CLOC13.0 lands:
- The analyzer's empty `Vec`s become populated.
- Every consumer pass's candidate scan starts finding real work.
- The pass-output `changed` field stays hard-pinned at `false` —
  the apply step is intentionally a separate PR per pass to keep
  review surface small.

---

## 6. Open questions for CLOC13.0

These are deliberately deferred to the analyzer-body PR — flagging
them here so the contributor doesn't trip over them mid-PR.

1. **Var hoisting.** A `var x = …` inside a block scope binds in
   the enclosing *function* scope. The analyzer must walk into
   block bodies but emit the `Binding` against the function
   scope's `bindings` vec. Pattern: pre-walk to collect var
   declarations, then walk normally.
2. **Function-name hoisting.** `function f() { … }` hoists its
   binding to the enclosing function scope; non-strict mode also
   hoists the *initializer*. Analyzer should emit the binding;
   passes can read source order from `declared_at`.
3. **TDZ-like binding ordering.** `let` / `const` create
   "Temporal Dead Zone" reads before declaration. The analyzer
   doesn't enforce TDZ — that's the runtime's job — but it should
   record `Reference`s in source order so downstream linters can.
4. **Block scope vs. function scope ambiguity in legacy `var`.**
   `for (var i = …)` hoists `i` to the enclosing function; the
   `for` body itself does NOT get a fresh `Block` scope. Need to
   distinguish from `for (let i = …)`.
5. **Catch-clause scope.** `try { } catch (e) { }` introduces a
   one-binding `Block` scope holding `e`. Reserved `ScopeKind`
   variant when this lands.
6. **Strict-mode handling.** Affects `function` binding scope
   (block vs. function). Read the program's `SourceType` and the
   nearest `"use strict"` directive.

These are tractable; none require API changes to the scaffold.

---

## 7. References

- **Spec siblings:** CLOC06 (canonical pass set), CLOC09 (AST
  taxonomy), CLOC10.A (PassRegistry), CLOC11 (drop-in CLI compat).
- **Source-of-truth files:**
  `code/packages/rust/closure-scope-analyzer/src/lib.rs`,
  `code/packages/rust/closure-pass-{rename,inline,treeshake,collapse-properties,remove-unused-vars}/src/lib.rs`,
  `lessons.md` (the changed-hard-pin and cascade-rebase lessons).
- **PR thread of the implementation:** #4763 → {#4766, #4773,
  #4775, #4777, #4778} → (queued) CLOC13.0.

---

*This spec was written after the wire-step PRs landed, per the
"Specs must stay in sync — if implementation diverged, update
spec and call out the divergence" rule. The divergence: the
scaffold (PR #4763) shipped before the spec because the
unblocker pattern is most valuable as an early move and the
five sibling pass bodies were already queued. Spec-first
discipline applies to the next layer (CLOC13.0 analyzer body
+ CLOC13.A.1..E.1 apply steps) — those should not start until
this spec lands.*
