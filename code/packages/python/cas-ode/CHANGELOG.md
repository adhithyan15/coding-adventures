# Changelog — cas-ode

All notable changes to this project will be documented in this file.

## [0.6.0] — 2026-05-16

### Added

- **Named variable-coefficient 2nd-order ODE recognition** — Phase 21
  - Recognises four classical ODE families whose solutions are named
    special functions.  Pattern matching is *numerical*: coefficients
    are evaluated at four interior test points `(0.3, 0.6, −0.25, 0.85)`
    and compared to the expected algebraic form.  This handles any
    syntactically equivalent tree the MACSYMA compiler might generate
    for the same equation without requiring structural tree normalisation.

  - **Legendre ODE** (`_try_legendre_ode`)
    `(1−x²)y'' − 2x·y' + n(n+1)·y = 0` → `Equal(y, %c1·LegendreP(n,x) + %c2·LegendreQ(n,x))`
    - P≈1−x² and Q≈−2x checked numerically; R extracted as constant n(n+1).
    - `_legendre_n_from_lambda(lam)` finds non-negative integer n with n(n+1)=λ
      using the quadratic formula and a round-trip integer check.

  - **Bessel ODE** (`_try_bessel_ode`)
    `x²y'' + x·y' + (x²−ν²)·y = 0` → `Equal(y, %c1·BesselJ(ν,x) + %c2·BesselY(ν,x))`
    - P≈x² and Q≈x checked numerically.
    - ν extracted from R(x) = x²−ν² via `_nu_from_r_minus_xsq` which evaluates
      R at x=1 and x=2, verifies R(2)−R(1)≈3 (confirms x²-offset structure),
      then finds the positive rational ν=p/q (denominator ≤ 20) by trial.
    - Integer ν represented as `IRInteger`; fractional ν as `IRRational`.
      Half-integer orders ν = n+1/2 (spherical Bessel functions) fully supported.

  - **Hermite ODE** (`_try_hermite_ode`)
    `y'' − 2x·y' + 2n·y = 0` → `Equal(y, %c1·HermiteH(n,x) + %c2·HermiteH2(n,x))`
    - P must be exactly 1 (constant leading coefficient); Q≈−2x.
    - R is constant = 2n; n must be a non-negative integer (reject non-even R).

  - **Chebyshev ODE** (`_try_chebyshev_ode`)
    `(1−x²)y'' − x·y' + n²·y = 0` → `Equal(y, %c1·ChebyshevT(n,x) + %c2·ChebyshevU(n,x))`
    - P≈1−x² and Q≈−x checked numerically; R = n² requires perfect-square check.
    - **Tried before Legendre** in the dispatcher because both have P≈1−x² but
      differ on Q (−x vs −2x). Priority ordering prevents misclassification.

  - **Phase 21 dispatcher** (`_try_var_coeff_named_ode`)
    - Tries Chebyshev → Legendre → Bessel → Hermite in order and returns the
      first match, or `None` if none of the families match.
    - Inserted in `solve_ode` after Euler-Cauchy (step 5 — see Changed).

- **New helper functions** (Section 3b):
  - `_split_out_factor(term, target) → IRNode | None` — extracts K from
    K·target in a nested Mul tree.  Handles `Neg(...)` wrappers, three-level
    nesting, and the identity case term == target (returns `IRInteger(1)`).
  - `_collect_var2_coeffs(expr, y, x) → (P, Q, R) | None` — generalises
    `_collect_second_order_coeffs` to allow polynomial/rational coefficient
    functions P(x), Q(x), R(x).  Flattens the Add tree and uses
    `_split_out_factor` to attribute each term to y'', y', or y.
  - `_eval_ir_at_x(node, x, xv) → float | None` — numerically evaluates an
    x-only IR expression at a scalar point using the existing `_eval_at_xy`
    helper with a dummy y symbol.
  - `_coeff_matches_func(node, x, expected, tol=1e-9) → bool` — checks whether
    `node` agrees with `expected(xv)` at all four canonical test points.
  - `_extract_const_val(node, x) → float | None` — returns the numeric value
    of a constant-w.r.t.-x node (None if it contains x).
  - `_legendre_n_from_lambda(lam) → int | None` — quadratic-formula root
    finder for n(n+1) = lam.
  - `_nu_from_r_minus_xsq(R_node, x) → (p, q) | None` — rational-ν extractor
    for Bessel R(x) = x²−ν² (denominator search up to 20).
  - `_build_named_solution(sym1, sym2, param_ir, y, x) → IRNode` — builds
    `Equal(y, %c1·sym1(param,x) + %c2·sym2(param,x))`.

