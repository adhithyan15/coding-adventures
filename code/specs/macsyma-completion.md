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

## Current status (as of symbolic-vm 0.33.0 / macsyma-runtime 1.11.0)

### Fully working end-to-end

| Area                | IR heads / MACSYMA names                        | Package              |
|---------------------|-------------------------------------------------|----------------------|
| Arithmetic          | `Add Mul Pow Neg Sub Div Inv`                   | `symbolic-vm` (built-in) |
| Variables / functions | `Assign Define`                               | `symbolic-vm` (built-in) |
| Lambda / closures   | `lambda([params], body)` β-reduction            | `symbolic-vm` (built-in) |
| Simplification      | `Simplify Expand Collect Together RatSimplify Apart` | `cas-simplify`, `symbolic-vm` |
| Substitution        | `Subst`                                         | `cas-substitution` |
| Factoring           | `Factor` (rational-root + Kronecker + BZH)      | `cas-factor` (0.3.0) |
| Solving             | `Solve` (deg 1–4 + linear systems), `NSolve`   | `cas-solve` (0.6.0) |
| List operations     | `Length First Rest Last Append Reverse Range Map Apply Select Sort Part Flatten Join MakeList` | `cas-list-operations` |
| Matrix operations   | `Matrix Transpose Determinant Inverse Dot Trace Dimensions IdentityMatrix ZeroMatrix Rank RowReduce` | `cas-matrix` (0.2.0) |
| Limits              | `Limit` (direct substitution)                   | `cas-limit-series` |
| Taylor series       | `Taylor` (polynomial + transcendental fallback) | `cas-limit-series`, `symbolic-vm` |
| Differentiation     | `D`                                             | `symbolic-vm` |
| Integration         | `Integrate` (Risch Phases 1–14: poly, exp, log, trig, IBP, partial fractions, inverse-trig, hyperbolic, hyperbolic powers, exp×hyperbolic) | `symbolic-vm` |
| Numeric ops         | `Abs Cbrt Floor Ceiling Mod Gcd Lcm`           | `symbolic-vm` |
| Trigonometric simplification | `TrigSimplify TrigExpand TrigReduce`   | `cas-trig` (0.1.0) |
| Complex numbers     | `Re Im Conjugate Arg RectForm PolarForm`, `%i`  | `cas-complex` (0.1.0) |
| Number theory       | `IsPrime NextPrime PrevPrime FactorInteger Divisors Totient MoebiusMu JacobiSymbol ChineseRemainder IntegerLength` | `cas-number-theory` (0.1.0) |
| Constants           | `%pi %e %i`                                     | `macsyma-runtime` |
| REPL mechanics      | `;` / `$` terminators, history `%/%iN/%oN`, `kill`, `ev(numer/expand/factor/trigsimp)`, `at`, `lhs`, `rhs`, `makelist` | `macsyma-runtime` (1.3.0) |
| Pretty printing     | MACSYMA / Mathematica / Maple / Lisp dialects   | `cas-pretty-printer` (0.2.0) |
| 2D pretty printing  | `display2d` fraction bars / superscripts / sqrt | `cas-pretty-printer` (0.4.0) |
| Newton's method     | `MNewton` (`mnewton(f, x, x0)`)                 | `cas-mnewton` (0.1.0) |

---

## Group A — Complete (all phases shipped)

### A1 · `cas-factor` — Polynomial factoring over Z

**Status: ✅ Complete** (rational-root + Kronecker + BZH, `cas-factor` 0.3.0)

Phase 1 (rational-root), Phase 2 (Kronecker), and Phase 3 (BZH) are all shipped.
New BZH cases (previously unevaluated):
- `factor(x^5-1)` → `(x-1)*(x^4+x^3+x^2+x+1)`
- `factor(x^8-1)` → `(x-1)*(x+1)*(x^2+1)*(x^4+1)`
- `factor(x^9-1)` → `(x-1)*(x^2+x+1)*(x^6+x^3+1)`
- `factor(x^4+1)` → `x^4+1` (confirmed irreducible over Q by both Kronecker and BZH)

---

### A2 · `cas-solve` — Equation solving (all phases)

**Status: ✅ Complete** (`cas-solve` 0.6.0, "Phases 1–5")

| Phase | Algorithm | Status |
|-------|-----------|--------|
| A2a — Degree 1 | Linear | ✅ |
| A2b — Degree 2 | Quadratic formula | ✅ |
| A2c — Degree 3 | Cardano's formula (returns `Cbrt` IR) | ✅ |
| A2d — Degree 4 | Ferrari's method | ✅ |
| A2e — NSolve   | Durand-Kerner numeric iteration | ✅ |
| A2f — Linear systems | Gaussian elimination with Fraction coefficients | ✅ |

---

### A3 · `cas-simplify` — Rational function operations

**Status: ✅ Complete** (wired in `symbolic-vm` via `cas_handlers.py`)

