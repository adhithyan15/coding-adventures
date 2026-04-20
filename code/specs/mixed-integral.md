# Phase 2f — Mixed Partial-Fraction Integration

## Status

Phase 2f of the rational-function integration roadmap. Closes out the
remaining common class of rational integrands: those whose squarefree
denominator has **both rational (linear) roots and one irreducible
quadratic factor**. After Phase 2f, the only integrands still left
unevaluated are those whose denominator has an irreducible factor of
degree ≥ 3 over Q (e.g. `1/(x³+x+1)`), or multiple distinct irreducible
quadratic factors (e.g. `1/((x²+1)(x²+4))`). Both require algebraic
extensions and are deferred to Phase 4.

## Scope

### What this phase handles

A squarefree denominator `E(x)` that factors as `L(x) · Q(x)` where:

- `L(x) = c · ∏(x − rᵢ)` — a product of distinct linear factors over Q
  (i.e., all rational roots of `E`).
- `Q(x) = ax² + bx + c` — a single irreducible quadratic over Q
  (degree 2, discriminant < 0, no rational roots).
- `deg num < deg E` (proper fraction — Hermite's guarantee).

Examples handled:
- `∫ 1/((x−1)(x²+1)) dx`
- `∫ x/((x+2)(x²+4)) dx`
- `∫ 1/((x−1)(x+1)(x²+2x+5)) dx`
- `∫ (x²+1)/((x−1)(x−2)(x²+1)) dx`

### What this phase does NOT handle

- Denominators with multiple distinct irreducible quadratic factors
  (e.g. `(x²+1)(x²+4)`). Factoring a quartic polynomial over Q without
  a general factoring algorithm is out of scope; these stay unevaluated.
- Irreducible factors of degree ≥ 3. These require algebraic extensions
  (Phase 4).
- Repeated factors. Hermite reduction guarantees the log part is squarefree,
  so this is not a concern here.

## Algorithm

### Overview

Use the Bézout identity to split the numerator into a "linear-factors
part" and a "quadratic-factor part", then integrate each piece with
the already-implemented Phase 2d (RT) and Phase 2e (arctan) tools.

### Step-by-step

```
mixed_integral(num_q, den_q, x_sym) → IRNode | None

  Preconditions (enforced by _try_mixed_integral gate, not re-checked):
    1. RT returned None on (num_q, den_q)
    2. arctan_integral returned None on (num_q, den_q)
    3. den_q is squarefree

  Steps:
    a. roots = rational_roots(den_q)
       If roots is empty: return None  (no linear factors — out of scope)

    b. Build L = ∏(x − rᵢ) over all distinct rational roots rᵢ.
       L is monic with rational coefficients. Note: each rᵢ is a root of
       full multiplicity 1 in den_q (squarefree guarantee).

    c. Compute Q = den_q / L via exact polynomial long division.
       (Remainder is zero — L divides den_q by construction.)

    d. If deg Q ≠ 2: return None  (not a single irreducible quadratic)
    e. If rational_roots(Q) is non-empty: return None
       (Q has rational roots → was already handled or degenerate)

    f. (g, u, v) = extended_gcd(L, Q)
       Since gcd(L, Q) = 1 (they share no roots), g is a non-zero
       constant. Scale to get the normalised Bézout coefficients:
         u′ = u / g,   v′ = v / g
       Satisfying:  u′ · L  +  v′ · Q  =  1

    g. Split the numerator:
         C_L = (num_q · v′) mod L    — numerator for the L-part
         C_Q = (num_q · u′) mod Q    — numerator for the Q-part

       Key invariant: C_L · Q + C_Q · L ≡ num_q  (mod den_q)
       Degrees: deg C_L < deg L,  deg C_Q < deg Q = 2.

    h. Integrate the L-part via Rothstein–Trager:
         rt_pairs = rothstein_trager(C_L, L)
       Since L is a product of distinct linear factors over Q, every log
       coefficient is rational. RT is guaranteed to succeed here; if it
       returns None (which would indicate a bug), we bail.

    i. Integrate the Q-part via arctan_integral:
         at_ir = arctan_integral(C_Q, Q, x_sym)
       Always succeeds under preconditions (deg Q = 2, irreducible, deg C_Q < 2).

    j. Convert rt_pairs to IR via rt_pairs_to_ir(rt_pairs, x_sym).

    k. Combine the two IR pieces into a binary Add tree and return.
```

### Correctness of the Bézout split

From the identity `u′ · L + v′ · Q = 1`:

    num / (L · Q) = num · (u′ · L + v′ · Q) / (L · Q)
                 = num · u′ / Q  +  num · v′ / L

Both numerators `num · u′` and `num · v′` may have degree ≥ deg Q or
≥ deg L respectively, so we reduce them modulo Q and L:

    num · u′ = k_Q · Q + C_Q    (C_Q = (num · u′) mod Q, deg C_Q < deg Q)
    num · v′ = k_L · L + C_L    (C_L = (num · v′) mod L, deg C_L < deg L)

The polynomial parts k_Q and k_L cancel out in the split:

    num / (L · Q) = C_Q / Q  +  C_L / L  +  k_Q / L  +  k_L / Q

And since `k_Q / L + k_L / Q` integrates to a polynomial (which Hermite's
polynomial division already extracted from the full integrand before the
log-part step), the residual `k_Q + k_L` is the zero polynomial when
`deg num < deg(L · Q)`. Formally: `k_Q · Q + k_L · L = num - C_Q·L - C_L·Q`
which has degree < deg(L·Q) and is divisible by L·Q only if it's zero.

### Worked example

`∫ 1/((x−1)(x²+1)) dx`:

- `L = x − 1`,  `Q = x² + 1`
- `extended_gcd(x−1, x²+1)` yields `g = 2`, `u = −(x+1)`, `v = 1`
  (since `−(x+1)(x−1) + 1·(x²+1) = −x²+1+x²+1 = 2`).
  Scale: `u′ = −(x+1)/2`, `v′ = 1/2`.
- `C_Q = 1·u′ mod (x²+1) = −(x+1)/2`  (degree 1 < 2 ✓)
- `C_L = 1·v′ mod (x−1)  = 1/2`        (degree 0 < 1 ✓)
- RT on `(1/2)/(x−1)` → `[(1/2, x−1)]` → `(1/2)·log(x−1)`
- arctan on `(−(x+1)/2)/(x²+1)`:
  `A = (−1/2)/(2) = −1/4`,  `B = −1/2 − 0 = −1/2`,  `D = 2`
  → `−(1/4)·log(x²+1) − (1/2)·arctan(x)`
- Combined: `(1/2)·log(x−1) − (1/4)·log(x²+1) − (1/2)·arctan(x)`

Verification (differentiate):
`1/(2(x−1)) − x/(2(x²+1)) − 1/(2(x²+1))`
`= [(x²+1) − x(x−1) − (x−1)] / [2(x−1)(x²+1)]`
`= [x²+1 − x²+x − x+1] / [2(x−1)(x²+1)]`
`= 2 / [2(x−1)(x²+1)]`
`= 1/((x−1)(x²+1))` ✓

## IR and dependency changes

No new IR primitives required. Depends on:

- `coding-adventures-polynomial ≥ 0.4.0` (for `extended_gcd`, `rational_roots`)
- `coding-adventures-symbolic-ir ≥ 0.2.0` (for `ATAN`)
- Phase 2d and 2e already in `symbolic-vm` (RT + arctan)

One refactor: `_rt_pairs_to_ir` (currently private in `integrate.py`) is
lifted to `polynomial_bridge.py` as `rt_pairs_to_ir` so that
`mixed_integral.py` can use it without a circular import.

## Integration into the Integrate handler

The handler chain in `_integrate_rational` gains a fourth step:

    1. Hermite → poly_part, rat_part, log_num, log_den
    2. RT on log part → log terms, or None
    3. Phase 2e (arctan) if RT returned None and deg(log_den)==2, irreducible
    4. **Phase 2f (mixed) if Phase 2e returned None: split into L·Q parts**
    5. Unevaluated Integrate if all above returned None

The progress gate is extended: Phase 2f success counts as "progress".

## Test strategy

| Test class | Cases |
|---|---|
| `TestOneLinearOneQuadratic` | `1/((x−1)(x²+1))`, `x/((x+2)(x²+4))` |
| `TestTwoLinearOneQuadratic` | `1/((x−1)(x+1)(x²+1))` |
| `TestMixedNumerator` | `(x²+1)/((x−2)(x²+2x+5))` |
| `TestBezoutSplitIdentity` | verify `C_Q·L + C_L·Q == num` algebraically |
| `TestFallsThrough` | two quadratics: `1/((x²+1)(x²+4))` → unevaluated |
| `TestEndToEnd` | via full VM Integrate handler |

All tests that produce a result verify via numerical re-differentiation
(same pattern as test_arctan_integral.py).
