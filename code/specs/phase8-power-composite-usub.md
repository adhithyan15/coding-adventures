# Phase 8 — Power-of-Composite U-Substitution

## Motivation

Phases 1–7 of the symbolic integrator handle rational functions, transcendentals,
individual trig powers, mixed sinⁿ·cosᵐ, and u-substitution for **single-argument**
outer functions (SIN, COS, EXP, LOG, TAN, SQRT).  Phase 7's core guard rejects any
outer function with more than one argument — in particular `POW(base, n)` — because
the power rule and trig-power rules (Phases 1 and 5) already cover the most
common power integrals.

Phase 8 closes the remaining gap: **integrands of the form `f(g(x))ⁿ · c·g'(x)`**
where the outer layer is a power of a composite expression.

Representative examples that Phase 7 cannot handle:

| Integrand | g(x) | n | Result |
|---|---|---|---|
| `cos(x)·sin²(sin(x))` | `sin(x)` | 2 | `sin(x)/2 − sin(2sin(x))/4` |
| `2x·(x²+1)³` | `x²+1` | 3 | `(x²+1)⁴/4` |
| `exp(x)·(exp(x)+1)⁴` | `exp(x)+1` | 4 | `(exp(x)+1)⁵/5` |
| `3x²·(x³+5)⁻²` | `x³+5` | −2 | `−1/(x³+5)` |

---

## Mathematical Foundation

### Chain-Rule Reversal for Power Composites

The fundamental theorem underlying Phase 8 is:

```
∫ h(g(x))ⁿ · c·g'(x) dx  =  c · ∫ h(u)ⁿ du  evaluated at u = g(x)
```

This is the standard u-substitution `u = g(x)`, `du = g'(x) dx`.

The result `∫ h(u)ⁿ du` is computed by recursive delegation to existing phases:

- `h = SIN`  →  `∫ sinⁿ(u) du`  — Phase 5b (sinⁿ reduction formula)
- `h = COS`  →  `∫ cosⁿ(u) du`  — Phase 5b (cosⁿ reduction formula)
- `h = TAN`  →  `∫ tanⁿ(u) du`  — Phase 5c (tanⁿ reduction formula)
- `h = identity` (base = g(x)) →  `∫ uⁿ du`  — Phase 1 power rule

### Worked Example — Case A (1-arg outer function raised to a power)

`∫ cos(x)·sin²(sin(x)) dx`

Let `u = sin(x)`, `du = cos(x) dx`.

```
∫ sin²(u) du  =  u/2 − sin(2u)/4    (Phase 5b, n=2, a=1, b=0)
```

Back-substitute `u = sin(x)`:

```
sin(x)/2 − sin(2·sin(x))/4
```

### Worked Example — Case B (base is the substitution itself)

`∫ 2x·(x²+1)³ dx`

Let `u = x²+1`, `du = 2x dx`.

```
∫ u³ du  =  u⁴/4    (Phase 1 power rule)
```

Back-substitute `u = x²+1`:

```
(x²+1)⁴/4
```

### Special sub-case n = −1 in Case B

`∫ 2x·(x²+1)⁻¹ dx`

`u = x²+1`, `∫ u⁻¹ du = log(u)`.  Result: `log(x²+1)`.

### Linear-base single-factor integral (POW branch extension)

A natural companion: `∫ (ax+b)^n dx` where only one factor exists.  Previously
the POW branch only handled `base == x`.  With a linear base:

```
n ≠ −1:   ∫ (ax+b)^n dx = (ax+b)^(n+1) / ((n+1)·a)
n = −1:   ∫ (ax+b)^(−1) dx = log(ax+b) / a
```

This is separate from the MUL-branch u-sub but completes the picture for
polynomial-composition integrals.

---

## Algorithm

### Extending `_diff_ir`

Phase 7 added symbolic differentiation for constants, polynomials, single-arg
functions (SIN/COS/EXP/LOG/SQRT), and integer-exponent POW.  Phase 8 adds:

