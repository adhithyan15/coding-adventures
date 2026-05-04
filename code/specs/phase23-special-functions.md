# Phase 23 — Special Functions as Integration Fallback

> **Status**: Implementation complete.
> **PR**: TBD
>
> **Versions**: `symbolic-ir` 0.11.0, `symbolic-vm` 0.43.0,
> `macsyma-runtime` 1.14.0

---

## Motivation

The Risch integration algorithm implemented in Phases 1–17 handles all
elementary integrals: rational functions, algebraic functions, and
transcendental elementary combinations. But many natural integrands —
`exp(-x²)`, `sin(x)/x`, `log(x)/(1-x)` — have antiderivatives that are
provably *not* elementary. Historical MACSYMA (1982–1994) returned answers
in terms of well-known named special functions rather than leaving such
integrals unevaluated. Phase 23 adds that fallback layer.

---

## Scope

Five families of special functions are introduced:

| Family | Functions | MACSYMA names |
|--------|-----------|---------------|
| Error functions | erf, erfc, erfi | `erf`, `erfc`, `erfi` |
| Trigonometric integrals | Si, Ci, Shi, Chi | `si`, `ci`, `shi`, `chi` |
| Dilogarithm | Li₂ | `li[2]` / `li2` |
| Gamma / Beta | Γ(z), B(a,b) | `gamma`, `beta` |
| Fresnel integrals | S(x), C(x) | `fresnel_s`, `fresnel_c` |

Each family contributes:
1. **IR head(s)** in `symbolic-ir 0.11.0`
2. **Integration patterns** recognizing when `∫ f dx` reduces to a
   special-function form
3. **Differentiation rules** so `diff(erf(x), x)` evaluates symbolically
4. **Numeric evaluation** at concrete numeric arguments
5. **MACSYMA surface syntax** in `macsyma-runtime 1.14.0`

---

## 23a — Error Functions

### Definitions

```
erf(x)  = (2/√π) ∫₀^x exp(-t²) dt          [error function]
erfc(x) = 1 - erf(x)                         [complementary error function]
erfi(x) = (2/√π) ∫₀^x exp(t²) dt            [imaginary error function]
```

### Integration patterns

```
∫ exp(-x²) dx        = √π/2 · erf(x)
∫ exp(x²) dx         = √π/2 · erfi(x)
∫ exp(-a²x²) dx      = √π/(2a) · erf(ax)    (a rational ≠ 0)
∫ exp(a²x²) dx       = √π/(2a) · erfi(ax)
∫ exp(-ax²+bx+c) dx  → complete the square, reduce to erf form
```

Recognition: integrand = `exp(p(x))` where `p(x)` is a quadratic in `x`
with *negative* (resp. positive) leading coefficient.

### Differentiation rules

```
d/dx erf(f(x))  = (2/√π) · exp(-f²) · f'
d/dx erfc(f(x)) = -(2/√π) · exp(-f²) · f'
d/dx erfi(f(x)) = (2/√π) · exp(f²) · f'
```

### Special values

```
erf(0) = 0,  erf(∞) = 1,  erf(-x) = -erf(x)
erfc(0) = 1, erfc(∞) = 0
erfi(0) = 0
```

### Numeric evaluation

For floating-point arguments: use the continued-fraction / series expansion
(pure Python implementation, no external CAS library required).

---

## 23b — Trigonometric Integrals

### Definitions

```
Si(x)  = ∫₀^x sin(t)/t dt                   [sine integral]
Ci(x)  = -∫ₓ^∞ cos(t)/t dt                  [cosine integral]
       = γ + log(x) + ∫₀^x (cos(t)-1)/t dt
Shi(x) = ∫₀^x sinh(t)/t dt                  [hyperbolic sine integral]
Chi(x) = γ + log(x) + ∫₀^x (cosh(t)-1)/t dt [hyperbolic cosine integral]
```

where γ = 0.5772… is the Euler–Mascheroni constant.

### Integration patterns

```
∫ sin(x)/x dx         = Si(x)
∫ sin(ax)/x dx        = Si(ax)          (a rational ≠ 0, by subst)
∫ cos(x)/x dx         = Ci(x)
∫ cos(ax)/x dx        = Ci(ax)
∫ sinh(x)/x dx        = Shi(x)
∫ cosh(x)/x dx        = Chi(x)
```

### Differentiation rules

```
d/dx Si(f(x))  = sin(f(x))/f(x) · f'
d/dx Ci(f(x))  = cos(f(x))/f(x) · f'
d/dx Shi(f(x)) = sinh(f(x))/f(x) · f'
d/dx Chi(f(x)) = cosh(f(x))/f(x) · f'
```

---

## 23c — Dilogarithm Li₂

### Definition

```
Li₂(z) = -∫₀^z log(1-t)/t dt = Σₖ₌₁^∞ zᵏ/k²  (|z| ≤ 1)
```

### Integration patterns

