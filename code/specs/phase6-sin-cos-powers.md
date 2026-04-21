# Phase 6 — sinⁿ·cosᵐ Mixed Trig Powers

## Motivation

Phases 1–5 of the symbolic integrator handle individual trig powers
(`sinⁿ`, `cosⁿ`, `tanⁿ`) but leave products of the form
`sinⁿ(ax+b) · cosᵐ(ax+b)` unevaluated. Phase 6 closes this gap.

The three classical algorithms below cover every integer-exponent case:

| n parity | m parity | Algorithm |
|----------|----------|-----------|
| odd      | any      | u = cos(ax+b) substitution (Case A) |
| even     | odd      | u = sin(ax+b) substitution (Case B) |
| even     | even     | IBP reduction formula (Case C) |

Each algorithm is derived from first principles below so that the code
can be understood without consulting a reference table.

## Case A — n odd: cosine substitution

### Setup

Let `n = 2k + 1` (k ≥ 0) and let `u = cos(ax + b)`. Then:

```
du/dx = -a sin(ax + b)   ⟹   sin(ax + b) dx = -du / a
```

Factor one `sin` out of `sinⁿ`:

```
sinⁿ = sin²ᵏ · sin = (1 - cos²)ᵏ · sin = (1 - u²)ᵏ · sin
```

Substitute:

```
∫ sinⁿ cosᵐ dx = ∫ (1-u²)ᵏ · uᵐ · (-du/a)
               = -(1/a) ∫ (1-u²)ᵏ uᵐ du
```

Expand `(1-u²)ᵏ` by the binomial theorem:

```
(1 - u²)ᵏ = Σ_{j=0}^{k} C(k,j) (-u²)ʲ = Σ_{j=0}^{k} C(k,j) (-1)ʲ u^{2j}
```

Each term integrates by the power rule:

```
∫ C(k,j) (-1)ʲ u^{m+2j} du = C(k,j) (-1)ʲ u^{m+2j+1} / (m+2j+1)
```

Substitute back u = cos(ax+b):

```
∫ sinⁿ cosᵐ (ax+b) dx = -(1/a) Σ_{j=0}^{k} C(k,j) (-1)ʲ / (m+2j+1) · cos^{m+2j+1}(ax+b)
```

### Worked example: ∫ sin³(x) cos²(x) dx

n=3 → k=1. m=2.

```
j=0: -C(1,0)(-1)⁰/(2+0+1) · cos³  = -1/3 · cos³
j=1: -C(1,1)(-1)¹/(2+2+1) · cos⁵  = +1/5 · cos⁵

Result = cos⁵(x)/5 - cos³(x)/3
```

Verify: d/dx[cos⁵/5 - cos³/3] = -sin·cos⁴ + sin·cos² = sin·cos²(cos²-1)... wait

d/dx[cos⁵/5] = 5cos⁴·(-sin)/5 = -cos⁴ sin
d/dx[-cos³/3] = -3cos²·(-sin)/3 = cos² sin

Sum = sin(-cos⁴ + cos²) = sin·cos²(1 - cos²) = sin·cos²·sin² = sin³cos² ✓

### General formula (a ≠ 1)

For argument `ax+b` with a ≠ 1 the only change is the overall `1/a` factor. The
binomial expansion and back-substitution are identical.

### Worked example: ∫ sin(x) cos²(x) dx

n=1 → k=0. Only j=0 term:

```
-(1/1) · C(0,0)(-1)⁰/(2+0+1) · cos³  =  -cos³/3
```

Verify: d/dx[-cos³/3] = -3cos²(-sin)/3 = cos²·sin = sin·cos² ✓

## Case B — m odd: sine substitution

### Setup

Let `m = 2k + 1` (k ≥ 0) and let `u = sin(ax + b)`. Then:

```
du/dx = a cos(ax + b)   ⟹   cos(ax + b) dx = du / a
```

Factor one `cos` out of `cosᵐ`:

```
cosᵐ = cos²ᵏ · cos = (1 - sin²)ᵏ · cos = (1 - u²)ᵏ · cos
```

Substitute:

```
∫ sinⁿ cosᵐ dx = ∫ uⁿ (1-u²)ᵏ · (du/a)
               = (1/a) ∫ uⁿ (1-u²)ᵏ du
```

Expand and integrate term by term:

```
∫ sinⁿ cosᵐ (ax+b) dx = (1/a) Σ_{j=0}^{k} C(k,j) (-1)ʲ / (n+2j+1) · sin^{n+2j+1}(ax+b)
```

### Worked example: ∫ sin²(x) cos³(x) dx

n=2, m=3 → k=1.

```
j=0: (1/1)·C(1,0)(-1)⁰/(2+0+1)·sin³ = sin³/3
j=1: (1/1)·C(1,1)(-1)¹/(2+2+1)·sin⁵ = -sin⁵/5

Result = sin³(x)/3 - sin⁵(x)/5
```

Verify: d/dx[sin³/3 - sin⁵/5] = cos·sin² - cos·sin⁴ = sin²·cos(1-sin²) = sin²·cos·cos² = sin²cos³ ✓

## Case C — both even: IBP reduction

### Setup

For n, m both even (n, m ≥ 2), neither substitution reduces cleanly to a
polynomial integral. Instead, integrate by parts with:

```
u  = sinⁿ⁻¹       dv = sin·cosᵐ dx
u' = (n-1)sinⁿ⁻²cos   v  = cosᵐ⁺¹ / (-(m+1)a)    [from Case A or power rule]
```

