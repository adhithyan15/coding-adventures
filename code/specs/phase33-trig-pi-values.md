# Phase 33 — Trig Special Values at Rational Multiples of π

## Context

Phases 30–32 added algebraic identity rules (log/exp cancellation, trig
symmetry, inverse-trig symmetry) to `symbolic_vm/cas_handlers.py`.  The
`sin`, `cos`, and `tan` handlers that Phase 31 introduced perform:

1. Numeric fold — `to_number(arg)` → float → `IRFloat`
2. Odd/even symmetry — `Neg(inner)` detected and handled
3. Arc-cancellation — `Asin(x)` / `Acos(x)` / `Atan(x)` inner detected

But because `%pi` is represented as `IRSymbol("%pi")`, `to_number()` returns
`None` for it, and expressions like `sin(%pi)`, `cos(%pi/2)`, `tan(%pi/4)`
fall through all three rules and stay **unevaluated**.

Phase 33 adds a fourth rule — **π-multiple detection** — to the three trig
handlers, so every rational multiple of π triggers an exact evaluation.

---

## Mathematical Content

### Values at rational multiples of π

For `q = p/d` (in lowest terms), `sin(q·π)` and `cos(q·π)` reduce to known
algebraic constants whenever `d ∈ {1, 2, 3, 4, 6}` (the "clock" denominators
that tile the unit circle into 12 equal sectors).

| q (mod 2) | sin(q·π) | cos(q·π) | tan(q·π) |
|-----------|----------|----------|----------|
| 0         | 0        | 1        | 0        |
| 1/6       | 1/2      | √3/2     | 1/√3     |
| 1/4       | √2/2     | √2/2     | 1        |
| 1/3       | √3/2     | 1/2      | √3       |
| 1/2       | 1        | 0        | ∞ (undef)|
| 2/3       | √3/2     | −1/2     | −√3      |
| 3/4       | √2/2     | −√2/2    | −1       |
| 5/6       | 1/2      | −√3/2    | −1/√3    |
| 1         | 0        | −1       | 0        |
| 7/6       | −1/2     | −√3/2    | 1/√3     |
| 5/4       | −√2/2    | −√2/2    | 1        |
| 4/3       | −√3/2    | −1/2     | √3       |
| 3/2       | −1       | 0        | ∞ (undef)|
| 5/3       | −√3/2    | 1/2      | −√3      |
| 7/4       | −√2/2    | √2/2     | −1       |
| 11/6      | −1/2     | √3/2     | −1/√3    |

**Periodicity**: sin and cos are 2π-periodic, so `q` is reduced mod 2 before
lookup.  tan is π-periodic, so `q` is reduced mod 1.

### IR representation of exact values

| Value  | IR                                  |
|--------|-------------------------------------|
| 0      | `IRInteger(0)`                      |
| ±1     | `IRInteger(±1)`                     |
| ±1/2   | `IRRational(±1, 2)`                 |
| √2/2   | `Div(Sqrt(2), 2)`                   |
| −√2/2  | `Neg(Div(Sqrt(2), 2))`              |
| √3/2   | `Div(Sqrt(3), 2)`                   |
| −√3/2  | `Neg(Div(Sqrt(3), 2))`              |
| √3     | `Sqrt(3)`                           |
| −√3    | `Neg(Sqrt(3))`                      |
| 1/√3   | `Div(Sqrt(3), 3)` (rationalised)    |
| −1/√3  | `Neg(Div(Sqrt(3), 3))`              |

`tan(π/2)` and `tan(3π/2)` are **undefined** — the handler returns the
expression unevaluated (`Tan(arg)`) rather than raising an exception, so
downstream code can decide what to do.

---

## π-Multiple Extraction

A single helper `_try_pi_multiple(arg: IRNode) -> Fraction | None` inspects
`arg` and returns `q` if `arg` is recognisably `q·%pi`, otherwise `None`.

