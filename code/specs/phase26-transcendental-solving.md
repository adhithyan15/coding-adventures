# Phase 26 — Transcendental Equation Solving

## Context

Phase 25 completed symbolic summation (`sum`/`product`). The original gap-analysis
document (`macsyma-gap-analysis-phases18-25.md`) labelled transcendental solving as
"Phase 24" but the commit labelled `phase24` in the repository implemented definite
integration via FTC instead. This spec picks up the unimplemented work:
extend `cas-solve` to handle the most important transcendental equation families.

Target: `cas-solve` 0.7.0 · `symbolic-ir` 0.13.0 · `symbolic-vm` 0.46.0 ·
`macsyma-runtime` 1.17.0.

---

## Mathematical Families Covered

### 26a — Trigonometric equations

For `sin(ax+b) = c`, `cos(ax+b) = c`, `tan(ax+b) = c` with linear argument
`ax+b` and constant `c` (w.r.t. the solve variable):

```
sin(u) = c  →  u = arcsin(c) + 2k·π   and   u = π − arcsin(c) + 2k·π
cos(u) = c  →  u = arccos(c) + 2k·π   and   u = −arccos(c) + 2k·π
tan(u) = c  →  u = arctan(c) + k·π
```

`k` is a new free-integer constant `%k` (IR head `FreeInteger`).
After finding the argument values, solve `ax+b = val` for x (linear).

### 26b — Exponential / logarithmic equations

```
exp(u) = c  →  u = log(c)
log(u) = c  →  u = exp(c)
```

Both also support a linear argument: `exp(ax+b) = c` → `ax+b = log(c)` → `x` solved.

### 26c — Lambert W equations

Pattern: `f·exp(f) = c` where `f` is linear in the solve variable.

```
x·exp(x) = c     →  x = W(c)          [LambertW head]
(ax+b)·exp(ax+b) = c  →  ax+b = W(c)  →  x = (W(c)−b)/a
x^x = c          →  x = exp(W(log(c)))  [via x·log(x)·exp(x·log(x))=log(c)]
```

New IR head `LambertW`. Numeric evaluation: `lambert_w_handler` evaluates
`LambertW(n)` when `n` is an integer or float using Newton's method on
`w·exp(w) = n`.

### 26d — Hyperbolic equations

```
sinh(u) = c  →  u = asinh(c)            (unique inverse)
cosh(u) = c  →  u = acosh(c)  and  u = −acosh(c)
tanh(u) = c  →  u = atanh(c)            (unique inverse)
```

Same linear-argument pattern as trig above.

### 26e — Compound forms (polynomial-in-transcendental substitution)

When the equation is a polynomial in some transcendental function `f(x)`:

```
sin(x)^2 + sin(x) = 0     →  u = sin(x),  u^2 + u = 0  →  u ∈ {−1, 0}
                              → x from sin(x)=−1  and  sin(x)=0

exp(x)^2 − 3·exp(x) + 2 = 0  →  u = exp(x),  u^2 − 3u + 2 = 0  →  u ∈ {1, 2}
                                → x from exp(x)=1 → x=0  and  exp(x)=2 → x=log(2)
```

The solver detects this by trying to replace `f(var)` with a fresh symbol `u`,
checking if the result is a rational polynomial in `u`, solving that polynomial,
then solving `f(var) = u_sol` for each root (via the same transcendental dispatcher,
recursively).  Only fires when the entire expression (minus constant term) is
expressible as a polynomial in exactly one `f(var)`.

---

## New IR Heads (symbolic-ir 0.13.0)

```python
FREE_INTEGER = IRSymbol("FreeInteger")   # %k — free integer in periodic solutions
LAMBERT_W    = IRSymbol("LambertW")      # W(x) — principal Lambert W function
```

Both exported from `symbolic_ir/__init__.py`.

---

## New File: `cas-solve/src/cas_solve/transcendental.py`

Public entry point:

```python
def try_solve_transcendental(eq_ir: IRNode, var: IRSymbol) -> list[IRNode] | None:
    """Try to solve eq_ir (Equal(lhs,rhs) or bare expr=0) for var.
    Returns a list of IR solution nodes, or None if not recognised.
    """
```

Internal helpers (not exported):
- `_try_func_eq_const(func_side, const_side, var)` — handles 26a/26b/26d
- `_try_lambert(lhs, rhs, var)` — handles 26c
- `_try_compound(lhs, rhs, var)` — handles 26e
- `_extract_linear(arg, var)` → `(a: Fraction, b: Fraction) | None`
- `_solve_linear_for_val(a, b, val_ir, var)` → `IRNode`
- `_is_const_wrt(node, var)` → `bool`
- `_frac_ir(c: Fraction)` → `IRNode`

---

## Changes to symbolic-vm 0.46.0

