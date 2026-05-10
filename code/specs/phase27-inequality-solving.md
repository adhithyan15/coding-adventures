# Phase 27 — Polynomial Inequality Solving

## Motivation

After Phase 26 (transcendental equation solving), the CAS can solve polynomial
equations up to degree 4 and a broad range of transcendental equations.  The
natural complement is **inequality solving**: given a polynomial expression and a
strict or non-strict comparison with a constant, find all real values of the
variable that satisfy the condition.

```
solve(x^2 - 1 > 0, x)          →  [x < -1, x > 1]
solve(x^2 - 3*x + 2 <= 0, x)   →  [and(x >= 1, x <= 2)]
solve(2*x + 5 < 0, x)          →  [x < -5/2]
```

This is Phase 27 of the symbolic-vm roadmap.

---

## Scope

### Supported

* Polynomial inequalities of degree 1–4 with rational coefficients:
  `p(x) > 0`, `p(x) >= 0`, `p(x) < 0`, `p(x) <= 0`
* The right-hand side need not be zero; `p(x) > q(x)` is handled by
  normalising to `p(x) − q(x) > 0` before analysis.
* `solve(ineq, x)` surface syntax via MACSYMA (no new compiler changes needed).

### Not covered in this phase

* Polynomial degree > 4.
* Transcendental inequalities (`sin(x) > 0`).
* System of inequalities (multiple constraints simultaneously).
* Complex-valued polynomials.

---

## Mathematical Algorithm

### Step 1 — Normalise direction

Given `lhs op rhs` (op ∈ {<, >, ≤, ≥}):

```
f(x) = lhs − rhs
```

Determine the desired sign from `op`:

| op  | want_sign | strict |
|-----|-----------|--------|
| `>` | positive  | yes    |
| `>=`| positive  | no     |
| `<` | negative  | yes    |
| `<=`| negative  | no     |

### Step 2 — Extract polynomial

Use `symbolic_vm.polynomial_bridge.to_rational(f, x)` to extract the
ascending-degree rational coefficient tuple `(c₀, c₁, …, cₙ)`.

Fall through to unevaluated if `f` is not a polynomial in `x`.

### Step 3 — Find real roots numerically

Convert ascending-degree tuple to descending-degree float coefficients
and call `cas_solve.durand_kerner.nsolve_poly`.  Filter roots where
`|Im(r)| < 1e-8` to obtain the real roots `r₁ ≤ r₂ ≤ … ≤ rₙ`.

For degrees 1–2, also extract exact Fraction roots (for exact IR output).
For degrees 3–4, use the numeric roots as boundary points and reconstruct
their exact IR form from the symbolic solvers (`solve_cubic`, `solve_quartic`)
where possible; otherwise emit `IRFloat`.

### Step 4 — Sign analysis

Create the boundary sequence:

```
−∞  r₁  r₂  …  rₙ  +∞
```

For each open interval `(rᵢ, rᵢ₊₁)` pick the midpoint as test point.
For the unbounded intervals use `r₁ − 1` and `rₙ + 1`.

Evaluate `f` at the test point using Horner's rule on the float coefficients.
Record the sign: `+1`, `0`, or `−1`.

### Step 5 — Build solution set

The solution is the union of all intervals where the sign matches
`want_sign`.

**Boundary treatment**:
* Strict inequality: roots themselves are excluded (they are zeroes of f).
* Non-strict inequality: roots are included (f = 0 satisfies f ≥ 0 and f ≤ 0).

**Interval representations** (x is the variable IR symbol, a < b are roots
expressed as IR nodes):

| Interval            | IR form                                     |
|---------------------|---------------------------------------------|
| `(−∞, a)`           | `Less(x, a)`                                |
| `(−∞, a]`           | `LessEqual(x, a)`                           |
| `(a, +∞)`           | `Greater(x, a)`                             |
| `[a, +∞)`           | `GreaterEqual(x, a)`                        |
| `(a, b)`            | `And(Greater(x, a), Less(x, b))`            |
| `[a, b]`            | `And(GreaterEqual(x, a), LessEqual(x, b))`  |
| `(a, b]`            | `And(Greater(x, a), LessEqual(x, b))`       |
| `[a, b)`            | `And(GreaterEqual(x, a), Less(x, b))`       |
| entire real line    | `GreaterEqual(IRInteger(0), IRInteger(0))`  |
| empty set           | `[]` (empty list returned to solve_handler) |