Recognised forms (evaluated left-to-right; first match wins):

| Structural pattern | Extracted q |
|--------------------|-------------|
| `IRSymbol("%pi")` | `Fraction(1)` |
| `IRApply(NEG, (%pi,))` | `Fraction(-1)` |
| `IRApply(MUL, (n, %pi))` | `Fraction(n)` (n numeric) |
| `IRApply(MUL, (%pi, n))` | `Fraction(n)` (n numeric) |
| `IRApply(DIV, (%pi, n))` | `Fraction(1, n)` (n nonzero int/rat) |
| `IRApply(NEG, (Mul(n, %pi),))` | `Fraction(-n)` |
| `IRApply(NEG, (Div(%pi, n),))` | `Fraction(-1, n)` |

All other forms return `None` — the handler falls through to "leave
unevaluated" without attempting to guess.

---

## Handler Pseudocode

```python
def _try_pi_multiple(arg: IRNode) -> Fraction | None:
    """If arg = q·%pi, return q as a Fraction. Otherwise None."""
    ...  # structural matching as above

_SQRT2_OVER_2 = IRApply(DIV, (IRApply(SQRT, (IRInteger(2),)), IRInteger(2)))
_SQRT3_OVER_2 = IRApply(DIV, (IRApply(SQRT, (IRInteger(3),)), IRInteger(2)))
_SQRT3       = IRApply(SQRT, (IRInteger(3),))
_SQRT3_OVER_3 = IRApply(DIV, (IRApply(SQRT, (IRInteger(3),)), IRInteger(3)))

_SIN_TABLE: dict[Fraction, IRNode] = {
    Fraction(0):    IRInteger(0),
    Fraction(1,6):  IRRational(1, 2),
    Fraction(1,4):  _SQRT2_OVER_2,
    Fraction(1,3):  _SQRT3_OVER_2,
    Fraction(1,2):  IRInteger(1),
    Fraction(2,3):  _SQRT3_OVER_2,
    Fraction(3,4):  _SQRT2_OVER_2,
    Fraction(5,6):  IRRational(1, 2),
    Fraction(1):    IRInteger(0),
    Fraction(7,6):  IRRational(-1, 2),
    Fraction(5,4):  IRApply(NEG, (_SQRT2_OVER_2,)),
    Fraction(4,3):  IRApply(NEG, (_SQRT3_OVER_2,)),
    Fraction(3,2):  IRInteger(-1),
    Fraction(5,3):  IRApply(NEG, (_SQRT3_OVER_2,)),
    Fraction(7,4):  IRApply(NEG, (_SQRT2_OVER_2,)),
    Fraction(11,6): IRRational(-1, 2),
}

_COS_TABLE: dict[Fraction, IRNode] = {
    Fraction(0):    IRInteger(1),
    Fraction(1,6):  _SQRT3_OVER_2,
    Fraction(1,4):  _SQRT2_OVER_2,
    Fraction(1,3):  IRRational(1, 2),
    Fraction(1,2):  IRInteger(0),
    Fraction(2,3):  IRRational(-1, 2),
    Fraction(3,4):  IRApply(NEG, (_SQRT2_OVER_2,)),
    Fraction(5,6):  IRApply(NEG, (_SQRT3_OVER_2,)),
    Fraction(1):    IRInteger(-1),
    Fraction(7,6):  IRApply(NEG, (_SQRT3_OVER_2,)),
    Fraction(5,4):  IRApply(NEG, (_SQRT2_OVER_2,)),
    Fraction(4,3):  IRRational(-1, 2),
    Fraction(3,2):  IRInteger(0),
    Fraction(5,3):  IRRational(1, 2),
    Fraction(7,4):  _SQRT2_OVER_2,
    Fraction(11,6): _SQRT3_OVER_2,
}

_TAN_TABLE: dict[Fraction, IRNode] = {
    # undefined at 1/2 and 3/2 — omitted from table → handler returns unevaluated
    Fraction(0):    IRInteger(0),
    Fraction(1,6):  _SQRT3_OVER_3,
    Fraction(1,4):  IRInteger(1),
    Fraction(1,3):  _SQRT3,
    Fraction(2,3):  IRApply(NEG, (_SQRT3,)),
    Fraction(3,4):  IRInteger(-1),
    Fraction(5,6):  IRApply(NEG, (_SQRT3_OVER_3,)),
    Fraction(1):    IRInteger(0),
}

def sin_handler(vm, expr):
    ...
    q = _try_pi_multiple(arg)
    if q is not None:
        q_mod = q % 2
        if q_mod in _SIN_TABLE:
            return _SIN_TABLE[q_mod]
        # q*π not a special angle; fall through to leave unevaluated
    return IRApply(expr.head, (arg,))
```

