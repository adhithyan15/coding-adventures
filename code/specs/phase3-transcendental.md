# Phase 3 — Transcendental Integration

## Status

Phase 3 of the symbolic integration roadmap. Extends the integrator
beyond rational functions to handle the most common transcendental
integrands: products of polynomials with a single exponential or
logarithm of a **linear** argument, plus bare sin/cos/exp of a linear
argument. After Phase 3, the only integrands still unevaluated are
those that genuinely require algebraic extensions (Phase 4) or the full
Risch differential-equation machinery for rational × transcendental
products (a future Phase 3 extension).

## Scope

### What this phase handles

All five cases share the same restriction: the transcendental argument
is a **linear** polynomial `a·x + b` with `a ∈ Q \ {0}`, `b ∈ Q`.

| Case | Integrand | Antiderivative |
|------|-----------|----------------|
| 3a | `exp(a·x + b)` | `exp(a·x + b) / a` |
| 3b | `sin(a·x + b)` | `−cos(a·x + b) / a` |
| 3c | `cos(a·x + b)` | `sin(a·x + b) / a` |
| 3d | `p(x) · exp(a·x + b)` | `g(x) · exp(a·x + b)` (Risch DE) |
| 3e | `p(x) · log(a·x + b)` | `[P(x) − r/a] · log(a·x+b) − S(x)` (IBP) |

For cases 3a–3c the special case `a = 1, b = 0` was already handled by
Phase 1 (`∫ exp(x) = exp(x)`, `∫ sin(x) = −cos(x)`, etc.). Phase 3
generalises these to any linear argument.

### What this phase does NOT handle

- **Rational × transcendental**: `∫ (1/x)·eˣ dx`, `∫ r(x)·eˣ dx` for
  general rational `r`. The Risch DE over the rational field requires
  partial-fraction decomposition of `r` and yields a decision procedure
  for non-elementarity. Deferred to a future Phase 3 extension.
- **Non-linear transcendental argument**: `∫ exp(x²) dx` (Gaussian,
  non-elementary), `∫ exp(1/x) dx`.
- **Products of two transcendentals**: `∫ exp(x)·log(x) dx`,
  `∫ sin(x)·eˣ dx`. These require integration by parts in two
  variables or complex Risch extensions.
- **Nested towers**: `∫ exp(exp(x)) dx`. Phase 4 territory.
- **Trig × polynomial**: `∫ x·sin(x) dx` etc. Phase 1 currently handles
  only bare `sin(x)` and `cos(x)`; extending to polynomial × trig is
  straightforward with the same IBP mechanism but is out of scope here
  to keep the phase focused.

## Algorithm — Case 3d: Polynomial × Exp (Risch DE)

### Overview

Given `∫ p(x)·exp(a·x + b) dx` with `p ∈ Q[x]` and `a ∈ Q \ {0}`:

Seek `g ∈ Q[x]` such that `d/dx[g(x)·exp(a·x + b)] = p(x)·exp(a·x + b)`.

Differentiation gives `[g′(x) + a·g(x)]·exp(a·x + b)`, so we need
to solve the **Risch differential equation** in `Q[x]`:

    g′(x) + a·g(x) = p(x)

### Why `g` must be a polynomial of the same degree

If `p` has degree `n`, then `a·g` contributes the leading term of `g′ + a·g`,
which means `g` must also have degree `n` (if `a ≠ 0`). The leading
coefficient of `g′` has degree `n−1`, so it does not affect the degree-`n`
equation.

### Step-by-step coefficient recovery

Write `p(x) = Σᵢ pᵢ·xⁱ` and `g(x) = Σᵢ gᵢ·xⁱ` (both degree `n`).

From `g′ + a·g = p`, matching `xⁿ`:

    a·gₙ = pₙ  →  gₙ = pₙ / a

Matching `xᵏ` for `k = n−1, n−2, ..., 0`:

    (k+1)·gₖ₊₁ + a·gₖ = pₖ  →  gₖ = (pₖ − (k+1)·gₖ₊₁) / a

This is a clean **back-substitution** over `Q` — no GCDs, no resultants.

### Worked example

`∫ x²·eˣ dx`:

- `p = (0, 0, 1)` (representing `x²`), `a = 1`, `b = 0`, `n = 2`
- `g₂ = p₂ / 1 = 1`
- `g₁ = (p₁ − 2·g₂) / 1 = 0 − 2 = −2`
- `g₀ = (p₀ − 1·g₁) / 1 = 0 − (−2) = 2`
- Result: `g(x) = x² − 2x + 2`
- Antiderivative: `(x² − 2x + 2)·eˣ`

Verification: `d/dx[(x²−2x+2)eˣ] = (2x−2)eˣ + (x²−2x+2)eˣ = x²eˣ` ✓

### Special case: bare `exp(a·x + b)` (constant polynomial `p = 1`)