| Head         | MACSYMA name | Status |
|--------------|--------------|--------|
| `Expand`     | `expand`     | ✅ Full polynomial distribution |
| `Collect`    | `collect`    | ✅ Groups by powers of variable |
| `Together`   | `together`   | ✅ Combines fractions |
| `RatSimplify`| `ratsimp`    | ✅ Cancels GCD of num/denom |
| `Apart`      | `partfrac`   | ✅ Partial fraction decomposition |

---

## Group B — Complete (all packages shipped)

### B1 · `cas-trig` — Trigonometric simplification

**Status: ✅ Complete** (`cas-trig` 0.1.0)

| Head           | MACSYMA name   | Description |
|----------------|----------------|-------------|
| `TrigSimplify` | `trigsimp`     | Applies Pythagorean identities and reduces |
| `TrigExpand`   | `trigexpand`   | Expands compound angles, power-reduction |
| `TrigReduce`   | `trigreduce`   | Rewrites powers of sin/cos as multiple angles |

---

### B2 · `cas-complex` — Complex number support

**Status: ✅ Complete** (`cas-complex` 0.1.0)

`ImaginaryUnit` pre-bound as `%i`. Rules `%i^2 → -1`, etc. fire automatically
in `SymbolicBackend`.  Full `Re/Im/Conjugate/Arg/RectForm/PolarForm` handlers.

---

### B3 · `cas-number-theory` — Integer number theory

**Status: ✅ Complete** (`cas-number-theory` 0.1.0)

`primep` / `is_prime`, `next_prime`, `prev_prime`, `ifactor`, `divisors`,
`totient`, `moebius`, `jacobi`, `chinese`, `numdigits` all wired.

---

## Group C — Complete (all MACSYMA wiring done)

| Item | Status | Notes |
|------|--------|-------|
| C1 · `%i` binding | ✅ | Pre-bound in `SymbolicBackend`; `%i^2 → -1` rule fires |
| C2 · `makelist` | ✅ | `MakeList(expr, var, n)` and range forms |
| C3 · `ev` flags | ✅ | `numer`, `expand`, `factor`, `ratsimp`, `trigsimp`, `float` |
| C4 · `at` | ✅ | `At(expr, Equal(x, a))` and list-of-rules form |
| C5 · `lhs` / `rhs` | ✅ | `Lhs(Equal(a,b)) → a`, `Rhs(Equal(a,b)) → b` |

---

## Group D — In Progress

### D1 · `mnewton` — Newton's method numeric root finder

**Status: ✅ Complete** (`cas-mnewton` 0.1.0)

`mnewton(f, x, x0)` iterates Newton's method `x_{n+1} = x_n − f(x_n)/f'(x_n)`.
Returns `IRFloat(root)` on convergence; falls through to unevaluated on
zero-derivative or non-numeric input.

---

### D2 · 2D pretty printing

**Status: ✅ Complete** (`cas-pretty-printer` 0.4.0)

`pretty(node, dialect, style="2d")` uses a box-model layout engine (fractions
with `─` bars, superscript exponents, `√` radicals). The `Box(lines, baseline)`
dataclass aligns operands at their mathematical baseline.

---

### D3 · ODE solving

**Status: ✅ Complete** (`cas-ode` 0.1.0)

`ode2(eqn, y, x)` solves four classes of ODEs:

- First-order linear `y' + P(x)·y = Q(x)` via integrating factor.
- Separable `y' = f(x)·g(y)` (linear-in-y cases).
- Second-order constant-coefficient homogeneous `a·y'' + b·y' + c·y = 0`
  via characteristic equation — all three root cases (distinct real,
  repeated, complex conjugate).

Integration constants: `%c` (first-order), `%c1`/`%c2` (second-order).

Not implemented: Bernoulli equations, variable-coefficient 2nd order,
non-homogeneous 2nd order (method of undetermined coefficients).

---

### D5 — ✅ Complete

**Algebraic number extensions**: `cas-algebraic` 0.1.0 — factoring univariate
polynomials over quadratic algebraic extensions Q[√d].

- Pattern 1: depressed monic quartics x⁴ + p·x² + q → two monic quadratics
  over Q[√d] when q is a perfect rational square and (2s−p)/d is also a
  perfect rational square.
- Pattern 2: monic quadratics x² + bx + c → two linear factors when
  discriminant b²−4c = d·(2β)² for rational β.
- `ALG_FACTOR = IRSymbol("AlgFactor")` head added to `symbolic-ir` 0.7.5.
- `alg_factor_handler` registered in `symbolic-vm` 0.32.7.
- `"algfactor": ALG_FACTOR` added to `macsyma-runtime` 1.9.0 name table.
- `"AlgFactor"` added to `_HELD_HEADS` so `Sqrt(d)` arg is not pre-evaluated.

Surface syntax: `algfactor(x^4+1, sqrt(2))` → `(x^2+sqrt(2)*x+1)*(x^2-sqrt(2)*x+1)`.

---

### D6 — ✅ Complete

