# Changelog

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
