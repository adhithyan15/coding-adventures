# Phase 10 — Generalized Partial-Fraction Integration

## Motivation

After Phases 2c–2f and Phase 9 the rational-function route handles:

| Denominator type | Handler |
|---|---|
| Product of distinct linear factors | Phase 2d — Rothstein–Trager |
| Single irreducible quadratic | Phase 2e — arctan formula |
| Linear-factor group × single quadratic | Phase 2f — Bézout split |
| Two distinct irreducible quadratics | Phase 9 — biquadratic factoring |

Two classes remain unevaluated:

```
∫ 1/((x²+1)(x²+4)(x²+9))   dx     Q₁·Q₂·Q₃ — degree 6, no linear factors
∫ 1/((x−1)(x²+1)(x²+4))    dx     L·Q₁·Q₂  — degree 5, one linear factor
∫ 1/((x−1)(x−2)(x²+1)(x²+4)) dx   L²·Q₁·Q₂ — degree 6, two linear factors
```

There is also a performance bug: Rothstein–Trager hangs on degree-6 denominators
because its Sylvester-matrix resultant computation uses exact Fraction arithmetic on
a 12×12 matrix, causing coefficient explosion. Measured: `(x²+1)(x²+4)(x²+9)` took
26,261 seconds before returning None.

Phase 10 fixes the RT hang and fills both remaining integration gaps with a single
generalized partial-fraction driver.

## Scope

### What Phase 10 handles

1. **RT performance guard** — skip Rothstein–Trager for degree ≥ 6 denominators,
   where it is known to hang and always returns None anyway (log coefficients are
   irrational for products of quadratics).

2. **Three distinct irreducible quadratic factors** (degree-6 squarefree denominator,
   no rational roots, factors into three degree-2 irreducible polynomials over Q):

   ```
   ∫ N(x) / (Q₁(x)·Q₂(x)·Q₃(x)) dx    deg N < 6, Qᵢ distinct irreducibles over Q
   ```

3. **Any number of rational linear factors × exactly two irreducible quadratics**
   (degree-5 or -6 squarefree denominator with at least one rational root and
   exactly-two-quadratic remainder):

   ```
   ∫ N(x) / (L(x)·Q₁(x)·Q₂(x)) dx    L = product of distinct (x−rᵢ), rᵢ ∈ Q
   ```

### What Phase 10 does NOT handle

- Denominators with four or more distinct irreducible quadratic factors (degree ≥ 8).
- Denominators that are irreducible of degree 4 over Q (e.g. `x⁴+1` factors over
  Q(√2) only — irrational coefficients).
- Denominators of degree > 6 in general.

## Mathematical Algorithm

### Step 1 — RT performance guard

In `_integrate_rational`, guard the Rothstein–Trager call:

```python
if len(normalize(den)) - 1 >= 6:
    rt_pairs = None   # skip; Phase 10 handles degree-6 cases
else:
    rt_pairs = rothstein_trager(num, den)
```

This is safe because for degree-6 denominators with any irreducible quadratic factor
the resultant polynomial R(z) has irrational roots, so RT would return None regardless —
but only after an impractically long computation.

### Step 2 — Degree-6 factoring into three irreducible quadratics

`_factor_triple_quadratic(E)` given monic degree-6 squarefree poly with no rational
roots:

For each candidate constant term `b₁` from `_rational_divisors(p₀)` and linear
coefficient `a₁` from `{0, ±1, ±2, ±3, ±4} ∪ _rational_divisors(p₅)`:
1. Check exact divisibility: `divmod_poly(E, (b₁, a₁, 1))` with zero remainder
2. If exact and `a₁²−4b₁ < 0` (Q₁ irreducible): call `_factor_biquadratic(quotient)`
3. If the degree-4 quotient factors, return `(Q₁, Q₂, Q₃)`

The candidate set covers all textbook cases with small rational coefficients.

### Step 3 — Generalized D×D partial-fraction system

