# Phase 25 — Symbolic Summation (`sum` / `product`)

## Context

Phase 24 (merged as PR #2069) added definite integration via the Fundamental
Theorem of Calculus.  Phase 25 closes the last major gap in historical MACSYMA
parity by implementing symbolic summation: closed-form evaluation of
`sum(f, k, a, b)` and `product(f, k, a, b)` for the most practically important
families of summands.

This is the final phase of the planned 18–25 roadmap from the gap analysis
(`macsyma-gap-analysis-phases18-25.md`).  After this phase the system reaches
~95% parity with historical MACSYMA (1982–1994).

---

## Mathematical Background

### Summation families handled

| Family | Form | Closed Form | Notes |
|--------|------|-------------|-------|
| Constant | `c` | `c * (b - a + 1)` | c independent of k |
| Geometric finite | `r^k` | `r^a * (r^(b-a+1) - 1)/(r-1)` | r ≠ 1 constant |
| Geometric infinite | `r^k`, b=∞ | `r^a / (1 - r)` | ∣r∣ < 1 assumed |
| Power, lo=0 | `k^m`, k=0..n | Faulhaber with leading 0^m correction | m=0..5 |
| Power, lo=1 | `k^m`, k=1..n | Faulhaber polynomial in n | m=0..5 |
| Scaled power | `c * k^m` | `c * sum(k^m, k, a, b)` | linearity |
| Basel | `1/k^2`, k=1..∞ | `%pi^2/6` | Euler 1734 |
| Basel-4 | `1/k^4`, k=1..∞ | `%pi^4/90` | |
| Leibniz | `(-1)^k/(2k+1)`, k=0..∞ | `%pi/4` | alternating |
| Factorial-exp | `1/k!`, k=0..∞ | `%e` | Taylor series |
| Exp power series | `x^k/k!`, k=0..∞ | `exp(x)` | formal power series |

### Faulhaber's formulas (Σ_{k=1}^n k^m)

```
m=0: n
m=1: n(n+1)/2
m=2: n(n+1)(2n+1)/6
m=3: [n(n+1)/2]^2
m=4: n(n+1)(2n+1)(3n^2+3n−1)/30
m=5: n^2(n+1)^2(2n^2+2n−1)/12
```

For general bounds [a, b]:
```
sum(k^m, k, a, b) = F(b, m) - F(a-1, m)
```
where `F(n, m) = Σ_{k=1}^n k^m` (Faulhaber).

For lo=0: F(n, m) unchanged (0^m = 0 for m > 0, or 1 for m=0 handled separately).

### Product families handled

| Form | Closed Form | Notes |
|------|-------------|-------|
| `k`, k=1..n | `GammaFunc(n+1)` | factorial; uses existing IR head |
| `c`, k=a..b | `c^(b-a+1)` | constant factor |
| `c*k`, k=1..n | `c^n * GammaFunc(n+1)` | scaled factorial |

---

## Files to Change

All paths under `code/packages/python/`.

| File | Change |
|------|--------|
| `code/specs/phase25-symbolic-summation.md` | **NEW** (this file) |
| `symbolic-ir/src/symbolic_ir/nodes.py` | Add `SUM`, `PRODUCT` head symbols |
| `symbolic-ir/src/symbolic_ir/__init__.py` | Export `SUM`, `PRODUCT` |
| `symbolic-ir/CHANGELOG.md` | 0.12.0 entry |
| `symbolic-ir/pyproject.toml` | Bump to 0.12.0 |
| `cas-summation/` | **NEW** package |
| `macsyma-compiler/src/macsyma_compiler/compiler.py` | Add `sum`→`SUM`, `product`→`PRODUCT` |
| `macsyma-compiler/CHANGELOG.md` | 0.9.0 entry |
| `macsyma-compiler/pyproject.toml` | Bump to 0.9.0, dep `symbolic-ir>=0.12.0` |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | Add SUM + PRODUCT handlers |
| `symbolic-vm/CHANGELOG.md` | 0.45.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.45.0, add `cas-summation>=0.1.0` dep |
| `macsyma-runtime/CHANGELOG.md` | 1.16.0 entry |
| `macsyma-runtime/pyproject.toml` | Bump to 1.16.0, dep `symbolic-vm>=0.45.0` |

---

## New Package: `cas-summation` 0.1.0

### Directory layout

```
cas-summation/
  BUILD
  CHANGELOG.md
  README.md
  pyproject.toml
  required_capabilities.json
  src/cas_summation/
    __init__.py        # public API
    poly_sum.py        # Faulhaber polynomial-sum formulas
    geometric_sum.py   # geometric series (finite + infinite)
    special_sums.py    # classic convergent series (Basel, e, exp(x), …)
    product_eval.py    # finite product evaluation
    summation.py       # main dispatcher: evaluate_sum / evaluate_product
  tests/
    test_poly_sum.py
    test_geometric_sum.py
    test_special_sums.py
    test_product_eval.py
    test_summation.py
```

### Public API (`__init__.py`)

```python
from cas_summation.summation import evaluate_product, evaluate_sum

__all__ = ["evaluate_sum", "evaluate_product"]
```

### `poly_sum.py`

Core routine:

```python
def faulhaber_ir(m: int, n: IRNode) -> IRNode | None:
    """IR tree for Σ_{k=1}^n k^m, or None if m > 5."""
```

And:

```python
def poly_sum_ir(m: int, coeff: Fraction, lo: IRNode, hi: IRNode) -> IRNode | None:
    """IR for c * sum(k^m, k, lo, hi) using Faulhaber with general bounds."""
```

### `geometric_sum.py`

```python
def geometric_sum_ir(
    coeff: IRNode,
    base: IRNode,
    lo: IRNode,
    hi: IRNode,
    is_infinite: bool,
) -> IRNode:
    """IR for c * Σ_{k=lo}^{hi} base^k."""
```

Finite formula: `coeff * base^lo * (base^(hi-lo+1) - 1) / (base - 1)`
Infinite formula: `coeff * base^lo / (1 - base)`

Special case `lo=0`, infinite: `coeff / (1 - base)` (base^0 = 1).

### `special_sums.py`

Lookup table for known infinite series:

```python
def try_special_infinite(
    f: IRNode, k: IRSymbol, lo: IRNode
) -> IRNode | None:
    """Return closed form for classic infinite series, or None."""
```

Recognised patterns:
- `f = POW(k, IRInteger(-2))` and `lo = IRInteger(1)` → `%pi²/6`
- `f = POW(k, IRInteger(-4))` and `lo = IRInteger(1)` → `%pi⁴/90`
- `f = DIV(POW(IRInteger(-1), k), ADD(MUL(2,k), 1))` and `lo = IRInteger(0)` → `%pi/4`
- `f = DIV(1, GAMMA_FUNC(ADD(k, 1)))` (i.e. `1/k!`) and `lo = IRInteger(0)` → `%e`
- `f = DIV(POW(x, k), GAMMA_FUNC(ADD(k,1)))` (i.e. `x^k/k!`) and `lo=0` → `EXP(x)`

### `product_eval.py`

```python
def evaluate_product_expr(
    f: IRNode, k: IRSymbol, lo: IRNode, hi: IRNode, vm
) -> IRNode | None:
    """Closed-form product evaluation, or None if not recognised."""
```

Cases:
1. Constant in k → `f^(hi - lo + 1)`
2. `f = k` and `lo = IRInteger(1)` → `GammaFunc(ADD(hi, 1))`
3. `f = MUL(c, k)` or `f = MUL(k, c)` and `lo = 1` → `c^hi * GammaFunc(hi+1)`

### `summation.py`

```python
def evaluate_sum(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm,
) -> IRNode:
    """Return closed form of sum(f, k, lo, hi), or SUM(f, k, lo, hi) if unevaluatable."""

def evaluate_product(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm,
) -> IRNode:
    """Return closed form of product(f, k, lo, hi), or PRODUCT(f, k, lo, hi) if not."""
```

`evaluate_sum` dispatch order:
1. `_is_constant_in(f, k)` → `vm.eval(MUL(f, ADD(SUB(hi, lo), 1)))`
2. `_is_geometric(f, k)` → `geometric_sum_ir(…)`
3. `_is_power_of_k(f, k)` → `poly_sum_ir(m, coeff, lo, hi)`
4. `_is_inf(hi)` → `try_special_infinite(f, k, lo)`
5. Fallback → `IRApply(SUM, (f, k, lo, hi))`

---

## IR Changes (`symbolic-ir` → 0.12.0)

Two new head symbols in `nodes.py` (after the Phase 23 Fresnel section):

```python
# ── Phase 25 — Summation and product ──────────────────────────────────────
#
# Symbolic summation and product operations.
#
# Sum(f, k, a, b):
#   Represents Σ_{k=a}^{b} f(k).
#   Four arguments: summand, index variable, lower bound, upper bound.
#   Unevaluated form when no closed form is known; the VM returns the
#   closed form (polynomial, geometric, special constant) when recognised.
#
# Product(f, k, a, b):
#   Represents Π_{k=a}^{b} f(k).
#   Four arguments: factor, index variable, lower bound, upper bound.
#   product(k, k, 1, n) → GammaFunc(n+1)  (factorial).
SUM = IRSymbol("Sum")
PRODUCT = IRSymbol("Product")
```

Both symbols are exported from `__init__.py`.

---

## MACSYMA Compiler Changes (`macsyma-compiler` → 0.9.0)

`compiler.py` `_STANDARD_FUNCTIONS` table gains two entries:

```python
"sum": SUM,
"product": PRODUCT,
```

Import list gains `SUM`, `PRODUCT` from `symbolic_ir`.

The compiler already handles N-ary function calls generically, so no other
changes are needed — `sum(k^2, k, 1, n)` compiles to
`IRApply(SUM, (POW(k,2), k, 1, n))` automatically.

---

## `symbolic-vm` Changes (→ 0.45.0)

### New dependency

`pyproject.toml` gains `"coding-adventures-cas-summation>=0.1.0"`.

### Handler additions to `cas_handlers.py`

Two new handlers after the existing `INTEGRATE` handler:

```python
from cas_summation import evaluate_product, evaluate_sum
from symbolic_ir import PRODUCT, SUM

# ── SUM handler ───────────────────────────────────────────────────────────

def _make_sum_handler() -> Callable:
    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 4:
            raise TypeError(f"Sum expects 4 arguments, got {len(expr.args)}")
        f, k, lo, hi = expr.args
        if not isinstance(k, IRSymbol):
            return expr
        lo_eval = vm.eval(lo)
        hi_eval = vm.eval(hi)
        return evaluate_sum(f, k, lo_eval, hi_eval, vm)
    return handler


# ── PRODUCT handler ───────────────────────────────────────────────────────

def _make_product_handler() -> Callable:
    def handler(vm, expr: IRApply) -> IRNode:
        if len(expr.args) != 4:
            raise TypeError(f"Product expects 4 arguments, got {len(expr.args)}")
        f, k, lo, hi = expr.args
        if not isinstance(k, IRSymbol):
            return expr
        lo_eval = vm.eval(lo)
        hi_eval = vm.eval(hi)
        return evaluate_product(f, k, lo_eval, hi_eval, vm)
    return handler
```

Registration (in `register_all_handlers`):

```python
vm.register_handler(SUM, _make_sum_handler())
vm.register_handler(PRODUCT, _make_product_handler())
```

---

## `macsyma-runtime` Changes (→ 1.16.0)

`macsyma-runtime/pyproject.toml` dependency bump:
`coding-adventures-symbolic-vm>=0.44.0` → `>=0.45.0`.

No new MACSYMA surface syntax needed — the compiler already handles
`sum(f, k, a, b)` and `product(f, k, a, b)` through the updated
`_STANDARD_FUNCTIONS` table, and the VM handler routes to `cas-summation`.

---

## Test Structure

### Unit tests within `cas-summation/tests/` (≥40 tests)

```
test_poly_sum.py       — 10 tests: m=0..5 symbolic, concrete bounds, scaled
test_geometric_sum.py  — 10 tests: finite/infinite, fractional base, lo>0
test_special_sums.py   —  8 tests: Basel, Basel-4, Leibniz, 1/k!, x^k/k!
test_product_eval.py   —  8 tests: k, constant, scaled factorial, unevaluated
test_summation.py      —  8 tests: dispatcher integration
```

### `symbolic-vm/tests/test_phase25.py` (≥60 tests)

```python
class TestPhase25_ConstantSum     # 6 tests — const f, various bounds
class TestPhase25_GeometricSum    # 10 tests — r^k, c*r^k, infinite, lo>0
class TestPhase25_PowerSum        # 12 tests — k^m for m=0..5, bounds 1..n and 0..n
class TestPhase25_SpecialSeries   # 8 tests  — Basel, Leibniz, e, exp(x)
class TestPhase25_Product         # 8 tests  — factorial, constant, scaled
class TestPhase25_Unevaluated     # 4 tests  — return SUM/PRODUCT unchanged
class TestPhase25_Regressions     # 4 tests  — Phase 24 definite integral still works
class TestPhase25_Macsyma         # 8 tests  — end-to-end MACSYMA syntax
```

Target: ≥60 tests total, ≥80% coverage on `cas-summation` and `symbolic-vm`.

---

## MACSYMA Surface Syntax Examples

```
sum(k^2, k, 1, n);                /* n*(n+1)*(2*n+1)/6          */
sum(1/2^k, k, 0, inf);            /* 2                          */
sum(1/k^2, k, 1, inf);            /* %pi^2/6                    */
product(k, k, 1, n);              /* GammaFunc(n+1)  (= n!)     */
sum(3^k, k, 0, 5);                /* 364  (numeric bounds)      */
sum(k^0, k, 1, n);                /* n    (zeroth power)        */
sum(k, k, 1, 10);                 /* 55   (concrete)            */
```

---

## Verification

```bash
cd code/packages/python/cas-summation
.venv/bin/pytest tests/ -q            # ≥40 tests pass, ≥80% coverage

cd ../symbolic-vm
.venv/bin/pytest tests/ -q            # ≥1322 + 60 tests, ≥80% coverage
.venv/bin/ruff check src/ tests/      # zero errors
```

Spot checks:
```python
# sum(k^2, k, 1, n)          → n*(n+1)*(2*n+1)/6   (symbolic)
# sum(1/2^k, k, 0, inf)      → 2
# sum(1/k^2, k, 1, inf)      → %pi^2/6
# product(k, k, 1, n)        → GammaFunc(n+1)
# sum(3^k, k, 0, 4)          → (3^5 - 1)/(3-1) = 242/2 = 121
```

---

## Implementation Notes

- `sum(k^m, k, a, b)` with **concrete** lo/hi but large range: do NOT expand
  numerically — always use the Faulhaber closed form to keep the output compact.
- `sum(k^m, k, 1, 10)` with **concrete** n: substitute n=10 into the Faulhaber
  formula and let `vm.eval(…)` reduce it to a single integer.
- Geometric infinite sum: do not attempt to verify |r| < 1 symbolically;
  return the closed form `base^lo / (1 - base)` unconditionally, matching
  historical MACSYMA's behaviour (user responsible for convergence).
- `PRODUCT(f, k, lo, hi)` unevaluated form is returned when no pattern matches.
- No new MACSYMA grammar changes required — `sum` and `product` are already
  valid identifiers that the parser passes through as function calls.
