# Changelog

## 1.20.0 — 2026-05-04

**Phase 29 — ODE solving and Laplace transforms wired into MacsymaBackend.**

Connects two fully-implemented CAS substrate packages (`cas-ode` and
`cas-laplace`) to the MACSYMA backend so that `ode2`, `laplace`, `ilt`,
`delta`, and `hstep` surface names reach their handlers for the first time.

### New capabilities

#### ODE solving — `ode2(eqn, y, x)`

Seven ODE types are now live through the `ODE2` head:

1. First-order linear: `y' + P(x)·y = Q(x)` (integrating-factor method)
2. Separable: `y' = f(x)·g(y)` (separation of variables)
3. Bernoulli: `y' + P·y = Q·yⁿ` (substitution v = y^(1-n))
4. Exact: `M dx + N dy = 0` when `∂M/∂y = ∂N/∂x` (potential function)
5. Second-order constant-coefficient homogeneous (characteristic equation)
6. Second-order constant-coefficient non-homogeneous (undetermined coefficients)

```macsyma
ode2(y' + 2*y, y, x);          →  Equal(y, %c·exp(-2·x))
ode2(y'' + y, y, x);            →  Equal(y, exp(0)·(%c1·cos(x) + %c2·sin(x)))
ode2(y'' - 3*y' + 2*y, y, x);  →  Equal(y, %c1·exp(x) + %c2·exp(2·x))
```

#### Laplace transforms — `laplace`, `ilt`, `delta`, `hstep`

~15 standard Laplace transform pairs covering the most common circuit analysis
forms:

```macsyma
laplace(exp(2*t), t, s);   →  1/(s-2)
laplace(sin(t), t, s);     →  1/(s^2+1)
ilt(1/(s-3), s, t);        →  exp(3·t)
delta(0);                  →  1
hstep(-3);                 →  0
hstep(0);                  →  1/2
hstep(5);                  →  1
```

### `name_table.py` fix

`ODE2` was previously defined locally as `IRSymbol("ODE2")`.  It is now
imported from `symbolic_ir.nodes` (the canonical singleton), preventing any
accidental identity mismatch.

### What changed

- `src/macsyma_runtime/cas_handlers.py` — imports `build_ode_handler_table`
  and `build_laplace_handler_table`; `build_cas_handler_table()` now calls
  both and merges their results into the dispatch table.
- `src/macsyma_runtime/name_table.py` — imports `ODE2` from
  `symbolic_ir.nodes` instead of re-defining it locally.
- `pyproject.toml` — version 1.19.0 → 1.20.0; adds
  `coding-adventures-cas-ode>=0.2.0` and
  `coding-adventures-cas-laplace>=0.1.0` as runtime dependencies.
- `tests/test_ode_wiring.py` — new file: 18 integration tests for ODE wiring,
  Laplace transforms, DiracDelta/UnitStep folding, and SPICE smoke tests.

---

## 1.19.0 — 2026-05-04

**Phase 28 — `sign` surface syntax + assumption-driven `abs` folding.**

Exposes the Phase 21 assumptions infrastructure (already in `symbolic-vm`)
through the MACSYMA surface name `sign`, and fixes `Abs` handler delegation
so `abs(x)` folds correctly after `assume(x > 0)`.

### New `sign` function

`sign(x)` → `1`, `-1`, `0` for known-sign inputs (numeric or assumed);
`Sign(x)` unevaluated when sign is unknown.

### Assumption-driven simplification in `abs`

`abs(x)` now delegates to the full `symbolic_vm.cas_handlers.abs_handler`
(Phase 28) which consults `vm.assumptions`:

- `assume(x > 0)` or `assume(x >= 0)` → `abs(x)` = `x`
- `assume(x < 0)` → `abs(x)` = `-x`

Previously the local `abs_handler` only folded numeric inputs.

### Surface syntax examples

```macsyma
assume(x > 0);         →  done
is(x > 0);             →  true
is(x >= 0);            →  true    (inferred)
abs(x);                →  x       (sign-simplified)
sign(x);               →  1
forget(x > 0);         →  done
is(x > 0);             →  unknown
assume(x < 0);         →  done
abs(x);                →  -x
sign(x);               →  -1
sign(5);               →  1
sign(-3);              →  -1
```

