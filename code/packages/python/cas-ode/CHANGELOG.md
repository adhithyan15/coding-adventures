# Changelog — cas-ode

All notable changes to this project will be documented in this file.

## [0.1.0] — 2026-04-27

### Added

- **Package foundation** — `cas-ode` 0.1.0 initial release.

- **First-order linear ODE solver** (`solve_linear_first_order`)
  - Recognises the standard form `y' + P(x)·y = Q(x)` by inspecting the
    flattened summands of the ODE expression.
  - Computes the integrating factor `μ = exp(∫ P dx)` via the VM's
    Integrate handler.
  - Returns `Equal(y, (1/μ) · (∫ μQ dx + %c))`.
  - Falls through gracefully if either integral is unevaluated (returns
    the original `ODE2(...)` node unchanged).

- **Separable ODE recogniser** (`_try_separable`)
  - Handles `y' = f(x)` (pure quadrature).
  - Handles `y' = k·y` (constant-coefficient decay/growth) by delegating
    to the linear solver.
  - Handles `y' = f(x)·k·y` (separable linear product) via the linear
    solver with `P = -k·f(x)`.
  - Handles `y' = f(x)·g(y) + Q(x)` generically by decomposing the RHS
    into a y-coefficient and a constant-with-respect-to-y term.

- **Second-order constant-coefficient solver**
  (`solve_second_order_const_coeff`)
  - Recognises `a·y'' + b·y' + c·y = 0` by pattern-matching against the
    flattened Add tree, extracting rational (Fraction) coefficients.
  - Solves the characteristic equation `a·r² + b·r + c = 0`:
    - Distinct real roots: `y = C1·exp(r1·x) + C2·exp(r2·x)`
    - Repeated root: `y = (C1 + C2·x)·exp(r·x)`
    - Complex conjugate roots `α±βi`: `y = exp(αx)·(C1·cos(βx) + C2·sin(βx))`
  - Handles irrational discriminants with symbolic `Pow(disc, 1/2)` nodes.
  - Uses `Fraction` arithmetic throughout — no floats for exact cases.

- **ODE2 VM handler** (`ode2_handler`, `build_ode_handler_table()`)
  - Accepts `ODE2(eqn, y, x)` where `eqn` may be a raw expression
    (assumed `= 0`) or an `Equal(lhs, rhs)` form.
  - Returns `Equal(y, solution)` on success; returns the unevaluated
    `ODE2(...)` node on failure (graceful fall-through).

- **Integration constants**
  - `%c` (`C_CONST`) — first-order ODE constant.
  - `%c1` (`C1`), `%c2` (`C2`) — second-order ODE constants.
  - Defined as IR symbols in `symbolic_ir/nodes.py` (version bump to 0.7.4).

- **Utility helpers** — `_flatten_add`, `_extract_coeff`,
  `_is_const_wrt`, `_isqrt_exact`, `_exact_sqrt_fraction`, and IR
  node builders (`_add`, `_mul`, `_sub`, `_div`, `_pow`, `_exp`, etc.)

- **88 tests** across 14 test classes covering all code paths.
  Coverage: 82%.

### Wired into the VM

- `symbolic_vm/cas_handlers.py` — calls `build_ode_handler_table()` and
  merges into the handler table (version bump to 0.32.5).
- `symbolic_vm/backends.py` — added `"ODE2"` to `_HELD_HEADS` so that
  `D(y, x)` inside the ODE expression is not pre-evaluated to `0`.
- `macsyma_runtime/name_table.py` — maps `"ode2"` to `ODE2` symbol
  (version bump to 1.8.0).
- `symbolic_ir` — added `ODE2`, `C_CONST`, `C1`, `C2` symbols and
  exports (version bump to 0.7.4).

### Not implemented

- **Bernoulli ODEs** (`dy/dx + P(x)·y = Q(x)·y^n`) — requires a
  `y^(1-n)` substitution and general rational-power symbolic handling.
  Deferred to a future `cas-ode` release.
- **Second-order with variable coefficients** — returns unevaluated
  (correct fall-through).
- **Non-homogeneous second-order** — method of undetermined coefficients
  or variation of parameters; deferred.
