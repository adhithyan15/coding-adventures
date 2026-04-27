# Phase 5 — Higher Trig Powers and Tan Integration

## Overview

Phase 5 extends the symbolic integrator to cover the standard textbook
forms that the earlier phases deferred: the `tan` family and integer
powers of `sin`, `cos`, and `tan`.

Three sub-phases, each a clean layer on top of the existing integrator:

- **Phase 5a** — `∫ tan(ax+b) dx` via the log-cosine antiderivative.
- **Phase 5b** — `∫ sinⁿ(ax+b) dx` and `∫ cosⁿ(ax+b) dx` for integer
  `n ≥ 2`, solved by the classical **reduction formula** (one IBP step,
  expressed as a recursion that terminates at Phase 3).
- **Phase 5c** — `∫ tanⁿ(ax+b) dx` for integer `n ≥ 2`, solved by the
  **Pythagorean reduction** `tan² = sec² − 1` (also a recursion
  that terminates at Phase 5a or the trivial constant case).

No new IR primitives are required beyond `TAN = IRSymbol("Tan")`, which
is added to `symbolic_ir 0.3.0`. No new modules are introduced in
`symbolic_vm`; all Phase 5 logic lives as private helpers in
`integrate.py`.

---

## Phase 5a — `∫ tan(ax+b) dx`

### Formula

```
∫ tan(ax+b) dx  =  −log(cos(ax+b)) / a   +  C
```

### Derivation

Write tan as a quotient and use the substitution `u = cos(ax+b)`:

```
∫ tan(ax+b) dx  =  ∫ sin(ax+b) / cos(ax+b) dx

Let u = cos(ax+b),   du = −a · sin(ax+b) dx
⟹  sin(ax+b) dx = −du / a

∫ (−1/a) · du/u  =  −(1/a) · log|u|  +  C
                  =  −log(cos(ax+b)) / a   +  C
```

The absolute value is implicit; we work over the reals, and the CAS
omits it as Mathematica and Maple do by convention.

### IR shape produced

```
Neg(Div(Log(Cos(linear_to_ir(a, b, x))), _frac_ir(a)))
```

### Special case `a = 1, b = 0`

Phase 1's elementary section fires first for bare `tan(x)` (once `TAN`
is in the elementary set), but Phase 5a's helper also handles this as
`a = 1, b = 0` — the two paths agree.

### Edge cases

- `a = 0` (constant argument): `tan(b)` is x-free, handled by the
  constant rule before Phase 5a is reached.
- `n = 1` in the tan-power recursion: the base case calls this formula.

---

## Phase 5b — `∫ sinⁿ(ax+b) dx` and `∫ cosⁿ(ax+b) dx`

### Scope

Handles `POW(SIN(linear_arg), n)` and `POW(COS(linear_arg), n)` for
integer `n ≥ 2`.

- `n = 1` is already closed by Phase 3b/3c.
- `n = 0` evaluates to `1` (constant), handled by the constant rule.

Phase 4b already handles the `MUL(SIN, SIN)` shape via product-to-sum;
Phase 5b handles the `POW(SIN, n)` shape for all `n ≥ 2`.

### Reduction formulas

Derived by integration by parts (IBP) once, then using the Pythagorean
identity to convert the leftover `cos²` (or `sin²`) term.

**Sin reduction:**

```
Let I_n = ∫ sinⁿ(ax+b) dx.

IBP: u = sinⁿ⁻¹(ax+b),  dv = sin(ax+b) dx
     du = (n-1)·a·sinⁿ⁻²(ax+b)·cos(ax+b) dx
     v  = −cos(ax+b)/a

I_n = −sinⁿ⁻¹(ax+b)·cos(ax+b)/a
      + (n−1) · ∫ sinⁿ⁻²(ax+b) · cos²(ax+b) dx

Replace cos² = 1 − sin²:

I_n = −sinⁿ⁻¹(ax+b)·cos(ax+b)/a + (n−1)·I_{n−2} − (n−1)·I_n

n · I_n = −sinⁿ⁻¹(ax+b)·cos(ax+b)/a + (n−1) · I_{n−2}

         −sin^{n−1}(ax+b)·cos(ax+b)     n−1
I_n  =  ─────────────────────────────  +  ─── · I_{n−2}
                    n · a                   n
```