### What changed

- `src/macsyma_runtime/cas_handlers.py` — removes local `abs_handler`; imports
  and uses `symbolic_vm.cas_handlers.abs_handler` as `_abs_handler_full`.
- `src/macsyma_runtime/name_table.py` — adds `SIGN` symbol and `"sign": SIGN`
  mapping (Phase 28).
- `pyproject.toml` — version 1.18.0 → 1.19.0; bumps `symbolic-vm` dep to ≥ 0.48.0.

---

## 1.18.0 — 2026-05-05

**Phase 27 — Polynomial inequality solving via MACSYMA surface syntax.**

`solve(p(x) op c, x)` (op ∈ {<, >, ≤, ≥}) now returns a list of interval
conditions via the upgraded `symbolic-vm` 0.47.0.  No changes to the
runtime layer itself — this is a dependency bump.

```macsyma
solve(x - 1 > 0, x)           →  [x > 1]
solve(x^2 - 1 < 0, x)         →  [and(x > -1, x < 1)]
solve(x^2 + 1 > 0, x)         →  [0 >= 0]
```

Dependency bump: `symbolic-vm` ≥ 0.47.0 (was ≥ 0.46.0).

---

## 1.17.0 — 2026-05-05

**Phase 26 — Transcendental equation solving via MACSYMA surface syntax.**

Bumps `coding-adventures-symbolic-ir>=0.13.0` and
`coding-adventures-symbolic-vm>=0.46.0`.

### What changed

- `name_table.py`: imports `LAMBERT_W` from `symbolic_ir>=0.13.0` and adds
  `"lambert_w": LAMBERT_W` to `MACSYMA_NAME_TABLE`.  MACSYMA users can now
  call `lambert_w(x)` to invoke the principal Lambert W function W₀(x); the
  symbolic-vm `lambert_w_handler` evaluates it numerically when `x` is
  concrete.
- `solve(eq, var)` now automatically dispatches through the transcendental
  families (trig, exp/log, Lambert W, hyperbolic, compound substitution) via
  the updated `symbolic-vm 0.46.0` without any additional runtime changes.

---

## 1.16.0 — 2026-05-04

**Phase 25 — Symbolic summation and product evaluation via MACSYMA surface syntax.**

Bumps `coding-adventures-symbolic-ir>=0.12.0` and
`coding-adventures-symbolic-vm>=0.45.0`.

### What changed

- `cas_handlers.py`: imports `sum_handler` and `product_handler` from
  `symbolic_vm.cas_handlers`, registers them as `"Sum"` and `"Product"`.

The MACSYMA compiler (bumped to 0.9.0 separately) maps:
- `sum(f, k, a, b)` → `Sum(f, k, a, b)` IR node
- `product(f, k, a, b)` → `Product(f, k, a, b)` IR node

The VM's `sum_handler` and `product_handler` (added in symbolic-vm 0.45.0)
evaluate these via `cas-summation` 0.1.0.

## 1.15.0 — 2026-05-04

**Phase 24 — Definite integration end-to-end via MACSYMA surface syntax.**

Bumps `coding-adventures-symbolic-vm>=0.44.0`.

### What changed

No new `cas_handlers.py` or `name_table.py` entries are required.  The
MACSYMA compiler already maps `integrate(f, x, a, b)` → the 4-argument
`Integrate(f, x, a, b)` IR node, and the VM's `integrate()` handler
(updated in symbolic-vm 0.44.0) now processes the 4-argument form using
the Fundamental Theorem of Calculus.

The end-to-end surface form now works:

```
integrate(exp(-x^2), x, 0, %inf)   →   sqrt(%pi) / 2
integrate(x^2, x, 0, 1)            →   1/3
integrate(sin(x), x, 0, %pi)       →   2
integrate(sin(x)/x, x, 0, %inf)    →   %pi/2
integrate(log(x), x, 0, 1)         →   -1
```

Infinite limits `%inf` and `%minf` are recognised by the VM's definite-
integration machinery.

---

## 1.14.0 — 2026-05-04

**Phase 23 — Wire MACSYMA surface syntax for special functions (erf, Si/Ci,
Li₂, Gamma/Beta, Fresnel).**

Bumps `symbolic-ir>=0.11.0` and `symbolic-vm>=0.43.0`.

