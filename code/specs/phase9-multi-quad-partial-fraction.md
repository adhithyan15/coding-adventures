# Phase 9 — Multi-Quadratic Partial-Fraction Integration

## Motivation

After Phases 2c–2f the rational-function route can handle:

| Denominator type | Handler |
|---|---|
| Product of distinct linear factors | Phase 2d — Rothstein–Trager |
| Single irreducible quadratic | Phase 2e — arctan formula |
| Linear-factor group × single quadratic | Phase 2f — mixed partial fractions |

One class remains: **products of two or more distinct irreducible quadratic
factors over Q with no linear factors**. Examples:

```
∫ 1/((x²+1)(x²+4))   dx   →   (1/3)·atan(x)   − (1/6)·atan(x/2)
∫ 1/((x²+1)(x²+9))   dx   →   (1/8)·atan(x)   − (1/24)·atan(x/3)
∫ (x+1)/((x²+1)(x²+4)) dx →  (1/6)·log(x²+1) + (1/3)·atan(x)
                                − (1/6)·log(x²+4) − (1/6)·atan(x/2)
```

For these integrands, Rothstein–Trager returns `None` (the log coefficients are
complex, not rational), Phase 2e doesn't apply (degree ≠ 2), and Phase 2f
doesn't apply (no rational roots in the denominator). Phase 9 fills this gap.

Phase 9 also adds a **direct table entry for ∫ atan(ax+b) dx** — a single-factor
integral handled once and for all via the IBP formula, so the rational route
never needs to see it.

## Scope

### What Phase 9 handles

1. **Two distinct irreducible quadratic factors** (degree-4 squarefree denominator,
   no rational roots, factors into two degree-2 irreducible polynomials over Q):

   ```
   ∫ N(x) / (Q₁(x)·Q₂(x)) dx    deg N < 4, Q₁ ≠ Q₂, both irreducible over Q
   ```

2. **Direct ∫ atan(ax+b) dx** — IBP result added to the elementary table.

### What Phase 9 does NOT handle

- Denominators with three or more distinct irreducible quadratic factors (degree ≥ 6).
  These are deferred to Phase 10.
- Denominators that are irreducible of degree 4 over Q (e.g. `x⁴+1` factors as
  `(x²+√2 x+1)(x²−√2 x+1)` but requires algebraic extensions — irrational `a`).
- Mixed denominators with both linear factors and multiple quadratic factors
  (e.g. `(x−1)(x²+1)(x²+4)`). Phase 2f handles the simpler L·Q case; the general
  L·Q₁·Q₂···Qk case is deferred.

## Mathematical Algorithm

### Step 1 — Biquadratic factoring

Given monic degree-4 polynomial `E(x) = x⁴+p₃x³+p₂x²+p₁x+p₀` with no rational
roots, find `Q₁ = x²+ax+b` and `Q₂ = x²+cx+d` (both irreducible over Q) such that
`Q₁·Q₂ = E`.

**Coefficient matching** yields the system:

```
(1)  a + c = p₃
(2)  b + d + a·c = p₂
(3)  a·d + b·c = p₁
(4)  b·d = p₀
```

Eliminating `c = p₃ − a` from (1) and `d = p₀/b` from (4), then substituting into (3):

```
a·(p₀/b) + b·(p₃−a) = p₁
a·(p₀ − b²)/b = p₁ − b·p₃
a = b·(p₁ − b·p₃) / (p₀ − b²)     [valid when b² ≠ p₀]
```

Verification: check that eq (2) holds exactly.

**Candidate `b` values** — rational divisors of `p₀`: by the Rational Roots Theorem
applied to the product `b·d = p₀`, `b` must be rational with `d = p₀/b` also
rational. So candidates are `±(divisors of |p₀.numerator|) / (divisors of
p₀.denominator)`. This is a finite set covering all textbook cases.

**Irreducibility check**: `a² − 4b < 0` AND `c² − 4d < 0`.

### Step 2 — Partial-fraction decomposition

Given `N/(Q₁·Q₂)` with `deg N < 4`, find `A₁,B₁,A₂,B₂ ∈ Q` such that:

```
N  =  (A₁x+B₁)·Q₂  +  (A₂x+B₂)·Q₁
```

Expanding and matching coefficients of `x³, x², x, 1` gives the 4×4 linear system:

```
[ 1  0  1  0 ] [ A₁ ]   [ n₃ ]
[ c  1  a  1 ] [ B₁ ] = [ n₂ ]
[ d  c  b  a ] [ A₂ ]   [ n₁ ]
[ 0  d  0  b ] [ B₂ ]   [ n₀ ]
```