### Changed

- `solve_ode` dispatcher — Phase 21 step added after Euler-Cauchy:
  1. `_try_second_order_nonhom` (undetermined coefficients, Phase 18)
  2. `_try_vop` (variation of parameters, Phase 20)
  3. `_collect_second_order_coeffs` / `solve_second_order_const_coeff`
  4. `_try_euler_cauchy` (Phase 19)
  5. **`_try_var_coeff_named_ode`** ← new (Phase 21)
  6. `_try_bernoulli`
  7. `_collect_linear_first_order` / `solve_linear_first_order`
  8. `_try_separable`
  9. `_try_homogeneous_type`
  10. `_try_exact`

- Module docstring updated: "nine" → "thirteen" ODE classes; Phase 21 entries
  (Legendre, Bessel, Hermite, Chebyshev) added to the class enumeration and
  the literate reading guide (entries 26–38 added).
- `pyproject.toml` description updated to include named variable-coefficient
  ODEs.
- `symbolic-ir` dependency bumped to `>=0.14.0` (required for `LEGENDRE_P`,
  `LEGENDRE_Q`, `BESSEL_J`, `BESSEL_Y`, `HERMITE_H`, `HERMITE_H2`,
  `CHEBYSHEV_T`, `CHEBYSHEV_U` head symbols).

### Tests

- **95 new tests** in `tests/test_phase21.py` across eight new classes:
  - `TestSplitOutFactor` — 11 tests: identity, direct left/right, nested,
    Neg wrappers, unrelated terms, different-target None.
  - `TestCollectVar2Coeffs` — 6 tests: Legendre n=2, Bessel ν=1, Hermite n=3
    coefficient checks; no-y'' None; free-constant None; missing-Q defaults-0.
  - `TestLegendreNFromLambda` — 10 tests: n=0..4 round-trips; non-triangular
    λ=5, λ=7; negative λ; slightly-off float; float precision near n=3.
  - `TestNuFromRMinusXSq` — 8 tests: ν=0,1,2,3,1/2,3/2; non-x²-minus-const
    input; negative ν².
  - `TestTryLegendreOde` — 9 tests: n=0..3 recognised; λ=5 rejected; wrong-Q
    rejected; Bessel/Hermite/const-coeff fall through.
  - `TestTryBesselOde` — 10 tests: ν=0,1,2,1/2,3/2 recognised; C1/C2
    present; Legendre/Hermite/const-coeff fall through; R=x²+1 rejects.
  - `TestTryHermiteOde` — 10 tests: n=0..3 recognised; non-integer R; negative
    R; Legendre/Bessel fall through; P≠1 rejects.
  - `TestTryChebyshevOde` — 9 tests: n=0..3 recognised; non-square R; not
    confused with Legendre; Bessel/Hermite fall through.
  - `TestVarCoeffNamedOdeDispatcher` — 7 tests: dispatches all four families;
    Chebyshev-before-Legendre priority verified; unrelated and 1st-order None.
  - `TestPhase21EndToEnd` — 9 tests: solve_ode and VM-level dispatch for each
    family; rational ν parameter; Equal(y,...) structure; C1/C2 present.
  - `TestPhase21Regressions` — 6 tests: const-coeff real/complex roots;
    no cross-contamination with named ODEs; Euler-Cauchy; first-order linear;
    unrecognised variable-coeff stays unevaluated.
- Combined coverage: **86.86%** (332 tests total).

---

## [0.5.0] — 2026-05-08

### Added