```
∫ log(1-x)/x dx   = -Li₂(x)
∫ log(x)/(1-x) dx = -Li₂(1-x)
```

### Differentiation rule

```
d/dx Li₂(f(x)) = -log(1-f(x))/f(x) · f'
```

### Special values

```
Li₂(0) = 0
Li₂(1) = π²/6     [Basel problem]
Li₂(-1) = -π²/12
Li₂(1/2) = π²/12 - log(2)²/2
```

---

## 23d — Gamma and Beta Functions

### Definitions

```
Γ(n) = (n-1)!             for positive integer n
Γ(n+1) = n · Γ(n)         (recurrence)
Γ(1/2) = √π
B(a,b) = Γ(a)·Γ(b)/Γ(a+b) (Beta function)
```

### Evaluation rules

Integer arguments: `Γ(n) = (n-1)!` exactly.
Half-integer arguments: reduce via `Γ(n+1/2) = (2n-1)!!/(2^n) · √π`.
Other rational arguments: return unevaluated `Gamma(a/b)`.
Float arguments: Lanczos approximation (g=7, 9 coefficients).

```
Beta(a, b) = Gamma(a) * Gamma(b) / Gamma(a+b)
```

### MACSYMA surface

```
gamma(5);               /* 24 */
gamma(1/2);             /* sqrt(%pi) */
gamma(3/2);             /* sqrt(%pi)/2 */
beta(1/2, 1/2);         /* %pi */
beta(2, 3);             /* 1/12 */
```

---

## 23e — Fresnel Integrals

### Definitions

```
FresnelS(x) = ∫₀^x sin(π·t²/2) dt
FresnelC(x) = ∫₀^x cos(π·t²/2) dt
```

### Integration patterns

```
∫ sin(π·x²/2) dx = FresnelS(x)
∫ cos(π·x²/2) dx = FresnelC(x)
∫ sin(a·x²) dx   = √(π/(8a)) · FresnelS(x·√(2a/π))   (a > 0 rational)
∫ cos(a·x²) dx   = √(π/(8a)) · FresnelC(x·√(2a/π))
```

### Differentiation rules

```
d/dx FresnelS(f) = sin(π·f²/2) · f'
d/dx FresnelC(f) = cos(π·f²/2) · f'
```

---

## New IR Heads

All added to `symbolic-ir 0.11.0` and exported from `__init__.py`.

```python
# Error functions (Phase 23a)
ERF         = IRSymbol("Erf")
ERFC        = IRSymbol("Erfc")
ERFI        = IRSymbol("Erfi")

# Trigonometric integrals (Phase 23b)
SI          = IRSymbol("Si")
CI          = IRSymbol("Ci")
SHI         = IRSymbol("Shi")
CHI         = IRSymbol("Chi")

# Dilogarithm (Phase 23c)
LI2         = IRSymbol("Li2")

# Gamma / Beta (Phase 23d)
GAMMA_FUNC  = IRSymbol("GammaFunc")
BETA_FUNC   = IRSymbol("BetaFunc")

# Fresnel integrals (Phase 23e)
FRESNEL_S   = IRSymbol("FresnelS")
FRESNEL_C   = IRSymbol("FresnelC")
```

---

## Implementation Structure

### New file: `symbolic-vm/src/symbolic_vm/special_functions.py`

Contains:

1. **`_try_erf_integral(expr, x)`** — returns `IRNode | None`
   Pattern: `expr = IRApply(EXP, (quadratic,))` where quadratic is
   quadratic in `x` with rational coefficients.
   - Discriminant sign determines erf vs erfi.
   - Completes the square, maps to `erf(a*x + b) * coeff`.

2. **`_try_si_ci_integral(expr, x)`** — returns `IRNode | None`
   Pattern: `expr = IRApply(DIV, (trig_or_hyp_linear, linear_of_x))`.

3. **`_try_li2_integral(expr, x)`** — returns `IRNode | None`
   Patterns: `log(1-x)/x` and `log(x)/(1-x)`.

4. **`_try_fresnel_integral(expr, x)`** — returns `IRNode | None`
   Pattern: `sin/cos(a*x²)` where `a` is a rational multiple of `π/2`.

5. **Differentiation rules** — `diff_erf`, `diff_erfc`, `diff_erfi`,
   `diff_si`, `diff_ci`, `diff_shi`, `diff_chi`, `diff_li2`,
   `diff_fresnel_s`, `diff_fresnel_c`.

6. **Numeric evaluation** — `eval_gamma`, `eval_beta`, `eval_erf_numeric`,
   `eval_si_numeric`, etc. (used by VM handlers for float arguments).

### Changes to `integrate.py`

After all Risch / IBP / hyp-power dispatch attempts, add a final
"special function fallback" block:

```python
# Phase 23: special-function fallback
for _try_fn in (
    _try_erf_integral,
    _try_si_ci_integral,
    _try_li2_integral,
    _try_fresnel_integral,
):
    _result = _try_fn(integrand, x)
    if _result is not None:
        return _result
```

