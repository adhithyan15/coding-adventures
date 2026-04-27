# Changelog

## 0.2.0 — 2026-04-27

### Added

- `cas_handlers.py` — VM handler functions for every CAS substrate package:
  - **Simplify / Expand** — delegate to `cas_simplify.simplify` and
    `cas_simplify.canonical` respectively.
  - **Subst** — delegates to `cas_substitution.subst`; re-evaluates the
    substituted result through the VM so numeric simplification fires.
  - **Factor** — identifies the single free variable, converts the IR
    to an integer polynomial via `symbolic_vm.polynomial_bridge.to_rational`,
    calls `cas_factor.factor_integer_polynomial`, and reassembles the result
    as `Mul(content, Pow(factor, mult), …)` IR.
  - **Solve** — converts the polynomial to rational form, dispatches to
    `cas_solve.solve_linear` (degree 1) or `cas_solve.solve_quadratic`
    (degree 2), returns `List(solution, …)` IR.  Degree > 2 and non-polynomial
    expressions return unevaluated.  Handles `Equal(lhs, rhs)` by rewriting
    to `Sub(lhs, rhs)`.
  - **List operations** — `Length`, `First`, `Rest`, `Last`, `Append`,
    `Reverse`, `Range`, `Map`, `Apply`, `Select`, `Sort`, `Part`, `Flatten`,
    `Join`.  Each is a thin wrapper around the corresponding
    `cas_list_operations` function; `Map` and `Apply` route through the VM
    so element-wise evaluation fires.
  - **Matrix operations** — `Matrix` (shape validation), `Transpose`,
    `Determinant`, `Inverse`; all delegate to `cas_matrix`.
  - **Limit** — delegates to `cas_limit_series.limit_direct`; passes the
    result through `simplify` and the VM so numeric reduction fires.
  - **Taylor** — delegates to `cas_limit_series.taylor_polynomial`; returns
    unevaluated for non-polynomial expressions (`PolynomialError`).
  - **Numeric helpers** — `Abs`, `Floor`, `Ceiling`, `Mod`, `Gcd`, `Lcm`.
- `MacsymaBackend.__init__` now merges `build_cas_handler_table()` into
  `self._handlers` so every CAS head is handled automatically.
- Pre-bound constants: `%pi → IRFloat(π)` and `%e → IRFloat(e)` installed
  in `self._env` at construction time.
- 57 new integration tests in `tests/test_cas_handlers.py` covering:
  handler-table completeness, constant pre-binding, simplify, expand, subst,
  factor (difference of squares, perfect square, linear, no-variable),
  solve (linear, quadratic), all list ops, matrix transpose and determinant,
  limit, taylor, and all numeric helpers; plus edge-case / defensive tests for
  wrong arity, non-list inputs, multi-variate polynomials, and symbolic args.
- Added CAS substrate packages to `pyproject.toml` dependencies:
  `cas-pattern-matching`, `cas-substitution`, `cas-simplify`, `cas-factor`,
  `cas-solve`, `cas-list-operations`, `cas-matrix`, `cas-limit-series`.
- Updated `BUILD` to install CAS deps in correct leaf-to-root order.

## 0.1.0 — 2026-04-25

Initial release — Phase A skeleton.

- `MacsymaBackend` — `SymbolicBackend` subclass with MACSYMA-specific
  heads (`Display`, `Suppress`, `Kill`, `Ev`) and option flags.
- `History` — input/output table, resolves `%`, `%i1`, `%o1`, ...
  via a backend lookup hook.
- `Display` / `Suppress` heads (`;` vs `$` statement terminators).
  Identity handlers; the REPL inspects the wrapper before eval to
  decide whether to print.
- `Kill(symbol)` and `Kill(all)` handlers.
- `Ev(expr, ...flags)` — minimal first cut: only the `numer` flag
  is honored.
- `MACSYMA_NAME_TABLE` — extends `macsyma-compiler`'s standard-name
  map so identifiers like `expand`, `factor`, `subst`, `solve`,
  `taylor`, `limit` route to canonical heads (the substrate handlers
  may not yet exist; the user gets `Expand(...)` unevaluated until
  they do).
- Type-checked, ruff- and mypy-clean.