### `name_table.py`

Adds 13 new entries to `MACSYMA_NAME_TABLE`:

| MACSYMA name | IR head |
|---|---|
| `erf` | `ERF` |
| `erfc` | `ERFC` |
| `erfi` | `ERFI` |
| `si` | `SI` |
| `ci` | `CI` |
| `shi` | `SHI` |
| `chi` | `CHI` |
| `li2` | `LI2` |
| `gamma` | `GAMMA_FUNC` |
| `beta` | `BETA_FUNC` |
| `fresnel_s` | `FRESNEL_S` |
| `fresnel_c` | `FRESNEL_C` |

### `cas_handlers.py`

Delegates all 12 special-function handlers from
`symbolic_vm.cas_handlers` into the MacsymaBackend handler table.

### Example MACSYMA sessions

```
(%i1) integrate(exp(-x^2), x);
(%o1)                   sqrt(%pi)*erf(x)/2

(%i2) integrate(sin(x)/x, x);
(%o2)                   si(x)

(%i3) gamma(5);
(%o3)                   24

(%i4) gamma(1/2);
(%o4)                   sqrt(%pi)

(%i5) beta(1/2, 1/2);
(%o5)                   %pi
```

---

## 1.13.0 — 2026-05-04

**Phase 22 — Wire MACSYMA surface syntax for matchdeclare / defrule / apply1 / apply2 / tellsimp.**

### Added

- `name_table.py` — 5 new entries in `MACSYMA_NAME_TABLE` so the compiler
  maps the lowercase MACSYMA keywords to their IR heads:
  - `"matchdeclare"` → `MATCHDECLARE`
  - `"defrule"` → `DEFRULE`
  - `"apply1"` → `APPLY1`
  - `"apply2"` → `APPLY2`
  - `"tellsimp"` → `TELLSIMP`

- `cas_handlers.py` — 5 new handler table entries that delegate directly to
  the implementations in `symbolic_vm.cas_handlers`:
  - `"MatchDeclare"` → `matchdeclare_handler`
  - `"Defrule"` → `defrule_handler`
  - `"Apply1"` → `apply1_handler`
  - `"Apply2"` → `apply2_handler`
  - `"TellSimp"` → `tellsimp_handler`

  The heads are already in `_HELD_HEADS` via `SymbolicBackend` (inherited
  by `MacsymaBackend`), so pattern arguments reach handlers unevaluated.

### Dependency bumps

- `coding-adventures-symbolic-ir>=0.10.0`
- `coding-adventures-symbolic-vm>=0.42.0`
- `coding-adventures-cas-pattern-matching>=0.2.0`

---

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

---

## 1.12.0 — 2026-04-28

**Phase G (control-flow grammar) — tests and stale-comment cleanup.**

Control-flow constructs (`if/then/else`, `for…thru`, `for…in`, `while…do`,
`block([locals], …)`, `return()`) were already fully implemented across the
grammar, lexer, parser, compiler, and VM layers.  This release completes the
Phase G story for `macsyma-runtime`:

- `tests/test_control_flow.py` (**NEW**): 42 end-to-end tests that drive
  every control-flow construct through the full pipeline
  `parse_macsyma → compile_macsyma (with extended name table) → VM(MacsymaBackend)`.
  Covers interaction with CAS operations (factor, solve, length, etc.),
  multi-statement programs, function definitions inside blocks, and
  `MacsymaBackend`-specific features (history, kill).
