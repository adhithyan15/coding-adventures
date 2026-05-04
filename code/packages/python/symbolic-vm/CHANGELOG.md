# Changelog

## 0.41.0 — 2026-05-04

**Phase 21 — Assumption framework, `radcan`, `logcontract`/`logexpand`,
`exponentialize`/`demoivre`.**

Bumps `coding-adventures-cas-simplify` to `>=0.3.0` and
`coding-adventures-symbolic-ir` to `>=0.9.0`.

### Changes in `vm.py`

- `VM.__init__` gains `self.assumptions = AssumptionContext()`.  This
  per-session mutable store is shared across all handlers so `assume(x > 0)`
  recorded in one handler call is visible to `radcan`, `logexpand`, etc. in
  the same session.

### New handlers in `cas_handlers.py`

Nine new handlers registered in `build_cas_handler_table()`:

| Head              | Handler                 | Action                                   |
|-------------------|-------------------------|------------------------------------------|
| `Assume`          | `assume_handler`        | Record a relational or property fact     |
| `Forget`          | `forget_handler`        | Remove a specific fact or clear all      |
| `Is`              | `is_handler`            | → "true" / "false" / "unknown"           |
| `Sign`            | `sign_handler`          | → 1 / -1 / 0 / unevaluated              |
| `Radcan`          | `radcan_handler`        | Radical canonicalization                 |
| `LogContract`     | `logcontract_handler`   | Combine log sums into one log            |
| `LogExpand`       | `logexpand_handler`     | Expand log over products and powers      |
| `Exponentialize`  | `exponentialize_handler`| Trig/hyp → exp form                     |
| `DeMoivre`        | `demoivre_handler`      | exp(a+bi) → exp(a)·(cos b + i·sin b)    |

---

## 0.40.0 — 2026-05-04

**Phase 20 — Upgraded `limit_handler`: L'Hôpital, ±∞, indeterminate forms, one-sided limits.**

Bumps `coding-adventures-cas-limit-series` minimum version from unversioned to
`>=0.2.0` and upgrades the `Limit` VM handler to use `limit_advanced`.

### Changes in `cas_handlers.py`

- **`limit_handler`** now calls `cas_limit_series.limit_advanced` with injected
  `diff_fn=lambda e,v: vm.eval(_symbolic_diff(e,v))` and `eval_fn=vm.eval`.
  Supports the optional 4th argument for direction: `Limit(expr, var, point, plus)`
  or `Limit(expr, var, point, minus)`.
- The old Phase 1 direct-substitution-only path is fully replaced.
- Falls through to unevaluated `Limit(…)` when the limit cannot be determined
  (oscillating, unknown form, depth exceeded).

---

## 0.39.0 — 2026-05-04

**Phase 19 — Linear algebra completion: 8 new VM handlers for `cas-matrix` 0.3.0.**

Bumps `coding-adventures-cas-matrix` minimum version from `>=0.2.0` to
`>=0.3.0` and wires in handlers for all six new linear algebra operations
added in `cas-matrix` 0.3.0.

### New VM handlers in `cas_handlers.py`

| Handler | Expression form | Returns |
|---------|-----------------|---------|
| `eigenvalues_handler` | `Eigenvalues(M)` | `List(List(λ₁,m₁), …)` — eigenvalue/multiplicity pairs |
| `eigenvectors_handler` | `Eigenvectors(M)` | `List(List(λ,m,List(v₁,…)), …)` — eigenvalue + basis vectors |
| `charpoly_handler` | `CharPoly(M, λ)` | IR polynomial `det(λI−M)` in the given symbol |
| `lu_handler` | `LU(M)` | `List(L, U, P)` from Doolittle partial-pivoting decomposition |
| `nullspace_handler` | `NullSpace(M)` | `List(v₁,…)` of n×1 null-space basis column vectors |
| `columnspace_handler` | `ColumnSpace(M)` | `List(c₁,…)` of m×1 column-space basis vectors |
| `rowspace_handler` | `RowSpace(M)` | `List(r₁,…)` of 1×n row-space basis vectors |
| `norm_handler` | `Norm(v)` or `Norm(M, frobenius)` | Euclidean or Frobenius norm |

All handlers fall through to the unevaluated `IRApply` on `MatrixError` (symbolic
entries, wrong shape, size > 4×4 for eigenvalue routines, etc.).

`CharPoly` requires the second argument to be an `IRSymbol` naming the variable.
`Norm` with `frobenius` keyword accepts `IRSymbol("frobenius")` as the kind argument.

### Fallthrough semantics

All eight handlers return the original unevaluated `expr` on any `MatrixError`,
matching the pattern used by `rank_handler`, `row_reduce_handler`, and the other
existing matrix handlers.

---

## 0.38.0 — 2026-04-29

**Phase 18 — `cas-ode` 0.2.0 dependency bump (Bernoulli · Exact · Non-homogeneous 2nd-order).**

Bumps the `coding-adventures-cas-ode` minimum version from `>=0.1.0` to
`>=0.2.0` to pull in the three new ODE solver classes added in `cas-ode`
0.2.0.  No changes to `symbolic_vm` source code — this is a pure dependency
upgrade so users get the improved `ode2(...)` evaluation automatically.

### What changed in `cas-ode` 0.2.0

- **Bernoulli ODE solver** (`y' + P(x)·y = Q(x)·y^n`, n ≠ 0,1):
  substitution `v = y^(1−n)` reduces to first-order linear; returns
  `Equal(y, v_sol^(1/(1−n)))`.
- **Exact ODE solver** (`M dx + N dy = 0`, ∂M/∂y = ∂N/∂x):
  computes potential F and returns the implicit solution `Equal(F + g, %c)`.
  Exactness check uses numerical evaluation to handle structurally different
  but mathematically equal IR trees.
- **Second-order non-homogeneous solver** (`a·y'' + b·y' + c·y = f(x)`):
  undetermined coefficients for constant, polynomial (≤ degree 2),
  exponential, trig, and exp×trig forcing; full resonance handling
  (s = 0, 1, 2 multiplicity shifts).
- Dispatcher updated: non-homogeneous 2nd-order checked before homogeneous;
  exact solver runs last to preserve explicit `Equal(y, f(x))` forms.
- 47 new tests (135 total); combined coverage 82.89%.

---

## 0.37.0 — 2026-04-28

**Phase 17 — `∫ tanh^n(ax+b) dx` power reduction.**

Completes the hyperbolic power-reduction suite (Phases 14 and 16 covered the
other five functions; tanh was the only remaining gap).

### Algorithm

Uses the Pythagorean identity `tanh²(t) = 1 − sech²(t)`:

```
I_n = I_{n-2} − tanh^(n-1)(ax+b) / ((n-1)·a)

Base cases:
  n = 0  →  x
  n = 1  →  log(cosh(ax+b)) / a      [Phase 13 bare tanh integral]
```

This is the direct analog of `coth^n` (Phase 16) — same recursion structure,
different Pythagorean identity (`−sech²` vs `+csch²`).

**Verification examples:**
- n=2: `F = x − tanh(x)`.  `F' = 1 − sech² = tanh²`  ✓
- n=3: `F = log(cosh) − tanh²/2`.  `F' = tanh − tanh·sech² = tanh³`  ✓
- n=4: `F = x − tanh − tanh³/3`.  `F' = tanh²(1−sech²) = tanh⁴`  ✓

### Implementation

- `recip_hyp_power_integral.py` — new public function `tanh_power_integral(n, a, b, x)`;
  `COSH` added to imports (needed for the n=1 base-case `log(cosh)/a`).
- `integrate.py` — `_try_recip_hyp_power` extended to include `TANH` in the
  handled-head set; `tanh_power_integral` added to the import block.

### Tests (`test_phase17.py`) — 16 tests

| Class | Tests | What is verified |
|-------|-------|-----------------|
| `TestPhase17_TanhPowers` | 10 | n=2,3,4,5; a=2; b=1; a=1/2; structural (Tanh/log(cosh)) |
| `TestPhase17_Fallthrough` | 3 | poly×tanh², non-linear arg, poly×tanh (non-elementary) |
| `TestPhase17_Regressions` | 3 | Phase 16 sech², Phase 13 tanh bare, Phase 14 sinh^4 |

All antiderivatives verified numerically at two test points.

---

## 0.36.0 — 2026-04-28

**Phase 16 — Reciprocal hyperbolic power integrals: `sech^n`, `csch^n`, `coth^n`.**

Closes the gap left by Phase 15, which deferred `∫ sech²(x) dx` and all higher
powers of the three reciprocal hyperbolic functions.  Each family gets a full
IBP (or identity-based) reduction formula valid for any non-negative integer `n`.

### New module: `recip_hyp_power_integral.py`

Exports three public functions, all pure-recursive with no back-calls into
`integrate.py` (avoiding circular imports):

| Function | Formula |
|----------|---------|
| `sech_power_integral(n,a,b,x)` | IBP: `I_n = sech^(n-2)·tanh/((n-1)a) + (n-2)/(n-1)·I_{n-2}` |
| `csch_power_integral(n,a,b,x)` | IBP: `I_n = −csch^(n-2)·coth/((n-1)a) − (n-2)/(n-1)·I_{n-2}` |
| `coth_power_integral(n,a,b,x)` | Identity: `I_n = I_{n-2} − coth^(n-1)/((n-1)a)` |

**sech^n base cases:** `n=0→x`, `n=1→atan(sinh(ax+b))/a`, `n=2→tanh(ax+b)/a`.

**csch^n base cases:** `n=0→x`, `n=1→log(tanh((ax+b)/2))/a`, `n=2→−coth(ax+b)/a`.

