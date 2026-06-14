# Changelog

## 0.1.0 — 2026-04-27

Initial release.

**Newton's method numeric root finder for the MACSYMA symbolic VM.**

- `mnewton_solve(f_ir, x_sym, x0_ir, eval_fn, diff_fn, tol, max_iter)` — pure
  algorithm that iterates Newton's method symbolically/numerically. Derivative
  is computed once via `diff_fn` (symbolic) then evaluated on each step via
  `eval_fn` + `cas-substitution.subst`.
- `MNewtonError` — raised when the derivative is zero at the current iterate
  (flat tangent — Newton step undefined).
- `mnewton_handler(vm, expr)` — VM handler for `MNewton(f, x, x0)` and
  `MNewton(f, x, x0, tol)` IR applies. Returns `IRFloat(root)` on convergence,
  unevaluated on failure.
- `build_mnewton_handler_table()` — returns the `{"MNewton": mnewton_handler}`
  dict for wiring into `SymbolicBackend`.
- 22 tests covering: linear/quadratic/cubic convergence, multiple starting
  points, exact-root detection, non-numeric x0, zero-derivative guard,
  custom tolerance, sin(x) near π, wrong-arity inputs.
