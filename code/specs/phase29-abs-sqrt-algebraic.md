# Phase 29 — Algebraic `abs` and `sqrt` Simplification

## Context

Phase 28 (merged as PR #2119) added assumption-aware folding for `abs` and
`sign`: after `assume(x > 0)`, `abs(x)` returns `x`.  But the handler still
leaves many *structurally obvious* simplifications unresolved:

```
abs(-x)        →  Abs(Neg(x))    [unevaluated — should be abs(x)]
abs(x^2)       →  Abs(Pow(x,2)) [unevaluated — x² ≥ 0 always]
abs(abs(x))    →  Abs(Abs(x))   [unevaluated — idempotent]
sqrt(x^2)      →  Sqrt(Pow(x,2))[unevaluated — should be abs(x)]
sqrt(x^4)      →  Sqrt(Pow(x,4))[unevaluated — should be x²]
```

Phase 29 adds **purely algebraic** rules that require no assumptions — they
hold for all real (or complex) inputs.  All changes stay in
`symbolic_vm/cas_handlers.py`; no new IR heads, no new packages.

---

## Mathematical Rules

### Algebraic `abs` rules

| Pattern | Result | Reason |
|---------|--------|--------|
| `abs(Neg(x))` | `abs(x)` | `\|{-x}\| = \|x\|` always |
| `abs(Pow(x, 2k))` for even integer `2k ≥ 2` | `Pow(x, 2k)` | `x^{2k} ≥ 0` for all real `x` |
| `abs(abs(x))` | `abs(x)` | idempotency: `\|\|x\|\| = \|x\|` |
| `abs(Mul(-1, x))` | `abs(x)` | `-x` encoded as `Mul(-1, x)` after eval |

Note: `abs(x * y)` is *not* simplified to `abs(x) * abs(y)` in general
(holds over reals but not always over complex — and we want to stay
conservative since the VM may carry complex expressions).

### Algebraic `sqrt` rules

The `sqrt` handler in `handlers.py` is the `_elementary` factory: it only
does numeric fold.  We override it in `cas_handlers.py` via the standard
`handlers.update(build_cas_handler_table())` mechanism.

| Pattern | Result | Reason |
|---------|--------|--------|
| `sqrt(Pow(x, 2))` | `abs(x)` | `√(x²) = \|x\|` over reals |
| `sqrt(Pow(x, 4))` | `Pow(x, 2)` | `√(x⁴) = x²` (always ≥ 0) |
| `sqrt(Pow(x, 2k))` for even `2k` | `Pow(x, k)` if `k ≡ 0 mod 2`, else `Abs(Pow(x, k))` | generalised even-power rule |
| `sqrt(Pow(x, n))` where `n` odd | unevaluated | result not real for all `x` |
| Numeric fold | preserved | `sqrt(4) → 2`, `sqrt(2.0) → 1.414…` |
| `sqrt(0) → 0`, `sqrt(1) → 1` | preserved | special values from `_elementary` |
| Assumption-aware: `sqrt(Pow(x, 2))` after `assume(x ≥ 0)` | `x` | can drop abs when `x` known nonneg |

#### Detailed `sqrt(x^2n)` reduction:

```
sqrt(x^{2k}) = (x^{2k})^{1/2} = x^k  when k is even  (k = 2m → x^{2m} ≥ 0)
             = abs(x^k)              when k is odd   (sign of x^k depends on sign of x)
```

So:
- `sqrt(x^2)` → `abs(x)`  (k=1, odd)
- `sqrt(x^4)` → `x^2`    (k=2, even — x^2 ≥ 0)
- `sqrt(x^6)` → `abs(x^3)` or equivalently `x^2 * abs(x)` — we emit `abs(x^3)` (k=3, odd)
- `sqrt(x^8)` → `x^4`    (k=4, even)

---

## Files to Change

All paths under `code/packages/python/symbolic-vm/`.

| File | Change |
|------|--------|
| `code/specs/phase29-abs-sqrt-algebraic.md` | **NEW** (this file) |
| `src/symbolic_vm/cas_handlers.py` | Extend `abs_handler` (new rules 3a–3c) + add `sqrt_handler` + register in `build_cas_handler_table()` |
| `tests/test_phase29.py` | **NEW** ≥32 tests |
| `CHANGELOG.md` | 0.49.0 entry |
| `pyproject.toml` | Bump to 0.49.0 |

No changes to `macsyma-runtime` (sqrt/abs already in name table), no changes
to `symbolic-ir` (no new IR heads), no changes to `macsyma-compiler`.

---

## Implementation Details

### `abs_handler` extensions (new algebraic rules)

Insert *before* the existing "leave unevaluated" fallback, *after* the
Phase 28 assumption-aware symbol rules:

```python
# Rule 3a: strip negation — abs(-x) = abs(x)
if isinstance(inner, IRApply) and inner.head == NEG:
    return abs_handler(vm, IRApply(expr.head, inner.args))

# Rule 3b: idempotency — abs(abs(x)) = abs(x)
if isinstance(inner, IRApply) and inner.head is ABS_HEAD:
    return inner

# Rule 3c: even integer power — abs(x^{2k}) = x^{2k}
if isinstance(inner, IRApply) and inner.head == POW:
    if len(inner.args) == 2 and isinstance(inner.args[1], IRInteger):
        exp = inner.args[1].value
        if isinstance(exp, int) and exp % 2 == 0 and exp >= 2:
            return inner  # x^{2k} ≥ 0 always

# Rule 3d: Mul(-1, x) from eval — abs(-1 * x) = abs(x)
if isinstance(inner, IRApply) and inner.head == MUL:
    if len(inner.args) == 2 and inner.args[0] == IRInteger(-1):
        return abs_handler(vm, IRApply(expr.head, (inner.args[1],)))
```

The `ABS_HEAD` is just `expr.head` in the context of `abs_handler` — it can
be looked up as `IRSymbol("Abs")` or captured from the outer handler.

### `sqrt_handler` (new function)

```python
def sqrt_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Sqrt(x)`` — square root with algebraic simplification.

    Rules applied in order
    ----------------------
    1. Numeric fold (preserves sqrt(4)→2, sqrt(0)→0, sqrt(1)→1).
    2. sqrt(Pow(x, 2k)):
       - k even  →  Pow(x, k)       [x^k ≥ 0 always]
       - k odd   →  Abs(Pow(x, k))  [= abs(x^k)]
       - special case k=1: sqrt(x^2) → abs(x)
    3. Assumption-aware: sqrt(Pow(x, 2)) → x  when is_nonneg(x).
    4. Leave unevaluated otherwise.
    """
    if len(expr.args) != 1:
        return expr
    arg = vm.eval(expr.args[0])
    # Rule 1: numeric fold.
    n = to_number(arg)
    if n is not None:
        if n == 0: return IRInteger(0)
        if n == 1: return IRInteger(1)
        result = math.sqrt(float(n))
        int_result = int(result)
        if int_result * int_result == n:
            return IRInteger(int_result)
        return IRFloat(result)
    # Rule 2: sqrt(x^{2k})
    if isinstance(arg, IRApply) and arg.head == POW and len(arg.args) == 2:
        base = arg.args[0]
        exp_node = arg.args[1]
        if isinstance(exp_node, IRInteger):
            n_exp = exp_node.value
            if isinstance(n_exp, int) and n_exp > 0 and n_exp % 2 == 0:
                k = n_exp // 2
                # Rule 3: assumption-aware — sqrt(x^2) → x when x ≥ 0
                if k == 1 and isinstance(base, IRSymbol):
                    if vm.assumptions.is_nonneg(base.name) is True:
                        return base
                if k % 2 == 0:
                    # k even: x^k ≥ 0 always
                    if k == 1:
                        return base
                    return IRApply(POW, (base, IRInteger(k)))
                else:
                    # k odd: result is abs(x^k)
                    if k == 1:
                        return IRApply(IRSymbol("Abs"), (base,))
                    return IRApply(IRSymbol("Abs"), (IRApply(POW, (base, IRInteger(k))),))
    # Rule 4: leave unevaluated.
    return IRApply(expr.head, (arg,))
```

Register in `build_cas_handler_table()`:

```python
"Sqrt": sqrt_handler,
```

---

## Test Structure (`test_phase29.py`, ≥32 tests)

| Class | Count | What is verified |
|-------|-------|-----------------|
| `TestPhase29_AbsNeg` | 6 | `abs(-x)`, `abs(-3)`, `abs(neg(neg(x)))`, `abs(-1*x)` |
| `TestPhase29_AbsEvenPower` | 6 | `abs(x^2)`, `abs(x^4)`, `abs((x+1)^2)`, odd power stays unevaluated |
| `TestPhase29_AbsIdempotent` | 4 | `abs(abs(x))`, `abs(abs(-x))` |
| `TestPhase29_SqrtEvenPower` | 8 | `sqrt(x^2)→abs(x)`, `sqrt(x^4)→x^2`, `sqrt(x^6)→abs(x^3)`, `sqrt(x^8)→x^4` |
| `TestPhase29_SqrtNumeric` | 4 | `sqrt(4)→2`, `sqrt(9)→3`, `sqrt(2.0)→≈1.414`, `sqrt(0)→0` |
| `TestPhase29_SqrtAssumptions` | 4 | `assume(x≥0): sqrt(x^2)→x`; without assume: stays `abs(x)` |
| `TestPhase29_Regressions` | 4 | Phase 28 assumption abs, Phase 27 inequality, Phase 3 exp, Phase 14 sinh^4 |
| `TestPhase29_Macsyma` | 4 | `sqrt(x^2)`, `abs(-x)`, `abs(x^4)` via MACSYMA surface syntax |

---

## Verification

```bash
cd code/packages/python/symbolic-vm
.venv/bin/pytest tests/ -q          # ≥1135 tests, ≥80% coverage
.venv/bin/ruff check src/ tests/    # zero errors
```

Spot-checks:
```python
# abs(-x)         →  Abs(x)
# abs(x^2)        →  Pow(x, 2)
# abs(abs(x))     →  Abs(x)
# sqrt(x^2)       →  Abs(x)
# sqrt(x^4)       →  Pow(x, 2)
# after assume(x >= 0): sqrt(x^2) → x
```