- **Variation of parameters (VoP) ODE solver** — Phase 20
  - Handles `a·y'' + b·y' + c·y = f(x)` for any forcing function whose
    primitives the integration engine can evaluate.
  - Fires as a fallback after undetermined coefficients (Phase 18), so EPT-family
    forcing (const, poly ≤ 2, exp, sin/cos, exp×sin/cos) is still handled by
    the cleaner undetermined-coefficient solver first.
  - VoP runs *before* the homogeneous solver so that a non-EPT forcing is not
    misrouted to the homogeneous solver (which silently ignores the RHS).
  - **Wronskian closed forms** — hard-coded analytically for each root case to
    avoid symbolic Wronskian computation at runtime:
    - **Distinct real roots** `r₁ ≠ r₂` (disc > 0, rational √disc):
      `u₁' = f·e^{−r₁x}/(r₁−r₂)`, `u₂' = f·e^{−r₂x}/(r₂−r₁)`.
      Negative coefficient is placed at the *outer* level as `Neg(...)` so
      the integration engine's Neg-distribution rule fires before the
      exp-product recogniser — this prevents the buried `Mul(Neg(1), Exp(…))`
      structure that blocked integration.
    - **Repeated root** `r` (disc = 0):
      `u₁' = −f·x·e^{−rx}`, `u₂' = f·e^{−rx}`.
    - **Complex roots** `α ± βi` (disc < 0, rational β):
      `u₁' = −f·sin(βx)·e^{−αx}/β`, `u₂' = f·cos(βx)·e^{−αx}/β`.
  - Irrational discriminants and irrational β return `None` — the integrands
    would contain symbolic √ expressions the VM cannot integrate in general.
  - Integration falls through gracefully: if either `∫u₁' dx` or `∫u₂' dx` is
    unevaluated, `_try_vop` returns `None` and the ODE remains unevaluated.
  - **Trig resonance handled** — `y'' + y = sin(x)` was previously unevaluated
    (undetermined-coeff det = 0); VoP now returns the correct general solution
    with particular solution `y_p = −x·cos(x)/2`.
  - Entry point `_try_vop(expr, y, x, vm)` returns `Equal(y, y_h + y_p)` or
    `None` on pattern mismatch or integration failure.

- **`_signed_frac_to_ir`** — new module-level helper (replaces duplicate local
  `_ir_from_frac` closures in `solve_second_order_const_coeff` and
  `solve_euler_cauchy`).
  - Lifts a signed `Fraction` to the canonical IR literal, wrapping negative
    values with `Neg(...)` for clean printing.
  - `_signed_frac_to_ir(Fraction(-3, 2))` → `Neg(Rational(3, 2))`.

- **`_exp_r`** — new helper that builds `exp(r·x)` with trivial-case collapsing:
  - `r = 0` → `IRInteger(1)` (e⁰ = 1)
  - `r = 1` → `Exp(x)`
  - `r = -1` → `Exp(Neg(x))`
  - Other rational `r` → `Exp(Mul(r_ir, x))`
  - Avoids `Mul(0, x)` inside Exp for zero exponents.

- **`_vop_integrand_pair`** — Wronskian-derived VoP integrands for each root
  case (distinct real, repeated, complex).  Returns `(u1_prime, u2_prime, y1, y2)`
  or `None` for irrational roots.

### Changed

- `solve_ode` dispatcher — VoP step 2 added between undetermined coefficients
  and the homogeneous solver:
  1. `_try_second_order_nonhom` (undetermined coefficients)
  2. **`_try_vop`** ← new (Phase 20)
  3. `_collect_second_order_coeffs` / `solve_second_order_const_coeff`
  4. `_try_euler_cauchy`
  5. `_try_bernoulli`
  6. `_collect_linear_first_order` / `solve_linear_first_order`
  7. `_try_separable`
  8. `_try_homogeneous_type`
  9. `_try_exact`

- Module docstring updated: "eight" → "nine" ODE classes; Phase 20 description
  added.
- Literate reading guide: entries 21–25 added for `_signed_frac_to_ir`,
  `_exp_r`, `_vop_integrand_pair`, `_try_vop`, and `solve_ode`.