`cos_handler` and `tan_handler` follow the same pattern with `_COS_TABLE`
and `_TAN_TABLE` respectively (tan reduces mod 1 due to period π).

---

## Files to Change

All paths under `code/packages/python/`.

| File | Change |
|------|--------|
| `code/specs/phase33-trig-pi-values.md` | **THIS FILE** — spec |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | `_try_pi_multiple` + 3 updated handlers + constants |
| `symbolic-vm/tests/test_phase33.py` | **NEW** — ≥32 tests |
| `symbolic-vm/CHANGELOG.md` | 0.53.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.53.0 |

No new IR heads needed. No changes to `symbolic-ir`, `macsyma-compiler`, or
`macsyma-runtime`.

---

## Test Structure (`test_phase33.py`, ≥32 tests)

| Class | Count | What is verified |
|-------|-------|-----------------|
| `TestPhase33_SinPi` | 8 | `sin(%pi)=0`, `sin(%pi/2)=1`, `sin(%pi/6)=1/2`, `sin(%pi/4)=√2/2`, `sin(%pi/3)=√3/2`, negative q, double-period |
| `TestPhase33_CosPi` | 8 | `cos(%pi)=-1`, `cos(0)=1`, `cos(%pi/2)=0`, `cos(%pi/3)=1/2`, `cos(%pi/4)=√2/2`, `cos(%pi/6)=√3/2` |
| `TestPhase33_TanPi` | 5 | `tan(%pi)=0`, `tan(%pi/4)=1`, `tan(%pi/3)=√3`, `tan(3*%pi/4)=-1`, undef at `%pi/2` |
| `TestPhase33_Extraction` | 5 | `_try_pi_multiple` for all supported patterns; unsupported → None |
| `TestPhase33_Regressions` | 4 | Phase 31 sym, Phase 32 inv-trig sym, Phase 30 log/exp |
| `TestPhase33_Macsyma` | 6 | e2e surface syntax: `sin(%pi/6)`, `cos(%pi)`, `tan(%pi/4)`, `sin(2*%pi)` |

---

## Verification Spot-Checks

```
sin(%pi)      → IRInteger(0)
sin(%pi/2)    → IRInteger(1)
sin(%pi/6)    → IRRational(1, 2)
sin(%pi/4)    → Div(Sqrt(2), 2)
sin(%pi/3)    → Div(Sqrt(3), 2)
sin(2*%pi)    → IRInteger(0)
sin(-(%pi/6)) → Neg(IRRational(1,2))        [odd symmetry then π-lookup]
cos(%pi)      → IRInteger(-1)
cos(%pi/2)    → IRInteger(0)
cos(%pi/3)    → IRRational(1, 2)
cos(%pi/4)    → Div(Sqrt(2), 2)
cos(2*%pi)    → IRInteger(1)
tan(%pi/4)    → IRInteger(1)
tan(%pi/3)    → Sqrt(3)
tan(%pi/2)    → Tan(%pi/2)                  [unevaluated — undefined]
tan(%pi)      → IRInteger(0)
```
