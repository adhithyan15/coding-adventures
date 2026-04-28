# Phase 14 Deferred Fixes

## Context

Phase 14 (symbolic-vm 0.33.0) added hyperbolic power integration and exp×hyp double-IBP.
Three deferred cases were left unevaluated because their solutions require additional
machinery beyond the initial u-substitution framework.  This document specifies the
three closures.

---

## 14a-fix: exp(ax+b) × sinh/cosh(cx+d) when a² = c²

**Symptom:** `_try_exp_hyp` returns `None` when `D = a² − c² = 0`, leaving
`∫ exp(x)·sinh(x) dx` unevaluated.

**Algorithm:** Expand sinh/cosh into exponentials and integrate each term directly.

```
sinh(cx+d) = (e^(cx+d) - e^(-cx-d)) / 2
cosh(cx+d) = (e^(cx+d) + e^(-cx-d)) / 2
```

So `e^(ax+b) · sinh(cx+d) = (e^((a+c)x+b+d) - e^((a-c)x+b-d)) / 2`

Since `a² = c²`, exactly one of `a+c`, `a-c` is zero:

| Case   | Linear terms       | Non-zero integral             | Zero integral         |
|--------|--------------------|-------------------------------|-----------------------|
| `a=c`  | `a+c=2a`, `a-c=0`  | `e^(2ax+b+d) / (4a)`          | `±e^(b-d) · x / 2`   |
| `a=-c` | `a+c=0`, `a-c=2a`  | `±e^(2ax+b-d) / (4a)`         | `e^(b+d) · x / 2`    |

Sign convention: `+` for cosh, `−` for sinh in the ± position.

**Implementation file:** `exp_hyp_integral.py`

New function: `exp_hyp_degenerate(a, b, c, d, is_sinh, x_sym) → IRNode`

**Caller change:** `integrate.py` `_try_exp_hyp` — replace `return None` with a call to
`exp_hyp_degenerate`.

---

## 14b-fix: ∫ sinh^m(ax+b) · cosh^n(ax+b) dx for both m,n ≥ 2

**Symptom:** `sinh_times_cosh_power(m, n, …)` returns `None` when neither m nor n equals 1.

**Algorithm:** Three sub-cases based on parity:

### Sub-case A: m is odd (m = 2p+1 ≥ 3)

Use identity `sinh²(u) = cosh²(u) − 1`, pull out one `sinh`, substitute `u = cosh(ax+b)`:

```
∫ sinh^(2p+1) · cosh^n dx  =  (1/a) ∫ (u²−1)^p · u^n du   [u = cosh(ax+b)]
```

Expand `(u²−1)^p` by binomial theorem:

```
(u²−1)^p = Σ_{k=0}^{p} C(p,k) · (-1)^(p-k) · u^(2k)
```

Resulting antiderivative (back-substituting `u = cosh(ax+b)`):

```
Σ_{k=0}^{p} [C(p,k)·(-1)^(p-k) / (a·(2k+n+1))] · cosh^(2k+n+1)(ax+b)
```

### Sub-case B: n is odd (n = 2q+1 ≥ 3)

Symmetric: use `cosh²(u) = sinh²(u) + 1`, substitute `u = sinh(ax+b)`:

```
∫ sinh^m · cosh^(2q+1) dx  =  (1/a) ∫ (u²+1)^q · u^m du   [u = sinh(ax+b)]
```

Antiderivative:

```
Σ_{k=0}^{q} [C(q,k) / (a·(2k+m+1))] · sinh^(2k+m+1)(ax+b)
```

Note: all signs are positive because `(u²+1)^q` has only positive binomial coefficients.

### Sub-case C: both m, n even

Use `sinh^(2p)(u) = (cosh²(u)−1)^p`:

```
∫ sinh^(2p) · cosh^(2q) dx  =  Σ_{k=0}^{p} C(p,k)·(-1)^(p-k) · ∫ cosh^(2k+2q) dx
```

Each `∫ cosh^N dx` is handled by the existing `cosh_power_integral(N, a, b, x)`.

**Implementation file:** `hyp_power_integral.py`

Extend `sinh_times_cosh_power` — replace `return None` with the three sub-cases above.
Add helper `_fold_add(terms)` that left-folds a non-empty list into binary ADD nodes.
Import `from math import comb as _comb`.

---

## 14c-fix: ∫ P(x) · atanh(ax+b) dx for P ∈ Q[x]

**Algorithm:** IBP with `u = atanh(ax+b)`, `dv = P(x) dx`:

```
∫ P · atanh = Q(x)·atanh(ax+b) − a · ∫ Q(x)/(1−(ax+b)²) dx
```

where `Q(x) = ∫ P(x) dx`.

**Residual:** Divide `Q` by `D(x) = 1−(ax+b)² = −a²x² − 2abx + (1−b²)`:

```
Q = S·D + R    (deg R ≤ 1)
R(x) = r₁·x + r₀
```

The rational residual decomposes (substituting `t = ax+b`):

```
∫ R(x)/D dx = T(x) − (r₁/(2a²))·log(1−(ax+b)²) + (r₀/a − r₁·b/a²)·atanh(ax+b)
```

where `T(x) = ∫ S(x) dx`.

**Final closed form:**

```
[Q(x) − (r₀ − r₁·b/a)] · atanh(ax+b)  −  a·T(x)  +  (r₁/(2a))·log(1−(ax+b)²)
```

Verification (P=1): `Q=x`, `S=0`, `r₀=0`, `r₁=1`:
→ `(x + b/a)·atanh(ax+b) + (1/(2a))·log(1−(ax+b)²)` ✓ matches Phase 13 bare formula.

**Implementation file:** `atanh_poly_integral.py` (new, mirrors `atan_poly_integral.py`).

**Caller changes:**

1. Add `_try_atanh_product` function in `integrate.py` (after `_try_acosh_product`).
2. Wire the call in the MUL block after the Phase 13 acosh block.

---

## Files Changed

| File | Change |
|------|--------|
| `exp_hyp_integral.py` | Add `exp_hyp_degenerate` + update `__all__` |
| `hyp_power_integral.py` | Extend `sinh_times_cosh_power`; add `_fold_add`; `import comb` |
| `atanh_poly_integral.py` | **NEW** — polynomial × atanh IBP |
| `integrate.py` | 4 changes (see above) |
| `tests/test_phase14.py` | 3 fallthroughs → evaluated; new `TestPhase14_AtanhPoly` class |
| `symbolic-vm/CHANGELOG.md` | 0.34.0 entry |
| `symbolic-vm/pyproject.toml` | version 0.33.0 → 0.34.0 |

---

## Version Bump

`coding-adventures-symbolic-vm`: `0.33.0` → `0.34.0`

No changes to `macsyma-runtime` (these are purely integration-engine fixes, no new
Macsyma surface names).
