# Changelog

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
