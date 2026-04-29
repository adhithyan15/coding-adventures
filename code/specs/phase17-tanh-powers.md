# Phase 17 — `∫ tanh^n(ax+b) dx`

## Status

**Complete** — shipped in `symbolic-vm` 0.37.0.

---

## Motivation

Phases 14 and 16 completed the power-reduction formulas for five of the six
hyperbolic functions:

| Function | Formula | Phase |
|---|---|---|
| `sinh^n` | IBP: `I_n = sinh^(n-1)·cosh/(na) − (n-1)/n · I_{n-2}` | 14 ✅ |
| `cosh^n` | IBP: `I_n = cosh^(n-1)·sinh/(na) + (n-1)/n · I_{n-2}` | 14 ✅ |
| `sech^n` | IBP: `I_n = sech^(n-2)·tanh/((n-1)a) + (n-2)/(n-1) · I_{n-2}` | 16 ✅ |
| `csch^n` | IBP: `I_n = −csch^(n-2)·coth/((n-1)a) − (n-2)/(n-1) · I_{n-2}` | 16 ✅ |
| `coth^n` | Identity: `I_n = I_{n-2} − coth^(n-1)/((n-1)a)` | 16 ✅ |
| **`tanh^n`** | **Identity: `I_n = I_{n-2} − tanh^(n-1)/((n-1)a)`** | **17 ← this phase** |

The `tanh^n` formula is the last remaining hyperbolic power reduction.

---

## Mathematical Algorithm

### Derivation — identity `tanh² = 1 − sech²`

```
∫ tanh^n dt = ∫ tanh^(n-2) · tanh² dt
            = ∫ tanh^(n-2) · (1 − sech²) dt
            = I_{n-2} − ∫ tanh^(n-2) · sech² dt
```

For the last integral, substitute `u = tanh(t)`, `du = sech²(t) dt`:

```
∫ tanh^(n-2) · sech² dt = tanh^(n-1) / (n-1)
```

Solving:

```
I_n = I_{n-2} − tanh^(n-1)(ax+b) / ((n-1)·a)
```

**Base cases:**

```
I_0 = x
I_1 = log(cosh(ax+b)) / a          [Phase 13 tanh bare integral]
```

### Verification

**n = 2:** `F = x − tanh(ax+b)/a`
```
F' = 1 − sech²(ax+b) = tanh²(ax+b)  ✓
```

**n = 3:** `F = log(cosh(ax+b))/a − tanh²(ax+b)/(2a)`
```
F' = tanh(ax+b) − 2·tanh·sech²·a/(2a)
   = tanh(1 − sech²) = tanh·tanh² = tanh³  ✓
```

**n = 4:** `F = x/a − tanh/a − tanh³/(3a)`  (a=1)
```
F' = 1 − sech² − tanh²·sech²
   = (1 − sech²)(1 + ...) ... = tanh²(1 − sech²) = tanh⁴  ✓
```

---

## Implementation

### Changes to `recip_hyp_power_integral.py`

Add `tanh_power_integral(n, a, b, x)` to the existing module (alongside
`coth_power_integral` which uses the identical structural pattern).

Also add `COSH` to the existing imports (needed for the n=1 base case
`log(cosh(ax+b))/a`).

```
I_n = I_{n-2} − tanh^(n-1)(ax+b) / ((n-1)·a)

Base cases: I_0 = x, I_1 = log(cosh(ax+b))/a
```

### Changes to `integrate.py`

Extend `_try_recip_hyp_power` to also accept `TANH` head (in addition to
`SECH`, `CSCH`, `COTH`) and call `tanh_power_integral` for the TANH case.

---

## Fallthrough Behaviour (deferred cases)

| Input | Reason |
|---|---|
| `∫ P(x)·tanh^n(ax+b) dx` for deg P ≥ 1 | non-elementary (involves polylogarithm) |
| `∫ tanh^n(f(x)) dx` for non-linear `f` | non-linear argument not handled |
| `∫ tanh(x)·sech(x) dx` | mixed product — evaluates via Phase 14/15 dispatch |

Note: `∫ P(x)·tanh(ax+b) dx` for deg P ≥ 1 is **not elementary** —
it requires the dilogarithm Li₂.  Phase 13's test
`test_poly_times_tanh_unevaluated` remains correct and is unchanged.

---

## Package Versions

| Package | Before | After |
|---|---|---|
| `symbolic-vm` | 0.36.0 | 0.37.0 |

`symbolic-ir` and `macsyma-compiler` are **unchanged**.

---

## Files Changed

| File | Change |
|---|---|
| `symbolic-vm/src/symbolic_vm/recip_hyp_power_integral.py` | Add `tanh_power_integral`; add COSH import |
| `symbolic-vm/src/symbolic_vm/integrate.py` | Extend `_try_recip_hyp_power` for TANH |
| `symbolic-vm/tests/test_phase17.py` | **NEW** ≥16 tests |
| `symbolic-vm/CHANGELOG.md` | 0.37.0 entry |
| `symbolic-vm/pyproject.toml` | version 0.37.0 |

---

## Test Coverage

`test_phase17.py` (≥16 tests):

| Class | Tests | What is verified |
|---|---|---|
| `TestPhase17_TanhPowers` | 10 | n=2,3,4,5; a=2; b=1; a=1/2; structure (tanh/log(cosh) in result) |
| `TestPhase17_Fallthrough` | 3 | poly×tanh², tanh(x²) non-linear arg, x·tanh(x) (non-elementary) |
| `TestPhase17_Regressions` | 3 | Phase 16 sech², Phase 13 tanh bare, Phase 14 sinh^4 |

Antiderivative correctness verified numerically: `F'(x) ≈ f(x)` at two test
points per case (x₀ = 0.3, x₁ = 0.8 — strictly away from 0 and atanh singularities).
