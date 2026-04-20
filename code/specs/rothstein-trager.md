# Rothstein–Trager — Logarithmic Part of Rational-Function Integration

## Why this module exists

After Hermite reduction (Phase 2c) the integrand has been split into a
closed-form rational part plus a *residual* `C(x)/E(x)` with `E`
squarefree. The antiderivative of the residual is a sum of logarithms:

    ∫ C/E dx  =  Σᵢ  cᵢ · log(vᵢ(x))

for some constants `cᵢ` and polynomials `vᵢ`. Producing that sum in
closed form — without ever factoring `E` into linear factors over Q̄ — is
what **Rothstein–Trager** does. It's the final piece that closes
rational-function integration over Q.

## Scope

Phase 2d delivers Rothstein–Trager for integrands whose **resultant
polynomial has only rational roots**. This is the overwhelming majority
of textbook examples (every proper rational function whose denominator
factors into distinct linear factors over Q ends up here). For
integrands with irrational or complex `cᵢ` — `1/(x² + 1)` is the
canonical example — the implementation returns `None` and the handler
leaves the piece as an unevaluated `Integrate(C/E, x)`, preserving the
current behavior.

### What Phase 2d ships

1. `polynomial.resultant(a, b)` — scalar resultant of two polynomials
   over any field (the coefficient ring we actually exercise is Q, via
   `Fraction`). Computed by the Euclidean-PRS recurrence — no Sylvester
   matrix, no subresultant bookkeeping — since the downstream caller
   does its multivariate resultant by evaluation + Lagrange
   interpolation, and only scalar resultants are needed on the
   polynomial-package side.
2. `polynomial.rational_roots(p)` — the set of distinct rational roots
   of a polynomial in Q[z]. Uses the Rational Roots Theorem: clear
   denominators, enumerate `±p/q` with `p | a₀`, `q | aₙ`, check by
   evaluation, and peel each confirmed root by polynomial division so
   the search shrinks.
3. `symbolic_vm.rothstein_trager` — `rothstein_trager(num, den)`
   returning either a list of `(c, v)` pairs such that
   `∫ num/den dx = Σ c · log(v)`, or `None` when the answer escapes Q.
4. `Integrate` handler grows an after-Hermite step: if Hermite produced
   a non-trivial squarefree residual, try RT; when it returns pairs,
   emit the log sum as IR; otherwise preserve the unevaluated
   `Integrate` shape that Phase 2c already emits.

### What Phase 2d does not ship

- **Algebraic number outputs.** When the resultant has an irrational
  root `α`, the closed form involves `α · log(gcd(C − αE', E))` over
  Q(α). We don't carry `RootOf`/`RootSum` constructs yet; that's a
  bigger IR change and belongs in Phase 3. For now: irrational roots →
  `None` → unevaluated.
- **Coefficient extensions beyond Q.** The polynomial package already
  works over any field that supports `+ − * /`, but RT as shipped here
  treats the ground field as Q concretely (rational-roots check,
  `Fraction` coercions). Extending to Q(ξ) for transcendental Risch is
  a Phase 3 concern.
- **Multivariate resultants as a public polynomial primitive.** We
  compute `resₓ(C − z·E', E)` via evaluation + Lagrange interpolation
  inside `rothstein_trager` — every internal resultant stays scalar.
  A first-class `resultant_bivariate` can wait until a second caller
  needs it.

## The algorithm

Given `C, E ∈ Q[x]` with `deg C < deg E` and `E` squarefree:

