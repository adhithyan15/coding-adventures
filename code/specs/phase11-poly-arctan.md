# Phase 11 — Polynomial × arctan(linear) Integration

## Motivation

After Phase 9 added `∫ atan(ax+b) dx` (the bare arctan integral via IBP), the
natural extension is `∫ P(x)·atan(ax+b) dx` for any polynomial P.  This family
is currently unevaluated:

```
∫ x·atan(x) dx               P = x,   a = 1, b = 0
∫ x²·atan(2x+1) dx           P = x², a = 2, b = 1
∫ (x³+x)·atan(x) dx          P = x³+x
```

Phase 11 fills this gap with a dedicated IBP formula implemented in a new
module `atan_poly_integral.py` and a thin dispatcher hook in `integrate.py`.

## Scope

### What Phase 11 handles

`∫ P(x)·atan(ax+b) dx` where:
- `P(x)` is a polynomial with rational coefficients (`P ∈ Q[x]`)
- `ax+b` is a non-zero linear argument (`a ∈ Q \\ {0}`, `b ∈ Q`)

Degree of P is unrestricted (the algorithm terminates in O(deg P) arithmetic
operations).

### What Phase 11 does NOT handle

- `∫ atan(g(x))` for non-linear `g` (e.g. `atan(x²)`)
- `∫ atan(ax+b)^n dx` for `n ≥ 2`
- `∫ R(x)·atan(ax+b) dx` for rational (non-polynomial) `R`
- Any integrand where `atan` appears in a non-product position

## Mathematical Algorithm

### IBP decomposition

Apply integration by parts with `u = atan(ax+b)`, `dv = P(x) dx`:

```
du = a / ((ax+b)² + 1) dx
v  = Q(x)  =  ∫ P(x) dx   (polynomial antiderivative, integration constant = 0)
```

Result:

```
∫ P(x)·atan(ax+b) dx  =  Q(x)·atan(ax+b)  −  a · ∫ Q(x)/((ax+b)²+1) dx
```

### Resolving the residual rational integral

Let `D(x) = (ax+b)² + 1 = a²x² + 2abx + (b²+1)`.

`D` is always an irreducible quadratic over Q (discriminant `= (2ab)² − 4a²(b²+1)
= −4a² < 0`).

Polynomial long division:

```
Q(x) = S(x) · D(x) + R(x),    deg R < 2
```

Then:

```
∫ Q(x)/D(x) dx  =  ∫ S(x) dx  +  ∫ R(x)/D(x) dx
               =  T(x)        +  arctan_integral(R, D)
```

- `T(x) = ∫ S(x) dx` — polynomial antiderivative (Phase 1, closed-form).
- `arctan_integral(R, D)` — existing Phase 2e helper; handles the irreducible
  quadratic denominator with linear-or-constant numerator.

### Final formula

```
∫ P(x)·atan(ax+b) dx
    =  Q(x) · atan(ax+b)
     − a · T(x)
     − a · arctan_integral(R, D)
```

All three terms involve only `Fraction` arithmetic; the result is always a
closed-form IR tree.

### Special case: P = 1 (bare arctan)

For `P = (1,)`, `Q = (0, 1) = x`, `Q mod D = Q` (since `deg Q = 1 < 2`),
`S = 0`, `T = 0`. The formula reduces to:

```
x · atan(ax+b)  −  a · arctan_integral((0, 1), D)
```

where `arctan_integral((0,1), (b²+1, 2ab, a²))` integrates `x/((ax+b)²+1)`:

```
= (1/(2a²)) · log((ax+b)²+1)  −  (b/a²) · atan(ax+b)
```

Substituting: `(x + b/a)·atan(ax+b) − (1/(2a))·log((ax+b)²+1)`, which
matches the Phase 9 inline formula exactly — consistency verified.

## New Code

### `atan_poly_integral.py` (new module)

```
atan_poly_integral(poly, a, b, x_sym) → IRNode
```

- `poly`: ascending `Fraction` coefficient tuple for P(x)
- `a, b`: `Fraction` coefficients of the linear argument (a ≠ 0)
- `x_sym`: `IRSymbol` for the integration variable

Internally calls `arctan_integral` from `arctan_integral.py` and
`from_polynomial` from `polynomial_bridge.py`.

### Hook in `integrate.py`

```python
# Phase 11: atan(linear) × polynomial via IBP.
result = _try_atan_product(a, b, x) or _try_atan_product(b, a, x)
if result is not None:
    return result
```

Inserted in the MUL handler after the `_try_log_product` call (Phase 3e),
before Phase 4b (trig × trig). The dispatcher `_try_atan_product(atan_node,
poly_candidate, x)` mirrors the structure of `_try_log_product`:

1. Check `atan_node.head == ATAN`.
2. Extract linear arg: `_try_linear(atan_node.args[0], x)` → `(a, b)`.
3. Check `poly_candidate` is a polynomial via `to_rational` + zero denominator test.
4. Call `atan_poly_integral(poly, a, b, x)`.

## Interaction with Earlier Phases

| Integrand | Route |
|-----------|-------|
| `atan(ax+b)` | Phase 9 bonus (inline IBP, `_integrate` dispatch) |
| `c·atan(ax+b)` | Constant-factor rule → Phase 9 bonus |
| `P(x)·atan(ax+b)` | **Phase 11** (deg P ≥ 1) |
| `P(x)·atan(g(x))`, g non-linear | Unevaluated |

## Limitations and Future Work

- Non-linear arctan arguments (`atan(x²)`, `atan(sin(x))`) remain unevaluated.
- `atan^n` for n ≥ 2 remains unevaluated.
- `R(x)·atan(ax+b)` for rational (non-polynomial) R remains unevaluated.

## Files Changed

| File | Change |
|------|--------|
| `code/specs/phase11-poly-arctan.md` | **NEW** — this document |
| `code/packages/python/symbolic-vm/src/symbolic_vm/atan_poly_integral.py` | **NEW** |
| `code/packages/python/symbolic-vm/src/symbolic_vm/integrate.py` | MODIFY — hook |
| `code/packages/python/symbolic-vm/tests/test_phase11.py` | **NEW** — ≥ 40 tests |
| `code/packages/python/symbolic-vm/CHANGELOG.md` | MODIFY — 0.16.0 entry |
| `code/packages/python/symbolic-vm/pyproject.toml` | MODIFY — version 0.16.0 |