`g₀ = 1/a`, giving antiderivative `(1/a)·exp(a·x + b)`. This unifies
case 3a with case 3d.

---

## Algorithm — Case 3e: Polynomial × Log (Integration by Parts)

### Overview

Given `∫ p(x)·log(a·x + b) dx` with `p ∈ Q[x]` and `a ∈ Q \ {0}`:

Use the IBP identity `d/dx[P(x)·log(a·x+b)] = p(x)·log(a·x+b) + P(x)·a/(a·x+b)`:

    ∫ p(x)·log(a·x+b) dx  =  P(x)·log(a·x+b)  −  ∫ P(x)·a/(a·x+b) dx

where `P(x) = ∫ p(x) dx` (polynomial antiderivative via power rule).

### Reducing the residual integral

`P(x)·a/(a·x+b)` is a rational function (polynomial over linear). Apply
polynomial long division:

    P(x)·a = Q(x)·(a·x + b) + r       (r ∈ Q, the remainder)

The remainder is `r = P(−b/a)·a` (the value of `P(x)·a` at the root of
the denominator). Then:

    ∫ P(x)·a/(a·x+b) dx  =  ∫ Q(x) dx  +  r · ∫ 1/(a·x+b) dx
                          =  S(x)  +  (r/a)·log(a·x+b)

where `S(x) = ∫ Q(x) dx` is a polynomial.

### Assembling the result

    ∫ p(x)·log(a·x+b) dx  =  [P(x) − r/a]·log(a·x+b) − S(x)

All arithmetic is over `Q[x]` — no transcendental sub-integrals, no
recursion into Phase 2. The log coefficient `P(x) − r/a` is a polynomial
in `x` (specifically, `r/a = P(−b/a)` is a constant, so we're subtracting
a scalar from a polynomial).

### Correctness: why the log coefficient is P(x) − P(−b/a)

From the definition `r = P(−b/a)·a` and `r/a = P(−b/a)`:

    P(x) − r/a  =  P(x) − P(−b/a)

This polynomial vanishes at `x = −b/a` — which makes sense because the
antiderivative must be well-defined there (the log is undefined at `−b/a`
and would blow up unless the coefficient goes to zero at the same point).

### Worked examples

**Example 1**: `∫ x·log(x) dx`

- `p = (0, 1)`, `P = (0, 0, 1/2)` (i.e., `x²/2`), `a = 1`, `b = 0`
- `P(x)·a = x²/2`, divide by `x`: `Q = x/2`, remainder `r = 0`
- `S = ∫ (x/2) dx = x²/4`
- Result: `(x²/2 − 0)·log(x) − x²/4 = (x²/2)·log(x) − x²/4`
- Verify: `d/dx = x·log(x) + (x²/2)·(1/x) − x/2 = x·log(x) + x/2 − x/2 = x·log(x)` ✓

**Example 2**: `∫ log(2x + 1) dx`

- `p = (1,)` (constant `1`), `P = (0, 1)` (i.e., `x`), `a = 2`, `b = 1`
- `P(x)·a = 2x`, divide by `(2x+1)`: `Q = 1`, remainder `r = −1`
  (check: `2x = 1·(2x+1) + (−1)`)
- `S = ∫ 1 dx = x`
- `r/a = −1/2`
- Result: `(x − (−1/2))·log(2x+1) − x = (x + 1/2)·log(2x+1) − x`
- Verify: `d/dx = log(2x+1) + (x+1/2)·2/(2x+1) − 1 = log(2x+1) + 1 − 1 = log(2x+1)` ✓

**Example 3** (shows the cancellation): `∫ log(x) dx`

- `p = (1,)`, `P = (0, 1)` (i.e., `x`), `a = 1`, `b = 0`
- `P(x)·a = x`, divide by `x`: `Q = 1`, `r = 0`
- `S = ∫ 1 dx = x`
- Result: `x·log(x) − x` — matches the hard-coded Phase 1 result. ✓

---

## Recognition in the IR

### Detecting a linear argument

A helper `_try_linear(node, x) → (a: Fraction, b: Fraction) | None`
walks the IR and returns `(a, b)` if `node` represents `a·x + b`, else
`None`. Cases handled:

| IR shape | a | b |
|----------|---|---|
| `IRSymbol(x)` | `1` | `0` |
| `IRInteger(n)` | `0` | `n` |
| `IRRational(p,q)` | `0` | `p/q` |
| `Neg(v)` | `−aᵥ` | `−bᵥ` |
| `Mul(c, x)` with c free of x | `c` | `0` |
| `Mul(x, c)` with c free of x | `c` | `0` |
| `Add(u, v)` | `aᵤ+aᵥ` | `bᵤ+bᵥ` |
| `Sub(u, v)` | `aᵤ−aᵥ` | `bᵤ−bᵥ` |

