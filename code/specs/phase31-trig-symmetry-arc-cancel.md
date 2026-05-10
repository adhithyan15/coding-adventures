# Phase 31 — Trig Symmetry and Arc-Cancellation Identities

## Context

Phase 30 (v0.50.0) added algebraic `log`/`exp` cancellation identities — the
same mechanism used in Phase 29 for `abs`/`sqrt`. Each phase extends
`cas_handlers.py` with overrides for IR heads whose `_elementary`-factory
handlers only do numeric folding.

Phase 31 closes the same gap for the six primary trig and hyperbolic functions:
`sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`. The `_elementary` factory already
handles numeric evaluation and a few special values (e.g. `sin(0)=0`). Phase 31
adds **two orthogonal families** of algebraic rules on top of those bases:

1. **Odd/even symmetry** — detects a negated argument and either negates the
   result (odd functions) or strips the negation (even functions).
2. **Arc-function cancellation** — detects when the argument is the corresponding
   inverse function and returns the inner expression directly (structural identity,
   no assumption needed).

---

## Mathematical Rules

All rules are unconditional and verifiable by definition or construction.

### Negation symmetry

| Function | Parity | Rule |
|----------|--------|------|
| `sin` | odd  | `sin(-x) = -sin(x)` |
| `cos` | even | `cos(-x) =  cos(x)` |
| `tan` | odd  | `tan(-x) = -tan(x)` |
| `sinh`| odd  | `sinh(-x) = -sinh(x)` |
| `cosh`| even | `cosh(-x) =  cosh(x)` |
| `tanh`| odd  | `tanh(-x) = -tanh(x)` |

**Trigger**: argument is `IRApply(NEG, (inner,))` (one-argument NEG node).

**Odd handler**: recurse — return `IRApply(NEG, (sin_handler(vm, IRApply(SIN, (inner,))),))`.
Recursing means `sin(-(-x)) = sin(x)` collapses correctly across multiple negations.

**Even handler**: recurse — return `cos_handler(vm, IRApply(COS, (inner,)))`.
Stripping the negation and re-evaluating handles `cos(-0)`, float propagation, etc.

### Arc-function cancellation

| Outer | Inner | Rule |
|-------|-------|------|
| `sin` | `asin` | `sin(asin(x)) = x` |
| `cos` | `acos` | `cos(acos(x)) = x` |
| `tan` | `atan` | `tan(atan(x)) = x` |
| `sinh`| `asinh`| `sinh(asinh(x)) = x` |
| `cosh`| `acosh`| `cosh(acosh(x)) = x` |
| `tanh`| `atanh`| `tanh(atanh(x)) = x` |

**Justification**:
- `sin(asin(x))`: `asin` maps `[-1,1]→[-π/2,π/2]`; `sin` restricted to that
  interval is injective with left inverse `asin`, so `sin∘asin = id`. Structural
  — no assumption needed.
- `cos(acos(x))`: `acos` maps `[-1,1]→[0,π]`; `cos∘acos = id` similarly.
- `tan(atan(x))`: `atan` maps `ℝ→(-π/2,π/2)` where `tan` is defined; `tan∘atan = id`.
- Hyperbolic analogues follow the same logic: each arc-function is the exact
  left inverse of its base function on the principal branch.

**Trigger**: argument is `IRApply(ASIN/ACOS/…, (inner,))` with exactly one arg.
Return `inner` directly.

---

## Files to Change

All paths under `code/packages/python/`.

| File | Change |
|------|--------|
| `code/specs/phase31-trig-symmetry-arc-cancel.md` | **THIS FILE** — spec |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | Add 6 handlers + imports + registration |
| `symbolic-vm/tests/test_phase31.py` | **NEW** — ≥32 tests |
| `symbolic-vm/CHANGELOG.md` | 0.51.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.51.0 |

No new IR heads needed. No changes to `symbolic-ir`, `macsyma-compiler`, or
`macsyma-runtime`.

---

## Handler Pseudocode

### `sin_handler`

```python
def sin_handler(vm, expr):
    if len(expr.args) != 1: return expr
    arg = vm.eval(expr.args[0])
    # Numeric fold (pass to _elementary base)
    n = to_number(arg)
    if n is not None:
        return IRFloat(math.sin(float(n)))  # special 0→0 already handled
    # Odd symmetry: sin(-x) = -sin(x)
    if isinstance(arg, IRApply) and arg.head == NEG and len(arg.args) == 1:
        return IRApply(NEG, (sin_handler(vm, IRApply(SIN, (arg.args[0],))),))
    # Arc-cancellation: sin(asin(x)) = x
    if isinstance(arg, IRApply) and arg.head == ASIN and len(arg.args) == 1:
        return arg.args[0]
    return IRApply(expr.head, (arg,))
```

`cos_handler`, `tan_handler`, `sinh_handler`, `cosh_handler`, `tanh_handler`
follow the identical pattern — even functions strip NEG and recurse without
wrapping in NEG.

---

## Test Structure (`test_phase31.py`, ≥32 tests)

| Class | Count | Coverage |
|-------|-------|----------|
| `TestPhase31_SinSymmetry` | 6 | `sin(-x)`, `sin(-0.5)`, `sin(--x)`, `sin(-expr)` |
| `TestPhase31_CosSymmetry` | 6 | `cos(-x)`, `cos(-0.5)`, `cos(--x)`, `cos(-expr)` |
| `TestPhase31_TanSymmetry` | 4 | `tan(-x)`, `tan(-0.5)`, `tan(-expr)` |
| `TestPhase31_HypSymmetry` | 6 | sinh/cosh/tanh odd/even on `-x` and `-0.5` |
| `TestPhase31_ArcCancel` | 8 | all 6 arc-cancellations + two with compound inner |
| `TestPhase31_Regressions` | 4 | Phase 30 log/exp, Phase 29 abs/sqrt, Phase 3 exp(2x) |
| `TestPhase31_Macsyma` | 4+ | `sin(-x)`, `cos(-x)`, `sin(asin(x))`, `tanh(atanh(y))` |

---

## Verification Spot-Checks

```
sin(-x)           → Neg(Sin(x))     [via odd symmetry]
cos(-x)           → Cos(x)          [via even symmetry, NEG stripped]
tan(-y)           → Neg(Tan(y))
sinh(-z)          → Neg(Sinh(z))
cosh(-z)          → Cosh(z)
tanh(-t)          → Neg(Tanh(t))
sin(asin(x))      → x               [arc-cancel]
cos(acos(x))      → x
tan(atan(x))      → x
sinh(asinh(x))    → x
cosh(acosh(x))    → x
tanh(atanh(x))    → x
sin(-(-x))        → Sin(x)          [double neg, via recursion]
sin(-3.14)        → IRFloat(math.sin(-3.14))  [numeric fallthrough]
```
