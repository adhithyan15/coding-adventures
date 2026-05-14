# Changelog

## 0.2.1 — 2026-05-14

**Bug fix: `ZeroDivisionError` from direct substitution now routes to L'Hôpital.**

In `limit_advanced.py`, the direct-substitution step called `eval_fn(subst_result)` without
catching arithmetic exceptions.  For expressions like `sin(x)/x` at `x=0`, the VM raises
`ZeroDivisionError` when evaluating `sin(0)/0 = 0/0`.  Previously this exception propagated
to the caller instead of routing to `_handle_form` (the L'Hôpital / rewrite engine).

Fixed by wrapping the `eval_fn` call in `try/except (ZeroDivisionError, ArithmeticError)`
and delegating to `_handle_form` on any arithmetic singularity.  This makes
`limit(sin(x)/x, x, 0)` evaluate to `1` end-to-end.

## 0.2.0 — 2026-05-04

**Phase 20 — Full limit evaluation: L'Hôpital, ±∞, indeterminate forms, one-sided limits.**

### New public API

| Symbol | Description |
|--------|-------------|
| `limit_advanced(expr, var, point, direction, *, diff_fn, eval_fn)` | Full limit evaluator |
| `INF` | `IRSymbol("inf")` sentinel for +∞ |
| `MINF` | `IRSymbol("minf")` sentinel for −∞ |

### Capabilities added

- **Direct substitution** — tried first; returns substituted + simplified result when
  the expression is continuous at the point.
- **L'Hôpital's rule** — applied iteratively (up to depth 8) for `0/0` and `∞/∞`
  indeterminate ratios.  Requires an injected `diff_fn` callable.
- **Limits at ±∞** — `IRSymbol("inf")` / `IRSymbol("minf")` as limit point.
  Numeric evaluator maps these to `math.inf` using Python IEEE 754.
- **Indeterminate forms** — all six standard forms handled:
  - `0/0`, `∞/∞` → L'Hôpital
  - `0·∞` → rewrite `MUL(a,b)` as `DIV(b, DIV(1,a))` then L'Hôpital
  - `1^∞`, `0^0`, `∞^0` → `EXP(MUL(e, LOG(b)))` transform then recurse
- **One-sided limits** — `direction="plus"` (right) / `direction="minus"` (left);
  a `±1×10⁻³⁰⁰` perturbation classifies the form while exact point is used
  for symbolic substitution.
- **Architecture** — `diff_fn` and `eval_fn` are injected callables; no dependency
  on `symbolic-vm` (avoids circular import).

### New heads in `heads.py`

- `INF = IRSymbol("inf")` — positive infinity sentinel
- `MINF = IRSymbol("minf")` — negative infinity sentinel

### New module

- `cas_limit_series.limit_advanced` — contains all Phase 20 logic

---

## 0.1.0 — 2026-04-25

Initial release — Phase 1 foundation.

- Sentinel heads: ``LIMIT``, ``TAYLOR``, ``SERIES``, ``BIG_O``.
- ``limit_direct(expr, var, point)`` — direct substitution; falls
  back to unevaluated ``Limit(...)``.
- ``taylor_polynomial(p, var, point, order)`` — Taylor expansion for
  polynomial expressions (Add, Mul, Pow, Neg of literals and a
  single ``var``). Pure-Python; no dependency on symbolic-vm.
- Type-checked, ruff- and mypy-clean.

Deferred to follow-ups: L'Hôpital, transcendental Taylor, asymptotic
series.
