# MACSYMA Completion Roadmap

> **Status**: Living document. Tracks what is implemented, what is
> specified but not built, and what needs a new spec. Updated as
> work lands.

## Guiding principle

Every operation must live at the IR layer — in a `cas-*` package
wired into `SymbolicBackend` — before it appears in any language
frontend. MACSYMA's `factor(x^2-1)` maps to `Factor(x^2-1)` IR;
a future Maple `factor(x^2-1)` maps to the same `Factor(x^2-1)` IR;
both share the same handler without touching each other's code.

The "Russian nesting dolls" stack:

```
SymbolicBackend          ← universal algebraic substrate
  └── MacsymaBackend     ← MACSYMA surface: Display/Suppress/Kill/Ev
  └── (future) MapleBackend
  └── (future) MathematicaBackend
```

New CAS operations always go in `SymbolicBackend`. Language-specific
syntax differences go in the frontend name table and backend subclass.

---

## Current status (as of symbolic-vm 0.19.0 / macsyma-runtime 0.2.0)

### Fully working end-to-end

| Area                | IR heads                                         | Package              |
|---------------------|--------------------------------------------------|----------------------|
| Arithmetic          | `Add Mul Pow Neg Sub Div Inv`                    | `symbolic-vm` (built-in) |
| Variables / functions | `Assign Define`                                | `symbolic-vm` (built-in) |
| Simplification      | `Simplify Expand`                                | `cas-simplify` |
| Substitution        | `Subst`                                          | `cas-substitution` |
| Factoring           | `Factor` (Phase 1–2: rational-root test)         | `cas-factor` |
| Solving             | `Solve` (linear + quadratic over Q)              | `cas-solve` |
| List operations     | `Length First Rest Last Append Reverse Range Map Apply Select Sort Part Flatten Join` | `cas-list-operations` |
| Matrix operations   | `Matrix Transpose Determinant Inverse`           | `cas-matrix` |
| Limits              | `Limit` (direct substitution)                    | `cas-limit-series` |
| Taylor series       | `Taylor` (polynomial expressions)                | `cas-limit-series` |
| Differentiation     | `D`                                              | `symbolic-vm` |
| Integration         | `Integrate` (Risch Phases 1–13)                  | `symbolic-vm` |
| Numeric ops         | `Abs Floor Ceiling Mod Gcd Lcm`                  | `cas-handlers` (in `symbolic-vm`) |
| Constants           | `%pi %e`                                         | `macsyma-runtime` |
| REPL mechanics      | `;` / `$` terminators, history `%/%iN/%oN`, `kill`, `ev(numer)` | `macsyma-runtime` |

---

## Group A — Extend existing packages (specs exist, Phase N not yet built)

These are specified in their respective `.md` files; the listed phases
are confirmed unimplemented.

### A1 · `cas-factor` Phases 3–4

**Spec**: `cas-factor.md` → "Phase 3: Kronecker" and "Phase 4: Berlekamp-Zassenhaus-Hensel".

**Gap**: The current implementation (Phase 2, rational-root test) correctly
factors polynomials whose irreducible factors are all linear over Q.
It leaves `x^4 + 1`, `x^4 + x^2 + 1`, and similar non-linear irreducibles
unevaluated even though they can be factored over Q (`x^4+1` is irreducible,
so that case is correct, but `x^4-4 = (x^2-2)(x^2+2)` should factor and does
not today).

**What to implement**:
- `Phase 3 (Kronecker)`: evaluate `p(x)` at `deg+1` integer points, enumerate
  candidate divisor coefficients, verify by polynomial division. Handles all
  polynomials up to degree 6 reliably.
- `Phase 4 (Berlekamp + Zassenhaus + Hensel)`: factor `p mod small_prime`
  in GF(p)[x], Hensel-lift to Z[x], combinatorially reconstruct. Production
  algorithm for arbitrary degree.

**New IR heads**: none (still `Factor`).