**IR heads used**: `LESS`, `GREATER`, `LESS_EQUAL`, `GREATER_EQUAL`, `AND`
(all already defined in `symbolic_ir`).

---

## Public API

New module `cas_solve/inequality.py`, one public function:

```python
def try_solve_inequality(
    ineq_ir: IRNode,
    var: IRSymbol,
) -> list[IRNode] | None:
    """Try to solve a polynomial inequality in one variable.

    ineq_ir must be IRApply with head in {Less, Greater, LessEqual, GreaterEqual}.
    Returns a list of condition IR nodes (each representing one disjoint interval),
    or None if the pattern is not recognised / the polynomial bridge is unavailable.

    An empty list means no real solutions exist.
    A list containing GreaterEqual(0, 0) means all reals are solutions.
    """
```

---

## Files Changed

| File | Change |
|------|--------|
| `code/specs/phase27-inequality-solving.md` | NEW (this file) |
| `cas-solve/src/cas_solve/inequality.py` | NEW |
| `cas-solve/src/cas_solve/__init__.py` | export `try_solve_inequality` |
| `cas-solve/tests/test_inequality.py` | NEW (≥28 tests) |
| `cas-solve/pyproject.toml` | 0.7.0 → 0.8.0 |
| `cas-solve/CHANGELOG.md` | 0.8.0 entry |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | inequality dispatch in `solve_handler` |
| `symbolic-vm/tests/test_phase27.py` | NEW (≥20 tests) |
| `symbolic-vm/pyproject.toml` | 0.46.0 → 0.47.0; cas-solve ≥ 0.8.0 |
| `symbolic-vm/CHANGELOG.md` | 0.47.0 entry |
| `macsyma-runtime/pyproject.toml` | 1.17.0 → 1.18.0; symbolic-vm ≥ 0.47.0 |
| `macsyma-runtime/CHANGELOG.md` | 1.18.0 entry |

No changes to `symbolic-ir` (no new IR heads) or `macsyma-compiler`
(no new surface syntax; `solve` already exists).

---

## Examples

```macsyma
solve(x - 1 > 0, x);             →  [x > 1]
solve(x - 1 >= 0, x);            →  [x >= 1]
solve(x^2 - 1 > 0, x);           →  [x < -1, x > 1]
solve(x^2 - 1 >= 0, x);          →  [x <= -1, x >= 1]
solve(x^2 - 1 < 0, x);           →  [and(x > -1, x < 1)]
solve(x^2 - 1 <= 0, x);          →  [and(x >= -1, x <= 1)]
solve(x^2 - 3*x + 2 <= 0, x);    →  [and(x >= 1, x <= 2)]
solve(x^2 - 2*x + 1 > 0, x);     →  [x < 1, x > 1]   (double root)
solve(x^2 + 1 > 0, x);           →  [0 >= 0]          (all reals)
solve(x^2 + 1 < 0, x);           →  []                (no solution)
```

---

## Test Plan (`test_inequality.py` — ≥28 tests)

| Class | Tests |
|-------|-------|
| `TestLinearIneq` | x>1, x>=1, x<1, x<=1, negative slope, shifted |
| `TestQuadIneqTwoRoots` | x²-1>0, x²-1>=0, x²-1<0, x²-1<=0, x²-3x+2<0, x²-3x+2<=0 |
| `TestQuadIneqDoubleRoot` | (x-1)²>0, (x-1)²>=0 (all reals) |
| `TestQuadIneqNoRoots` | x²+1>0 (all reals), x²+1<0 (empty) |
| `TestHighDegree` | cubic/quartic: x³-x>0, x⁴-1>0 |
| `TestFallthrough` | non-polynomial arg → None, Equal head → None |

---

## Verification Spot-checks

```python
# Linear
try_solve_inequality(Greater(Sub(x, 1), Zero), x) == [Greater(x, One)]

# Quadratic, two roots
result = try_solve_inequality(Greater(Sub(Pow(x,2), One), Zero), x)
# result should contain Less(x, -1) and Greater(x, 1)

# Quadratic, interval
result = try_solve_inequality(LessEqual(Sub(Sub(Pow(x,2),Mul(3,x)),Neg(2)), Zero), x)
# should contain And(GreaterEqual(x, 1), LessEqual(x, 2))
```