### Changes to `diff.py`

Add a dispatch table `_SPECIAL_DIFF_TABLE` mapping head symbols to
chain-rule handlers. Called from the existing `_diff_apply` dispatcher.

### Changes to `cas_handlers.py`

Add handlers registered in `build_cas_handler_table()`:

| Head | Handler | Action |
|------|---------|--------|
| `GammaFunc` | `gamma_handler` | Exact integer/half-int; Lanczos for float |
| `BetaFunc` | `beta_handler` | Reduce via Gamma ratio |
| `Erf` | `erf_handler` | erf(0)=0; float eval via series; else unevaluated |
| `Erfc` | `erfc_handler` | 1-erf(x); numeric; else unevaluated |
| `Erfi` | `erfi_handler` | erfi(0)=0; numeric; else unevaluated |
| `Si` | `si_handler` | Si(0)=0; numeric; else unevaluated |
| `Ci` | `ci_handler` | numeric; else unevaluated |
| `Li2` | `li2_handler` | Li₂(0)=0, Li₂(1)=π²/6; numeric; else unevaluated |
| `FresnelS` | `fresnel_s_handler` | FresnelS(0)=0; numeric; else unevaluated |
| `FresnelC` | `fresnel_c_handler` | FresnelC(0)=0; numeric; else unevaluated |

---

## MACSYMA Surface Syntax

### `name_table.py` additions

```python
# Special functions (Phase 23)
"erf": ERF,
"erfc": ERFC,
"erfi": ERFI,
"si": SI,
"ci": CI,
"shi": SHI,
"chi": CHI,
"li2": LI2,
"gamma": GAMMA_FUNC,
"beta": BETA_FUNC,
"fresnel_s": FRESNEL_S,
"fresnel_c": FRESNEL_C,
```

### `cas_handlers.py` additions

All 10 handlers above registered by head name in `build_cas_handler_table()`.

### Example MACSYMA sessions

```
(%i1) integrate(exp(-x^2), x);
(%o1)                         sqrt(%pi)*erf(x)/2

(%i2) integrate(sin(x)/x, x);
(%o2)                         si(x)

(%i3) integrate(log(1-x)/x, x);
(%o3)                         -li2(x)

(%i4) gamma(5);
(%o4)                         24

(%i5) gamma(1/2);
(%o5)                         sqrt(%pi)

(%i6) beta(1/2, 1/2);
(%o6)                         %pi

(%i7) diff(erf(x^2), x);
(%o7)                         4*x*exp(-x^4)/sqrt(%pi)

(%i8) integrate(cos(%pi*x^2/2), x);
(%o8)                         fresnel_c(x)
```

---

## Test Plan

### `test_phase23.py` (≥ 60 tests)

| Class | Tests | Coverage |
|-------|-------|---------|
| `TestPhase23_ErfIntegral` | 10 | `∫exp(-x²)`, `∫exp(-4x²)`, `∫exp(x²)` (erfi), shifted, MACSYMA e2e |
| `TestPhase23_SiCiIntegral` | 10 | `∫sin(x)/x`, `∫cos(x)/x`, `∫sin(2x)/x`, `∫sinh(x)/x`, `∫cosh(x)/x`, MACSYMA |
| `TestPhase23_Li2Integral` | 8 | `∫log(1-x)/x`, `∫log(x)/(1-x)`, fallthrough for other logs, MACSYMA |
| `TestPhase23_FresnelIntegral` | 8 | `∫sin(πx²/2)`, `∫cos(πx²/2)`, `∫sin(2x²)` → FresnelS scaled, MACSYMA |
| `TestPhase23_GammaBeta` | 14 | `Γ(1..6)`, `Γ(1/2)`, `Γ(3/2)`, `B(1/2,1/2)`, `B(2,3)`, float eval, MACSYMA |
| `TestPhase23_Differentiation` | 10 | `d/dx erf(x)`, `d/dx erf(x²)`, `d/dx Si(x)`, `d/dx Li2(x)`, `d/dx FresnelS(x)`, chain rule |
| `TestPhase23_Regressions` | 6 | Phase 22 pattern matching, Phase 15 bare sech/csch, Phase 3 exp(2x), Phase 14 sinh⁴, no regressions |

---

## Version Bumps

| Package | Old | New |
|---------|-----|-----|
| `coding-adventures-symbolic-ir` | 0.10.0 | 0.11.0 |
| `coding-adventures-symbolic-vm` | 0.42.0 | 0.43.0 |
| `coding-adventures-macsyma-runtime` | 1.13.0 | 1.14.0 |

`symbolic-vm 0.43.0` depends on `symbolic-ir>=0.11.0`.
`macsyma-runtime 1.14.0` depends on `symbolic-vm>=0.43.0` and `symbolic-ir>=0.11.0`.