**Test cases to add**:
- `Factor(x^4 - 4)` → `Mul(Sub(Pow(x,2), 2), Add(Pow(x,2), 2))`.
- `Factor(x^6 - 1)` → `(x-1)(x+1)(x^2+x+1)(x^2-x+1)`.
- `Factor(x^4 + 1)` → unevaluated (irreducible over Q).

---

### A2 · `cas-solve` Phases 3–5

**Spec**: `cas-solve.md` → "Cubic and quartic", "Degree ≥ 5", "Linear systems".

**Gap**: Current implementation handles degree 1 and 2 only.

**What to implement**:

| Phase | Algorithm | New IR heads |
|-------|-----------|--------------|
| A2a — Cubic | Cardano's formula. Returns 3 roots (real + complex conjugate pair when discriminant < 0). Requires `cas-complex` for the complex case. | `Cbrt` (cube root, for Cardano's formula output) |
| A2b — Quartic | Ferrari's method (reduce to cubic resolvent). Returns 4 roots. | none beyond A2a |
| A2c — Degree ≥ 5 | `NSolve`: Durand–Kerner (Weierstrass) iteration, returns `IRFloat` roots. | `NSolve` |
| A2d — Linear systems | `Solve([eq1, eq2, ...], [x, y, ...])`: Gaussian elimination with `Fraction` coefficients. Returns `List(Rule(x, val), ...)`. | `Rule` (already in symbolic-ir standard heads), `RuleDelayed` |

**Dependency note**: A2a and A2b should be implemented after `cas-complex`
(spec: `cas-complex.md`) so that complex roots can be returned as
`ImaginaryUnit`-containing IR expressions rather than floats.

**Test cases to add** (see `cas-solve.md` test strategy):
- `Solve(x^3 - 6*x^2 + 11*x - 6, x)` → `[1, 2, 3]` (all real roots).
- `Solve(x^3 + 1, x)` → `[-1, complex roots using %i]`.
- `Solve([x + y = 3, x - y = 1], [x, y])` → `[Rule(x, 2), Rule(y, 1)]`.
- `NSolve(x^5 + x + 1, x)` → 5 `IRFloat` roots.

---

### A3 · `cas-simplify` — `Collect`, `Together`, `Apart`, polynomial `Expand`

**Spec**: `cas-simplify.md` → `Collect`, `Together`, `Apart`, and the
polynomial-distribution pass of `Expand`.

**Gap**: Current `Expand` only calls `canonical()` (normalizes sort order /
flattens); it does not distribute `Mul` over `Add` (e.g.,
`(x+1)*(x+2)` stays unexpanded).

**What to implement**:

| Head      | Algorithm |
|-----------|-----------|
| `Expand` (full) | Convert sub-expressions to polynomial via `polynomial-bridge`, multiply out, convert back. |
| `Collect` | `Collect(expr, x)`: group terms by powers of `x`, return `IRApply(Add, [Mul(coeff, Pow(x,n)), ...])`. |
| `Together` | Combine fractions over common denominator: GCD of denominators, multiply out. |
| `Apart`   | Partial-fraction decomposition via `polynomial-bridge` (already implemented in hermite/Rothstein-Trager; surface it here). |
| `RatSimplify` | `ratsimp` in MACSYMA: cancel common polynomial factors in numerator and denominator. |

**New IR heads**: `Collect`, `Together`, `Apart`, `RatSimplify`.

**MACSYMA name table additions**:
```python
{"collect":   IRSymbol("Collect"),
 "together":  IRSymbol("Together"),
 "partfrac":  IRSymbol("Apart"),
 "ratsimp":   IRSymbol("RatSimplify")}
```

---

## Group B — New packages (no implementation exists yet)

These need both the package and the spec to be implemented.
Specs are written; implementation is pending.

### B1 · `cas-trig` — Trigonometric simplification

**Spec**: `cas-trig.md` ← written.

**Priority**: High. `trigsimp` and `trigexpand` come up constantly in
a real MACSYMA session.

**New IR heads**: `TrigSimplify`, `TrigExpand`, `TrigReduce`.

