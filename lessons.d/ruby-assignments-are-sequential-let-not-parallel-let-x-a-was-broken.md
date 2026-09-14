# Ruby assignments are sequential (`let*`), not parallel (`let`) — `x = a` was broken

Investigating the conformance "array index" gap (`x = a[1]` fails SIR
validation) revealed the real bug is NOT about indexing: **any first-sighting
`newvar = <expr that reads an earlier local>` failed to compile on every
backend.** `a = 5\nb = a + 1` was rejected with `var-ref scope=local references
unknown name 'a'`.

Root cause: the SIR distinguishes `LetBinding` (parallel `let` — all RHSs
evaluate BEFORE any name binds) from `LetStarBinding` (sequential `let*` — each
RHS sees prior bindings). The validator (`validator::check_stmt_seq`) correctly
treats a *run of consecutive `LetBinding`s* as one parallel-`let` group. But the
Ruby frontend lowered EVERY first-sighting assignment to a parallel `LetBinding`,
even though Ruby assignments are sequential. So `[LetBinding(a); LetBinding(b =
a+1)]` put `b`'s RHS in a scope that didn't yet include `a`.

Fix (frontend, `sequentialize_let_bindings` post-pass): after a block's
statements are lowered, rewrite a `LetBinding` to a `LetStarBinding` (identical
fields) exactly when its value reads a name bound by an EARLIER statement in the
block. `let*` binds immediately and breaks the parallel run, so the reference
resolves. Independent bindings stay `LetBinding` (minimal churn). Both variants
lower to the same sequential declaration on every backend — behaviour-preserving.

**Lessons:**
- Shape-only frontend tests (asserting `Stmt::LetBinding {…}`) do NOT run the
  SIR validator, so they happily locked in a shape the validator would reject.
  ~16 such tests asserted the buggy parallel-`let` form; they were updated to
  accept either variant. When adding a frontend feature, run a program through
  `semantic_ir::validate` (or the conformance harness), not just a shape assert.
- Don't "fix" this in the validator by making `LetBinding` sequential — that
  would break genuine parallel-`let` frontends (Lisp `let`). The frontend that
  emits the wrong binding kind is what's wrong.
- The array/hash INDEX-READ parser bug (`a[1]` → `a` + `[1]`) is separate and
  still open (grammar precedence); only the assignment-scoping half is fixed.

Discovered: 2026-07-03, expanding the conformance corpus toward array indexing.

---
