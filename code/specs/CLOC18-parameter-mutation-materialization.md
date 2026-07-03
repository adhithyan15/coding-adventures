# CLOC18 — parameter-mutation materialization in `inline`

**Status:** IMPLEMENTED in `closure-pass-inline` 0.15.0. The design below was
specced first (it **reverses the 0.13.1 soundness guard**, PR #6272) and then
implemented exactly as written: `materialize_args` returns the 3-tuple,
mutated parameters route through the rename map at both splice sites, and the
three #6272 decline tests were flipped to materialisation-positive. Verified
end-to-end that the #6272 miscompile now inlines correctly (`g === 8`).

## Motivation

The inliner substitutes each parameter occurrence with its **argument
expression**, treating the parameter as an immutable value. A helper that
*reassigns* a parameter therefore cannot be inlined that way — `function f(x){
x = x + 1; return x; }` at `var g = f(7)` would miscompile to `g = 7` (the
pre-assignment argument) instead of `8`. PR #6272 made this **sound by
declining** such helpers (`body_assigns_to_param` → `return None`).

Parameter reassignment is *ubiquitous* in real JavaScript — the
default-argument idiom `function f(x){ x = x || DEFAULT; … }`, accumulator
loops, normalization (`x = String(x)`), etc. Declining all of them leaves a
large class of real helpers un-inlined. CLOC18 makes them inlinable by
**materialising the mutated parameter into a fresh mutable local seeded from the
argument**, which is exactly the semantics a real call has.

## The core finding (why a mutated param can't use `substitute`)

`substitute` (`closure-pass-inline/src/lib.rs`, the `AssignmentExpression`
arm, ~L1120) **deliberately does not rewrite a bare-identifier assignment
target**:

```rust
Expression::AssignmentExpression(ae) => {
    // ... only the member-object side and the RHS are substituted;
    // a bare-identifier target is left as-is (substituting a literal
    // there is impossible and an identifier target keeps the write
    // well-defined).
    if let AssignmentTarget::MemberExpression(m) = &mut ae.left { /* … */ }
    substitute(&mut ae.right, map);
}
```

So routing a mutated parameter through `substitute` would leave `x = x + 1`'s
target `x` untouched (writing a stray global) while rewriting the RHS — the
exact #6272 miscompile. The **`rename` walk**, by contrast, *does* rewrite a
bare-identifier assignment target (`rename_in_expr`'s `AssignmentExpression`
arm renames `AssignmentTarget::Identifier`). Therefore a mutated parameter must
flow through the **rename** path, not the **substitute** path.

## Design

Treat a mutated parameter like a callee **local** seeded from the argument:

1. **Materialise** it once into a fresh, *mutable* binding at the top of the
   spliced body: `let <p_fresh> = <arg>;`. Always materialise (even a simple
   argument) — you cannot reassign a substituted literal or a caller-scope
   identifier, and the argument must be evaluated exactly once regardless of
   how many times the parameter is read or written.
2. **Route the parameter through the rename map** (`<p> → <p_fresh>`), so every
   read *and every assignment target* of the parameter in the body (and, for
   the value-capture path, in the captured return expression) becomes
   `<p_fresh>`.

This is observationally identical to a real call: a parameter is a local
binding initialised from the argument value; reassigning it never affects the
caller's argument (JS is pass-by-value — the binding holds a copy of the
primitive or a copy of the object *reference*); and the argument is evaluated
once. The fresh `let` is block-scoped, but because `<p_fresh>` appears nowhere
else in the program (minted from the `avoid` set) the scoping is inert — the
same argument that makes `var`-local admission sound (CLOC15 Open Q3).

**Member targets are unaffected.** `x.k = 5` mutates a *property* of the
argument, not the parameter binding, so `x` is not a "mutated parameter"; it
stays on the substitute path (`arg.k = 5`), exactly as today.

### Worked example (the #6272 case, now inlined correctly)

