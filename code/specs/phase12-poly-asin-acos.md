# Phase 12 — Polynomial × asin/acos Integration via IBP

## Goal

Add closed-form integration for `∫ P(x) · asin(ax+b) dx` and
`∫ P(x) · acos(ax+b) dx`, where `P ∈ Q[x]` and `a ∈ Q \ {0}`.

This completes all three inverse-trig × polynomial families (Phase 2e gave bare arctan; Phase 11 gave poly × arctan; Phase 12 gives poly × asin and poly × acos).

---

## Mathematical Derivation

### asin IBP

Integration by parts with `u = asin(ax+b)`, `dv = P(x) dx`:

```
du = a / √(1 − (ax+b)²) dx
v  = Q(x) = ∫ P(x) dx     (polynomial antiderivative)
```

Applying IBP:

```
∫ P(x)·asin(ax+b) dx = Q(x)·asin(ax+b) − a · ∫ Q(x)/√(1−(ax+b)²) dx
```

### Resolving the Residual

Substitute `t = ax+b`, so `x = (t−b)/a`, `dx = dt/a`:

```
a · ∫ Q(x)/√(1−(ax+b)²) dx = ∫ Q((t−b)/a)/√(1−t²) dt = ∫ Q̃(t)/√(1−t²) dt
```

where `Q̃(t) = Q((t−b)/a)` is a polynomial of the same degree in `t`.

By linearity, each monomial `tⁿ/√(1−t²)` contributes independently. The reduction formula is:

```
∫ tⁿ/√(1−t²) dt = −tⁿ⁻¹/n · √(1−t²) + (n−1)/n · ∫ tⁿ⁻²/√(1−t²) dt

Base cases:
  n = 0 → asin(t)
  n = 1 → −√(1−t²)
```

Applying this to `Q̃(t)` by linearity gives:

```
∫ Q̃(t)/√(1−t²) dt = A(t) · √(1−t²) + B(t) · asin(t)
```

where `A(t)` and `B(t)` are polynomials over Q.

### asin Final Result

Back-substituting `t = ax+b`:

```
∫ P(x) · asin(ax+b) dx
    = Q(x) · asin(ax+b) − [A(ax+b) · √(1−(ax+b)²) + B(ax+b) · asin(ax+b)]
    = [Q(x) − B(ax+b)] · asin(ax+b) − A(ax+b) · √(1−(ax+b)²)
```

### acos IBP

For `u = acos(ax+b)`:  `du = −a/√(1−(ax+b)²) dx` — the sign flips.

```
∫ P(x) · acos(ax+b) dx = Q(x) · acos(ax+b) + a · ∫ Q(x)/√(1−(ax+b)²) dx
```

The residual is identical to the asin case (same `Q`, same sign change accounts for `−du` → `+`). After back-substitution:

```
∫ P(x) · acos(ax+b) dx
    = Q(x) · acos(ax+b) + A(ax+b) · √(1−(ax+b)²) + B(ax+b) · asin(ax+b)
```

**Note:** The `B · asin` term is non-zero for `deg(P) ≥ 1`. This is mathematically correct — `∫ acos(x) dx` produces no `asin` term (B=0), but `∫ x·acos(x) dx = x²/2·acos(x) − x/4·√(1−x²) + 1/4·asin(x)` does.

---

## Worked Examples

### ∫ asin(x) dx

`P = 1`, `a = 1`, `b = 0`. `Q(x) = x`.

`Q̃(t) = t`.  `_sqrt_integral_decompose((0, 1))`:
- `n=1`: `A = (−1,)`, `B = ()`.

Back-substitute (identity, `a=1`, `b=0`): `A_x = (−1,)`, `B_x = ()`.

Result: `[x − 0] · asin(x) − (−1) · √(1−x²) = x·asin(x) + √(1−x²)` ✓

### ∫ x·asin(x) dx

`P = (0, 1)`, `Q = x²/2`.  `Q̃(t) = t²/2`.