### `cas_handlers.py`

Import additions:
```python
from symbolic_ir import FREE_INTEGER, LAMBERT_W
from cas_solve.transcendental import try_solve_transcendental as _try_transcendental
```

In `solve_handler`, after `coeffs = _ir_to_fraction_poly(poly_ir, var_ir)` returns
`None` (line ~1068), before the existing fallback `return expr`:

```python
if coeffs is None:
    # Phase 26: try transcendental families before giving up.
    trans_sols = _try_transcendental(eq_ir, var_ir)
    if trans_sols is not None:
        return IRApply(IRSymbol("List"), tuple(trans_sols))
    return expr
```

New `lambert_w_handler`:

```python
def lambert_w_handler(_vm: VM, expr: IRApply) -> IRNode:
    """LambertW(x) — principal branch of the Lambert W function.
    Evaluates numerically when x is a rational or float constant.
    Returns unevaluated for symbolic x.
    """
```

Register in `build_cas_handler_table()`:
```python
"LambertW": lambert_w_handler,
```

---

## Changes to macsyma-runtime 1.17.0

### `cas_handlers.py`

Import and re-register `lambert_w_handler` from `symbolic_vm.cas_handlers`.

### `name_table.py`

```python
"lambert_w": LAMBERT_W,   # lambert_w(x) → LambertW(x)
```

The `%k` free integer appears only in solve *output*, not as a user-input
function; no name-table entry needed.

---

## Test Structure (`test_phase26.py`, ≥50 tests)

| Class | Tests |
|-------|-------|
| `TestPhase26_Trig` | 14 — sin/cos/tan, a=1, a=2, b≠0, a=1/2; structure checks (ASIN in result) |
| `TestPhase26_ExpLog` | 8 — exp(x)=c, log(x)=c, exp(2x+1)=c, log(3x-1)=c |
| `TestPhase26_Hyp` | 8 — sinh/cosh/tanh; a=1, a=2 |
| `TestPhase26_LambertW` | 6 — x·exp(x)=1 (numeric approx), (2x+1)·exp(2x+1)=c, structure |
| `TestPhase26_Compound` | 8 — sin^2+sin=0, cos^2−cos=0, exp^2−3exp+2=0, tanh^2−1=0 |
| `TestPhase26_Fallthrough` | 4 — non-linear mixed, degree>4 (uses NSolve), `solve(x^2+y,x)` |
| `TestPhase26_Regressions` | 4 — Phase 25 sum(k^2,k,1,n), Phase 24 FTC, Phase 23 erf, polynomial solve |
| `TestPhase26_Macsyma` | 8 — solve(sin(x)=1/2,x), solve(exp(2*x)=3,x), solve(log(x+1)=2,x), etc. |

---

## Files Changed

| File | Change |
|------|--------|
| `code/specs/phase26-transcendental-solving.md` | **NEW** this spec |
| `symbolic-ir/src/symbolic_ir/nodes.py` | Add `FREE_INTEGER`, `LAMBERT_W` |
| `symbolic-ir/src/symbolic_ir/__init__.py` | Export both |
| `symbolic-ir/pyproject.toml` | Bump to 0.13.0 |
| `symbolic-ir/CHANGELOG.md` | 0.13.0 entry |
| `cas-solve/src/cas_solve/transcendental.py` | **NEW** |
| `cas-solve/src/cas_solve/__init__.py` | Export `try_solve_transcendental`, `FREE_INTEGER`, `LAMBERT_W` |
| `cas-solve/pyproject.toml` | Bump to 0.7.0 |
| `cas-solve/CHANGELOG.md` | 0.7.0 entry |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | Import + wire in solve_handler + lambert_w_handler |
| `symbolic-vm/pyproject.toml` | Bump to 0.46.0 |
| `symbolic-vm/CHANGELOG.md` | 0.46.0 entry |
| `symbolic-vm/tests/test_phase26.py` | **NEW** ≥50 tests |
| `macsyma-runtime/src/macsyma_runtime/cas_handlers.py` | Register lambert_w_handler |
| `macsyma-runtime/src/macsyma_runtime/name_table.py` | `"lambert_w"` → `LAMBERT_W` |
| `macsyma-runtime/pyproject.toml` | Bump to 1.17.0 |
| `macsyma-runtime/CHANGELOG.md` | 1.17.0 entry |

No new package needed — `transcendental.py` lives inside `cas-solve`.

---

## Implementation Order

1. Spec ← you are here
2. `symbolic-ir` heads
3. `cas-solve/transcendental.py` + unit tests there
4. Wire into `symbolic-vm`
5. Wire into `macsyma-runtime`
6. `test_phase26.py`
7. Bump changelogs + versions
8. pytest + ruff → clean
9. Commit → `/security-review` → push → PR → `/babysit-pr`
