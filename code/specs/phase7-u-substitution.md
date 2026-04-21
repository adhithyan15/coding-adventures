# Phase 7 — u-Substitution (Chain-Rule Reversal)

## Motivation

Phases 1–6 handle constants, polynomials, rational functions, elementary
transcendentals of linear arguments, products of specific forms (exp×poly,
log×poly, trig×poly, exp×trig), identical-trig products, and sinⁿ·cosᵐ
mixed powers.  What they cannot yet do is reverse the chain rule:

```
∫ f(g(x)) · g'(x) dx  =  F(g(x)) + C
```

where `F` is an antiderivative of `f`.  Phase 7 closes this gap for the most
common class of outer functions.

Examples that require Phase 7:

| Integrand | Substitution | Result |
|---|---|---|
| `x · sin(x²)` | u = x², du = 2x dx | `−cos(x²)/2` |
| `x² · exp(x³)` | u = x³, du = 3x² dx | `exp(x³)/3` |
| `cos(x) · exp(sin(x))` | u = sin(x), du = cos(x) dx | `exp(sin(x))` |
| `sin(x) · exp(cos(x))` | u = cos(x), du = −sin(x) dx | `−exp(cos(x))` |
| `cos(x) · tan(sin(x))` | u = sin(x), du = cos(x) dx | `−log(cos(sin(x)))` |
| `cos(x) · log(sin(x))` | u = sin(x), du = cos(x) dx | `sin(x)·log(sin(x)) − sin(x)` |

## Algorithm

### Guard conditions

`_try_u_sub` considers a MUL node `fa · fb`.  For each ordered pair (outer,
gp_candidate) in {(fa,fb), (fb,fa)} it calls `_try_u_sub_one(outer, gp_candidate, x)`:

1. `outer` must be an `IRApply` with **exactly one argument** (SIN, COS, EXP,
   LOG, TAN, SQRT).  Two-argument nodes (POW) are deferred to a later phase.
2. Extract `g = outer.args[0]` — the inner function.
3. **Skip if g = x**: the bare-variable case is already handled by Phase 1.
4. **Skip if g is linear**: `_try_linear(g, x) != None` with a ≠ 0.  Phases 3–5
   already cover every function of a linear argument.
5. Compute `gprime = _diff_ir(g, x)`.  Return `None` if differentiation fails.
6. Compute `c = _ratio_const(gp_candidate, gprime, x)`.  Return `None` if
   `gp_candidate` is not a rational-constant multiple of `gprime`.
7. Introduce a fresh dummy symbol `u = IRSymbol("__u__")`.
8. Build `F_u = _subst(outer, g, u)` — replace `g` with `u` inside `outer`.
9. Compute `G_u = _integrate(F_u, u)`.  Return `None` if the outer integral
   cannot be evaluated.
10. Substitute back: `G_gx = _subst(G_u, u, g)`.
11. If `c = 1` return `G_gx`; otherwise return `MUL(c, G_gx)`.

The dummy symbol `__u__` never appears in user expressions (valid mathematical
names are single letters), so there is no capture risk.

### `_diff_ir` — symbolic differentiation of g(x)

Covers the inner functions encountered in practice:

| g(x) form | d(g)/dx |
|---|---|
| Constant (free of x) | 0 |
| Bare x | 1 |
| Polynomial in x (via `to_rational`) | polynomial derivative |
| `SIN(u)` | `cos(u) · u'` |
| `COS(u)` | `−sin(u) · u'` |
| `EXP(u)` | `exp(u) · u'` |
| `LOG(u)` | `u' / u` |
| `SQRT(u)` | `u' / (2·sqrt(u))` |
| `POW(base, n)` integer n | `n · base^(n−1) · base'` |

Chain rule is applied recursively.  Returns `None` for unknown forms (free
symbols other than x, floats, nested structures not covered above).

### `_ratio_const` — check f = c·g for rational c

Checks in order:

1. Structural equality `f == g` → `c = 1`.
2. `f` is a rational literal → c = f/g (only if g is also a literal).
3. `f = MUL(c_ir, g)` or `f = MUL(g, c_ir)` for rational literal `c_ir` → `c`.
4. `g = MUL(c_ir, f)` or `g = MUL(f, c_ir)` → `c = 1/c_ir`.
5. `g = NEG(f)` → `c = −1`; `f = NEG(g)` → `c = −1`.
6. Both `f` and `g` are rational functions of x (`to_rational` succeeds):
   compute the polynomial ratio `(f_num · g_den) / (f_den · g_num)` and check
   that all non-zero coefficient pairs have the same ratio.

### `_subst` — structural node substitution

`_subst(node, old, new)` recursively replaces every occurrence of `old` with
`new` using structural equality.  Single pass, no capture issues (the only
bound variable is `__u__`, which never appears in the original expression).

## Integration point in the VM

Phase 7 hooks into the `MUL` branch of `_integrate`, after Phase 6 (`_try_sin_cos_power`):

```python
# Phase 7: u-substitution — f(g(x)) · c·g'(x).
result = _try_u_sub(a, b, x)
if result is not None:
    return result
```

This ordering ensures:

- Phase 6 handles sinⁿ·cosᵐ (both outer functions have two args or are trig
  powers with the same linear argument) before Phase 7 is tried.
- Phase 4c (`exp × trig`) and Phase 4a (`poly × trig`) still fire for linear-arg
  cases because Phase 7 skips them (guard 4 above).
- Phase 3 (`exp × poly`) fires before Phase 7 for standard poly × exp cases,
  but Phase 7 is not reached anyway since Phase 3 returns first.

## Scope

Phase 7 is intentionally narrow:

- **Outer functions**: SIN, COS, EXP, LOG, TAN, SQRT (exactly 1 argument).
  `POW(f(g), n)` outer functions are deferred to Phase 8 (power-of-composite
  u-sub), which can use the sinⁿ/cosⁿ reduction formulas as inner integrals.
- **Inner function g(x)**: anything `_diff_ir` can differentiate.
- **Scale factor c**: must be a rational constant; irrational or symbolic
  scale factors return `None` (integral left unevaluated).

## New helpers summary

| Function | Purpose |
|---|---|
| `_poly_deriv(p)` | Differentiate polynomial coefficient tuple |
| `_poly_mul(p, q)` | Multiply two polynomial coefficient tuples |
| `_diff_ir(g, x)` | Return dg/dx as IR, or None |
| `_ratio_const(f, g, x)` | Return c if f = c·g (rational c), else None |
| `_subst(node, old, new)` | Structural substitution |
| `_try_u_sub_one(outer, gp, x)` | Core u-sub attempt (one ordering) |
| `_try_u_sub(fa, fb, x)` | Try both orderings |

## Test strategy

All correctness tests: numerical re-differentiation at x = 0.4 and x = 0.7.

Key cases:

- **Power inner** (g = polynomial): `x·sin(x²)`, `x²·exp(x³)`, `3x²·sin(x³−1)`, etc.
- **Trig inner** (g = sin/cos): `cos(x)·exp(sin(x))`, `sin(x)·exp(cos(x))`,
  `cos(x)·tan(sin(x))`, `cos(x)·log(sin(x))`, etc.
- **Exp inner** (g = exp): `exp(x)·sin(exp(x))`, `exp(x)·cos(exp(x))`, etc.
- **Linear-arg bypass**: confirm that `x·sin(x)`, `x·sin(2x+1)` still go to
  Phase 4a, not Phase 7.
- **Fallthrough**: expressions Phase 7 should not handle — return None.
- **Regressions**: Phase 5 and Phase 6 results still correct.
- **MACSYMA end-to-end**: full string-to-IR pipeline tests.