**coth^n base cases:** `n=0→x`, `n=1→log(sinh(ax+b))/a`.

The `coth^n` recursion is derived from `coth²=1+csch²` (Pythagorean identity)
rather than IBP — it produces a cleaner telescoping result with no outer product.

No new IR heads required; all output uses existing heads (`TANH`, `COTH`, `SINH`,
`ATAN`, `LOG`).

### `integrate.py` changes

1. Imported the three new functions from `recip_hyp_power_integral`.
2. Added `_try_recip_hyp_power(base, exponent, x)` dispatcher — fires for
   `SECH`/`CSCH`/`COTH` base with integer exponent ≥ 2 and linear argument.
3. Call site added immediately after `_try_hyp_power` (~line 544).

### `test_phase15.py` update

`test_sech_squared_unevaluated` renamed to `test_sech_squared_now_evaluates`
and updated to assert `_was_evaluated` (previously asserted `_is_unevaluated`).

### Tests (`test_phase16.py`) — 34 tests

| Class | Tests | What is verified |
|-------|-------|-----------------|
| `TestPhase16_SechPowers` | 8 | n=2,3,4,5; a=2; b=1; a=1/2; Tanh in result |
| `TestPhase16_CschPowers` | 8 | n=2,3,4,5; a=2; b=1; a=1/2; Coth in result |
| `TestPhase16_CothPowers` | 8 | n=2,3,4,5; a=2; b=1; a=1/2; Coth power in result |
| `TestPhase16_Fallthrough` | 3 | poly×sech², non-linear arg, mixed product |
| `TestPhase16_Regressions` | 3 | Phase 15 bare, Phase 14 sinh^4, Phase 3 exp |
| `TestPhase16_Macsyma` | 4 | end-to-end via MACSYMA string interface |

Antiderivative correctness verified numerically at two test points per case.

---

## 0.35.0 — 2026-04-28

**Phase 15 — Reciprocal hyperbolic functions: `coth`, `sech`, `csch`.**

Completes the hyperbolic set started in Phase 13 by adding the three reciprocal
functions.  Each one gets a numeric evaluation handler, symbolic differentiation
rules, and a closed-form bare integral.

### New handlers (`handlers.py`)

| Handler | `numeric_fn` | Exact identity |
|---------|-------------|----------------|
| `coth(simplify)` | `cosh(x)/sinh(x)` | none (pole at 0) |
| `sech(simplify)` | `1/cosh(x)` | `sech(0) = 1` |
| `csch(simplify)` | `1/sinh(x)` | none (pole at 0) |

All three registered in `build_handler_table` after the existing ATANH entry.

### Differentiation rules (`integrate.py`)

```
d/dx coth(u) = −u' / sinh²(u)
d/dx sech(u) = −u'·sinh(u) / cosh²(u)
d/dx csch(u) = −u'·cosh(u) / sinh²(u)
```

Derivatives are expressed via `SINH`/`COSH` to avoid self-referential recursion.
When `u = x` (darg is the integer literal 1) the chain-rule factor is omitted.

### Bare integration formulas (`integrate.py`)

```
∫ coth(ax+b) dx = (1/a)·log(sinh(ax+b))
∫ sech(ax+b) dx = (1/a)·atan(sinh(ax+b))
∫ csch(ax+b) dx = (1/a)·log(tanh((ax+b)/2))
```

Three private helpers added after `_atanh_integral`: `_coth_integral`,
`_sech_integral`, `_csch_integral`.