**MACSYMA name table**:
```python
{"trigsimp":   IRSymbol("TrigSimplify"),
 "trigexpand": IRSymbol("TrigExpand"),
 "trigreduce": IRSymbol("TrigReduce")}
```

**Integration into `SymbolicBackend`**: call `build_trig_handler_table()`
in `SymbolicBackend.__init__` alongside `build_cas_handler_table()`.
Add the trig special-value rules to the rules list so `Sin(Pi)` → `0`
fires automatically inside `Simplify`.

---

### B2 · `cas-complex` — Complex number support

**Spec**: `cas-complex.md` ← written.

**Priority**: High. Needed by `cas-solve` for cubic/quartic roots.

**New IR heads**: `Re`, `Im`, `Conjugate`, `Arg`, `RectForm`, `PolarForm`.

**New constant**: `ImaginaryUnit` pre-bound in every backend.

**Backend wiring**: `COMPLEX_SIMPLIFY_RULES` added to `SymbolicBackend`
rule list so `ImaginaryUnit^2 → -1` fires automatically.

---

### B3 · `cas-number-theory` — Integer number theory

**Spec**: `cas-number-theory.md` ← written.

**Priority**: Medium. Needed for `ifactor`, `primep`, `totient`.

**New IR heads**: `IsPrime`, `NextPrime`, `PrevPrime`, `FactorInteger`,
`Divisors`, `Totient`, `MoebiusMu`, `JacobiSymbol`, `ChineseRemainder`,
`IntegerLength`.

---

## Group C — MACSYMA wiring (language-layer, no new IR needed)

These items require changes to `macsyma-runtime` or `macsyma-compiler`
but no new CAS packages.

### C1 · `%i` constant pre-binding

**What**: Once `cas-complex` exists, add to `MacsymaBackend.__init__`:

```python
from cas_complex import IMAGINARY_UNIT
self._env["%i"] = IMAGINARY_UNIT
```

Add to `MACSYMA_NAME_TABLE`:
```python
{"%i": IRSymbol("ImaginaryUnit")}
```

---

### C2 · `makelist` → `MakeList` head

**What**: MACSYMA's `makelist(expr, var, n)` generates a list by
evaluating `expr` for `var = 1, 2, ..., n`. Equivalent forms:
`makelist(expr, var, from, to)` and `makelist(expr, var, from, to, step)`.

**IR head**: `MakeList(expr, var, from, to, step)`.

**Implementation**: Add to `cas-list-operations`:

```python
def make_list(
    expr: IRNode, var: IRSymbol,
    start: int, stop: int, step: int = 1
) -> IRNode:
    from cas_substitution import subst
    results = [
        subst(IRInteger(i), var, expr)
        for i in range(start, stop + 1, step)
    ]
    return IRApply(LIST, tuple(results))
```

The VM handler evaluates each substituted expression through `vm.eval()`.

**MACSYMA name table**:
```python
{"makelist": IRSymbol("MakeList")}
```

---

### C3 · `ev` flag improvements

**What**: `Ev(expr, flag1, flag2, ...)` in MACSYMA re-evaluates `expr`
with specified flags. Current implementation only honors `numer`.

**Flags to add**:

| Flag       | Meaning                                         | Implementation |
|------------|-------------------------------------------------|----------------|
| `expand`   | Apply `Expand` to result before returning.      | `vm.eval(IRApply(EXPAND, (result,)))` |
| `factor`   | Apply `Factor` to result.                       | `vm.eval(IRApply(FACTOR, (result,)))` |
| `ratsimp`  | Apply `RatSimplify` to result.                  | Requires A3 above |
| `trigsimp` | Apply `TrigSimplify` to result.                 | Requires B1 above |
| `float`    | Force float evaluation (same as `numer`).       | Already works |

**File**: `macsyma_runtime/handlers.py` in the `ev_handler` function.

---

### C4 · `at` / point evaluation

**What**: MACSYMA's `at(expr, x = a)` evaluates `expr` at `x = a`.
This is syntactic sugar over `Subst(a, x, expr)`.