1. **Build the resultant polynomial in `z`.**

       R(z)  =  resₓ( C(x) − z · E'(x),  E(x) )

   `R` is a polynomial in `z` alone, `deg_z R ≤ deg E`. Computed
   numerically at `deg E + 1` distinct sample points `z = z₀, z₁, …`
   (we use `0, 1, 2, …`), each via the scalar `polynomial.resultant`,
   then Lagrange-interpolated back to a polynomial in Q[z].

   This is the standard "multivariate-by-evaluation" trick: it sidesteps
   the bookkeeping of pseudo-division over Q(z) and keeps every internal
   arithmetic scalar.

2. **Find the roots of `R(z)` in Q.**

   Use `polynomial.rational_roots(R)`. If the multiset of rational
   roots has fewer elements than `deg R`, the resultant has a non-Q
   root — give up (return `None`).

3. **For each distinct root `α ∈ Q`, emit one log term.**

       vα(x)  =  gcd( C(x) − α · E'(x),  E(x) )
       term   =  α · log(vα(x))

   Rothstein's theorem (Bronstein, Ch. 2) guarantees:
   - Each `vα` is a non-constant polynomial in Q[x];
   - The `vα` are pairwise coprime;
   - `∏α vα  =  E`   (so the log factors reassemble the denominator);
   - `∫ C/E dx = Σα α · log(vα(x))` holds as formal antiderivatives.

4. **Return the list `[(α, vα), …]`** to the handler, which emits the
   IR `Add` chain of `Mul(IRRational(num, den), Log(<vα IR>))`.

### Why it works — the one-line intuition

If `E` splits in Q̄ as `E = ∏ (x − βⱼ)` with distinct `βⱼ`, then partial
fractions gives

    C/E  =  Σⱼ  cⱼ / (x − βⱼ)  with  cⱼ = C(βⱼ)/E'(βⱼ).

The distinct *values* `{cⱼ}` are precisely the roots of the RT
resultant `R(z)`, and `vα = ∏_{j: cⱼ = α} (x − βⱼ)` groups the simple
poles whose residues collide on the same value. Integrating gives
`Σα α · log(vα)`. The beauty: we never have to produce the individual
`βⱼ` to get the answer. The resultant + gcd combo projects the whole
partial-fraction computation down onto Q whenever the `cⱼ` happen to
all lie in Q.

## Inputs and outputs

### `polynomial.resultant(a, b)`

- **Inputs**: two polynomials over a field (we test on Q via
  `Fraction`; integer coefficients work too, just promoted through
  division).
- **Output**: a scalar (same coefficient type) equal to
  `res(a, b) = lc(a)^deg(b) · ∏ b(αᵢ) = lc(b)^deg(a) · ∏ a(βⱼ)`
  where `αᵢ, βⱼ` are the roots of `a, b` in an algebraic closure.
- **Edge cases**: `res(_, ()) = res((), _) = 0`;
  `res(a, c) = c^deg(a)` for a nonzero constant `c`;
  `res(a, b) = 0` iff `a` and `b` share a root (⇔ `deg gcd ≥ 1`).
- **Recurrence**: if `deg a ≥ deg b > 0` and `r = a mod b`, then

      res(a, b) = (−1)^(deg a · deg b) · lc(b)^(deg a − deg r) · res(b, r)

  with the trivial base case `res(a, const) = const^deg a`. This is the
  Euclidean-PRS resultant; it stays in the base field throughout.

### `polynomial.rational_roots(p)`

- **Input**: a polynomial in Q[z] (Fraction or int coefficients).
- **Output**: a list of distinct Fraction roots, in ascending numeric
  order. Empty list when `p` is zero/constant or has no rational root.
- **Method**: promote to Fraction, rescale so all coefficients are
  integers (multiply through by the LCM of denominators), then enumerate
  candidate rationals `±p/q` with `p ∣ |a₀|` and `q ∣ |aₙ|`. For each
  hit, divide it out and search the quotient — so a root of
  multiplicity `m` is returned once, and the search shrinks.
- **Note**: "distinct" matters because RT consumes the roots with the
  assumption that each corresponds to one log term. Multiplicity >1 in
  the resultant would only occur in degenerate constructed cases which
  Phase 2d does not attempt to handle.

### `rothstein_trager(num, den)`

- **Preconditions**: `den` is squarefree (guaranteed by Hermite);
  `deg num < deg den`; both are `Polynomial` tuples over Q.
- **Output**:
  - `[(c₁, v₁), (c₂, v₂), …]` with `cᵢ ∈ Fraction` and `vᵢ` a
    non-constant monic polynomial in Q[x] when RT succeeds — the log
    sum `Σ cᵢ · log(vᵢ)` is the antiderivative;
  - `None` when any root of the RT resultant lies outside Q — the
    handler must keep the unevaluated form.

### Handler wiring

Inside `_integrate_rational` in `symbolic_vm.integrate`, after
`hermite_reduce` returns `((rat_num, rat_den), (log_num, log_den))`:

    if has_log:
        rt = rothstein_trager(log_num, log_den)
        if rt is None:
            # Phase 2d gives up — emit unevaluated Integrate exactly as
            # Phase 2c did.
            pieces.append(IRApply(INTEGRATE, (integrand_ir, x)))
        else:
            pieces.append(_rt_pairs_to_ir(rt, x))

`_rt_pairs_to_ir` builds `Σ cᵢ · log(vᵢ)` as a left-associative binary
Add chain (the arithmetic handlers are strictly binary). Each term is
`Mul(IRRational(c.numerator, c.denominator), Log(from_polynomial(v, x)))`,
simplified to just `Log(v)` when `c == 1` and to `Neg(Log(v))` when
`c == −1`.

## Non-goals

- **General polynomial factoring over Q.** RT specifically avoids it —
  the whole point is that resultant + gcd gets you the answer without
  having to factor `E`. We won't add a Q-factoring routine as part of
  this PR.
- **Simplification of the log sum.** We emit `log(v)` literally as
  `Log(v_as_IR)`. Whether the VM further simplifies `log(x − 1) +
  log(x + 1)` into `log(x² − 1)` or not is irrelevant to correctness;
  our universal test gate works at the derivative level.
- **Arctangent / partial closure over C.** `∫ 1/(x² + 1) dx` stays
  unevaluated in this PR. A later phase can add RootSum/RootOf or
  convert to arctan via the logarithmic closure `log((x−i)/(x+i)) / 2i`.

## Test strategy

### `resultant` tests

- Scalar-on-coprime: `res(x − 1, x − 2) = −1` (or sign-appropriate
  constant).
- Shared-root vanishing: `res((x − 1)(x + 2), (x − 1)(x + 3)) = 0`.
- Constant second arg: `res(p, c) = c^deg p`.
- Cross-check via `lc(a)^deg b · ∏ b(αᵢ)` on polynomials whose roots
  we know by construction.

### `rational_roots` tests

- `(x − 1)(x − 2)(x − 3)` → `[1, 2, 3]`.
- `(2x − 1)(3x + 2)` → `[−2/3, 1/2]`.
- Irrational (no rational roots): `x² − 2` → `[]`.
- Integer coefficients vs. Fraction coefficients parity.
- Multiplicity is collapsed: `(x − 1)³` → `[1]`.

### `rothstein_trager` unit tests

The universal gate is the **re-differentiation identity** evaluated on
the log sum:

    d/dx ( Σ cᵢ · log(vᵢ) )  =  Σ cᵢ · vᵢ' / vᵢ  =  num/den

Since `∏ vᵢ = den` (a guaranteed invariant), this reduces to a purely
polynomial check: `Σ cᵢ · vᵢ' · ∏_{j≠i} vⱼ == num`. Every test ends by
running this check.

Known-answer spot tests:

- `∫ 1/(x − 1) dx` → `[(1, x − 1)]` → `log(x − 1)`.
- `∫ 1/(x² − 1) dx` → `[(1/2, x − 1), (−1/2, x + 1)]`.
- `∫ x/((x − 1)(x − 2)) dx` → `[(−1, x − 1), (2, x − 2)]`.
- `∫ (3x + 1)/(x(x + 1)) dx` → `[(1, x), (2, x + 1)]`.
- `∫ 1/(x² + 1) dx` → `None` (roots of `R(z)` are `±i/2`).

### End-to-end handler tests

- `integrate(1/(x − 1), x)` → `log(x − 1)` (as IR).
- `integrate(1/((x − 1)(x + 1)), x)` → rational-coefficient log sum.
- `integrate(1/((x − 1)²·(x + 1)), x)` — Hermite rational part plus
  RT log sum, **no** `Integrate` left in the output.
- `integrate(1/(x² + 1), x)` stays unevaluated (RT returns `None`).
- Phase 1 rules still handle `integrate(sin(x), x)` etc.

## Dependencies

- `polynomial` — adds `resultant`, `rational_roots`; consumes
  `deriv`, `divmod_poly`, `gcd`, `monic`, `evaluate`, arithmetic.
- `symbolic-vm` — new module `rothstein_trager`; `integrate.py` gets a
  short post-Hermite step.

## Forward compatibility

- Extending to `RootSum(R, α → α · log(gcd(C − α·E', E)))` is a pure
  additive change: when `rational_roots(R) ≠` all roots, return a
  structured result instead of `None`. No change to the existing shape.
- Extending the ground field from Q to Q(t₁, …, tₖ) (Risch's
  transcendental tower) requires replacing the rational-roots test
  with a factoring routine over the larger field. The RT core — the
  resultant + gcd — is unchanged.