`_sqrt_integral_decompose((0, 0, 1/2))`:
- `n=2`: `A_new = (0, −1/2)`, then add `(1/2)·A_rec = (1/2)·() = ()`.
  So `A(t) = (1/2)·(0, −1/2) = (0, −1/4)`.
  `B_t = (1/2)·(1/2,) = (1/4,)`.

Back-substitute: `A_x = (0, −1/4)` → `−x/4`, `B_x = (1/4,)` → `1/4`.

Result: `(x²/2 − 1/4)·asin(x) − (−x/4)·√(1−x²) = (x²/2−1/4)·asin(x) + x/4·√(1−x²)` ✓

### ∫ acos(x) dx

Same `Q`, `A_x`, `B_x` as `∫ asin(x) dx`. `B_x = ()`, so no asin term.

Result: `x·acos(x) + (−1)·√(1−x²) = x·acos(x) − √(1−x²)` ✓

### ∫ x·acos(x) dx

`A_x = (0, −1/4)`, `B_x = (1/4,)`.

Result: `x²/2·acos(x) + (−x/4)·√(1−x²) + 1/4·asin(x)` ✓

---

## Implementation

### symbolic-ir 0.4.0

Added to `nodes.py` (elementary-functions group, after ATAN):
```python
ASIN = IRSymbol("Asin")
ACOS = IRSymbol("Acos")
```

Exported from `__init__.py`.

### symbolic-vm 0.17.0

**`handlers.py`** — two new `_elementary` handlers:
```python
def asin(simplify): return _elementary("Asin", math.asin, {0: ZERO}, simplify)
def acos(simplify): return _elementary("Acos", math.acos, {}, simplify)
```
Registered in `build_handler_table` as `ASIN.name` and `ACOS.name`.

**`asin_poly_integral.py`** — new module implementing:
- `asin_poly_integral(poly, a, b, x_sym)` — IBP formula for asin family.
- `acos_poly_integral(poly, a, b, x_sym)` — IBP formula for acos family.
- Private: `_compose_to_t`, `_sqrt_integral_decompose` (with memoization), `_poly_compose_linear`, `_compute_AB`, `_integrate_poly`, `_poly_add`, `_poly_mul`, `_poly_scale`, `_normalize`.

**`integrate.py`** — wired up as:
- ASIN/ACOS added to the elementary-function dispatch set.
- Bare `asin/acos(linear)` handled by calling `asin_poly_integral`/`acos_poly_integral` with `poly = (1,)`.
- `_try_asin_product` / `_try_acos_product` dispatcher functions inserted after Phase 11.
- `d/dx asin(u) = u'/√(1−u²)` and `d/dx acos(u) = −u'/√(1−u²)` added to `_diff_ir`.

**`macsyma_compiler/compiler.py`** — added `"asin": ASIN` and `"acos": ACOS` to the standard function table so the Macsyma string interface recognizes `asin(...)` and `acos(...)`.

---

## Test Coverage

`tests/test_phase12.py` — 43 tests across 6 classes:

| Class | Count | Description |
|-------|-------|-------------|
| `TestPhase12_AsinCanonical` | 12 | `∫ xⁿ·asin(x) dx` (n=0..5), combinations, commutativity, constant factor |
| `TestPhase12_AcosCanonical` | 8 | `∫ xⁿ·acos(x) dx` (n=0..4), combinations, commutativity, constant factor |
| `TestPhase12_LinearArg` | 8 | `a≠1` or `b≠0` cases for both asin and acos |
| `TestPhase12_Fallthrough` | 4 | Non-linear args, powers of asin, rational factor |
| `TestPhase12_Regressions` | 6 | Phases 11, 3e, 4a, 1, 2e — earlier phases unaffected |
| `TestPhase12_Macsyma` | 5 | End-to-end via Macsyma string interface |

All correctness tests use numerical finite-difference verification: `F'(x) ≈ f(x)` at test points `(0.3, 0.6)` safely within `|ax+b| < 1`.