**Cos reduction:**

By the same argument (with `sin → cos`, `−cos dx → sin dx/a`):

```
         cos^{n−1}(ax+b)·sin(ax+b)    n−1
I_n  =  ───────────────────────────  +  ─── · I_{n−2}
                   n · a                  n
```

### Recursion structure

```
sin_power(n, a, b, x)  →  term_n  +  (n−1)/n · sin_power(n−2, a, b, x)
cos_power(n, a, b, x)  →  term_n  +  (n−1)/n · cos_power(n−2, a, b, x)
```

The coefficient `(n−1)/n` is rational, so the recursion stays over Q.

**Base cases:**

| n | sin        | cos        |
|---|-----------|-----------|
| 0 | `x`       | `x`       |
| 1 | `−cos(ax+b)/a` (Phase 3b) | `sin(ax+b)/a` (Phase 3c) |

The recursion always decrements by 2, so `n` reaches 0 (for even) or 1
(for odd). Both bases are handled by earlier phases.

### IR shape produced (sin, example n = 3)

```
n = 3, a = 1, b = 0:

I_3 = −sin²(x)·cos(x)/3  +  (2/3) · I_1
    = −sin²(x)·cos(x)/3  +  (2/3) · (−cos(x))
    = Add(
        Mul(IRRational(-1, 3),
            Mul(Pow(Sin(x), 2), Cos(x))),
        Mul(IRRational(2, 3),
            Neg(Cos(x))))
```

### Worked example: `∫ sin³(x) dx`

```
I_3 = −sin²(x)·cos(x)/3 + (2/3) · ∫ sin(x) dx
    = −sin²(x)·cos(x)/3 + (2/3) · (−cos(x))
    = −sin²(x)·cos(x)/3 − 2cos(x)/3
```

Verification: differentiate:
```
d/dx [−sin²(x)cos(x)/3 − 2cos(x)/3]
  = −(2sin(x)cos²(x) − sin³(x))/3 + 2sin(x)/3
  = −2sin(x)cos²(x)/3 + sin³(x)/3 + 2sin(x)/3
  = sin(x)[−2cos²(x)/3 + sin²(x)/3 + 2/3]
  = sin(x)[−2cos²(x)/3 + (1−cos²(x))/3 + 2/3]
  = sin(x)[−2cos²(x)/3 − cos²(x)/3 + 1/3 + 2/3]
  = sin(x)[−cos²(x) + 1]
  = sin(x)·sin²(x) = sin³(x) ✓
```

### Worked example: `∫ cos⁴(2x) dx`

```
I_4 = cos³(2x)·sin(2x)/(4·2) + (3/4)·I_2
    = cos³(2x)·sin(2x)/8 + (3/4)·I_2

I_2 = cos(2x)·sin(2x)/(2·2) + (1/2)·∫ 1 dx
    = cos(2x)·sin(2x)/4 + x/2

I_4 = cos³(2x)·sin(2x)/8 + (3/4)·[cos(2x)·sin(2x)/4 + x/2]
    = cos³(2x)·sin(2x)/8 + 3cos(2x)·sin(2x)/16 + 3x/8
```

---

## Phase 5c — `∫ tanⁿ(ax+b) dx`

### Scope

Handles `POW(TAN(linear_arg), n)` for integer `n ≥ 2`.

- `n = 1` is Phase 5a (bare tan).
- `n = 0` evaluates to `1` (constant).

### Reduction formula

The key identity: `tan²(u) = sec²(u) − 1`.

```
I_n = ∫ tanⁿ(ax+b) dx
    = ∫ tanⁿ⁻²(ax+b) · tan²(ax+b) dx
    = ∫ tanⁿ⁻²(ax+b) · (sec²(ax+b) − 1) dx
    = ∫ tanⁿ⁻²(ax+b) · sec²(ax+b) dx  −  I_{n−2}
```

For the first integral, let `u = tan(ax+b)`, `du = a·sec²(ax+b) dx`:

```
∫ tanⁿ⁻²(ax+b) · sec²(ax+b) dx
  = (1/a) · ∫ u^{n−2} du
  = (1/a) · u^{n−1}/(n−1)
  = tan^{n−1}(ax+b) / ((n−1)·a)
```