**IR mapping**: The MACSYMA compiler already compiles `x = a` to
`Equal(x, a)`. The `at` handler needs to recognize
`At(expr, Equal(x, a))` and rewrite to `Subst(a, x, expr)`.

**New IR head**: `At(expr, rule_or_list_of_rules)`.

**MACSYMA name table**:
```python
{"at": IRSymbol("At")}
```

Implementation lives in `macsyma-runtime` (not `SymbolicBackend`) since
the `Equal`-as-substitution-rule convention is MACSYMA-specific.
(In Mathematica the same operation uses `Rule`: `expr /. x -> a`.)

---

### C5 · `lhs` / `rhs` — equation sides

**What**: `lhs(a = b)` → `a`, `rhs(a = b)` → `b`.

**IR**: `Equal(a, b)` is the IR for equations. Add `Lhs` and `Rhs` handlers:

```python
def lhs_handler(_vm, expr):
    if len(expr.args) != 1: return expr
    eq = expr.args[0]
    if isinstance(eq, IRApply) and eq.head.name == "Equal" and len(eq.args) == 2:
        return eq.args[0]
    return expr
```

These are one-liners added to `build_cas_handler_table()` in `symbolic-vm`.

**New IR heads**: `Lhs`, `Rhs`.

**MACSYMA name table**:
```python
{"lhs": IRSymbol("Lhs"), "rhs": IRSymbol("Rhs")}
```

---

## Group D — Deferred (out of scope for now)

| Feature | Reason deferred |
|---------|-----------------|
| `ode2` — ODE solving | Complex algorithms (variation of parameters, integrating factors, Lie symmetries); warrants its own `cas-ode` spec |
| `laplace` / `ilt` | Laplace transform and inverse; needs `cas-complex` + a residue-theorem-based partial-fraction algorithm |
| 2D pretty printing | `display2d` mode with fraction bars and superscript exponents; UI concern, not CAS algebra |
| `fourier` / `ifourier` | Symbolic Fourier transform; requires distribution theory |
| `mnewton` — Newton's method | Numeric; easier to implement than ODE but not blocking any current gap |
| Multivariate `factor`/`solve` | Gröbner bases (Buchberger's algorithm); large scope, future `cas-multivariate` package |
| Algebraic number extensions | Factoring over `Q[√2]`, `Q[ζ_n]` etc.; depends on Berlekamp-Zassenhaus lift |

---

## Suggested implementation order

Based on user impact and dependency ordering:

```
1. C2 (makelist)          ← 1 day, pure list-operations extension, no deps
2. C5 (lhs/rhs)           ← 1 hour, trivial handler addition
3. B2 (cas-complex)       ← 3 days, unlocks complex roots in cas-solve
4. A3 (Expand/Collect/Together/Apart/RatSimplify)  ← 3 days, high user demand
5. B1 (cas-trig)          ← 3 days, high user demand
6. A2a/A2b (cubic/quartic in cas-solve)  ← 3 days, depends on B2
7. B3 (cas-number-theory) ← 2 days, self-contained
8. C1 (%i binding)        ← 1 hour, depends on B2
9. C3 (ev flags)          ← 1 day, depends on A3 and B1
10. C4 (at head)          ← 0.5 day
11. A1 (factor Phase 3–4) ← 1 week, mathematically dense
12. A2c/A2d (NSolve, linear systems) ← 3 days
```

Total estimate to "feature-complete MACSYMA Phase 1": ~4 weeks of focused
implementation effort.

---

## How to wire a new package into the VM

Every new `cas-*` package follows the same four-step integration:

```
1. Implement pure Python functions operating on IRNode.
2. Write a build_<name>_handler_table() → dict[str, Handler].
3. In symbolic_vm/backends.py SymbolicBackend.__init__:
       handlers.update(build_<name>_handler_table())
4. In macsyma_runtime/name_table.py:
       NAME_TABLE.update({macsyma_name: IRSymbol(HeadName), ...})
```

No changes to the VM core, no changes to MacsymaBackend, no changes to
the parser or compiler. The head just starts working everywhere.