- `pyproject.toml` description updated to include variation-of-parameters.
- `tests/test_phase18.py` — `test_trig_resonance_unevaluated` renamed to
  `test_trig_resonance_solved_by_vop` and updated to assert that VoP *does*
  return a solution for `y'' + y = sin(x)` (previously expected unevaluated
  fall-through; VoP now handles this resonance case correctly).

### Tests

- **32 new tests** in `tests/test_ode.py` across five new classes:
  - `TestSignedFracToIr` — 5 tests: positive integer, zero, positive rational,
    negative integer, negative rational.
  - `TestExpR` — 5 tests: r=0, r=1, r=-1, positive fraction, negative fraction.
  - `TestVopIntegrandPair` — 11 tests: distinct/repeated/complex root tuples,
    y1/y2 structure, alpha=0 no-exp check, irrational disc/beta None, sign checks.
  - `TestTryVop` — 7 tests: homogeneous None, irrational-disc None, distinct-roots
    poly-3 success, C1/C2 present, complex-roots poly-3 success, C1/C2, EPT success.
  - `TestSolveOdeVopDispatch` — 4 tests: dispatch to VoP, distinct-roots dispatch,
    EPT not routed through VoP, homogeneous not routed.
- Combined coverage: **85.54%** (237 tests total).

---

## [0.4.0] — 2026-05-08

### Added

- **Euler-Cauchy equidimensional ODE solver** — Phase 19
  - Recognises `a·x²·y'' + b·x·y' + c·y = 0` via `_collect_euler_cauchy_coeffs`.
  - Each term must have the same *weight* (power of x equals derivative order);
    the recogniser rejects bare constants, plain `y'`, or any 3-factor products.
  - Extracts rational coefficients `(a, b, c)` using `_flatten_product`, the Mul-tree
    analogue of the existing `_flatten_add` helper.
  - Solves via the **indicial equation** `a·r² + (b−a)·r + c = 0` (derived by
    the ansatz `y = x^r`):
    - **Distinct real roots** `r₁ ≠ r₂` → `y = C₁·x^{r₁} + C₂·x^{r₂}`
    - **Repeated root** `r` → `y = (C₁ + C₂·ln x)·x^r`
    - **Complex conjugate roots** `α ± βi` → `y = x^α·(C₁·cos(β ln x) + C₂·sin(β ln x))`
  - Irrational discriminants are represented as symbolic `Pow(disc, 1/2)` nodes
    (exact arithmetic throughout — no floats).
  - Entry point `_try_euler_cauchy(expr, y, x)` returns `Equal(y, solution)` or
    `None` on pattern mismatch.

- **`_flatten_product`** — new helper in Section 2b
  - Recursively decomposes a `Mul` tree into `(total_rational_coefficient, [non_rational_factors])`.
  - Handles `Neg(...)` (flips sign), `IRInteger`, `IRRational`, and any other
    node (treated as a single factor with coefficient 1).
  - Mirrors `_flatten_add` in spirit: enables the Euler-Cauchy recogniser to
    extract coefficients without knowing the nesting depth of the `Mul` tree.

### Changed

- `solve_ode` dispatcher — new step 3 added:
  1. `_try_second_order_nonhom`
  2. `_collect_second_order_coeffs` / `solve_second_order_const_coeff`
  3. **`_try_euler_cauchy`** ← new (Phase 19)
  4. `_try_bernoulli`
  5. `_collect_linear_first_order` / `solve_linear_first_order`
  6. `_try_separable`
  7. `_try_homogeneous_type`
  8. `_try_exact`

- Module docstring updated to list Euler-Cauchy as the 8th ODE class.
- Literate reading guide entries 13–16 added for the four new helpers/functions;
  former entries 13–17 renumbered to 17–21.

### Tests

- **47 new tests** in `tests/test_ode.py` across four new classes:
  - `TestFlattenProduct` — 9 tests: integer, rational, symbol, Neg, Mul(int, sym),
    triple product, etc.
  - `TestCollectEulerCauchyCoeffs` — 8 tests: full 3-term, two-term, scaled leading
    coefficient, missing-x² returns None, const-coeff returns None, single term,
    bare-x term.
  - `TestSolveEulerCauchy` — 12 tests: all three root cases (distinct real ×2,
    repeated ×3, complex ×3), solution head/structure, C1/C2 presence.
  - `TestEulerCauchyViaDispatcher` — 6 tests: full `solve_ode` pipeline for each
    root type, const-coeff not consumed by EC, `eval_ode` dispatch, scaled coeffs.