Therefore:

```
I_n = tan^{n−1}(ax+b) / ((n−1)·a)  −  I_{n−2}
```

### Recursion structure

```
tan_power(n, a, b, x) → tan_term(n, a, b, x)  −  tan_power(n−2, a, b, x)
```

**Base cases:**

| n | result                        |
|---|-------------------------------|
| 0 | `x` (constant integral)       |
| 1 | `−log(cos(ax+b))/a` (Phase 5a) |

The recursion decrements by 2, so it always reaches n=0 or n=1.

Note: There is **no `(n−1)/n` damping factor** in the tan recursion —
the subtracted integral appears with a full negative sign. This means
the recursion does not simplify to a closed sum of trig terms; it
expands into a telescoping alternating series. For even `n` it
terminates at `−x`; for odd `n` it terminates at
`−(−log(cos(ax+b))/a)`.

### Worked example: `∫ tan²(x) dx`

```
I_2 = tan(x)/1 − I_0
    = tan(x) − x
```

Verification: d/dx[tan(x) − x] = sec²(x) − 1 = tan²(x) ✓

### Worked example: `∫ tan³(x) dx`

```
I_3 = tan²(x)/2 − I_1
    = tan²(x)/2 − (−log(cos(x)))
    = tan²(x)/2 + log(cos(x))
```

Verification:
```
d/dx[tan²(x)/2 + log(cos(x))]
  = tan(x)·sec²(x) + (−sin(x)/cos(x))
  = tan(x)·sec²(x) − tan(x)
  = tan(x)·(sec²(x) − 1)
  = tan(x)·tan²(x) = tan³(x) ✓
```

---

## Implementation

### symbolic_ir 0.3.0

Add `TAN = IRSymbol("Tan")` to `nodes.py` in the elementary-functions
group alongside `SIN` and `COS`. Export from `__init__.py`.

### macsyma-compiler 0.2.0

Add `"tan": TAN` to `_STANDARD_FUNCTIONS` in `compiler.py`. Import
`TAN` from `symbolic_ir`.

### symbolic_vm 0.10.0

**handlers.py** — Add `tan` handler alongside `sin` and `cos`:

```python
def tan(simplify: bool) -> Handler:
    return _elementary("Tan", math.tan, {0: ZERO}, simplify)
```

Register `TAN.name: tan(simplify)` in the handler table.

**derivative.py** — Add the chain-rule derivative:

```
d/dx tan(u)  =  sec²(u) · u'  =  u' / cos²(u)
```

IR shape: `Mul(Pow(Cos(inner), IRInteger(-2)), _diff(inner, x))` or
equivalently `Div(_diff(inner, x), Pow(Cos(inner), IRInteger(2)))`.

**integrate.py** — All Phase 5 logic is inline (no new module):

1. Import `TAN`.
2. In the `len(f.args) == 1` elementary section, add:
   ```python
   if head == TAN:
       # ∫ tan(ax+b) dx = −log(cos(ax+b)) / a
       # Bare tan(x) case (a=1, b=0) fires first.
       return IRApply(NEG, (IRApply(LOG, (IRApply(COS, (x,)),)),))
   ```
   For the linear-argument branch:
   ```python
   if head == TAN:
       return _tan_integral(a_frac, b_frac, x)
   ```
3. In the `POW` branch, before `return None`, call `_try_trig_power`.
4. New private helpers:
   - `_tan_integral(a, b, x)` — builds `−log(cos(ax+b))/a`.
   - `_try_trig_power(base, exponent, x)` — dispatches on `SIN`/`COS`/`TAN`.
   - `_sin_power(n, a, b, x)` — sin reduction, calls `_integrate`.
   - `_cos_power(n, a, b, x)` — cos reduction, calls `_integrate`.
   - `_tan_power(n, a, b, x)` — tan reduction, calls `_integrate`.

### Dispatch order in the POW branch

```
if head == POW:
    base, exponent = f.args
    # Existing: power rule (x^n) and exponential (a^x)
    ...
    # Phase 5: trig^n for integer n ≥ 2
    result = _try_trig_power(base, exponent, x)
    if result is not None:
        return result
    return None
```