- `src/macsyma_runtime/heads.py`: corrected stale comment on `BLOCK`
  ("reserved for Phase G; not yet handled" → "handled by symbolic-vm's
  `block_` handler").
- `pyproject.toml`: bumped `symbolic-vm` floor to `>=0.34.0`.

Total tests: **177** (up from 135). Coverage: **98.71 %**.

---

## 1.11.0 — 2026-04-28

**Add Group E matrix operation names to the MACSYMA name table.**

- `name_table.py`: added seven new `IRSymbol` singletons:
  `DOT`, `TRACE`, `DIMENSIONS`, `IDENTITY_MATRIX`, `ZERO_MATRIX`, `RANK`,
  `ROW_REDUCE`.
- Added seven new entries to `MACSYMA_NAME_TABLE`:

  | MACSYMA name | IR head |
  |---|---|
  | `dot` | `Dot` |
  | `mattrace` | `Trace` |
  | `matrix_size` | `Dimensions` |
  | `ident` | `IdentityMatrix` |
  | `zeromatrix` | `ZeroMatrix` |
  | `rank` | `Rank` |
  | `rowreduce` | `RowReduce` |

MACSYMA users can now write:
- `dot(A, B)` → matrix product
- `mattrace(M)` → sum of diagonal (MACSYMA canonical spelling is `mattrace`)
- `matrix_size(M)` → `[rows, cols]` shape
- `ident(n)` → n×n identity matrix
- `zeromatrix(m, n)` → m×n zero matrix
- `rank(M)` → rank of matrix (integer)
- `rowreduce(M)` → reduced row-echelon form

Bumped `symbolic-vm` dependency floor to `>=0.33.0`.

---

## 1.10.0 — 2026-04-28

**Add Gröbner basis and multivariate solve operations to the MACSYMA name table.**

- Imported `GROEBNER`, `POLY_REDUCE`, `IDEAL_SOLVE` from `symbolic_ir`.
- Added `"groebner": GROEBNER`, `"poly_reduce": POLY_REDUCE`,
  `"ideal_solve": IDEAL_SOLVE` to `MACSYMA_NAME_TABLE`.
- Bumped `symbolic-ir` dependency to `>=0.7.6` and `symbolic-vm` to `>=0.32.8`.

MACSYMA users can now write:
- `groebner([x^2+y-1, x+y^2-1], [x,y])` → Gröbner basis
- `ideal_solve([x+y-1, x-y], [x,y])` → `[Rule(x,1/2), Rule(y,1/2)]`
- `poly_reduce(f, [g1,...], [x,y])` → reduced polynomial

---

## 1.9.0 — 2026-04-27

**Add `algfactor` to the MACSYMA name table (D5 — cas-algebraic).**

- `name_table.py`: added `ALG_FACTOR = IRSymbol("AlgFactor")` and the mapping
  `"algfactor": ALG_FACTOR` so MACSYMA users can write
  `algfactor(x^4+1, sqrt(2))` and have it compile to `AlgFactor(x^4+1, Sqrt(2))`
  IR, which the `cas-algebraic` handler then factors over Q[√d].
- Bumped `symbolic-ir` dependency to `>=0.7.5` and `symbolic-vm` to `>=0.32.7`.

---

## 1.8.0 — 2026-04-27

**Add `ode2` to the MACSYMA name table (D3).**

- `name_table.py`: added `ODE2 = IRSymbol("ODE2")` and the mapping
  `"ode2": ODE2` so MACSYMA users can write `ode2(eqn, y, x)` and have
  it compile to `ODE2(eqn, y, x)` IR, which the `cas-ode` handler then
  solves.

---

## 1.7.0 — 2026-04-27

**Add Fourier transform operations to the MACSYMA name table.**

- Imported `FOURIER` and `IFOURIER` from `symbolic_ir` (added in 0.7.3).
- Added `"fourier": FOURIER` and `"ifourier": IFOURIER` to `MACSYMA_NAME_TABLE`.
- Bumped `symbolic-ir` dependency to `>=0.7.3` and `symbolic-vm` to `>=0.32.4`.

MACSYMA users can now write `fourier(f, t, omega)` and `ifourier(F, omega, t)`.

---

## 1.6.0 — 2026-04-27

**Add Laplace transform operations to the MACSYMA name table.**

- Imported `LAPLACE`, `ILT`, `DIRAC_DELTA`, `UNIT_STEP` from `symbolic_ir` into
  `name_table.py`.
- Added five entries to `MACSYMA_NAME_TABLE`:
  - `"laplace": LAPLACE`
  - `"ilt": ILT`
  - `"delta": DIRAC_DELTA` — Dirac delta δ(t), MACSYMA convention
  - `"hstep": UNIT_STEP` — Heaviside step H(t), MACSYMA convention
  - `"unit_step": UNIT_STEP` — common alias

Now MACSYMA users can write e.g. `laplace(sin(t), t, s)` and get `1/(s^2+1)`.

---

## 1.5.0 — 2026-04-27

**Add `mnewton` to the MACSYMA name table.**

- Added `MNEWTON = IRSymbol("MNewton")` to `name_table.py`.
- Added `"mnewton": MNEWTON` to `MACSYMA_NAME_TABLE` in the numeric/solve
  section so `mnewton(f, x, x0)` MACSYMA surface syntax compiles to the
  canonical `MNewton(f, x, x0)` IR head handled by `cas-mnewton`.
- Bumped `coding-adventures-symbolic-vm>=0.32.2` to pull in the MNewton
  handler wired in 0.32.2.

Usage in MACSYMA::

    mnewton(x^2 - 2, x, 1.5);    → 1.4142135623730951
    mnewton(sin(x), x, 3.0);      → 3.141592653589793

---

## 1.4.0 — 2026-04-27

**Bump dependency pins to pick up Phase G control flow and Phase 13 hyperbolic functions.**

No source changes to `macsyma-runtime` itself. The two upstream dependencies are
pinned higher so that installing this package pulls in all recently-landed VM
capabilities:

- `coding-adventures-symbolic-ir>=0.7.0` — picks up the Phase 13 IR heads for
  hyperbolic functions (`Sinh`, `Cosh`, `Tanh`, `Asinh`, `Acosh`, `Atanh`) and
  their evaluation, differentiation, and integration rules.
- `coding-adventures-symbolic-vm>=0.32.1` — picks up Phase G control-flow VM
  handlers (`while`-loop, `for..thru`/`for..in`, `block` with local scope,
  `return`, `if/elseif/else`) and the 0.32.1 bug fix: missing hyperbolic
  differentiation rules in `derivative.py` that caused `diff(sinh(x),x)` to
  raise `RecursionError`.

Test count and coverage unchanged (135 tests, ≥80 %).

## 1.3.0 — 2026-04-27

**Add `is_prime` alias for `primep` in the MACSYMA name table.**

MACSYMA's canonical name for the primality predicate is `primep`, but
interactive users often type `is_prime` (following Python/Julia/etc. naming
conventions). The runtime's name table now maps both identifiers to the same
`IsPrime` IR head, so `is_prime(17)` evaluates to `True` instead of returning
the expression unevaluated.

2 new pipeline tests added (`test_pipeline_is_prime_alias_true/false`);
total test count 135, coverage maintained at ≥ 80 %.

## 1.2.0 — 2026-04-27

**Wire `ratsimp` and `trigsimp` flags in `ev` handler (A3 + B1).**

Completes the `ev` flag set by implementing the two previously-documented
but not-yet-implemented flags:

- `ratsimp` — applies `RatSimplify` (cancel GCD of numerator/denominator)
  to the evaluated result. Example: `ev((x^2-1)/(x-1), ratsimp)` → `x+1`.
- `trigsimp` — applies `TrigSimplify` (Pythagorean and related identities)
  to the evaluated result. Example: `ev(sin(x)^2 + cos(x)^2, trigsimp)` → `1`.

Both flags use the A3/B1 substrate handlers already registered on
`SymbolicBackend`, so no new CAS code was needed — only the `ev` dispatch
layer required updating.

2 new tests added to `test_ev.py`; total test count 133, coverage 98.6%.

## 1.1.0 — 2026-04-27

**Comprehensive pipeline test coverage (Sections S and T).**

Adds 20 new end-to-end pipeline tests covering operations that were already
wired in `symbolic-vm` but had no MACSYMA surface coverage:

**Section S — Calculus**:
- `diff(x^3, x)`, `diff(x^2+2x+1, x)` — polynomial differentiation.
- `diff(sin(x), x)` → `cos(x)`.
- `diff(cos(x), x)` → `-sin(x)`.
- `diff(exp(x), x)` → `exp(x)`.
- `integrate(x^2, x)` — power rule.
- `integrate(sin(x), x)` → `-cos(x)` (verifies `Cos` in result).
- `integrate(cos(x), x)` → `sin(x)`.
- `integrate(exp(x), x)` → `exp(x)`.
- `integrate(3, x)` → `3*x` (constant rule).
- `integrate(x+1, x)` — linearity.

**Section T — Matrix + Numeric**:
- `matrix([1,2],[3,4])` → `Matrix` node with 2 rows.
- `determinant(matrix([1,2],[3,4]))` → `IRInteger(-2)`.
- `transpose(matrix([1,2],[3,4]))` → `Matrix` node.
- `gcd(12, 8)` → `IRInteger(4)`.
- `lcm(4, 6)` → `IRInteger(12)`.
- `mod(17, 5)` → `IRInteger(2)`.
- `floor(3.7)` → `IRInteger(3)`.
- `ceiling(3.2)` → `IRInteger(4)`.
- `abs(-5)` → `IRInteger(5)`.

Total tests: 131, coverage 98.6%.

## 1.0.0 — 2026-04-27

**Roadmap item A1 — Kronecker polynomial factoring surfaced through MACSYMA `factor`.**

Bumps dependency to `symbolic-vm>=0.27.0` which ships `cas-factor 0.2.0`
with Kronecker's algorithm.  No changes to `macsyma-runtime` source code;
the improvement flows through automatically since `factor` → `Factor` IR head
→ `factor_handler` → `factor_integer_polynomial` → Kronecker.

4 new pipeline tests in `test_cas_pipeline.py` (Section R):
- `factor(x^4 + 4)` → `Mul` (Sophie Germain: `(x²+2x+2)(x²-2x+2)`).
- `factor(x^4 + x^2 + 1)` → `Mul` (cyclotomic: `(x²+x+1)(x²-x+1)`).
- `factor(x^3 - 2x^2 + x - 2)` → `Mul` (mixed: linear `(x-2)` + `(x²+1)`).
- `factor(x^2 + 1)` → `Factor(x^2+1)` unevaluated (irreducible over Z).

## 0.9.0 — 2026-04-27

**Rational function operations wired into MACSYMA name table (A3).**

Adds `MACSYMA_NAME_TABLE` entries:
`collect→Collect`, `together→Together`, `ratsimp→RatSimplify`,
`partfrac→Apart`.

These map to four new IR heads in `symbolic-vm` 0.26.0:
- `collect(expr, x)` — collect terms by powers of x.
- `together(expr)` — combine fractions into one rational expression.
- `ratsimp(expr)` — cancel common polynomial factors.
- `partfrac(expr, x)` — partial fraction decomposition.

Also adds `COLLECT`, `TOGETHER`, `RAT_SIMPLIFY`, `APART` IR symbol constants
to `name_table.py`.

6 new pipeline tests in `test_cas_pipeline.py` (Section Q) cover the full
MACSYMA surface syntax end-to-end.

## 0.8.0 — 2026-04-27

**Trig operations wired into MACSYMA name table (B1).**

Adds `MACSYMA_NAME_TABLE` entries:
`trigsimp→TrigSimplify`, `trigexpand→TrigExpand`, `trigreduce→TrigReduce`.

5 new pipeline tests cover the MACSYMA surface syntax end-to-end:
- `trigsimp(sin(x)^2 + cos(x)^2)` → `1`.
- `trigsimp(sin(%pi))` → `0`.
- `trigsimp(cos(%pi))` → `-1`.
- `trigexpand(sin(2*x))` → expanded form.
- `trigreduce(sin(x)^2)` → multiple-angle form.

## 0.7.0 — 2026-04-27

**NSolve and linear system pipeline tests added (A2c / A2d).**

Adds MACSYMA name-table entries `nsolve→NSolve` and `linsolve→Solve`
(linear-system form).  4 new pipeline tests cover:
- `nsolve(x^3 - 6*x^2 + 11*x - 6, x)` → 3 numeric IRFloat roots.
- `nsolve(x^5 - 1, x)` → 5 roots.
- `linsolve([x+y=3, x-y=1], [x,y])` → `[Rule(x,2), Rule(y,1)]`.
- `linsolve([x+y+z=6, 2*x+y=5, z=3], [x,y,z])` → 3 rules.

## 0.6.0 — 2026-04-27

**Cubic and quartic solve pipeline tests added (A2a / A2b).**

5 new pipeline tests in `test_cas_pipeline.py` cover cubic and quartic
equation solving end-to-end through the MACSYMA surface syntax:
`solve(x^3 - 6*x^2 + 11*x - 6, x)` → `[1, 2, 3]`, etc.
These tests exercise the full pipeline: MACSYMA parser → compiler →
`solve_handler` → `solve_cubic` / `solve_quartic` from `cas-solve`.

## 0.5.0 — 2026-04-27

**Complex number MACSYMA names wired (B2).**

Adds `MACSYMA_NAME_TABLE` entries for complex-number operations:
`%i`→`ImaginaryUnit`, `realpart`→`Re`, `imagpart`→`Im`,
`conjugate`→`Conjugate`, `cabs`→`Abs`, `carg`→`Arg`,
`rectform`→`RectForm`, `polarform`→`PolarForm`.

Pre-binds `%i` to `ImaginaryUnit` in `MacsymaBackend.__init__` so the
imaginary-unit constant is available without a prior `%i : ImaginaryUnit`
assignment.

11 new pipeline tests in `test_cas_pipeline.py` cover the MACSYMA surface
syntax end-to-end (`%i`, `realpart`, `imagpart`, `conjugate`, `%i^n`).

## 0.4.0 — 2026-04-27

**Number theory MACSYMA names wired (B3).**

Adds `MACSYMA_NAME_TABLE` entries for all number-theory heads:
`primep`→`IsPrime`, `next_prime`→`NextPrime`, `prev_prime`→`PrevPrime`,
`ifactor`→`FactorInteger`, `divisors`→`Divisors`, `totient`→`Totient`,
`moebius`→`MoebiusMu`, `jacobi`→`JacobiSymbol`, `chinese`→`ChineseRemainder`,
`numdigits`→`IntegerLength`.

6 new pipeline tests in `test_cas_pipeline.py` cover the MACSYMA surface
syntax end-to-end.

## 0.3.0 — 2026-04-27

**MACSYMA completion roadmap items C2, C3, C4, C5 wired.**

Implements the language-layer bindings for the new IR heads added to
`symbolic-vm` 0.20.0, plus improvements to `ev` flag handling.

**Name-table additions** (`MACSYMA_NAME_TABLE`):
- `lhs` → `Lhs`  — left-hand side of an equation (C5).
- `rhs` → `Rhs`  — right-hand side of an equation (C5).
- `at`  → `At`   — point evaluation (C4).
- `makelist` corrected: now maps to `MakeList` (proper generative list)
  instead of `Range` (plain integer range).

**`ev` flag improvements** (C3):
- `expand` flag: applies `Expand` to the result.
- `factor` flag: applies `Factor` to the result.
- `float` flag: alias for `numer` (force floating-point collapse).
- Unknown flags continue to be silently ignored.

**Tests added**:
- 9 new pipeline tests in `test_cas_pipeline.py` covering `lhs`, `rhs`,
  `makelist` (3-arg, 4-arg, 5-arg), and `at` (single rule, multi-rule).
- 3 new ev tests in `test_ev.py` covering `float`, `expand`, `factor` flags.

## 0.2.0 — 2026-04-27

**Name table wired; constants pre-bound.**

This release completes the two missing connections that prevented the MACSYMA
REPL from dispatching algebraic operations to the CAS substrate.

**`backend.py`** — `MacsymaBackend.__init__` now pre-binds:
- `%pi` → `IRFloat(math.pi)`
- `%e`  → `IRFloat(math.e)`

Users can now type `%pi` and `%e` without defining them first.

**`language.py` (macsyma-repl)** — `extend_compiler_name_table(_STANDARD_FUNCTIONS)`
is now called at REPL module load time. This merges `MACSYMA_NAME_TABLE`
into the compiler's `_STANDARD_FUNCTIONS` dict so that `factor`, `expand`,
`simplify`, `solve`, `subst`, `limit`, `taylor`, `length`, `first`, etc.
all compile to canonical IR heads (`Factor`, `Expand`, `Simplify`, …) rather
than opaque user-function calls.

**Architecture note** (see also `symbolic-vm` 0.19.0): The substrate handlers
themselves (`Factor`, `Solve`, `Simplify`, `Length`, `Determinant`, `Limit`,
…) now live in `symbolic-vm`'s `SymbolicBackend` — the inner doll. The
`MacsymaBackend` (outer doll) only adds MACSYMA-specific operations:
`Display`, `Suppress`, `Kill`, `Ev`, and the two constant bindings above.
Future Maple and Mathematica backends will extend `SymbolicBackend` directly
and inherit all algebraic operations without touching any MACSYMA code.

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
