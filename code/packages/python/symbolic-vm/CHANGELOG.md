# Changelog

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