`_try_trig_power` guards:
- `exponent` must be `IRInteger(n)` with `n ≥ 2`.
- `base` must be `IRApply(SIN/COS/TAN, (linear_arg,))`.
- `linear_arg` must satisfy `_try_linear`, yielding `(a, b)` with `a ≠ 0`.

---

## Test strategy

New file `tests/test_phase5.py`, organised in five test classes.

### TestTanIntegral (Phase 5a)

Unit tests on `_tan_integral` and through the handler:

| integrand         | expected (up to additive constant)              |
|-------------------|-------------------------------------------------|
| `tan(x)`          | `−log(cos(x))`                                  |
| `tan(2x)`         | `−log(cos(2x)) / 2`                             |
| `tan(3x+1)`       | `−log(cos(3x+1)) / 3`                           |
| `tan(x/2)`        | `−log(cos(x/2)) · 2`                            |

All verified by numerical re-differentiation at multiple x values.

### TestSinPower (Phase 5b — sin)

| integrand         | expected form                           |
|-------------------|-----------------------------------------|
| `sin²(x)`         | `−sin(x)cos(x)/2 + x/2`                 |
| `sin³(x)`         | `−sin²(x)cos(x)/3 − 2cos(x)/3`          |
| `sin⁴(x)`         | reduction, verified numerically          |
| `sin⁵(x)`         | reduction, verified numerically          |
| `sin²(2x+1)`      | linear-arg case, verified numerically    |
| `sin³(3x)`        | linear-arg case, verified numerically    |

### TestCosPower (Phase 5b — cos)

Symmetric to TestSinPower.

### TestTanPower (Phase 5c)

| integrand | expected                                    |
|-----------|---------------------------------------------|
| `tan²(x)` | `tan(x) − x`                                |
| `tan³(x)` | `tan²(x)/2 + log(cos(x))`                   |
| `tan⁴(x)` | `tan³(x)/3 − tan(x) + x`                   |
| `tan²(2x)`| `tan(2x)/2 − x`                             |
| `tan³(3x+1)` | reduction, verified numerically          |

### TestFallsThrough (guards)

- `sin^(-1)(x)` — stays unevaluated (negative exponent).
- `sin^0(x)` — trivial constant; handled by earlier rules.
- `tan(x^2)` — non-linear arg; stays unevaluated.
- `POW(TAN(x), IRFloat(2.0))` — float exponent; stays unevaluated.

### TestRegressions (end-to-end via VM)

- `integrate(tan(x), x)` via MACSYMA string.
- `integrate(sin(x)^2, x)` via MACSYMA string.
- `integrate(tan(x)^2, x)` via MACSYMA string.
- Re-differentiation roundtrips: `diff(integrate(f, x), x) == f`.

---

## Correctness verification

Each test evaluates the antiderivative `F(x)` numerically at two points
`x₀ = 1.5` and `x₁ = −0.7`, and checks:

```
(F(x₀+h) − F(x₀−h)) / (2h)  ≈  f(x₀)
```

with `h = 1e-7` and combined tolerance `1e-6 + 1e-6·|f(x₀)|`.

For the tan family, avoid `x₀ = π/2 + kπ` (poles). The test points
`1.5` and `−0.7` are safely away from all poles for the standard cases.

---

## Package versions after Phase 5

| Package              | Before | After |
|----------------------|--------|-------|
| symbolic_ir          | 0.2.0  | 0.3.0 |
| macsyma-compiler     | 0.1.0  | 0.2.0 |
| symbolic_vm          | 0.9.0  | 0.10.0 |

`symbolic_vm` declares `coding-adventures-symbolic-ir >= 0.3.0` after
this phase.

---

## What remains deferred

- `u`-substitution (automatic change-of-variable detection).
- `sinⁿ · cosᵐ` for mixed trig powers (requires Weirstrass-style
  half-angle substitution for odd+even mixtures, or beta-function for
  even+even).
- Hyperbolic functions (`sinh`, `cosh`, `tanh`).
- Algebraic extensions: Trager's algorithm for `sqrt` integrands.
- The full Risch decision procedure for towers of logarithms and
  exponentials.