Given `N / (P₁·P₂·…·Pₖ)` with `deg Pᵢ = dᵢ` and `D = Σdᵢ`, find numerators
`Nᵢ` with `deg Nᵢ < dᵢ` such that:

```
N  =  Σᵢ Nᵢ · (∏_{j≠i} Pⱼ)  =  Σᵢ Nᵢ · Mᵢ
```

Expanding the equation gives a D×D linear system over Q. The column for the j-th
coefficient of Nᵢ is the cofactor Mᵢ = ∏_{j≠i} Pⱼ shifted left by j positions.
Gaussian elimination over `Fraction` always yields a unique solution when the Pᵢ
are pairwise coprime (a consequence of the Chinese Remainder Theorem for polynomials).

`_solve_pf_general(num, factors)` implements this for any list of coprime factors.

### Step 4 — Integrate each piece

```
∫ N / (L₁·L₂·…·Q₁·Q₂·…) dx
  =  Σᵢ Aᵢ · log(x − rᵢ)           [linear pieces, via rt_pairs_to_ir]
  +  Σⱼ arctan_integral(Nⱼ, Qⱼ, x)  [quadratic pieces, via Phase 2e helper]
```

## Interaction with Earlier Phases

| Integrand | Route |
|-----------|-------|
| `1/((x²+1)(x²+4))` | Phase 9: degree 4 |
| `1/((x−1)(x²+1))` | Phase 2f: L·Q |
| `1/((x²+1)(x²+4)(x²+9))` | Phase 10: three quadratics |
| `1/((x−1)(x²+1)(x²+4))` | Phase 10: L·Q₁·Q₂ |
| `1/((x−1)(x−2)(x²+1)(x²+4))` | Phase 10: L²·Q₁·Q₂ |

## Limitations and Future Work

- **Degree ≥ 8**: four or more irreducible quadratics remain unevaluated.
- **Irrational factorizations**: `1/(x⁴+1)` requires algebraic extensions.
- **Degree 7+**: `(x−1)(x²+1)(x²+4)(x²+9)` has degree 7 and remains unevaluated.

## New Code

All new functions are added to `integrate.py`.

### `_factor_triple_quadratic(E: tuple) → tuple[tuple, tuple, tuple] | None`

Given a degree-6 monic squarefree polynomial (ascending Fraction coefficient tuple),
tries to write `E = Q₁·Q₂·Q₃` with all three irreducible monic quadratics over Q.
Uses polynomial division to test each candidate, then delegates to `_factor_biquadratic`
for the degree-4 quotient. Returns `(Q₁, Q₂, Q₃)` or None.

### `_solve_pf_general(num: tuple, factors: list[tuple]) → list[tuple] | None`

Solves the D×D partial-fraction system by Gaussian elimination over Fraction.
`factors` is a list of coprime polynomial tuples with total degree D. Returns a
list of D numerator-coefficient tuples (one per factor, in the same order), or None
if the system is singular.

### `_try_general_rational_integral(num: tuple, den: tuple, x_sym: IRSymbol) → IRNode | None`

Top-level Phase 10 driver. Fires after Phase 9 returns None. Handles degree-5 and
degree-6 squarefree denominators by: finding rational roots → extracting quadratic
remainder → factoring it into two or three irreducible quadratics → solving the
partial-fraction system → integrating each piece. Returns an ADD tree of log/arctan
IR nodes, or None.

## Files Changed

| File | Change |
|------|--------|
| `code/specs/phase10-generalized-partial-fractions.md` | **NEW** — this document |
| `code/packages/python/symbolic-vm/src/symbolic_vm/integrate.py` | MODIFY — RT guard, 3 new helpers, Phase 10 hook |
| `code/packages/python/symbolic-vm/tests/test_phase10.py` | **NEW** — ≥ 40 tests |
| `code/packages/python/symbolic-vm/CHANGELOG.md` | MODIFY — 0.15.0 entry |
| `code/packages/python/symbolic-vm/pyproject.toml` | MODIFY — version 0.15.0 |