- Combined coverage: **84.54%** (205 tests total).

---

## [0.3.0] — 2026-05-06

### Added

- **Homogeneous-type ODE solver** (`_try_homogeneous_type`) — Phase 18c
  - Recognises `dy/dx = f(y/x)`, where the right-hand side depends only
    on the ratio `y/x`.
  - Uses structural pattern matching (`_subst_ratio_ir`) to replace every
    `Div(y, x)` node with the temporary symbol `_hom_v`, yielding `f(v)`.
    Returns `None` immediately if any bare `y` appears outside `Div(y, x)`.
  - Builds the separable equation `dv/(f(v)−v) = dx/x`, then delegates
    both integrations to the existing VM `Integrate` handler (including
    the Hermite partial-fraction path for rational `1/(f(v)−v)`).
  - **Degenerate case** `f(v) = v` (i.e. `y' = y/x`): denominator is zero,
    so `v = const`, and the solution is returned directly as
    `Equal(y, Mul(%c, x))`.
  - **Implicit solution** for the general case:
    `Equal(H(y/x), Add(Log(x), %c))` where `H(v) = ∫ dv/(f(v)−v)`.
  - Falls through to `None` if the RHS integrand has no closed-form
    antiderivative (e.g. `f(v) = exp(v)`).
  - Runs in the dispatcher after separable and before exact, ensuring
    that linear/separable ODEs that happen to be expressible as `f(y/x)`
    are handled by the simpler (explicit) routes first.

- **IR tree substitution helper** (`_subst_ir`)
  - Pure structural tree walk replacing every occurrence of a given
    `IRSymbol` with a replacement IR node.  Used for back-substitution
    `v → y/x` after computing `H(v)`.

- **Structural ratio substitution helper** (`_subst_ratio_ir`)
  - Replaces exactly the pattern `Div(y, x)` with a symbol `v`, without
    needing algebraic simplification.  Returns `None` if `y` appears in
    any form other than `Div(y, x)`, ensuring correctness for the VM
    that cannot simplify `(v·x)/x → v`.

### Changed

- `solve_ode` dispatcher — updated order (step 6 added):
  1. `_try_second_order_nonhom`
  2. `_collect_second_order_coeffs` / `solve_second_order_const_coeff`
  3. `_try_bernoulli`
  4. `_collect_linear_first_order` / `solve_linear_first_order`
  5. `_try_separable`
  6. **`_try_homogeneous_type`** ← new (Phase 18c)
  7. `_try_exact`

- Module docstring updated to list homogeneous-type as the 7th ODE class.
- Literate reading guide updated: entries 10–12 added for new helpers;
  former entries 10–14 renumbered to 13–17.

### Tests

- **26 new tests** in `tests/test_ode.py` across three new classes:
  - `TestSubstIr` — 7 tests: symbol replaced, no-op, integer, rational,
    nested Add, Pow back-sub, absent symbol unchanged.
  - `TestSubstRatioIr` — 9 tests: Div(y,x)→v, bare y→None, Pow,
    integer/symbol passthrough, Add of two ratios, y-in-Add-numerator→None,
    unrelated symbol, Mul(y,x)→None.
  - `TestHomogeneousTypeODE` — 15 tests: degenerate case, ratio², ratio²+ratio,
    2·(y/x), transcendental fall-through, const/linear/y·x fall-through,
    no-y'-term, full `solve_ode` degenerate, ODE2 VM dispatch ×2,
    linear ODE captured by separable not homogeneous.

---

## [0.2.0] — 2026-04-29

### Added

- **Bernoulli ODE solver** (`_try_bernoulli`)
  - Recognises `y' + P(x)·y = Q(x)·y^n` (n ≠ 0, 1) in zero form by
    scanning for `D(y,x)`, `y^n`, and `y` terms.
  - Applies the substitution `v = y^(1-n)` to reduce to the first-order
    linear ODE `v' + (1-n)·P·v = (1-n)·Q`, delegating to the existing
    integrating-factor solver.
  - Back-substitutes to return `Equal(y, v_sol^(1/(1-n)))`.
  - Handles integer `n` (positive or negative), arbitrary x-only P(x) and Q(x).