Returns `None` if the expression has a quadratic or higher term, or if
it contains any non-rational constant or non-x symbol.

### Splitting a product into polynomial × transcendental

For a `Mul(f₁, f₂)` node, the recogniser checks both orderings:
- Is one factor `Exp(linear)` and the other `to_rational(·, x)` with
  denominator 1 (i.e., it's a polynomial)?
- Is one factor `Log(linear)` and the other a polynomial?

Both `Exp` and `Log` are single-argument IR nodes, so the check is:
`f.head ∈ {EXP, LOG} and _try_linear(f.args[0], x) is not None`.

### Where Phase 3 fires in `_integrate`

Phase 3 is wired into the existing `_integrate` function at two points:

1. **Elementary function section** (currently handles `head(x)` only):
   Extended to call `_try_linear` on the argument. If linear with `a ≠ 1`
   or `b ≠ 0`, applies the case 3a/3b/3c formula.

2. **`MUL` branch** (currently bails when both factors depend on `x`):
   Before returning `None`, tries `_try_exp_product` and `_try_log_product`
   to handle cases 3d and 3e.

No new "route" is needed (unlike Phase 2's rational route); Phase 3 extends
Phase 1's existing pattern table.

---

## New modules

### `symbolic_vm/exp_integral.py`

```
exp_integral(poly: Polynomial, a: Fraction, b: Fraction, x: IRSymbol) → IRNode
```

Implements case 3d (and 3a as the `poly = (1,)` degenerate). Given a
polynomial `p` (as a `Polynomial` tuple) and the linear-arg coefficients
`a, b`, returns the IR for `g(x)·exp(a·x + b)` where `g` solves
`g′ + a·g = p`.

```
_solve_risch_de_poly(p: Polynomial, a: Fraction) → Polynomial
```

Private helper: computes `g` from `p` and `a` by back-substitution.
Returns the `Polynomial` tuple for `g`.

### `symbolic_vm/log_integral.py`

```
log_poly_integral(poly: Polynomial, a: Fraction, b: Fraction, x: IRSymbol) → IRNode
```

Implements case 3e (and case `p = 1, a = 1, b = 0` = bare `log(x)` as
handled by Phase 1, extended to linear argument). Computes
`[P − r/a]·log(a·x+b) − S` entirely in `Q[x]`, emitting the result as IR.

---

## Integration into the Integrate handler

The `_integrate` function in `integrate.py` gains the following additions:

```
# Case 3a/3b/3c: elementary function of linear argument
if head in {EXP, SIN, COS} and len(f.args) == 1:
    lin = _try_linear(f.args[0], x)
    if lin is not None:
        a, b = lin
        if a != 0:
            # exp(ax+b)/a, -cos(ax+b)/a, sin(ax+b)/a
            ...

# Cases 3d/3e: product with both factors depending on x
if head == MUL:
    ...
    # (after the constant-factor checks fail)
    result = _try_exp_product(f.args[0], f.args[1], x)
    if result is None:
        result = _try_exp_product(f.args[1], f.args[0], x)
    if result is not None:
        return result
    result = _try_log_product(f.args[0], f.args[1], x)
    if result is None:
        result = _try_log_product(f.args[1], f.args[0], x)
    return result  # None if both fail → unevaluated
```

---

## Dependency changes

No new IR primitives required. All five cases use existing IR nodes
(`EXP`, `LOG`, `SIN`, `COS`, `MUL`, `ADD`, `SUB`, `DIV`, `NEG`).

Depends on:
- `coding-adventures-symbolic-ir ≥ 0.2.0` (already the minimum)
- `coding-adventures-polynomial ≥ 0.4.0` (for `divmod_poly`,
  `multiply`, `normalize`; already the minimum)

---

## Test strategy

| Test class | Cases |
|---|---|
| `TestExpIntegral` | `exp(2x)`, `exp(3x+1)`, `x·eˣ`, `x²·eˣ`, `(x²+2x+3)·e^(−x)` |
| `TestLogIntegral` | `log(x)` (regression), `x·log(x)`, `x²·log(x)`, `log(2x+1)`, `x·log(2x+1)` |
| `TestLinearArgTrig` | `sin(2x)`, `cos(3x)`, `sin(x+1)`, `cos(2x+π/3)` |
| `TestFallsThrough` | `(1/x)·eˣ` → unevaluated, `exp(x²)` → unevaluated, `exp(x)·log(x)` → unevaluated |
| `TestEndToEnd` | via full VM: `(x+1)·eˣ`, `x²·log(x)`, `sin(2x)·3` |

All correctness tests use numerical re-differentiation (same pattern as
`test_mixed_integral.py`).

Regression tests verify that Phase 1 cases (`∫ exp(x)`, `∫ log(x)`,
`∫ sin(x)`, `∫ cos(x)`) still produce the same output after Phase 3 is
wired in.