where `Q₁ = x²+ax+b` and `Q₂ = x²+cx+d`. Solved by Gaussian elimination over Q.
The system is always non-singular when Q₁ ≠ Q₂ (a consequence of coprimality).

### Step 3 — Integrate each piece

```
∫ N/(Q₁·Q₂) dx  =  arctan_integral((A₁x+B₁), Q₁, x)
                  +  arctan_integral((A₂x+B₂), Q₂, x)
```

Each piece is handled by the existing Phase 2e `arctan_integral()` function, which
produces the combined `A·log(Qᵢ) + B·atan(...)` output automatically.

### Bonus — ∫ atan(ax+b) dx

Integration by parts with `u = atan(ax+b)`, `dv = dx`, `du = a/((ax+b)²+1) dx`,
`v = x`:

```
∫ atan(ax+b) dx  =  x·atan(ax+b)  −  (1/(2a))·log((ax+b)²+1)
```

Added to the elementary linear-arg dispatch in `_integrate` (Phase 3 section),
alongside `sin`, `cos`, `exp`, `log`, `tan` for linear arguments. Fires for any
`ATAN(linear(x))` before the rational route gets involved.

## Interaction with Earlier Phases

| Integrand | Route |
|-----------|-------|
| `x/((x²+1)(x²+4))` | Phase 2d (RT): log coefficients ±1/6 ∈ Q → RT succeeds |
| `1/((x²+1)(x²+4))` | Phase 9: RT returns None, arctan coefficients ±1/3, ±1/6 |
| `1/((x−1)(x²+1))` | Phase 2f: L·Q denominator |
| `atan(x)` | Phase 9 bonus: direct table entry |

The `x/((x²+1)(x²+4))` case is handled by RT before Phase 9 ever fires, which
is correct — RT succeeds whenever all log coefficients lie in Q, regardless of
the denominator's degree.

## Limitations and Future Work

- **Degree 6+**: three irreducible quadratics (`1/((x²+1)(x²+4)(x²+9))`) remain
  unevaluated. Phase 10 will extend `_factor_biquadratic` to iterative factoring
  and generalise the partial-fraction solve to k factors.
- **Irrational factorizations**: `1/(x⁴+1)` splits as `(x²+√2 x+1)(x²−√2 x+1)`
  but `√2 ∉ Q`; the biquadratic factoring algorithm returns None, leaving it
  unevaluated. Algebraic extensions are a longer-term goal.
- **Mixed linear + multiple quadratics**: `L(x)·Q₁(x)·Q₂(x)` denominators remain
  unevaluated unless Phase 2f or Phase 9 happens to match a sub-case. A full
  partial-fraction over multiple coprime factors is Phase 10+.

## New Code

All new functions live in `integrate.py` to keep the rational route self-contained.

### `_rational_divisors(p: Fraction) → list[Fraction]`

Returns all ±(integer divisors of `p.numerator`) / (integer divisors of
`p.denominator`) as a deduplicated list, excluding zero.

### `_factor_biquadratic(E: tuple) → tuple[tuple, tuple] | None`

Given degree-4 monic squarefree polynomial as an ascending Fraction coefficient
tuple, tries to write `E = Q₁·Q₂` with both irreducible monic quadratics. Returns
`(Q₁, Q₂)` or None.

### `_solve_pf_2quad(num, Q1, Q2) → tuple[Fraction, Fraction, Fraction, Fraction] | None`

Solves the 4×4 partial-fraction system by Gaussian elimination over Fraction.
Returns `(A₁, B₁, A₂, B₂)` or None if singular.

### `_try_multi_quad_integral(num, den, x_sym) → IRNode | None`

Top-level Phase 9 driver. Returns an `ADD(ir1, ir2)` IR tree or None.

## Files Changed

| File | Change |
|------|--------|
| `code/specs/phase9-multi-quad-partial-fraction.md` | **NEW** — this document |
| `code/packages/python/symbolic-vm/src/symbolic_vm/integrate.py` | MODIFY — add 4 helpers, 2 hooks |
| `code/packages/python/symbolic-vm/tests/test_phase9.py` | **NEW** — ≥ 40 tests |
| `code/packages/python/symbolic-vm/CHANGELOG.md` | MODIFY — 0.14.0 entry |
| `code/packages/python/symbolic-vm/pyproject.toml` | MODIFY — version 0.14.0 |
