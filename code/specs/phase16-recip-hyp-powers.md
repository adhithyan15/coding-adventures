# Phase 16 — Reciprocal Hyperbolic Power Integrals

## Status

**Complete** — shipped in `symbolic-vm` 0.36.0.

---

## Motivation

Phase 15 added bare (n=1) integration for `coth`, `sech`, `csch` and explicitly
deferred the power case, leaving `∫ sech²(x) dx` unevaluated. Phase 16 closes
that gap with **IBP reduction formulas** for all three functions:

```
∫ sech^n(ax+b) dx
∫ csch^n(ax+b) dx
∫ coth^n(ax+b) dx
```

This directly mirrors Phase 14's treatment of `sinh^n` / `cosh^n`
(in `hyp_power_integral.py`). No new IR heads are needed — all output uses
existing heads (`TANH`, `COTH`, `SINH`, `ATAN`, `LOG`, `TANH`).

---

## Mathematical Algorithms

### sech^n  (IBP with dv = sech² du → v = tanh)

```
I_n = sech^(n-2)(ax+b)·tanh(ax+b) / ((n-1)·a)  +  (n-2)/(n-1) · I_{n-2}

Base cases:
  n = 0  →  x
  n = 1  →  atan(sinh(ax+b)) / a           [Phase 15 _sech_integral]
  n = 2  →  tanh(ax+b) / a                 [direct; ← most useful identity]
```

**Derivation** — IBP with `u = sech^(n-2)(t)`, `dv = sech²(t) dt`:
```
v = tanh(t)
du = -(n-2) sech^(n-2)(t)·tanh(t) dt

∫ sech^n dt = sech^(n-2)·tanh + (n-2) ∫ sech^(n-2)·tanh² dt
            = sech^(n-2)·tanh + (n-2) ∫ sech^(n-2)·(1−sech²) dt
            = sech^(n-2)·tanh + (n-2)·I_{n-2} − (n-2)·I_n

⟹  (n-1)·I_n = sech^(n-2)·tanh + (n-2)·I_{n-2}
```

**Verification** (n=4): `F = sech²·tanh/3 + 2·tanh/3`
```
F′ = [sech²(-2tanh² + sech²) + 2sech²] / 3
   = sech²[-2tanh² + sech² + 2] / 3
   = sech²[-2(1-sech²) + sech² + 2] / 3
   = sech²[3sech²] / 3 = sech⁴  ✓
```

---

### csch^n  (IBP with dv = csch² du → v = −coth)

```
I_n = −csch^(n-2)(ax+b)·coth(ax+b) / ((n-1)·a)  −  (n-2)/(n-1) · I_{n-2}

Base cases:
  n = 0  →  x
  n = 1  →  log(tanh((ax+b)/2)) / a       [Phase 15 _csch_integral]
  n = 2  →  −coth(ax+b) / a               [direct; note negative sign]
```

**Derivation** — IBP with `u = csch^(n-2)(t)`, `dv = csch²(t) dt`:
```
v = -coth(t)
du = -(n-2) csch^(n-2)(t)·coth(t) dt

∫ csch^n dt = -csch^(n-2)·coth − (n-2) ∫ csch^(n-2)·coth² dt
            = -csch^(n-2)·coth − (n-2) ∫ csch^(n-2)·(1+csch²) dt
            = -csch^(n-2)·coth − (n-2)·I_{n-2} − (n-2)·I_n

⟹  (n-1)·I_n = -csch^(n-2)·coth − (n-2)·I_{n-2}
```

**Verification** (n=4): `F = -csch²·coth/3 + 2·coth/3`
```
F′ = [csch²(2coth² + csch²) - 2csch²] / 3
   = csch²[2coth² + csch² - 2] / 3
   = csch²[2(1+csch²) + csch² - 2] / 3
   = csch²[3csch²] / 3 = csch⁴  ✓
```

---

### coth^n  (identity coth² = 1 + csch²)