Note: `∫ csch(ax+b) dx = −atanh(cosh(ax+b))/a` is algebraically equivalent but
not numerically safe on the reals (cosh ≥ 1 always, outside `atanh`'s domain).
The `log(tanh(half_arg))` form is used instead.

Poly×coth/sech/csch integration is deferred to a future phase.

### Phase 3 head set extended

`COTH`, `SECH`, `CSCH` added to the head-recognition set in Phase 3 of
`_integrate`, enabling bare dispatch for any linear argument `ax+b`.

### Tests (`test_phase15.py`) — 41 tests

| Class | Tests |
|-------|-------|
| `TestPhase15_HandlerEval` | 6 |
| `TestPhase15_Differentiation` | 9 |
| `TestPhase15_CothIntegral` | 5 |
| `TestPhase15_SechIntegral` | 5 |
| `TestPhase15_CschIntegral` | 5 |
| `TestPhase15_Fallthrough` | 3 |
| `TestPhase15_Regressions` | 3 |
| `TestPhase15_Macsyma` | 5 |

Depends on `symbolic-ir >= 0.8.0`.

---

## 0.34.0 — 2026-04-28

**Phase 14 deferred fixes: exp×hyp degenerate case, sinh^m·cosh^n (both≥2), atanh×poly.**

### 14a-fix: `∫ exp(ax+b)·sinh/cosh(cx+d) dx` when `a² = c²`

`exp_hyp_integral.py` gains `exp_hyp_degenerate(a, b, c, d, is_sinh, x_sym)`.

Previously `_try_exp_hyp` fell through to unevaluated when `D = a²−c² = 0`.
The fix expands sinh/cosh into exponentials, giving two terms — one
exponential and one constant — whose antiderivatives are trivial:

```
a=c:   e^(2ax+b+d)/(4a)  ±  e^(b-d)·x/2
a=-c:  e^(b+d)·x/2  ±  e^(2ax+b-d)/(4a)
```

`+` for cosh, `−` for sinh.

### 14b-fix: `∫ sinh^m·cosh^n dx` for both `m, n ≥ 2`

`hyp_power_integral.sinh_times_cosh_power` is extended with three sub-cases
(replacing the old `return None`).  A new private helper `_fold_add` left-folds
term lists into binary ADD nodes.

| Sub-case | Condition | Method |
|----------|-----------|--------|
| A | m odd | u=cosh, expand (u²−1)^p by binomial → sum of cosh powers |
| B | n odd | u=sinh, expand (u²+1)^q by binomial → sum of sinh powers |
| C | both even | sinh²p=(cosh²−1)^p → reduce to Σ cosh_power_integral calls |

### 14c-fix: `∫ P(x)·atanh(ax+b) dx` for `P ∈ Q[x]`

New file: `atanh_poly_integral.py`.  IBP with u=atanh gives the closed form:

```
[Q(x) − (r₀ − r₁·b/a)]·atanh(ax+b)  −  a·T(x)  +  (r₁/(2a))·log(1−(ax+b)²)
```

where `Q = ∫P`, `T = ∫S`, and `r₁x+r₀` is the remainder from dividing `Q`
by `1−(ax+b)²`.

`integrate.py` gains `_try_atanh_product` wired in the MUL block after the
Phase 13 inverse-hyp handlers.

### Tests

`test_phase14.py`: three formerly-unevaluated fallthroughs changed to
closed-form assertions; new class `TestPhase14_DeferredFixes` (18 tests).

`test_phase13.py`: `test_atanh_times_x` updated from unevaluated to evaluated.

Total: 1018 tests, 86% coverage.

---

## 0.33.0 — 2026-04-28

**Group E: complete matrix handler set + Phase 14 hyperbolic integration.**

### Group E — matrix operation completeness

All seven remaining matrix handlers wired into `SymbolicBackend` via
`cas_handlers.py`:

| Handler | Operation |
|---------|-----------|
| `Dot(A, B)` | Matrix product (rows of A × cols of B) |
| `Trace(M)` | Sum of main diagonal; left-folded into binary ADDs |
| `Dimensions(M)` | `List(rows, cols)` shape query |
| `IdentityMatrix(n)` | n×n identity matrix |
| `ZeroMatrix(m, n)` / `ZeroMatrix(n)` | m×n (or n×n) zero matrix |
| `Rank(M)` | Rank via forward REF over `Fraction` (exact arithmetic) |
| `RowReduce(M)` | Reduced row-echelon form via Gauss-Jordan elimination |

`trace_handler` now left-folds any n-ary `IRApply(ADD, ...)` returned by
`cas_matrix.trace` into a chain of binary additions to stay within the VM's
binary-ADD contract.

### Phase 14 — hyperbolic power and exp×hyperbolic integration

Three new integration families added to `integrate.py`:

**14a: `∫ exp(ax+b)·sinh(cx+d) dx` and `∫ exp(ax+b)·cosh(cx+d) dx`**

New file: `exp_hyp_integral.py`.  Uses the exponential expansion of sinh/cosh
to reduce to two pure-exponential integrals, then recombines:

```
∫ e^(ax+b)·sinh(cx+d) dx = e^(ax+b)·[a·sinh(cx+d) − c·cosh(cx+d)] / (a²−c²)
∫ e^(ax+b)·cosh(cx+d) dx = e^(ax+b)·[a·cosh(cx+d) − c·sinh(cx+d)] / (a²−c²)
```

Falls through (returns unevaluated) when `a² = c²` (degenerate denominator).

**14b: `∫ sinh^n(ax+b) dx` and `∫ cosh^n(ax+b) dx`** (n ≥ 2)

New file: `hyp_power_integral.py`.  Recursive IBP reduction formulas:

```
I_n(sinh) = (1/(na))·sinh^(n-1)·cosh − (n-1)/n · I_{n-2}   (−)
I_n(cosh) = (1/(na))·cosh^(n-1)·sinh + (n-1)/n · I_{n-2}   (+)
```

**14c: `∫ sinh^m · cosh^n dx`** when min(m,n) = 1

u-substitution: if m=1, u=cosh → cosh^(n+1)/(n+1)/a; if n=1, u=sinh →
sinh^(m+1)/(m+1)/a.  Returns `None` (falls through) when both m,n ≥ 2.

### Dispatcher functions added to `integrate.py`

- `_try_hyp_power(base, exponent, x)` — fires for `Pow(Sinh/Cosh(linear), n≥2)`
- `_try_exp_hyp(exp_node, hyp_node, x)` — fires for `exp(linear)×sinh/cosh(linear)`
- `_try_sinh_cosh_product(f1, f2, x)` — fires for `sinh^m × cosh^n` (m or n = 1)

### Tests

New `tests/test_phase14.py` with 62 tests covering:
- `TestPhase14_ExpSinh` (7) — exp×sinh integration cases
- `TestPhase14_ExpCosh` (5) — exp×cosh integration cases
- `TestPhase14_SinhPowers` (7) — sinh^n for n=2..5, linear args
- `TestPhase14_CoshPowers` (7) — cosh^n for n=2..5, linear args
- `TestPhase14_SinhCoshProduct` (7) — u-sub mixed products
- `TestPhase14_MatrixOps` (12) — all 7 new matrix handlers
- `TestPhase14_Fallthroughs` (6) — degenerate/unsupported cases
- `TestPhase14_Regressions` (5) — Phases 1/4/12/13 still work
- `TestPhase14_Macsyma` (4) — end-to-end via Macsyma string interface

`test_phase13.py`: removed `test_sinh_squared` fallthrough (Phase 14 now
evaluates `∫ sinh²(x) dx` via the reduction formula).

### Version

- Bumped to **0.33.0**; `cas-matrix>=0.2.0` dependency floor raised.

---

## 0.32.8 — 2026-04-28

**Wire `cas-multivariate` into `SymbolicBackend` (Gröbner bases).**

- `cas_handlers.py`: imports `build_multivariate_handler_table` from `cas_multivariate`
  and merges it into the handler table via `**_build_multivariate()`.
- `pyproject.toml`: added `"coding-adventures-cas-multivariate>=0.1.0"` as a dependency.

This wires `Groebner(List(polys), List(vars))`, `PolyReduce(f, List(polys), List(vars))`,
and `IdealSolve(List(polys), List(vars))` into the symbolic VM.

---

## 0.32.7 — 2026-04-27

**Wire `cas-algebraic` into `SymbolicBackend`; add `AlgFactor` to held heads.**

- `cas_handlers.py`: imports `build_alg_factor_handler_table` from `cas_algebraic`
  and merges it into the handler table via `**_build_algebraic()`.
- `backends.py`: added `"AlgFactor"` to `_HELD_HEADS` so the `Sqrt(d)` second
  argument is not pre-evaluated to a float before the handler can inspect it.
  This is the same pattern used for `"ODE2"`.
- `pyproject.toml`: added `"coding-adventures-cas-algebraic>=0.1.0"` as a dependency.

This wires `AlgFactor(poly, Sqrt(d))` into the symbolic VM so that
`algfactor(x^4+1, sqrt(2))` compiles to `AlgFactor(x^4+1, Sqrt(2))` IR and
evaluates to `(x^2+sqrt(2)*x+1)*(x^2-sqrt(2)*x+1)`.

---

## 0.32.6 — 2026-04-28

**Bump `cas-factor` dependency to 0.3.0 (BZH Phase 3).**

No source changes to `symbolic-vm` itself. The upgraded `cas-factor 0.3.0`
adds Berlekamp-Zassenhaus-Hensel factoring as a fallback for monic polynomials
of degree ≥ 4 that Kronecker misses. This means `factor(x^5 - 1)`,
`factor(x^8 - 1)`, `factor(x^9 - 1)`, and other high-degree cyclotomic
polynomials now factor correctly through the MACSYMA `factor(...)` surface
syntax without any VM changes.

---

## 0.32.5 — 2026-04-27

**Wire `cas-ode` into `SymbolicBackend`; add `ODE2` to held heads.**

- `cas_handlers.py`: imports `build_ode_handler_table` from `cas_ode` and
  merges it into the handler table at the end of `build_cas_handler_table()`.
- `backends.py`: added `"ODE2"` to `_HELD_HEADS` so that `D(y, x)` inside
  the ODE expression argument is not pre-evaluated to zero before the ODE
  handler sees it. This is the correct semantic: the ODE solver needs to
  inspect the derivative structure of the equation.
- Added `coding-adventures-cas-ode>=0.1.0` as a dependency.

---

## 0.32.4 — 2026-04-27

**Wire `cas-fourier` into `SymbolicBackend`.**

- Added `from cas_fourier import build_fourier_handler_table as _build_fourier`
  to `cas_handlers.py`.
- Added `**_build_fourier()` to the `build_cas_handler_table()` return dict.
- Added `"coding-adventures-cas-fourier>=0.1.0"` to `pyproject.toml` dependencies.

This wires `Fourier(f, t, ω)` and `IFourier(F, ω, t)` into the symbolic VM.
Both operations follow the graceful fall-through contract: unknown inputs return
the expression unevaluated.

---

## 0.32.3 — 2026-04-27

**Wire `cas-laplace` into `SymbolicBackend`.**

- Added `from cas_laplace import build_laplace_handler_table as _build_laplace`
  to `cas_handlers.py`.
- Added `**_build_laplace()` to the `build_cas_handler_table()` return dict.
- Added `"coding-adventures-cas-laplace>=0.1.0"` to `pyproject.toml` dependencies.

This wires `Laplace(f, t, s)`, `ILT(F, s, t)`, `DiracDelta(x)`, and `UnitStep(x)`
into the symbolic VM. All four operations follow the graceful fall-through contract.

---

## 0.32.2 — 2026-04-27

**Wire `cas-mnewton` into `SymbolicBackend`.**

- Added `from cas_mnewton import build_mnewton_handler_table as _build_mnewton`
  to `cas_handlers.py`.
- Added `**_build_mnewton()` to the `build_cas_handler_table()` return dict
  so `MNewton(f, x, x0)` is handled by every `SymbolicBackend`-derived VM.
- Added `coding-adventures-cas-mnewton` to `pyproject.toml` dependencies.
- `MNewton(f, x, x0)` now evaluates to `IRFloat(root)` for numeric x0, and
  falls through to unevaluated on non-numeric input or zero-derivative.

---

## 0.32.1 — 2026-04-27

**Bug fix — hyperbolic differentiation via `derivative.py` pathway.**

Phase 13 added differentiation rules for `sinh/cosh/tanh/asinh/acosh/atanh` to
`integrate.py`'s `_diff_ir` helper, but the standalone `derivative.py` module
(used when callers invoke `diff` directly rather than through the integrator) was
not updated.  Without this fix, `diff(sinh(x), x)` called through `derivative.py`
entered infinite recursion and raised `RecursionError`.

Added chain-rule branches to `derivative.py` for all six hyperbolic heads:

- `Sinh(u)` → `Cosh(u) · u'`
- `Cosh(u)` → `Sinh(u) · u'`
- `Tanh(u)` → `u' / Cosh(u)²`
- `Asinh(u)` → `u' / √(u²+1)`
- `Acosh(u)` → `u' / √(u²−1)`
- `Atanh(u)` → `u' / (1−u²)`

---

## 0.32.0 — 2026-04-27

**Phase 13 — Hyperbolic function evaluation, differentiation, and integration.**

New evaluation handlers (six `_elementary`-based handlers in `handlers.py`,
registered in `build_handler_table`):

- `sinh(simplify)` — `Sinh(u)`, exact identity `Sinh(0) = 0`.
- `cosh(simplify)` — `Cosh(u)`, exact identity `Cosh(0) = 1`.
- `tanh(simplify)` — `Tanh(u)`, exact identity `Tanh(0) = 0`.
- `asinh(simplify)` — `Asinh(u)`, exact identity `Asinh(0) = 0`.
- `acosh(simplify)` — `Acosh(u)`, exact identity `Acosh(1) = 0`.
- `atanh(simplify)` — `Atanh(u)`, exact identity `Atanh(0) = 0`.

New integration modules:

- `sinh_poly_integral.py` — tabular IBP for `∫ P(x)·sinh(ax+b) dx` and
  `∫ P(x)·cosh(ax+b) dx`. Sign alternation `(−1)^k` (period 2, vs trig's
  period 4). Public API: `sinh_poly_integral`, `cosh_poly_integral`.
- `asinh_poly_integral.py` — reduction IBP for `∫ P(x)·asinh(ax+b) dx` and
  `∫ P(x)·acosh(ax+b) dx`. Uses the reduction formula
  `Iₙ = (1/n)·tⁿ⁻¹·√(t²±1) ∓ (n−1)/n · Iₙ₋₂`. Public API:
  `asinh_poly_integral`, `acosh_poly_integral`.

Changes to `integrate.py` (9 change sites):

1. Imports: added `SINH`, `COSH`, `TANH`, `ASINH`, `ACOSH`, `ATANH` and
   the two new module imports.
2. Phase 1 head set: added all six hyperbolic heads.
3. Phase 3 head set: added all six.
4. Bare dispatch: `SINH`/`COSH` → tabular IBP; `TANH` → `log(cosh(ax+b))/a`;
   `ASINH`/`ACOSH` → reduction IBP; `ATANH` → inline IBP formula.
5. Dispatcher functions: `_try_sinh_product`, `_try_cosh_product`,
   `_try_asinh_product`, `_try_acosh_product`.
6. Phase 13 MUL hooks wired in after Phase 12 block.
7. Diff rules in `_diff_ir`: all six hyperbolic functions.
8. Helper functions: `_tanh_integral`, `_atanh_integral`.

New test file `tests/test_phase13.py` with 55 tests across 9 classes:
`TestPhase13_SinhCanonical` (8), `TestPhase13_CoshCanonical` (7),
`TestPhase13_LinearHyp` (8), `TestPhase13_AsinhCanonical` (8),
`TestPhase13_AcoshCanonical` (5), `TestPhase13_BareAtanhTanh` (4),
`TestPhase13_Fallthrough` (4), `TestPhase13_Regressions` (6),
`TestPhase13_Macsyma` (5).

Bumped `symbolic-ir` dependency to `>=0.7.0`.

New spec: `code/specs/phase13-hyperbolic.md`.

## 0.31.0 — 2026-04-27

**Phase G — Control-flow VM handlers.**

Added five new evaluation handlers to `handlers.py` and wired them into
`build_handler_table()`:

- `while_(simplify)` — `While(condition, body)` loop handler. Re-evaluates
  `condition` before each iteration; exits when condition is falsy or
  indeterminate. Returns the last body value (or `False` if never entered).
- `for_range_(simplify)` — `ForRange(var, start, step, end, body)` handler.
  Evaluates bounds once, iterates `var` from `start` to `end` inclusive
  (step can be negative). Saves and restores the loop variable's binding.
- `for_each_(simplify)` — `ForEach(var, list, body)` handler. Evaluates the
  list once; iterates over elements, binding `var` to each in turn.
- `block_(simplify)` — `Block(List(locals), stmt1, …, stmtN)` handler.
  Installs local bindings, evaluates statements in order, returns the last
  value, restores all bindings (including via `Return` exit).
- `return_(_simplify)` — `Return(value)` handler. Raises `_ReturnSignal`
  to unwind through any enclosing Block/While/ForRange/ForEach handler.

Also:

- Added `_ReturnSignal(BaseException)` exception class for early-exit
  signalling through nested control flow.
- Extended `_HELD_HEADS` in `backends.py` with `WHILE`, `FOR_RANGE`,
  `FOR_EACH`, `BLOCK` (args not pre-evaluated before dispatch).
- Added `unbind(name)` to `Backend` ABC (default no-op) and `_BaseBackend`
  (calls `self._env.pop(name, None)`) for block scope restoration.
- Bumped `symbolic-ir` dependency to `>=0.6.0`.

Required by `macsyma-grammar-extensions.md` (Phase G). Companion releases:
`symbolic-ir` 0.6.0, `macsyma-compiler` 0.6.0.

## 0.30.0 — 2026-04-27

**`Cbrt` evaluation handler — exact cube-root simplification.**

Added `cbrt_handler` and `_integer_cbrt` helper to `cas_handlers.py`,
registered under the `"Cbrt"` key in `build_cas_handler_table()`.

Evaluation rules:
- **Perfect integer cubes**: `Cbrt(8) → 2`, `Cbrt(-27) → -3`,
  `Cbrt(0) → 0`, `Cbrt(1) → 1`.
- **Exact rational**: `Cbrt(8/27) → 2/3`, `Cbrt(-8/27) → -2/3`.
  Both numerator and denominator must be perfect cubes; otherwise the
  node is left unevaluated.
- **Float**: `Cbrt(8.0) → 2.0`, `Cbrt(-27.0) → -3.0` (uses
  `n^(1/3)` with sign-aware handling for negatives).
- **Symbolic / imperfect**: `Cbrt(x)`, `Cbrt(2)`, `Cbrt(1/2)` all
  pass through unevaluated.

`_integer_cbrt(n)` uses a float estimate plus ±1 probe to find the
exact integer cube root without floating-point rounding errors.

20 new tests in `test_cas_handlers.py` covering every branch (positive
perfect cubes, zero, negative cubes, rational exact, rational imperfect
passthrough, float positive, float negative, symbolic passthrough).
Total test count 864; coverage 86 %.

## 0.29.0 — 2026-04-27

**REPL quality fixes — lambda beta-reduction and transcendental Taylor.**

Two VM-level bugs fixed, one import alias added in `derivative.py`:

1. **Lambda beta-reduction** (`vm.py`): `map(lambda([z], z^2), [1,2,3])`
   previously returned unevaluated because the VM's `_eval_apply` only
   handled named function calls (via `Define` records) and never dispatched
   when the head was itself an `IRApply(lambda, ...)`.  A new `_apply_lambda`
   method and a step 4b in `_eval_apply` now detect inline lambdas and perform
   the parameter substitution, making `Map` + `Select` + direct lambda calls
   all work correctly.

2. **Transcendental Taylor** (`cas_handlers.py`): `taylor(sin(y), y, 0, 4)`
   previously returned the expression unevaluated because
   `cas_limit_series.taylor_polynomial` only handles polynomial inputs.  The
   handler now falls back to `_taylor_derivative_fallback`, which computes each
   coefficient `f^(k)(a)/k!` via successive symbolic differentiation (using the
   existing `_diff` from `derivative.py`) followed by point-substitution.
   `_symbolic_diff` import added at module level.  Both polynomial and
   transcendental paths are tested.

3 new lambda tests (`test_map_with_lambda`, `test_lambda_direct_call`,
`test_lambda_two_params`) and 1 updated Taylor test
(`test_taylor_transcendental_sin`) added.  Total test count 850, coverage 86 %.

## 0.28.0 — 2026-04-27

**Phase 13 — Hyperbolic functions (sinh, cosh, tanh, asinh, acosh, atanh).**

**Roadmap item A1 — Kronecker polynomial factoring (Phase 2) wired through `cas-factor 0.2.0`.**

Upgrades `coding-adventures-cas-factor` dependency to `>=0.2.0`.

The `Factor` handler in `build_cas_handler_table()` transparently benefits
from `cas-factor`'s new Kronecker algorithm — no handler code changes needed.
Factoring now handles:

- **Sophie Germain identity**: `x⁴ + 4 = (x²+2x+2)(x²−2x+2)`
- **Cyclotomic**: `x⁴+x²+1 = (x²+x+1)(x²−x+1)`
- **Repeated irreducibles**: `x⁴+2x²+1 = (x²+1)²`
- **Mixed**: `(x²+1)(x−2)` correctly split

Updated `factor_handler` docstring to reflect Phase 2 capabilities.

2 new tests in `test_cas_handlers.py` (Sophie Germain, cyclotomic), verifying
that `Factor(x⁴+4)` and `Factor(x⁴+x²+1)` both return non-trivial `Mul` trees.

## 0.26.0 — 2026-04-27

**Roadmap item A3 — rational function operations (Collect, Together, RatSimplify, Apart, full Expand).**

`Expand` handler upgraded from a `canonical()`-only pass to full polynomial
distribution via the polynomial bridge.  Four new IR heads wired into
`SymbolicBackend` via `build_cas_handler_table()`:

- **`Expand`** (improved): calls `to_rational` + `from_polynomial` to distribute
  `Mul` over `Add` and expand integer powers for single-variable polynomials
  with rational coefficients. Falls back to `canonical` for multi-variable /
  transcendental expressions.
- **`Collect(expr, var)`**: groups terms by powers of `var` for single-variable
  polynomials with rational coefficients (same mechanism as `Expand` but takes
  an explicit variable argument). MACSYMA surface: `collect`.
- **`Together(expr)`**: combines a sum of rational functions into a single
  fraction `P(x)/Q(x)` with monic denominator. MACSYMA surface: `together`.
- **`RatSimplify(expr)`**: cancels the GCD of numerator and denominator,
  reducing the rational expression to lowest terms. MACSYMA surface: `ratsimp`.
- **`Apart(expr, var)`**: partial-fraction decomposition (Phase 1 — distinct
  rational linear factors only). Uses residue formula `A_i = P(r_i)/Q'(r_i)`.
  MACSYMA surface: `partfrac`. Falls back to unevaluated for irreducible
  quadratic or repeated factors.

**Dependencies**: `coding-adventures-polynomial` was already in
`pyproject.toml`; the new handlers import `gcd`, `monic`, `deriv`,
`evaluate`, `rational_roots`, `divmod_poly` directly from `polynomial`.

**New tests (18 in Section 14 of `test_cas_handlers.py`)** +
**6 new pipeline tests in `macsyma-runtime`**.

## 0.25.0 — 2026-04-27

**Roadmap item B1 (cas-trig) wired into SymbolicBackend.**

`cas-trig` is now a dependency. Its handler table is merged via
`_build_trig()` in `SymbolicBackend.__init__`.

**New IR heads** (3 total):
`TrigSimplify`, `TrigExpand`, `TrigReduce`.

- `TrigSimplify`: Pythagorean identity (`sin²+cos²→1`), sign rules
  (`sin(-x)→-sin(x)`, `cos(-x)→cos(x)`), and special-value lookup
  (`sin(π/6)→1/2`, etc.).
- `TrigExpand`: angle-addition formulas and Chebyshev recurrence for
  integer multiples (`sin(2x)→2sin(x)cos(x)`, `cos(3x)→...`).
- `TrigReduce`: power-to-multiple-angle reduction
  (`sin²(x)→(1-cos(2x))/2`, `cos³(x)→(3cos(x)+cos(3x))/4`, etc.).

**Dependencies updated:**
- `cas-trig>=0.1.0` added to `pyproject.toml`.

**New tests (5 in `test_cas_handlers.py`)** + **5 pipeline tests**.

## 0.24.0 — 2026-04-27

**Roadmap items A2c + A2d (NSolve and linear systems) wired in.**

- `solve_handler` extended to detect `Solve(List(eqs...), List(vars...))`
  and route it to `solve_linear_system` (Gaussian elimination, exact
  rational arithmetic). Returns `List(Rule(var, val), ...)`.
- `nsolve_handler` added for `NSolve(poly, var)`: Durand-Kerner iteration
  returning `IRFloat`/complex IR roots for any polynomial degree.
- `MACSYMA_NAME_TABLE` gains `"nsolve"→NSolve` and `"linsolve"→Solve`.
- `cas-solve>=0.6.0` dependency pin updated.
- 4 new tests in `test_cas_handlers.py` + 4 new pipeline tests.

## 0.23.0 — 2026-04-27

**Roadmap items A2a + A2b (cubic and quartic solvers) wired into `solve_handler`.**

The `Solve` handler in `cas_handlers.py` now supports degree-3 and degree-4
polynomials via `cas-solve`'s new `solve_cubic` and `solve_quartic`:

- **Degree 3**: routes through `solve_cubic` (rational-root theorem → Cardano).
  Returns a `List` of roots, or unevaluated for casus irreducibilis.
- **Degree 4**: routes through `solve_quartic` (rational-root theorem →
  biquadratic → Ferrari). Returns a `List` of roots, or unevaluated when the
  Ferrari resolvent has no rational root.
- Empty or "ALL" results are propagated as unevaluated expressions.

**Dependencies updated:**
- `cas-solve` bumped to `>=0.4.0` in `pyproject.toml`.

**New tests (4 in `test_cas_handlers.py`):**
`test_solve_cubic_three_rational`, `test_solve_cubic_one_rational_two_complex`,
`test_solve_quartic_four_rational`, `test_solve_degree_5_passthrough`.

## 0.22.0 — 2026-04-27

**Roadmap item B2 (cas-complex) wired into SymbolicBackend.**

`cas-complex` is now a dependency. Its handler table is merged into
`build_cas_handler_table()` via `**_build_complex()`, and two additional
integration points are set up in `SymbolicBackend.__init__`:

- `ImaginaryUnit` is pre-bound to itself so it evaluates as an inert
  symbol (rather than triggering the unresolved-symbol fall-through).
- A wrapper around the `Pow` handler routes `ImaginaryUnit^n` through
  `imaginary_power_handler` (reducing `i^n → {1, i, -1, -i}` via `n % 4`)
  before falling through to the standard power handler.
- `Abs` is extended: when its argument contains `ImaginaryUnit`, it
  delegates to `abs_complex_handler` (returning `sqrt(re² + im²)`).

**New IR heads** (7 total):
`Re`, `Im`, `Conjugate`, `Arg`, `RectForm`, `PolarForm`, `AbsComplex`.

## 0.21.0 — 2026-04-27

**Roadmap item B3 (cas-number-theory) wired into SymbolicBackend.**

The new `cas-number-theory` package is now a dependency and its handler
table is merged into `build_cas_handler_table()` via `**_build_nt()`.

**New IR heads** (10 total, all language-neutral):
`IsPrime`, `NextPrime`, `PrevPrime`, `FactorInteger`, `Divisors`,
`Totient`, `MoebiusMu`, `JacobiSymbol`, `ChineseRemainder`, `IntegerLength`.

## 0.20.0 — 2026-04-27

**Roadmap items C2, C4, C5 implemented** — three items from the MACSYMA
completion roadmap (`macsyma-completion.md`) are now live in `SymbolicBackend`.
All three are language-neutral IR heads; every future CAS frontend inherits
them automatically.

**New handlers installed on `SymbolicBackend`**:

- **`Lhs(Equal(a, b))` → `a`** (C5) — left-hand side of an equation.
- **`Rhs(Equal(a, b))` → `b`** (C5) — right-hand side of an equation.
- **`MakeList(expr, var, n)` / `MakeList(expr, var, from, to[, step])`** (C2)
  — generative list construction: evaluates `expr` for each integer value
  of `var` in the specified range.  Replaces the previous stub that mapped
  `makelist` → `Range`.
- **`At(expr, Equal(var, val))` / `At(expr, List(…))` → substitution then eval** (C4)
  — point evaluation; sugar over `Subst`.  Handles both single rules and
  lists of rules.

**Bug fix**: `MACSYMA_NAME_TABLE["makelist"]` previously routed to `Range`
(a plain integer range generator). It now routes to the correct `MakeList`
head, which evaluates an arbitrary expression over a range.

**New import**: `EQUAL` added to the `cas_handlers.py` imports from
`symbolic_ir`.

## 0.19.0 — 2026-04-27

**CAS substrate handlers wired into SymbolicBackend** — the universal inner
doll. Every CAS frontend that extends `SymbolicBackend` (MACSYMA, Maple,
Mathematica, …) now inherits the full algebraic operation set for free.
Language-specific quirks (Display/Suppress/Kill/Ev) remain in the language
backend subclass (the outer doll).

**New module**: `symbolic_vm/cas_handlers.py`

**New handlers installed on `SymbolicBackend`**:

- **Algebraic**: `Simplify`, `Expand`, `Factor` (Phase 1: integer-root
  factoring via rational-root theorem), `Solve` (linear and quadratic over Q),
  `Subst` (structural substitution + re-evaluation).
- **List operations**: `Length`, `First`, `Rest`, `Last`, `Append`, `Reverse`,
  `Range`, `Map`, `Apply`, `Select`, `Sort`, `Part`, `Flatten`, `Join`.
- **Matrix**: `Matrix`, `Transpose`, `Determinant`, `Inverse`.
- **Calculus**: `Limit` (direct-substitution Phase 1), `Taylor` (polynomial
  Taylor expansion).
- **Numeric**: `Abs`, `Floor`, `Ceiling`, `Mod`, `Gcd`, `Lcm`.

**Package dependencies added**: `cas-pattern-matching`, `cas-substitution`,
`cas-simplify`, `cas-factor`, `cas-solve`, `cas-list-operations`,
`cas-matrix`, `cas-limit-series`.

**Architecture note**: These handlers are the **inner doll** — universal
CAS operations that any symbolic algebra language can use unchanged. They
are explicitly *not* placed in `MacsymaBackend` so that future Maple and
Mathematica backends can extend `SymbolicBackend` directly and inherit
the complete algebraic substrate without touching any MACSYMA-specific code.

## 0.18.0 — 2026-04-23

Phase 13 of the integration roadmap — hyperbolic functions.

**New heads** (requires `symbolic-ir >= 0.5.0`): SINH, COSH, TANH, ASINH, ACOSH, ATANH.

**New capability**:
- `∫ P(x) · sinh(ax+b) dx` and `∫ P(x) · cosh(ax+b) dx` — tabular IBP with sign `(−1)^k`.
  Assembly: `C(x)·cosh(ax+b) + S(x)·sinh(ax+b)` for sinh; swapped for cosh.
- `∫ P(x) · asinh(ax+b) dx` — IBP + reduction formula `∫ tⁿ/√(t²+1) dt`.
  Final: `[Q(x)−B(ax+b)]·asinh(ax+b) − A(ax+b)·√((ax+b)²+1)`.
- `∫ P(x) · acosh(ax+b) dx` — same reduction formula with `√(t²−1)`.
  Final: `[Q(x)−B(ax+b)]·acosh(ax+b) − A(ax+b)·√((ax+b)²−1)`.
- `∫ tanh(ax+b) dx = (1/a)·log(cosh(ax+b))`.
- `∫ atanh(ax+b) dx = (ax+b)/a·atanh(ax+b) + (1/(2a))·log(1−(ax+b)²)`.

**New modules**: `sinh_poly_integral.py`, `asinh_poly_integral.py`.

**Differentiation rules**: all six hyperbolic functions wired into `_diff_ir`.

**poly×tanh and poly×atanh deferred** to a future phase.

**Tests**: 45 tests in `tests/test_phase13.py` using numerical finite-difference
verification. All correctness checks pass.

## 0.17.0 — 2026-04-23

Phase 12 of the integration roadmap — polynomial × asin/acos(linear) integration via IBP.

**New capability**: `∫ P(x) · asin(ax+b) dx` and `∫ P(x) · acos(ax+b) dx` for any
`P ∈ Q[x]` and `a ∈ Q \ {0}`, completing all three inverse-trig × polynomial families.

**Algorithm** (integration by parts):

- **asin IBP**: `u = asin(ax+b)`, `dv = P dx` → `du = a/√(1−(ax+b)²) dx`, `v = Q = ∫P dx`
  - Residual: `a · ∫ Q/√(1−(ax+b)²) dx = ∫ Q̃(t)/√(1−t²) dt`  (t = ax+b substitution)
  - Residual decomposed via reduction formula: `∫ Q̃/√(1−t²) dt = A(t)·√(1−t²) + B(t)·asin(t)`
  - Final result: `[Q(x) − B(ax+b)]·asin(ax+b) − A(ax+b)·√(1−(ax+b)²)`

- **acos IBP**: sign of `du` flips (`d/dx acos = −a/√`), giving
  - Final result: `Q(x)·acos(ax+b) + A(ax+b)·√(1−(ax+b)²) + B(ax+b)·asin(ax+b)`
  - The B·asin term is non-zero for deg(P) ≥ 1 — this is expected, not a bug.

**New module** `asin_poly_integral.py`:
- `asin_poly_integral(poly, a, b, x_sym)` — IBP closed-form for `∫ P(x)·asin(ax+b) dx`.
- `acos_poly_integral(poly, a, b, x_sym)` — IBP closed-form for `∫ P(x)·acos(ax+b) dx`.
- Private helpers: `_compose_to_t`, `_sqrt_integral_decompose`, `_poly_compose_linear`, `_compute_AB`.
- Reduction formula is memoized per monomial degree for efficiency.

**Dispatcher hooks** in `integrate.py`:
- `_try_asin_product` / `_try_acos_product` — check ASIN/ACOS head, validate linear arg and polynomial coefficient.
- Both hooks try both operand orders, inserted after Phase 11 in the MUL handler.
- Bare `asin/acos(linear)` cases (P = 1) handled in the elementary-function branch.
- Differentiation rules for `d/dx asin(u)` and `d/dx acos(u)` added to `_diff_ir`.

**VM handlers** in `handlers.py`:
- `asin(simplify)` and `acos(simplify)` handlers registered (numeric fold + symbolic passthrough).

**symbolic-ir**: bumped dependency to `>=0.4.0` (requires ASIN/ACOS head symbols).

**Limitations (future work)**:
- `∫ asin(g(x))` for non-linear `g`.
- `∫ asin(ax+b)^n dx` for `n ≥ 2`.
- `∫ asin(ax+b) · exp(x) dx` (mixed inverse-trig × exponential).

## 0.16.0 — 2026-04-22

Phase 11 of the integration roadmap — polynomial × arctan(linear) integration via IBP.

**New capability**: `∫ P(x) · atan(ax+b) dx` for any `P ∈ Q[x]` and `a ∈ Q \ {0}`.

**Algorithm** (integration by parts):
- `u = atan(ax+b)`, `dv = P(x) dx` → `v = Q(x) = ∫P dx` (polynomial antiderivative)
- Residual `∫ Q(x)/D(x) dx` resolved by polynomial long division `Q = S·D + R` (deg R < 2),
  where `D = (ax+b)² + 1`; polynomial part `T = ∫S dx` handled directly, remainder
  dispatched to `arctan_integral` (Phase 2e).
- Final result: `Q(x)·atan(ax+b) − a·T(x) − a·arctan_integral(R, D)`.

**New module** `atan_poly_integral.py`:
- `atan_poly_integral(poly, a, b, x_sym)` — full IBP closed-form IR for `∫ P(x)·atan(ax+b) dx`.
- `_integrate_poly` — ascending-coefficient polynomial antiderivative with Fraction arithmetic.

**Dispatcher hook** in `integrate.py`:
- `_try_atan_product(transcendental, poly_candidate, x)` — checks ATAN head, extracts linear
  arg via `_try_linear`, validates polynomial coefficient via `to_rational`.
- Tries both operand orders: `_try_atan_product(a, b, x) or _try_atan_product(b, a, x)`.
- Inserted after Phase 3e (poly × log) in the MUL handler.

**Special case**: `P = 1` (bare arctan) reduces to the same result as Phase 9 — verified.

**Limitations (future work)**:
- `∫ atan(g(x))` for non-linear `g` (e.g. `atan(x²)`, `atan(sin(x))`).
- `∫ atan(ax+b)^n dx` for `n ≥ 2`.
- `∫ R(x)·atan(ax+b) dx` for rational (non-polynomial) `R`.

## 0.15.0 — 2026-04-22

Phase 10 of the integration roadmap — generalized partial-fraction integration and
Rothstein–Trager performance fix.

**RT performance guard**:
- `_integrate_rational` now skips Rothstein–Trager for degree ≥ 6 denominators.
  RT's Sylvester-matrix determinant (Fraction arithmetic, 12×12) caused a 26,261-second
  hang on `(x²+1)(x²+4)(x²+9)` while always returning None. The guard is safe:
  degree-6 denominators with any irreducible quadratic factor always have irrational
  RT coefficients, so skipping is lossless.

**Three distinct irreducible quadratics** (`Q₁·Q₂·Q₃`, degree-6 squarefree):
- `_factor_triple_quadratic` — finite candidate search (rational divisors of the
  constant term × small linear-coefficient candidates) to factor a degree-6 poly
  into three monic irreducible quadratics over Q; delegates to `_factor_biquadratic`
  for the degree-4 quotient.
- Handles both diagonal (`1/((x²+1)(x²+4)(x²+9))`) and non-diagonal cases
  (`1/((x²+2x+2)(x²+4)(x²+9))`).

**Linear factors × two irreducible quadratics** (`Lᵐ·Q₁·Q₂`, degree 5–6):
- `_solve_pf_general` — D×D Gaussian elimination over Fraction for any list of
  coprime polynomial factors (total degree D = Σdᵢ).
- `_try_general_rational_integral` — Phase 10 driver: extracts rational linear
  factors, factors the quadratic remainder via `_factor_biquadratic` or
  `_factor_triple_quadratic`, solves the generalized partial-fraction system, then
  integrates linear pieces as logs and quadratic pieces via `arctan_integral`.
- Handles degree-5 (`1/((x−1)(x²+1)(x²+4))`) and degree-6
  (`1/((x−1)(x−2)(x²+1)(x²+4))`) cases.

**Limitations (future work)**:
- Denominators with four or more irreducible quadratic factors (degree ≥ 8).
- Denominators that are irreducible of degree 4 over Q (e.g. `x⁴+1`).
- Denominators of degree > 6 in general.

## 0.14.0 — 2026-04-22

Phase 9 of the integration roadmap — multi-quadratic partial-fraction integration.

Extends the rational-function route to handle denominators that are products of
**two distinct irreducible quadratic factors** over Q (no linear factors), closing
the gap left by Phases 2d–2f.

**Core — two-quadratic partial fractions**:
- Detects degree-4 squarefree denominators with no rational roots.
- Attempts to factor as `Q₁·Q₂` using a finite candidate search (rational divisors
  of the constant term, derived from the coefficient-match system).
- Solves the 4×4 partial-fraction linear system over Q by Gaussian elimination.
- Integrates each `(Aᵢx+Bᵢ)/Qᵢ` piece via the existing Phase 2e `arctan_integral`.
- Handles both pure-arctan outputs (`1/((x²+1)(x²+4))`) and mixed log+arctan outputs
  (`(x+1)/((x²+1)(x²+4))`).
- Non-diagonal quadratics (`x²+2x+5` etc.) are fully supported.

**Bonus — `∫ atan(ax+b) dx` table entry**:
- Added to the Phase 3 linear-arg dispatch alongside sin/cos/exp/log/tan.
- Result: `x·atan(ax+b) − (1/(2a))·log((ax+b)²+1)`.
- Covers all linear arguments including fractional coefficients.

**New helpers in `integrate.py`**:
- `_int_divisors`, `_rational_divisors` — finite candidate enumeration.
- `_factor_biquadratic` — splits degree-4 poly into two irreducible quadratics.
- `_solve_pf_2quad` — Gaussian elimination for the 4×4 partial-fraction system.
- `_try_multi_quad_integral` — Phase 9 driver; hooked into `_integrate_rational`
  after `mixed_integral` (Phase 2f).

New spec: `code/specs/phase9-multi-quad-partial-fraction.md`.

42 new tests (`tests/test_phase9.py`). Package at ~532 tests.

## 0.13.0 — 2026-04-22

Phase 8 of the integration roadmap — power-of-composite u-substitution.

Extends u-substitution (Phase 7) to handle integrands where the outer function
is a **power** of a composite: `POW(f(g(x)), n) · c·g'(x)` and `POW(g(x), n) · c·g'(x)`.

**Case A — `f(g(x))^n · c·g'(x)`**:
- Substitute `u = g(x)`, integrate `∫ f(u)^n du` via Phase 5 (sin/cos/tan reduction
  formulas for trig outers), back-substitute `u → g(x)`.
- Guard: `g` must not be bare `x` (Phase 5 handles) or linear with a≠0 (Phase 5 handles).

**Case B — `g(x)^n · c·g'(x)`**:
- Substitute `u = g(x)`, integrate `∫ u^n du` via Phase 1 power rule,
  back-substitute.
- Special case n=−1: `∫ u⁻¹ du = log(u)` → `log(g(x))`.
- Guard: `g` must not be bare `x` or linear with a≠0.

**Bonus — `(ax+b)^n` in the single-factor POW branch**:
- `∫ (ax+b)^n dx = (ax+b)^(n+1)/((n+1)·a)` for n≠−1.
- `∫ (ax+b)^(−1) dx = log(ax+b)/a`.
- Supports integer and symbolic exponents.

**`_diff_ir` extensions** (enables Case B for sums of functions):
- `NEG(f)`: `d/dx(−f) = −f'`
- `ADD(f, g)`: `d/dx(f+g) = f' + g'` (zero terms simplified)
- `SUB(f, g)`: `d/dx(f−g) = f' − g'`

New helpers in `integrate.py`: `_try_u_sub_pow_one`, `_try_u_sub_pow`.
Hook in MUL branch after Phase 7, before Phase 4c.

New spec: `code/specs/phase8-power-composite-usub.md`.

40 new tests (`tests/test_phase8.py`). Package at ~491 tests.

## 0.12.0 — 2026-04-20

Phase 7 of the integration roadmap — u-substitution (chain-rule reversal).

Handles integrands of the form `f(g(x)) · c·g'(x)` where `f` is a single-argument
function (SIN, COS, EXP, LOG, TAN, SQRT) and `c` is a rational constant.

**Algorithm**: for each factor pair (outer, gp_candidate):
1. Extract `g(x)` = argument of the outer function.
2. Skip if `g = x` (Phase 1) or `g` is linear (Phases 3–5).
3. Compute `g'(x)` symbolically via `_diff_ir`.
4. Check `gp_candidate = c · g'(x)` via `_ratio_const`.
5. Introduce dummy symbol `u`, compute `∫ F(u) du`, substitute `g(x)` back.

New helpers in `integrate.py`: `_poly_deriv`, `_poly_mul`, `_diff_ir`,
`_ratio_const`, `_subst`, `_try_u_sub_one`, `_try_u_sub`.

Hook placed in the MUL branch after Phase 6, before Phase 4c/4a — linear-arg
integrands are guarded away so earlier phases retain their cases.

New spec: `code/specs/phase7-u-substitution.md`.

44 new tests (`tests/test_phase7.py`). Package at 451 tests.

## 0.11.0 — 2026-04-21

Phase 6 of the integration roadmap — mixed trig powers `sinⁿ·cosᵐ`.

Three cases, each with a distinct algorithm:

**Phase 6a — n odd (cosine substitution)**:
- Substitute `u = cos(ax+b)`, `du = -a sin(ax+b) dx`.
- Write `sinⁿ⁻¹ = (1-cos²)^k` (k=(n-1)/2) and expand via the binomial theorem.
- Closed-form result: `-(1/a) · Σ C(k,j)(-1)^j / (m+2j+1) · cos^{m+2j+1}(ax+b)`
- No recursion — direct polynomial anti-differentiation.

**Phase 6b — m odd, n even (sine substitution)**:
- Substitute `u = sin(ax+b)`, `du = a cos(ax+b) dx`.
- Write `cosᵐ⁻¹ = (1-sin²)^k` (k=(m-1)/2) and expand.
- Closed-form result: `(1/a) · Σ C(k,j)(-1)^j / (n+2j+1) · sin^{n+2j+1}(ax+b)`

**Phase 6c — both even (IBP reduction on n)**:
- Reduction: `∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹cosᵐ⁺¹/((n+m)a) + (n-1)/(n+m) · ∫ sinⁿ⁻² cosᵐ dx`
- Derived via IBP with Pythagorean substitution `cosᵐ⁺² = cosᵐ(1-sin²)`.
- Recurses on n: at n=0 delegates to `∫ cosᵐ dx` → Phase 5b.

New helpers in `integrate.py`: `_extract_trig_power`, `_try_sin_cos_power`,
`_sin_cos_odd_sin`, `_sin_cos_odd_cos`, `_sin_cos_even`.

New spec: `code/specs/phase6-sin-cos-powers.md`.

44 new tests (`tests/test_phase6.py`). Package at 407 tests, 90% coverage.

## 0.10.0 — 2026-04-20

Phase 5 of the integration roadmap — trig-power integration. Three sub-phases
covering `tan`, `sinⁿ`, `cosⁿ`, and `tanⁿ` for any integer `n ≥ 2`.

**Phase 5a — tan(ax+b)**:
- `∫ tan(ax+b) dx = −log(cos(ax+b)) / a` derived via substitution `u = cos(ax+b)`.
- Bare `∫ tan(x) dx = −log(cos(x))` handled in the Phase 1 elementary section.
- Extended linear-arg dispatch table from `{EXP, SIN, COS, LOG}` to include `TAN`.
- New helper `_tan_integral(a, b, x)` in `integrate.py`.

**Phase 5b — sinⁿ(ax+b) and cosⁿ(ax+b) reduction formulas** (`n ≥ 2`):
- `∫ sinⁿ(ax+b) dx = −sinⁿ⁻¹(ax+b)·cos(ax+b)/(n·a) + (n−1)/n · ∫ sinⁿ⁻²(ax+b) dx`
- `∫ cosⁿ(ax+b) dx =  cosⁿ⁻¹(ax+b)·sin(ax+b)/(n·a) + (n−1)/n · ∫ cosⁿ⁻²(ax+b) dx`
- Derived by integration by parts + the Pythagorean identity.
- Recursion terminates at `n=0` (→ `x`) and `n=1` (→ Phase 3 sin/cos result).

**Phase 5c — tanⁿ(ax+b) reduction formula** (`n ≥ 2`):
- `∫ tanⁿ(ax+b) dx = tanⁿ⁻¹(ax+b)/((n−1)·a) − ∫ tanⁿ⁻²(ax+b) dx`
- Derived using `tan² = sec² − 1`, making `∫ tanⁿ⁻² · sec² dx` exact.
- Recursion terminates at `n=0` (→ `x`) and `n=1` (→ Phase 5a tan result).

**POW base-case fixes** (needed for recursion correctness):
- `f^0 = 1` in the `POW` branch of `_integrate` now returns `x` directly.
- `f^1 = f` in the `POW` branch delegates to `_integrate(base, x)`.
- Both cases are also correct in isolation (not purely reduction plumbing).

New helpers in `integrate.py`: `_tan_integral`, `_try_trig_power`,
`_sin_power`, `_cos_power`, `_tan_power`.

Requires `coding-adventures-symbolic-ir >= 0.3.0` (adds `TAN` head) and
`coding-adventures-macsyma-compiler >= 0.2.0` (maps MACSYMA `tan` to `TAN`).
Requires `coding-adventures-symbolic-vm >= 0.10.0` for the `Tan` evaluation handler.

44 new tests (`tests/test_phase5.py`). Package at 363 tests, 90% coverage.

## 0.9.0 — 2026-04-20

Phase 4 of the integration roadmap — trigonometric integration. Three
sub-phases, each a clean layer on top of the existing integrator.

**Phase 4a — Polynomial × sin/cos** (`∫ p(x)·sin(ax+b) dx`,
`∫ p(x)·cos(ax+b) dx`):
- New module `symbolic_vm.trig_poly_integral`: `trig_sin_integral` and
  `trig_cos_integral` implement the **tabular IBP** formula. IBP applied
  `deg(p)+1` times yields two coefficient polynomials C and S:
  `∫ p·sin = sin·S − cos·C`, `∫ p·cos = sin·C + cos·S`.
- `_cs_coeffs` builds C and S from the derivative sequence of `p`, using
  `sign = (−1)^(k//2)` and divisor `a^(k+1)` for each index `k`.
- Wired into the `MUL` branch of `_integrate` as `_try_trig_product`.

**Phase 4b — Trig products and squares**:
- No new module; logic in `integrate.py` as `_try_trig_trig`.
- Applies the product-to-sum identities at the IR level:
  `sin·sin = [cos(u−v)−cos(u+v)]/2`, `cos·cos = [cos(u−v)+cos(u+v)]/2`,
  `sin·cos = [sin(u+v)+sin(u−v)]/2`. The resulting linear combination of
  bare sin/cos is recursively integrated by Phase 3 (cases 3b/3c).
- Handles all three orderings (sin·sin, cos·cos, sin·cos) by skipping the
  cos·sin ordering and relying on the swapped-argument retry in the caller.

**Phase 4c — Exp × sin/cos** (`∫ exp(ax+b)·sin(cx+d) dx`,
`∫ exp(ax+b)·cos(cx+d) dx`):
- New module `symbolic_vm.exp_trig_integral`: `exp_sin_integral` and
  `exp_cos_integral` implement the **double-IBP closed form**:
  `∫ exp·sin = exp·[a·sin − c·cos]/(a²+c²)`,
  `∫ exp·cos = exp·[a·cos + c·sin]/(a²+c²)`.
- Wired into the `MUL` branch as `_try_exp_trig`, before `_try_trig_product`.

Updated regression: `test_integrate_two_x_factors_unevaluated` renamed to
`test_integrate_poly_times_sin_now_closed_by_phase4` — Phase 4a now closes
`∫ x·sin(x) dx`.

Also updated `symbolic-computation.md` (Phase 4 description updated from
"Algebraic extensions" to the practical trig-integration scope).

39 new tests (`tests/test_phase4.py`). Package at 319 tests, 90% coverage.

## 0.8.0 — 2026-04-20

Phase 3 of the integration roadmap — transcendental integration for the
most common single-extension cases. Extends the integrator to handle
polynomials multiplied by `exp`, `log`, `sin`, or `cos` of a **linear**
argument `a·x + b`.

Five new cases, two algorithms:

- **Case 3a**: `∫ exp(ax+b) dx = exp(ax+b)/a` — generalises the
  existing Phase 1 `exp(x)` rule to any linear argument.
- **Case 3b**: `∫ sin(ax+b) dx = −cos(ax+b)/a` — generalises `sin(x)`.
- **Case 3c**: `∫ cos(ax+b) dx = sin(ax+b)/a` — generalises `cos(x)`.
- **Case 3d**: `∫ p(x)·exp(ax+b) dx` for `p ∈ Q[x]` — solved by the
  **Risch differential equation** `g′ + a·g = p` via back-substitution;
  result is `g(x)·exp(ax+b)`.
- **Case 3e**: `∫ p(x)·log(ax+b) dx` for `p ∈ Q[x]` — solved by
  **integration by parts** followed by polynomial long division; result
  is `[P(x) − P(−b/a)]·log(ax+b) − S(x)`.

New modules:
- `symbolic_vm.exp_integral`: `exp_integral(poly, a, b, x_sym)` —
  implements cases 3a and 3d.
- `symbolic_vm.log_integral`: `log_poly_integral(poly, a, b, x_sym)` —
  implements case 3e (and the `log(x)` case of 3e extends Phase 1's
  hard-coded result to arbitrary linear arguments).

`polynomial_bridge.py` gains a public `linear_to_ir(a, b, x)` helper
shared by both new modules.

`_integrate` in `integrate.py` gains:
- Extended elementary-function section recognising `EXP`/`SIN`/`COS`/
  `LOG` of linear arguments (cases 3a–3c, 3e-bare).
- Two new helper functions `_try_exp_product` and `_try_log_product`
  wired into the `MUL` branch to handle cases 3d and 3e.
- `_try_linear` helper that recognises `a·x + b` in the IR.

New spec `code/specs/phase3-transcendental.md` documents all five
cases with step-by-step algorithms and worked examples.

33 new tests (`tests/test_phase3.py`). Package at 280 tests, 89% coverage.

## 0.7.0 — 2026-04-20

Phase 2f of the integration roadmap — mixed partial-fraction integration
for denominators of the form L(x)·Q(x) where L is a product of distinct
linear factors over Q and Q is a single irreducible quadratic. Closes
rational-function integration for all denominators of this shape,
completing the most common class of textbook integrals (e.g.
`1/((x−1)(x²+1))`, `x/((x+2)(x²+4))`).

- New module `symbolic_vm.mixed_integral`:
  - `mixed_integral(num, den, x_sym) → IRNode | None` applies the
    Bézout identity to split `C/(L·Q)` into `C_L/L + C_Q/Q`, then
    delegates to Rothstein–Trager (Phase 2d) for the log part and
    `arctan_integral` (Phase 2e) for the arctan part. Returns `None`
    when the denominator does not match the L·Q shape (no rational
    roots, deg Q ≠ 2, or Q has rational roots).
- `Integrate` handler gains a Phase 2f step between the arctan check
  and the unevaluated fallback. The progress gate was extended to
  treat a successful `mixed_ir` result as progress.
- `rt_pairs_to_ir` moved from a private helper in `integrate.py` to a
  public function in `polynomial_bridge.py`, avoiding a circular import
  from `mixed_integral.py`. The private wrapper in `integrate.py` now
  delegates to it.
- New spec `code/specs/mixed-integral.md` documents the Bézout
  algorithm, worked example for `1/((x−1)(x²+1))`, and correctness
  derivation.
- 18 new tests (`tests/test_mixed_integral.py`): one-linear-one-
  quadratic (5), two-linear-one-quadratic (2), mixed numerators (2),
  fall-through guards (3), Bézout split identity verification (1), and
  end-to-end VM tests (5). Package at 247 tests, 90% coverage.

## 0.6.0 — 2026-04-20

Phase 2e of the integration roadmap — arctan antiderivatives for
irreducible quadratic denominators. Closes the gap left by
Rothstein–Trager: `1/(x²+1)` and its kin now produce closed-form
`arctan` output instead of staying as unevaluated `Integrate`.

- New module `symbolic_vm.arctan_integral`:
  - `arctan_integral(num, den, x_sym) → IRNode` applies the direct
    formula `A·log(E) + (2B/D)·arctan((2ax+b)/D)` for any proper
    rational function with an irreducible quadratic denominator
    `ax²+bx+c`. When `D = √(4ac−b²)` is rational (perfect square),
    the output carries only rational/integer leaves. When `D` is
    irrational, the IR carries `Sqrt(D²)` which the symbolic backend
    leaves unevaluated and the numeric backend folds.
- `Integrate` handler gains a Phase 2e step between RT and the
  unevaluated fallback: if RT returns `None` and the log-part
  denominator is a degree-2 irreducible polynomial, `arctan_integral`
  closes it. The progress gate was extended to treat a successful
  arctan result as progress (prevents infinite recursion).
- `atan` handler added to the VM handler table (evaluates `math.atan`
  numerically; leaves symbolic arguments unevaluated in symbolic mode).
- Depends on `coding-adventures-symbolic-ir ≥ 0.2.0` (adds `ATAN`).
- 25 new tests (`tests/test_arctan_integral.py`): pure imaginary
  denominators, completed-square denominators, mixed numerators
  (log + arctan), irrational discriminant (Sqrt in output), gating
  wrapper tests, and 6 end-to-end VM tests. The 1 existing test that
  expected an unevaluated `Integrate` for `1/(x²+1)` was updated to
  assert `Atan(x)`. Package at 229 tests, 90% coverage.

## 0.5.0 — 2026-04-19

Phase 2d of the integration roadmap — Rothstein–Trager. The log part
that Hermite reduction left as an unevaluated `Integrate` is now
emitted in closed form whenever every log coefficient happens to lie
in Q (the overwhelming majority of textbook cases). Integrands whose
coefficients escape Q — canonically `1/(x² + 1)` — still stay
unevaluated, awaiting a future `RootSum`/`RootOf` phase.

- New module `symbolic_vm.rothstein_trager`:
  - `rothstein_trager(num, den) → [(c_i, v_i), …] | None` produces
    the log-part pairs for ``∫ num/den dx = Σ c_i · log(v_i(x))`` or
    returns `None` when any coefficient escapes Q.
  - Builds the resultant ``R(z) = res_x(C − z·E', E) ∈ Q[z]`` by
    evaluation at ``deg E + 1`` nodes plus Lagrange interpolation —
    every internal arithmetic stays scalar over Q.
  - For each rational root ``α`` of ``R`` the log factor is
    ``v_α = monic(gcd(C − α·E', E))``; Rothstein's theorem guarantees
    the ``v_α`` are pairwise coprime and multiply back to monic(den).
- `Integrate` handler now routes the Hermite log-part through RT
  before falling back to unevaluated `Integrate`. The progress gate
  in `_integrate_rational` was generalised to treat a successful RT
  result as progress, so squarefree integrands like ``1/(x-1)`` now
  close in one step instead of bouncing into Phase 1.
- `_rt_pairs_to_ir` emits a left-associative binary `Add` chain of
  log terms; coefficients of ±1 collapse to bare `Log` / `Neg(Log)`,
  integer coefficients render as `IRInteger`, and non-integer
  rationals render as `IRRational`.
- Depends on `coding-adventures-polynomial ≥ 0.4.0` for the new
  `resultant` and `rational_roots` primitives.
- 12 new unit tests (`tests/test_rothstein_trager.py`) plus four
  end-to-end handler tests, bringing the package to 204 tests at
  90 % coverage. The RT module itself is at 100 %.

## 0.4.0 — 2026-04-19

Phase 2c of the integration roadmap — Hermite reduction. Rational
integrands now get their *rational part* in closed form; the log part
stays as an unevaluated `Integrate` with a squarefree denominator
(Rothstein–Trager, Phase 2d, will close it).

- New module `symbolic_vm.hermite`:
  - `hermite_reduce(num, den) → ((rat_num, rat_den), (log_num, log_den))`
    performs the classical Hermite reduction on a proper rational
    function over Q. The log-part denominator is guaranteed squarefree.
  - The correctness gate (and the universal unit-test invariant) is
    the re-differentiation identity
    `d/dx(rat_num / rat_den) + log_num / log_den == num / den`.
- `Integrate` handler grows a pre-check that routes rational
  integrands with non-constant denominators through
  `to_rational → polynomial division → hermite_reduce → from_polynomial`.
  Pure polynomials still go through the Phase 1 linear-recursion path
  (preserves the existing IR shape the rest of the test suite and
  downstream consumers are written against).
- Depends on `coding-adventures-polynomial ≥ 0.3.0` for the new
  `extended_gcd` primitive.
- `from_polynomial` now emits a left-associative binary `Add` chain —
  the arithmetic handlers are strictly binary, so n-ary applies tripped
  the arity check on the first `vm.eval`. The bridge tests were
  updated to the new shape.
- 21 new tests (15 unit-level Hermite, 6 end-to-end handler), bringing
  the package to 187 tests and 90 % coverage.

## 0.3.0 — 2026-04-19

Phase 2b of the integration roadmap — the IR ↔ polynomial bridge.

- New module `symbolic_vm.polynomial_bridge`:
  - `to_rational(f, x)` — recognises rational functions of the named
    variable `x` and returns `(numerator, denominator)` as `Polynomial`
    tuples with `Fraction` coefficients. Returns `None` for anything
    outside Q(x) (transcendentals, symbolic or fractional exponents,
    floats, free symbols).
  - `from_polynomial(p, x)` — emits the canonical IR tree for a
    polynomial at `x`, matching the shape the existing differentiator
    and Phase 1 integrator already produce.
- No cancellation of common factors: `(x² − 1)/(x − 1)` round-trips
  verbatim. Hermite reduction (Phase 2c) is the right place for that.
- Adds a dependency on `coding-adventures-polynomial`.
- 51 new tests, 100 % coverage on the bridge.

## 0.2.0 — 2026-04-19

First phase of the integration roadmap toward Risch.

- New `Integrate` handler on `SymbolicBackend` (parallel to `D`)
  implementing the "reverse derivative table" integrator:
  - Constant rule, power rule (including `x^(-1) → log(x)`),
    linearity (`Add`, `Sub`, `Neg`), constant-factor `Mul`,
    `∫(a/b) dx` for constant denominator, `∫(a/x) dx`,
    `∫a^x dx = a^x / log(a)`.
  - Elementary direct forms: `sin`, `cos`, `exp`, `sqrt`,
    `log` (the hard-coded integration-by-parts case).
- Anything outside the rule set stays as `Integrate(f, x)` unevaluated.
- End-to-end tests cover `integrate(x^2, x)`, `integrate(sin(x), x)`,
  and the `diff(integrate(f, x), x) → f` fundamental-theorem roundtrip.

## 0.1.0 — 2026-04-18

Initial release.

- Generic tree-walking `VM` over `symbolic_ir` nodes.
- `Backend` ABC with `lookup`, `bind`, `on_unresolved`,
  `on_unknown_head`, `rules`, `handlers`, `hold_heads`.
- `StrictBackend`: Python-like semantics; raises on unbound names or
  unknown heads; requires arithmetic operands to be numeric.
- `SymbolicBackend`: Mathematica-like semantics; leaves unbound names
  as free symbols; applies identity/zero laws; knows calculus.
- Shared handler table for arithmetic (`Add`, `Sub`, `Mul`, `Div`,
  `Pow`, `Neg`, `Inv`), elementary functions (`Sin`, `Cos`, `Exp`,
  `Log`, `Sqrt`), comparisons, logic, assignment, and definition.
- `D` handler on the symbolic backend implements sum, difference,
  product, quotient, power, and chain rules.
- User-defined functions via `Define(name, List(params), body)` —
  the VM detects the bound record and performs parameter substitution.
- `If` is a held head; only the chosen branch is evaluated.
- End-to-end tests cover the full pipeline (MACSYMA source → tokens
  → AST → IR → evaluated result).