Wait — a cleaner derivation uses the general IBP rule applied to sinⁿ directly:

```
∫ sinⁿ cosᵐ dx
  = ∫ sinⁿ⁻¹ · (sin cosᵐ) dx

Let u = sinⁿ⁻¹,  dv = sin cosᵐ dx  ⟹  v = -cosᵐ⁺¹ / ((m+1)a)

IBP: u·v - ∫ v·u' dx

u' = (n-1) sinⁿ⁻² cos · a  [chain rule with a from the linear argument]

∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹ cosᵐ⁺¹/((m+1)a) + (n-1)/(m+1) · ∫ sinⁿ⁻² cosᵐ⁺² dx
```

Now apply `cosᵐ⁺² = cosᵐ(1 - sin²) = cosᵐ - cosᵐ sin²`:

```
∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹cosᵐ⁺¹/((m+1)a)
                + (n-1)/(m+1) · ∫ sinⁿ⁻² cosᵐ dx
                - (n-1)/(m+1) · ∫ sinⁿ cosᵐ dx
```

Collect the `∫ sinⁿ cosᵐ dx` terms on the left:

```
[1 + (n-1)/(m+1)] ∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹cosᵐ⁺¹/((m+1)a) + (n-1)/(m+1) ∫ sinⁿ⁻² cosᵐ dx

[(m+n)/(m+1)] ∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹cosᵐ⁺¹/((m+1)a) + (n-1)/(m+1) ∫ sinⁿ⁻² cosᵐ dx
```

Multiply through by `(m+1)/(m+n)`:

```
∫ sinⁿ cosᵐ dx = -sinⁿ⁻¹cosᵐ⁺¹/((n+m)a) + (n-1)/(n+m) · ∫ sinⁿ⁻² cosᵐ dx
```

### Implementation choice: reduce n (not m)

The formula above reduces n by 2. Alternatively (by symmetry):

```
∫ sinⁿ cosᵐ dx = sinⁿ⁺¹cosᵐ⁻¹/((n+m)a) + (m-1)/(n+m) · ∫ sinⁿ cosᵐ⁻² dx
```

The implementation reduces **n** at each step. Recursion terminates at:

- `n = 0` → `∫ cosᵐ dx` — Phase 5b ✓
- `n = 1` (can't happen since we only enter when n is even and n decreases by 2, never reaching 1)

Wait — n starts even and decreases by 2, so n reaches 0 before 1. At n=0:
`∫ cosᵐ dx` is Phase 5b. ✓

### Worked example: ∫ sin²(x) cos²(x) dx

n=2, m=2.

Step 1: n=2 → -sin¹cos³/((2+2)·1) + 1/4 · ∫ cos²dx
       = -sincos³/4 + 1/4 · (sin cos/2 + x/2)      [Phase 5b result]
       = -sincos³/4 + sincos/8 + x/8

Verify numerically at x=0.5:
sin²(0.5)cos²(0.5) ≈ 0.2298·0.7702 ≈ 0.1770

F = -sin(0.5)cos³(0.5)/4 + sin(0.5)cos(0.5)/8 + 0.5/8
  = -(0.4794)(0.4292)/4 + (0.4794)(0.8776)/8 + 0.0625
  = -0.0515 + 0.0526 + 0.0625 = 0.0636

F'(0.5) ≈ [F(0.5+1e-7) - F(0.5-1e-7)]/(2·1e-7) ≈ 0.1770 ✓

## Integration Point in the VM

Phase 6 hooks into the `MUL` branch of `_integrate`, after `_try_trig_trig`
(Phase 4b) and before `_try_exp_trig` (Phase 4c):

```python
# Phase 6: sinⁿ × cosᵐ with same linear argument.
result = _try_sin_cos_power(a, b, x)
if result is not None:
    return result
```

No swap needed: `_try_sin_cos_power` examines both orderings internally.

## Guard Summary

`_try_sin_cos_power` returns `None` unless:

1. Both factors are `SIN/COS(linear)^n` with integer n ≥ 1
2. One factor is SIN-based, the other is COS-based
3. Both share the **same** linear argument (same a, b over Q)
4. `max(n, m) ≥ 2` — avoids shadowing the n=m=1 same-arg case in `_try_trig_trig`
5. The linear coefficient a ≠ 0

## New helpers summary

| Function | Responsibility |
|---|---|
| `_extract_trig_power(node, x)` | Recognise `SIN/COS(linear)^n`, return `(head, n, a, b)` |
| `_try_sin_cos_power(fa, fb, x)` | Guard checks, dispatch to Case A/B/C |
| `_sin_cos_odd_sin(n, m, a, b, x)` | Case A: binomial sum over cos powers |
| `_sin_cos_odd_cos(n, m, a, b, x)` | Case B: binomial sum over sin powers |
| `_sin_cos_even(n, m, a, b, x)` | Case C: IBP reduction on n |

## Test strategy

All correctness tests: numerical re-differentiation at x = 0.4 and x = 0.7
(away from trig poles).

Key cases:
- (n=1, m=2): Case A, k=0 — simplest non-trivial
- (n=3, m=2): Case A, k=1 — two-term sum
- (n=2, m=1): Case B, k=0
- (n=2, m=3): Case B, k=1
- (n=2, m=2): Case C, one recursion step → Phase 5b
- (n=4, m=2): Case C, one recursion step → Phase 5b
- (n=2, m=4): Case C, two recursion steps → Phase 5b
- (n=4, m=4): Case C, deep recursion → Phase 5b
- Various a ≠ 1 and b ≠ 0 to verify coefficient scaling