```
I_n = I_{n-2}  −  coth^(n-1)(ax+b) / ((n-1)·a)

Base cases:
  n = 0  →  x
  n = 1  →  log(sinh(ax+b)) / a            [Phase 15 _coth_integral]
```

**Derivation** — expand via the Pythagorean identity `coth² = 1 + csch²`:
```
∫ coth^n dt = ∫ coth^(n-2)·coth² dt
            = ∫ coth^(n-2)·(1 + csch²) dt
            = I_{n-2} + ∫ coth^(n-2)·csch² dt

For the last term, substitute t = coth(u), dt = -csch²(u) du:
  ∫ coth^(n-2)·csch² dt = -coth^(n-1) / (n-1)
```

**Verification** (n=3): `F = log(sinh(x)) - coth²(x)/2`
```
F′ = coth(x) - 2·coth·(-csch²) / 2
   = coth + coth·csch²
   = coth(1 + csch²) = coth·coth² = coth³  ✓
```

---

## Implementation

### New file: `symbolic-vm/src/symbolic_vm/recip_hyp_power_integral.py`

Exports:
- `sech_power_integral(n, a, b, x)` — IBP reduction, n≥0
- `csch_power_integral(n, a, b, x)` — IBP reduction, n≥0
- `coth_power_integral(n, a, b, x)` — identity reduction, n≥0

All three are pure recursive functions with no back-call to `integrate.py`,
avoiding circular imports. `_frac_ir` is defined locally (same pattern as
`hyp_power_integral.py`). `linear_to_ir` imported from
`symbolic_vm.polynomial_bridge`.

### `integrate.py` changes

1. **Import** the three public functions from `recip_hyp_power_integral`.
2. **Dispatcher** `_try_recip_hyp_power(base, exponent, x)` — checks
   `base.head ∈ {SECH, CSCH, COTH}`, integer exponent ≥ 2, linear argument.
3. **Call site** — immediately after the `_try_hyp_power` call (~line 539).

---

## Fallthrough Behaviour (deferred cases)

| Input | Reason |
|-------|--------|
| `∫ P(x)·sech^n(ax+b) dx` for `deg P ≥ 1` | poly×recip-hyp IBP deferred |
| `∫ sech^n(f(x)) dx` for non-linear `f` | non-linear argument not handled |
| `∫ sech^n(x) · csch^m(x) dx` | mixed products deferred |

---

## Package Versions

| Package | Before | After |
|---------|--------|-------|
| `symbolic-vm` | 0.35.0 | 0.36.0 |

`symbolic-ir` and `macsyma-compiler` are **unchanged** — no new IR heads or
compiler mappings needed.

---

## Files Changed

| File | Change |
|------|--------|
| `symbolic-vm/src/symbolic_vm/recip_hyp_power_integral.py` | **NEW** |
| `symbolic-vm/src/symbolic_vm/integrate.py` | import + dispatcher + call site |
| `symbolic-vm/tests/test_phase16.py` | **NEW** ≥32 tests |
| `symbolic-vm/tests/test_phase15.py` | `test_sech_squared_unevaluated` updated |
| `symbolic-vm/CHANGELOG.md` | 0.36.0 entry |
| `symbolic-vm/pyproject.toml` | version 0.36.0 |

---

## Test Coverage

`test_phase16.py` (≥32 tests):

| Class | Tests | What is verified |
|-------|-------|-----------------|
| `TestPhase16_SechPowers` | 8 | n=2,3,4,5; a=2; b=1; a=1/2; structure |
| `TestPhase16_CschPowers` | 8 | n=2,3,4,5; a=2; b=1; a=1/2; structure |
| `TestPhase16_CothPowers` | 8 | n=2,3,4,5; a=2; b=1; a=1/2; structure |
| `TestPhase16_Fallthrough` | 3 | poly×sech², non-linear csch, mixed product |
| `TestPhase16_Regressions` | 3 | Phase 15 bare, Phase 14 sinh^4, Phase 3 exp |
| `TestPhase16_Macsyma` | 4 | end-to-end via MACSYMA string interface |