```js
function f(x){ x = x + 1; return x; } var g = f(7); use(g);
// CLOC18 (inline pass):
//   let p = 7; p = p + 1; const t = p; var g = t; use(g);
// SIMPLE (after fold/propagate/DCE):
//   var g = 8; use(g);   ✓  (was g = 7 — a miscompile — before #6272 declined it)
```

## Implementation plan

All in `code/packages/rust/closure-pass-inline/src/lib.rs`.

1. **`VoidStmtCandidate`** — add `mutated_params: HashSet<String>`.
2. **Candidate filter** (`void_candidate_from_function`) — replace the #6272
   decline
   ```rust
   if body_assigns_to_param(&fd.body.body, &param_set) { return None; }
   ```
   with a collector that records the mutated set and stores it on the
   candidate:
   ```rust
   let mutated_params = collect_mutated_params(&fd.body.body, &param_set);
   ```
   Convert the existing `body_assigns_to_param` / `stmt_assigns_to_param` /
   `expr_assigns_to_param` (bool) walkers into `collect_*` variants that insert
   matched parameter names into an out-set (same traversal — every expression
   position, `AssignmentTarget::Identifier` targets only).
3. **`materialize_args`** — return a **3-tuple** `(prelude, substitute_map,
   mutated_rename)`:
   - Fast path (no prelude, direct substitution) only when **all args are
     simple AND `cand.mutated_params` is empty**.
   - Otherwise, per `(param, arg)` in source order, mint a fresh temp:
     - param **mutated** ⇒ emit `let <fresh> = <arg>;`, insert `param → <fresh>`
       into `mutated_rename`;
     - param **not mutated** ⇒ emit `const <fresh> = <arg>;`, insert
       `param → Identifier(<fresh>)` into `substitute_map` (today's behaviour).
4. **`build_spliced_body`** and **`build_captured_body`** — merge
   `mutated_rename` into the local-`rename` map, apply rename when
   `!rename.is_empty()` (not `!cand.locals.is_empty()`), then substitute with
   `substitute_map`. `build_captured_body` must also rename the captured
   `return_value` with the merged map (it already does for locals).

## Tests

- **Flip** the three #6272 decline tests (`does_not_inline_helper_that_reassigns_param`,
  `…compound_param_assignment`, `…nested_param_assignment`) to
  materialisation-positive assertions.
- Add: a **value-capture** mutated-param case computing the correct value
  (`var g = f(7)` ⇒ `g === 8` after fold), a **multi-parameter mixed**
  case (one mutated, one not — mutated gets `let`+rename, other substitutes),
  a **side-effecting argument evaluated once** case (`f(side())` with a mutated
  param materialises `side()` once into the `let`), and a **collision** case (a
  caller binding sharing the parameter's spelling is untouched).
- Confirm **no closurec fixture churn** beyond programs that now inline where
  they previously did not.

## Soundness checklist (for the implementation's adversarial review)

- The materialised binding is reassignable (`let`, not `const`).
- The argument is evaluated **exactly once** (into the `let`), even when the
  parameter is read/written multiple times.
- `<p_fresh>` is program-fresh (from `avoid`), so the `let`'s block scope is
  inert and cannot collide with or shadow any caller binding.
- A mutated parameter never reaches `substitute` (which would drop the target
  rewrite) — it is in `mutated_rename`, applied by the target-aware `rename`.
- Member-target writes through a parameter (`x.k = …`) are **not** treated as
  parameter mutation and keep substituting.
- Interaction with the **multi-use void path**: each splice mints distinct
  fresh `let` names from a growing `avoid` set, so N call sites get N
  independent materialised locals — confirm no cross-site aliasing.

## Downstream

Once CLOC18 lands, the 0.13.1 parameter-mutation guard is fully superseded for
the common cases; the only remaining declines would be parameter mutation via
forms the typed AST cannot represent (e.g. `++`/`--` `UpdateExpression`, still
Phase 2) — which therefore cannot occur and need no handling.
