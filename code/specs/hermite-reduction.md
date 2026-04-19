# Hermite Reduction — Rational Part of Rational-Function Integration

## Why this module exists

Given a rational function `N(x)/D(x)` over Q, the antiderivative has
exactly two pieces:

    ∫ N/D dx  =  (rational part)  +  (logarithmic part)

The **rational part** is itself a rational function, produced by
integrating the "removable" poles — the parts of `N/D` whose
antiderivative is again rational. The **logarithmic part** is a sum
of `c_i · log(v_i(x))` terms; producing it from an arbitrary rational
integrand requires Rothstein–Trager (a later phase).

Hermite reduction is the decision procedure for the rational part. It
takes `N/D` and returns

- a rational function `A/B` (the rational part of the antiderivative), and
- a new rational function `C/E` with `E` *squarefree* (everything that
  remains is a pure logarithmic integrand).

After Hermite, `∫ C/E dx` has no rational contribution — every
antiderivative is a sum of logs (and arctans, but those are
`log` over `C`). Solving that last step is Rothstein–Trager's job.
Hermite's contribution: whatever rational part exists is now in closed
form; the residual integrand has a *squarefree* denominator, which is
the exact input the log-part algorithms need.

## Scope

Phase 2c delivers Hermite reduction only — end to end it turns a
rational integrand into `(closed-form rational part) + Integrate(C/E, x)`
where `E` is squarefree. The log part stays unevaluated; a later PR
brings Rothstein–Trager.

### What Phase 2c ships

1. `polynomial.extended_gcd(a, b) → (g, s, t)` — extended Euclidean
   algorithm returning `g = gcd(a, b)` and Bézout cofactors `s, t`
   satisfying `s·a + t·b = g`. Over Q[x] where `gcd(a, b) = 1`, the
   cofactors give the partial-fraction decomposition step that Hermite
   needs.
2. `symbolic_vm.hermite.hermite_reduce(num, den) →
   (rational_part, log_integrand)` — the reduction itself. Both outputs
   are `(Polynomial, Polynomial)` pairs over Q.
3. `Integrate` handler fires Hermite *before* the Phase 1 rules. When
   the integrand is a rational function of `x`:
   - Split off the polynomial part via polynomial division — this is
     trivially integrable and never needs Hermite.
   - Run Hermite on the proper rational remainder.
   - Emit `(integrated_poly) + (rational_part_as_IR) + Integrate(C/E, x)`.
     The `Integrate(C/E, x)` stays unevaluated; a later phase replaces
     it with the log-part sum.

### What Phase 2c does not ship

- **Rothstein–Trager.** The log part remains unevaluated. The handler
  emits `Integrate(...)` for whatever squarefree remainder Hermite
  produces. Users who ask for `∫ 1/(x² + 1) dx` today will see
  `Integrate(1/(x² + 1), x)` — exactly the pre-Phase-2c behavior for
  that specific input, but now with a guarantee that every *rational*
  part of the answer has been extracted.
- **Factoring over Q.** Squarefree factorization already exists
  (`polynomial.squarefree`). We do not attempt to factor an arbitrary
  squarefree polynomial over Q into irreducibles — that's a Phase 3+
  capability and not necessary for Hermite itself. Hermite's inner
  loop works over squarefree factors, not irreducibles.
- **Coefficient extensions.** Q only. Admitting Q(ξ) — rational
  functions with one extra parameter symbol — is the standard path to
  Rothstein–Trager and belongs in that PR.

## The algorithm

We use the **quadratic (Hermite–Ostrogradsky)** form because it maps
cleanly onto the primitives the polynomial package already exposes.

Given `N/D` with `deg N < deg D` (after polynomial-division split),
compute the squarefree factorization `D = ∏_{i=1..k} D_i^i`. Define
`D_1* = ∏_i D_i` (the squarefree part of `D`) and note `D_2* = D / D_1*`
(the repeated part). Then every `i ≥ 2` contributes exactly one Bézout
step:

For `k = max multiplicity` down to `2`:

    Let V = D_k, U = D / V^k.
    Solve  B·(U·V') + C·V  =  N   for polynomials B, C with deg B < deg V
                                  using extended_gcd(U·V', V).

    (gcd(U·V', V) divides gcd(D', V) which, for V = D_k squarefree and
     U coprime to V, is 1 — so extended_gcd returns (1, s, t) and
     Bézout gives us B = (N · s) mod V, C derived by substitution.)

    Subtract the derivative of the emitted rational piece and
    continue on the residual.

The fixed-point output is `(A/B_final, N_new / D_squarefree)` where
`D_squarefree = ∏ D_i` and `B_final = ∏ D_i^{i-1}`.

A simpler, equivalent loop — the one we actually implement — keeps
peeling one power at a time:

    while denominator has a repeated factor:
        1. Find V = a squarefree factor of den with multiplicity m ≥ 2.
        2. Split den = V^m · U with gcd(V, U) = 1.
        3. Solve B·U·V' + C·V = N for (B, C) with deg B < deg V.
           (Use extended_gcd(U·V', V) and scale the cofactor by N.)
        4. Emit  A/(V^{m-1})  as a piece of the rational part
           (A is derived directly from B and the integration-by-parts
           boundary term).
        5. Replace N, den by the residual after d/dx(A/V^{m-1}) is
           subtracted. The residual has multiplicity (m-1) for V and
           still proper.

This terminates in at most `sum(i-1 for i in multiplicities)` steps —
a small number for any human integrand.

### Why this works

Integration by parts applied to `a/V^m` (with `m ≥ 2`) gives
`a/V^m = d/dx(−a/((m-1)V^{m-1})) + (a' V − (m-1)·a·V')/((m-1)·V^m)`.
Choosing `a` cleverly — specifically, `a` derived from the Bézout
cofactors — cancels the `V^m` in the remainder, dropping the
multiplicity of `V` by one. Iterating kills every repeated factor and
leaves a squarefree denominator.

The full derivation is in any CAS textbook; see *Bronstein, Symbolic
Integration I*, Chapter 2, for the standard proof.

## Inputs and outputs

### `hermite_reduce(num, den)`

- **Inputs**: two `Polynomial` values over Q (`Fraction` coefficients).
  `num` is required to be proper (`deg num < deg den`); the handler
  peels off the polynomial part before calling. `den` must be non-zero.
- **Output**:
  `((rational_num, rational_den), (log_num, log_den))` where
  - `(rational_num, rational_den)` is the rational part of the
    antiderivative. If there is no rational part (denominator was
    already squarefree), `rational_num = ()` and `rational_den = (1,)`.
  - `(log_num, log_den)` is the residual integrand with squarefree
    denominator. If Hermite fully integrates `num/den` — possible when
    the log part happens to be zero — then `log_num = ()` and
    `log_den = (1,)`.

### Handler wiring

The `Integrate` handler grows a new pre-check:

    def handler(expr):
        f, x = expr.args
        # Try rational-function path first.
        r = to_rational(f, x)
        if r is not None:
            return _integrate_rational(r, x)
        # Fallback: Phase 1 rules.
        return _phase1(f, x)

`_integrate_rational` does:

1. Polynomial division of `num` by `den` to get `(q, r)` with
   `deg r < deg den`. Emit `∫ q dx` via `from_polynomial` + the
   trivial power-rule integrator (we reuse the existing logic by
   pushing `q` through `from_polynomial` and calling the Phase 1
   handler on the result).
2. Run `hermite_reduce(r, den)` to get `(rat, log_rem)`.
3. Emit `from_polynomial(rat_num, x) / from_polynomial(rat_den, x)`
   for the rational part.
4. Emit `Integrate(from_polynomial(log_num, x) / from_polynomial(log_den, x), x)`
   for the log part — unless `log_num` is zero, in which case skip it.

The final output is the IR sum of these three pieces.

## Non-goals

- Factoring log-part denominators over Q. Even when the denominator
  happens to be `(x − 1)`, we leave it as `Integrate(1/(x − 1), x)` for
  this PR. The Phase 1 rule `∫(1/x) dx = log x` will not rescue it
  because we skip Phase 1 for rational integrands — a deliberate choice
  so the new path is unambiguously *the* rational-function path. A
  later PR adds linear-factor log recognition as a bridge before full
  Rothstein–Trager.
- Partial fractions as an integration technique in its own right.
  Hermite is the modern replacement and produces the same rational
  parts without needing to factor the denominator into irreducibles.
- Arbitrary-field coefficients. Q only.

## Test strategy

- **Extended GCD**: round-trip `s·a + t·b == gcd(a, b)` on randomized
  polynomial pairs; degree bound `deg s < deg b` and `deg t < deg a`;
  edge cases for `b == 0`, `a == b`, coprime inputs.
- **Hermite on known integrands**:
  - `∫ 1/(x − 1)^2 dx = −1/(x − 1)` — pure rational part.
  - `∫ 1/((x − 1)^2 (x + 1)) dx` — mixed rational + log parts;
    verify the rational part via re-differentiation.
  - `∫ x/((x − 1)^3) dx` — numerator degree non-trivial.
  - `∫ 1/(x − 1) dx` — squarefree denominator; rational part zero,
    log integrand equals the input.
- **Re-differentiation check**: for any test case, `d/dx(rat) +
  log_integrand == original_integrand`. This is the universal
  correctness gate — it reduces to polynomial arithmetic only, no
  symbolic reasoning.
- **Handler end-to-end**: `integrate((x - 1)^-2, x) →
  Mul(-1, Pow(x - 1, -1))` (modulo the canonical shape the VM
  simplifier produces); `integrate(1/((x-1)^2 (x+1)), x)` emits a
  rational piece plus an unevaluated `Integrate(1/((x^2-1)), x)`.
- **Fallback preservation**: non-rational integrands still hit the
  Phase 1 rules — `integrate(sin(x), x) → -cos(x)`, etc.

## Dependencies

- `polynomial` — adds `extended_gcd`; otherwise consumes the existing
  `deriv`, `monic`, `squarefree`, `gcd`, `divmod_poly`, arithmetic.
- `symbolic-ir`, `symbolic-vm.polynomial_bridge` — unchanged.

## Forward compatibility

The rational part produced by Hermite is independent of anything in
Rothstein–Trager. When RT lands, it consumes the `log_integrand`
output of `hermite_reduce` and replaces the emitted
`Integrate(log_integrand, x)` with a closed-form log sum. No re-work
of Hermite itself.