**Multivariate polynomial operations**: `cas-multivariate` 0.1.0 — Gröbner bases
(Buchberger's algorithm), polynomial reduction, and ideal solving.

- `MPoly`: sparse multivariate polynomial over Q (Fraction coefficients), dict-based.
- Monomial orderings: grlex (default for Buchberger), lex (for back-substitution),
  grevlex.
- `s_poly(f, g)`: S-polynomial computation — the core of Buchberger's algorithm.
- `reduce_poly(f, G)`: multivariate polynomial reduction (normal form).
- `buchberger(F)`: full Buchberger algorithm with inter-reduction to canonical basis.
  Safety cap: degree ≤ 8, basis size ≤ 50.
- `ideal_solve(polys)`: lex Gröbner basis + back-substitution for exact rational
  solutions of polynomial systems.
- `GROEBNER`, `POLY_REDUCE`, `IDEAL_SOLVE` heads added to `symbolic-ir` 0.7.6.
- Handlers registered in `symbolic-vm` 0.32.8.
- `"groebner"`, `"poly_reduce"`, `"ideal_solve"` added to `macsyma-runtime` 1.10.0.

Surface syntax:
- `groebner([x^2+y-1, x+y^2-1], [x, y])` → `List(g1, g2, ...)` (reduced Gröbner basis)
- `poly_reduce(x^2, [x-1], [x])` → `1`
- `ideal_solve([x+y-1, x-y], [x, y])` → `List(List(Rule(x, 1/2), Rule(y, 1/2)))`

---

## Group E — Complete

### E1 · `cas-matrix` 0.2.0 — Complete matrix operation set

**Status: ✅ Complete** (all handlers wired in `symbolic-vm` 0.33.0)

#### Phase 1 (cas-matrix 0.1.0) — already shipped

| Head | MACSYMA name | Description |
|---|---|---|
| `Matrix` | `matrix` | Construction from row lists |
| `Transpose` | `transpose` | Swap rows and columns |
| `Determinant` | `determinant` | Cofactor expansion |
| `Inverse` | `invert` | Gauss-Jordan over Fraction |

#### Phase 2 (cas-matrix 0.2.0) — new in Group E

| Head | MACSYMA name | Description |
|---|---|---|
| `Dot` | `dot` | Matrix product; exact rational arithmetic |
| `Trace` | `mattrace` | Sum of main diagonal (left-folded into binary ADDs) |
| `Dimensions` | `matrix_size` | `List(rows, cols)` shape query |
| `IdentityMatrix` | `ident` | n×n identity matrix |
| `ZeroMatrix` | `zeromatrix` | m×n (or n×n) zero matrix |
| `Rank` | `rank` | Rank via forward row elimination |
| `RowReduce` | `rowreduce` | Reduced row-echelon form (Gauss-Jordan over Fraction) |

All operations use `Fraction` (Python stdlib) for exact rational arithmetic — no
floating-point rounding errors in pivoting.

The `RowReduce` and `Rank` implementations live in `cas_matrix/rowreduce.py`.
`Rank` stops after forward elimination; `RowReduce` continues with back-substitution
and normalization to produce RREF.

`trace_handler` folds any n-ary `IRApply(ADD, ...)` returned by `cas_matrix.trace`
into a chain of binary additions so output stays within the VM's binary-ADD contract.

---

### E2 · Phase 14 — exp×hyperbolic and hyperbolic-power integration

**Status: ✅ Complete** (wired in `symbolic-vm` 0.33.0)

Three new integration families added to `integrate.py`:

| Family | Formula | Dispatcher |
|---|---|---|
| `∫ exp(ax+b)·sinh(cx+d) dx` | `exp(ax+b)·[a·sinh−c·cosh]/(a²−c²)` | `_try_exp_hyp` |
| `∫ exp(ax+b)·cosh(cx+d) dx` | `exp(ax+b)·[a·cosh−c·sinh]/(a²−c²)` | `_try_exp_hyp` |
| `∫ sinh^n(ax+b) dx` (n ≥ 2) | IBP reduction: `I_n = (1/(na))·sinh^(n-1)·cosh − (n-1)/n·I_{n-2}` | `_try_hyp_power` |
| `∫ cosh^n(ax+b) dx` (n ≥ 2) | IBP reduction: `I_n = (1/(na))·cosh^(n-1)·sinh + (n-1)/n·I_{n-2}` | `_try_hyp_power` |
| `∫ sinh^m·cosh^n dx` (min(m,n)=1) | u-substitution: `cosh^(n+1)/(n+1)/a` or `sinh^(m+1)/(m+1)/a` | `_try_sinh_cosh_product` |

**Degenerate/unsupported (return unevaluated):**
- `∫ exp(x)·sinh(x) dx` — a=c, denominator D=a²−c²=0
- `∫ sinh^m·cosh^n dx` where both m,n ≥ 2 (general case deferred)
- Non-linear arguments to sinh/cosh (e.g. `sinh(x²)`)
- Fractional/negative exponents (e.g. `sinh(x)^(1/2)`)

New source files: `exp_hyp_integral.py`, `hyp_power_integral.py`.

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