| New case | Rule |
|---|---|
| `NEG(f)` | `d/dx(−f) = −f'`; returns `NEG(f')` (or 0 if f'=0) |
| `ADD(f, g)` | `d/dx(f+g) = f' + g'`; returns `f'` if g'=0, `g'` if f'=0 |
| `SUB(f, g)` | `d/dx(f−g) = f' − g'`; returns `f'` if g'=0 |

These rules enable differentiating `exp(x)+1 → exp(x)`, `sin(x)−x → cos(x)−1`,
etc., which are required for Case B with non-polynomial bases.

### `_try_u_sub_pow_one(pow_node, gp_candidate, x)`

```
1. Confirm pow_node = POW(base, exp_node).
2. Require not _depends_on(exp_node, x)  — exponent must be x-free.
3. Require _depends_on(base, x)          — base must involve x.

CASE A — base is a 1-arg IRApply:
  4a. Extract g = base.args[0].
  5a. Skip if g == x              (Phase 5 handles f(x)^n via _try_trig_power).
  6a. Skip if g is linear a≠0     (Phase 5 handles f(ax+b)^n).
  7a. Compute gprime = _diff_ir(g, x).  If None → skip.
  8a. Check c = _ratio_const(gp_candidate, gprime, x).  If None or 0 → skip.
  9a. Introduce dummy u = IRSymbol("__u__").
 10a. Build base_u = _subst(base, g, u)  [replaces g→u inside f].
 11a. Compute G_u = _integrate(POW(base_u, exp_node), u).  If None → skip.
 12a. Back-substitute: G_gx = _subst(G_u, u, g).
 13a. Return c·G_gx  (or G_gx if c=1).

CASE B — base is not a 1-arg IRApply:
  4b. Skip if base == x            (Phase 1 handles x^n).
  5b. Skip if base is linear a≠0   (new linear POW branch handles (ax+b)^n).
  6b. Compute gprime = _diff_ir(base, x).  If None → skip.
  7b. Check c = _ratio_const(gp_candidate, gprime, x).  If None or 0 → skip.
  8b. Introduce dummy u = IRSymbol("__u__").
  9b. Compute G_u = _integrate(POW(u, exp_node), u).  If None → skip.
 10b. Back-substitute: G_gx = _subst(G_u, u, base).
 11b. Return c·G_gx  (or G_gx if c=1).
```

### `_try_u_sub_pow(fa, fb, x)`

Tries both orderings (fa as pow_node, fb as gp) then (fb as pow_node, fa as gp).

### MUL Branch Hook

Placed after Phase 7 (`_try_u_sub`) and before Phase 4c (`_try_exp_trig`):

```python
# Phase 8: u-substitution for POW(f(g(x)), n) · c·g'(x).
result = _try_u_sub_pow(a, b, x)
if result is not None:
    return result
```

---

## Scope and Limitations

### What Phase 8 handles

| Pattern | Example |
|---|---|
| `POW(SIN(g(x)), n) · c·g'(x)` | `cos(x)·sin²(sin(x))` |
| `POW(COS(g(x)), n) · c·g'(x)` | `−sin(x)·cos³(cos(x))` |
| `POW(TAN(g(x)), n) · c·g'(x)` | `(1/cos²(x))·tan²(tan(x))` |
| `POW(g(x), n) · c·g'(x)` | `2x·(x²+1)^n` |
| `POW(EXP(g(x)), n) · c·g'(x)` | falls through if inner `∫ exp(u)^n du` unsolvable |
| `POW(LOG(g(x)), n) · c·g'(x)` | falls through (log^n not in Phase 5) |
| Single-factor `(ax+b)^n` | POW branch — no second factor needed |

### What remains deferred (Phase 9+)

- `DIV(g'(x), g(x))` → `log(g(x))` (quotient form of n=−1; needs DIV branch extension)
- `POW(f(g(x)), n)` where `∫ f(u)^n du` is unknown (EXP^n, LOG^n, SQRT^n for arbitrary n)
- Product rule / IBP for polynomials × arbitrary functions
- Two-argument outer functions (e.g., `POW(g₁(x), g₂(x))` where both depend on x)

---

## Implementation Notes

### Dummy symbol safety

The dummy integration variable `IRSymbol("__u__")` is safe because valid user
symbols are single lowercase letters.  The same `__u__` is shared between Phase 7
and Phase 8 calls; there is no nesting because `_try_u_sub_pow_one` calls
`_integrate(POW(base_u, n), u)` where `base_u` contains `__u__` but not `x`,
so no further u-sub hook fires.

### Interaction with `_try_trig_power`

When Case A encounters `POW(SIN(g), n)` with `g = __u__` (the dummy), the call
`_integrate(POW(SIN(__u__), n), __u__)` enters the POW branch, which calls
`_try_trig_power(SIN(__u__), n, __u__)`.  Since `_try_linear(__u__, __u__) = (1, 0)`
(linear with a=1, b=0), the trig power helper fires correctly.

### Interaction with Phase 7

Phase 7 already handles `f(g(x))¹ · g'(x)` (power 1 is unwrapped by the
`exponent.value == 1` case in the POW dispatch before `_try_trig_power` fires).
Phase 8 handles exponents ≥ 2 (and negative exponents).  No overlap.

### Guard priority

The MUL dispatch order ensures:
1. Constant factor (x-free operand) — always first
2. Phase 3 (exp/log products)
3. Phase 4b (trig × trig product-to-sum)
4. Phase 6 (sinⁿ·cosᵐ)
5. Phase 7 (single-arg u-sub)
6. **Phase 8 (power-of-composite u-sub)**  ← new
7. Phase 4c (exp × trig double-IBP)
8. Phase 4a (polynomial × trig tabular IBP)

---

## Tests

`tests/test_phase8.py` — ≥40 tests across 6 classes.  All correctness tests use
`_check_antiderivative` (numerical re-differentiation at x=0.4 and x=0.7, same
pattern as Phases 5–7).

| Class | Count | Focus |
|---|---|---|
| `TestPhase8_CaseA_Trig` | 10 | `POW(SIN/COS/TAN(g(x)), n) · g'` — polynomial and trig inner g |
| `TestPhase8_CaseB_Poly` | 10 | `POW(g(x), n) · g'` — polynomial, trig inner, exp inner |
| `TestPhase8_LinearPow` | 6 | Single-factor `(ax+b)^n` in POW branch |
| `TestPhase8_Fallthrough` | 4 | Guard rejection cases |
| `TestPhase8_Regressions` | 6 | Earlier phases unaffected |
| `TestPhase8_Macsyma` | 4 | End-to-end MACSYMA string tests |