- **Exact ODE solver** (`_try_exact`)
  - Recognises `M(x,y) + N(x,y)·y' = 0` by extracting M (y'-free terms)
    and N (coefficient of `D(y,x)`).
  - Exactness check `∂M/∂y = ∂N/∂x` uses numerical evaluation at four
    interior test points (`_exprs_equal_numerically`) to handle structurally
    different but mathematically equal IR expressions from the VM's
    differentiation rules.
  - Computes the potential `F = ∫M dx`, then `g'(y) = N − ∂F/∂y`,
    then `g = ∫g'(y) dy`.
  - Returns the implicit solution `Equal(F + g, %c)`.
  - Runs last in the dispatch order so that explicitly solvable ODEs
    (separable, linear) return the preferred explicit `Equal(y, f(x))` form.

- **Second-order non-homogeneous solver** (`_try_second_order_nonhom`)
  - Recognises `a·y'' + b·y' + c·y = f(x)` with constant rational
    coefficients and a closed-form forcing function.
  - `_collect_second_order_nonhom` — extends the coefficient collector to
    capture the forcing term `f(x)` (x-only terms moved to the RHS).
  - `_classify_forcing` — identifies seven forcing families: constant,
    polynomial (degree ≤ 2), `e^(αx)`, `sin(βx)`, `cos(βx)`,
    `e^(αx)·sin(βx)`, `e^(αx)·cos(βx)`.
  - `_compute_particular` — undetermined-coefficients method for each
    family with full resonance handling:
    - Exponential: s = 0, 1, or 2 based on multiplicity of α as char root.
    - Trig: 2×2 linear system; falls through if det = 0 (resonance).
    - Exp×trig: exponential shift theorem to reduce to trig case.
    - Polynomial: matches from highest degree down with resonance shift.
  - Homogeneous solution from existing `solve_second_order_const_coeff`.
  - General solution: `y_h + y_p`.
  - Checked before the homogeneous solver in the dispatcher (prevents
    mis-classification of non-homogeneous ODEs as homogeneous).

- **Auxiliary helpers** (Section 9):
  - `_fold_numeric` — folds `Mul(a, Mul(b, expr))` when a,b are rationals.
  - `_eval_at_xy` — numerical evaluation of an IR tree at (x, y) = (xv, yv).
  - `_exprs_equal_numerically` — numerical equality check at four test points.
  - `_extract_linear_coeff_x` — extracts α from `α·x` patterns.
  - `_try_polynomial_forcing` — recognises polynomial IR trees up to degree 2.
  - `_char_poly_at` — evaluates `a·r² + b·r + c`.
  - `_is_pow_y` — detects `Pow(y, n)` atoms.

### Changed

- `solve_ode` dispatcher — new order:
  1. `_try_second_order_nonhom` (Phase 18)
  2. `_collect_second_order_coeffs` / `solve_second_order_const_coeff`
  3. `_try_bernoulli` (Phase 18)
  4. `_collect_linear_first_order` / `solve_linear_first_order`
  5. `_try_separable`
  6. `_try_exact` (Phase 18, last)

### Tests

- **47 new tests** in `tests/test_phase18.py` across 5 classes:
  - `TestPhase18_Bernoulli` — 10 tests: n=2,3,-1; P=1,x; fallthrough; structure
  - `TestPhase18_Exact` — 10 tests: 2xy/x², polynomial M/N; not-exact; implicit form
  - `TestPhase18_NonHomogeneous2ndOrder` — 12 tests: all forcing families;
    resonance exp; polynomial; structure checks
  - `TestPhase18_Fallthrough` — 7 tests: variable coeff, unrecognised forcing,
    trig resonance
  - `TestPhase18_Regressions` — 7 tests: all Phase 0.1.0 solver types
- Combined coverage: **82.89%** (135 tests total)

---

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
