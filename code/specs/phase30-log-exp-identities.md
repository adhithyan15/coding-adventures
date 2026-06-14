# Phase 30 — Algebraic `log` and `exp` Cancellation Identities

## Context

Phase 29 (merged as PR #2149) added algebraic rules for `abs` and `sqrt`.
The elementary function handlers (`Log`, `Exp`) in `handlers.py` are still the
pure `_elementary` factory — they only do numeric fold.  A CAS user rightly
expects these simplifications to work:

```
log(exp(x))      →  x           [always true for real x]
exp(log(x))      →  x           [structural: log(x) requires x > 0]
log(x^n)         →  n*log(x)    [when x > 0 is known]
exp(n*log(x))    →  x^n         [structural: inverse of log(x^n)]
```

Phase 30 adds two new handlers — `log_handler` and `exp_handler` — that
override the `_elementary`-factory handlers via the same
`handlers.update(build_cas_handler_table())` mechanism used in Phase 29 for
`sqrt_handler`.

---

## Mathematical Rules

### `log` identities

| Pattern | Result | Condition | Reason |
|---------|--------|-----------|--------|
| `log(exp(x))` | `x` | none | `log` and `exp` are exact inverses over ℝ |
| `log(Pow(x, n))` | `n * log(x)` | `x > 0` assumed | `log(x^n) = n·log(x)` |
| Numeric fold | `IRFloat(math.log(n))` | `n > 0` | standard numeric eval |
| Special value | `0` | `n == 1` | `log(1) = 0` |
| Negative/zero arg | unevaluated | `n ≤ 0` | real log undefined |

**Why `log(exp(x)) → x` is unconditional:**
`exp` maps all of ℝ into ℝ⁺, and `log` is the exact inverse on ℝ⁺.
So `log(exp(x)) = x` for every real `x` without any assumption.

### `exp` identities

| Pattern | Result | Condition | Reason |
|---------|--------|-----------|--------|
| `exp(log(x))` | `x` | structural | Any expr containing `log(x)` requires `x > 0`; `exp(log(x)) = x` |
| `exp(Mul(n, log(x)))` | `Pow(x, n)` | structural | `e^(n·ln x) = x^n` (same domain argument) |
| `exp(Mul(log(x), n))` | `Pow(x, n)` | structural | commuted arg order |
| Numeric fold | `IRFloat(math.exp(n))` | any numeric `n` | standard numeric eval |
| Special value | `1` | `n == 0` | `exp(0) = 1` |

**Why `exp(log(x)) → x` is structural (no explicit assumption needed):**
The sub-expression `log(x)` is only meaningful when `x > 0`. Therefore any
surrounding `exp(log(x))` that the user wrote or the evaluator produced
already carries an implicit positivity constraint.  Simplifying to `x` is
safe under real-domain semantics.

---

## Files to Change

All paths under `code/packages/python/symbolic-vm/`.

| File | Change |
|------|--------|
| `code/specs/phase30-log-exp-identities.md` | **NEW** (this file) |
| `src/symbolic_vm/cas_handlers.py` | Add `EXP`/`LOG` imports; add `exp_handler` + `log_handler`; register in `build_cas_handler_table()` |
| `tests/test_phase30.py` | **NEW** ≥32 tests |
| `CHANGELOG.md` | 0.50.0 entry |
| `pyproject.toml` | Bump to 0.50.0 |

No changes to `macsyma-runtime` (`log`/`exp` already in name table and
`build_cas_handler_table()` from `symbolic_vm` is already included).
No changes to `symbolic-ir` (no new IR heads).

---

## Implementation Details

### Import additions

```python
# In the `from symbolic_ir import (` block:
    EXP,
    LOG,
```

### `log_handler`

```python
def log_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Log(x)`` — natural logarithm with algebraic simplification.

    Rules:
    1. Special value: log(1) = 0.
    2. Numeric fold: log(n) for n > 0.
    3. log(exp(x)) = x   (cancellation, always safe for real x).
    4. log(x^n) = n*log(x)  when assume(x > 0) or assume(x >= 0) active.
    5. Negative/zero numeric input: leave unevaluated (undefined in reals).
    6. Everything else: unevaluated.
    """
```

### `exp_handler`

```python
def exp_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Exp(x)`` — natural exponential with algebraic simplification.

    Rules:
    1. Special value: exp(0) = 1.
    2. Numeric fold: exp(n) for any numeric n.
    3. exp(log(x)) = x   (structural cancellation).
    4. exp(n*log(x)) = x^n  (structural; covers both Mul(n,log) and Mul(log,n)).
    5. Everything else: unevaluated.
    """
```

---

## Test Structure (`test_phase30.py`, ≥32 tests)

| Class | Count | What is verified |
|-------|-------|-----------------|
| `TestPhase30_LogExpCancel` | 6 | `log(exp(x))→x`, `log(exp(0))→0`, `log(exp(2x))→2x` |
| `TestPhase30_ExpLogCancel` | 6 | `exp(log(x))→x`, `exp(log(3))→3`, `exp(2*log(x))→x^2` |
| `TestPhase30_LogPower` | 6 | `log(x^n)→n*log(x)` with assumption; odd/no-assumption stays unevaluated |
| `TestPhase30_LogNumeric` | 5 | `log(1)→0`, `log(e)≈1`, `log(2.0)`, negative stays unevaluated |
| `TestPhase30_ExpNumeric` | 5 | `exp(0)→1`, `exp(1.0)≈e`, `exp(-1.0)`, `exp(2)` |
| `TestPhase30_Regressions` | 4 | Phase 29 abs/sqrt, Phase 28 assume/sign, Phase 3 cos(0) |
| `TestPhase30_Macsyma` | 6 | `log(exp(x))`, `exp(log(x))`, `exp(2*log(x))`, `log(x^2)` via MACSYMA surface |

---

## Verification

```bash
cd code/packages/python/symbolic-vm
.venv/bin/pytest tests/ -q          # ≥1549 tests, ≥80% coverage
.venv/bin/ruff check src/ tests/    # zero errors
```

Spot-checks:
```python
# log(exp(x))        →  x
# exp(log(x))        →  x
# exp(2*log(x))      →  Pow(x, 2)
# log(x^3) no assume →  Log(Pow(x, 3))  [unevaluated]
# assume(x>0): log(x^3) →  Mul(3, Log(x))
```
