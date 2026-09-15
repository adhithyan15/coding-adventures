## 0.7.0 — 2026-06-22 — `own` variables: static lifetime (LANG-FULL AL6)

ALGOL 60's `own` declarations (`own integer n`) now lower to **module globals**,
reusing the E6 global substrate (`global_load`/`global_store`). An `own` variable
is allocated once and retains its value across every call of its enclosing
block/procedure (ALGOL 60 §5.2.5), which is exactly the semantics a module
global gives — it zero-inits at module load and persists.

- `declare_var` gained an `is_own` flag; a declaration is materialised as a
  global when `is_own || captured` (E6). The slot is already unique per scope
  (`__algol_s<N>_<name>`, the per-procedure `scope_counter` differs), so two
  procedures' `own n` map to **distinct** globals — they don't alias.
- **A global is no longer given a per-declaration `const` zero-init.** For an
  `own` variable inside a procedure that init would re-zero it on every call,
  destroying persistence; for an E6-captured block scalar it was a dead
  register write shadowing the global. Globals zero-init once at module load,
  so the `const` is both unnecessary and wrong for them. Plain (register)
  scalars keep their zero-init.
- Proven by **running** on all 7 backends (`lang_matrix.rs`): `bump(d)` adds `d`
  to its `own integer n`; `bump(1) + bump(1) + bump(1)` accumulates `1 + 2 + 3
  = 6` (a non-`own` local would give `1 + 1 + 1 = 3`). Plus unit tests: lowering
  to `global_load`/`global_store` with no re-init `const`, VM-run persistence,
  and two procedures' `own` staying independent.
- Requires `coding-adventures-algol-parser` 0.2.0 (the `[ "own" ] type
  ident_list` grammar rule).

