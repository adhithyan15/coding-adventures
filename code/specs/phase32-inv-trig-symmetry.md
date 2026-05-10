# Phase 32 — Inverse Trig/Hyperbolic Odd Symmetry

## Context

Phase 31 (v0.51.0) added odd/even negation symmetry for the six primary
trig and hyperbolic functions (`sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`).
Phase 32 closes the mirror gap for the five inverse functions that admit
clean algebraic negation rules in the real domain:

| Function | Type | Rule |
|----------|------|------|
| `asin` | odd | `asin(-x) = -asin(x)` |
| `acos` | reflection | `acos(-x) = π - acos(x)` |
| `atan` | odd | `atan(-x) = -atan(x)` |
| `asinh`| odd | `asinh(-x) = -asinh(x)` |
| `atanh`| odd | `atanh(-x) = -atanh(x)` |

`acosh` is excluded: its domain is `[1, ∞)`, so `acosh(-x)` for positive `x`
is outside the real domain — no symmetry rule applies.

---

## Mathematical Rules

### Odd functions (asin, atan, asinh, atanh)

These four are odd on their natural domains:

- `asin(-x) = -asin(x)` for `x ∈ [-1, 1]`
- `atan(-x) = -atan(x)` for all real `x`
- `asinh(-x) = -asinh(x)` for all real `x`
- `atanh(-x) = -atanh(x)` for `x ∈ (-1, 1)`

**Trigger**: argument is `IRApply(NEG, (inner,))`.
**Output**: `IRApply(NEG, (handler(vm, IRApply(HEAD, (inner,))),))` — recursing
so that `asin(-(-x)) = asin(x)` collapses correctly.

### acos reflection

`acos` is not odd, but satisfies the reflection identity:

```
acos(-x) = π - acos(x)
```

This holds for `x ∈ [-1, 1]` (the domain of `acos`). The `π` constant is
represented as `IRSymbol("%pi")` — the same symbol used elsewhere in the VM
(see `_PI_SYM` already defined in `cas_handlers.py`).

**Output**: `IRApply(SUB, (IRSymbol("%pi"), IRApply(ACOS, (inner,))))`

---

## Files to Change

All paths under `code/packages/python/`.

| File | Change |
|------|--------|
| `code/specs/phase32-inv-trig-symmetry.md` | **THIS FILE** — spec |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | 5 new handlers + registration |
| `symbolic-vm/tests/test_phase32.py` | **NEW** — ≥32 tests |
| `symbolic-vm/CHANGELOG.md` | 0.52.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.52.0 |

No new IR heads needed. No changes to `symbolic-ir`, `macsyma-compiler`, or
`macsyma-runtime`.

---

## Handler Pseudocode

```python
def asin_handler(vm, expr):
    # numeric fold (preserve asin(0) = 0 exactly)
    # asin(-x) = -asin(x)
    # leave unevaluated otherwise

def acos_handler(vm, expr):
    # numeric fold (preserve acos(1) = 0 exactly)
    # acos(-x) = π - acos(x)  [uses _PI_SYM already in cas_handlers.py]
    # leave unevaluated otherwise

def atan_handler(vm, expr):
    # numeric fold (preserve atan(0) = 0 exactly)
    # atan(-x) = -atan(x)
    # leave unevaluated otherwise

def asinh_handler(vm, expr):
    # numeric fold (preserve asinh(0) = 0 exactly)
    # asinh(-x) = -asinh(x)
    # leave unevaluated otherwise

def atanh_handler(vm, expr):
    # numeric fold (preserve atanh(0) = 0 exactly)
    # atanh(-x) = -atanh(x)
    # leave unevaluated otherwise
```

---

## Test Structure (`test_phase32.py`, ≥32 tests)

| Class | Count | What is verified |
|-------|-------|-----------------|
| `TestPhase32_AsinSymmetry` | 5 | `asin(-x)`, double-neg, float, unevaluated |
| `TestPhase32_AcosReflection` | 5 | `acos(-x)→π-acos(x)`, float, double-neg |
| `TestPhase32_AtanSymmetry` | 5 | `atan(-x)`, double-neg, float, unevaluated |
| `TestPhase32_HypInvSymmetry` | 6 | `asinh(-x)`, `atanh(-x)` odd; float; double-neg |
| `TestPhase32_Regressions` | 5 | Phase 31 sin/cos, Phase 30 log/exp, sin(asin) cancel |
| `TestPhase32_Macsyma` | 6+ | `asin(-x)`, `acos(-x)`, `atan(-x)`, `asinh(-x)`, `atanh(-x)` |

---

## Verification Spot-Checks

```
asin(-x)    → Neg(Asin(x))
acos(-x)    → Sub(%pi, Acos(x))
atan(-x)    → Neg(Atan(x))
asinh(-x)   → Neg(Asinh(x))
atanh(-x)   → Neg(Atanh(x))
asin(-(-x)) → Asin(x)          [double neg via recursion]
acos(-(-x)) → Sub(%pi, Sub(%pi, Acos(x)))  [double acos reflection, stays symbolic]
asin(-0.5)  → IRFloat(math.asin(-0.5))     [numeric fold]
acos(-1)    → IRFloat(math.pi)             [numeric fold: acos(-1) = π]
atan(-0)    → IRInteger(0)                 [special value via n==0 check]
```
