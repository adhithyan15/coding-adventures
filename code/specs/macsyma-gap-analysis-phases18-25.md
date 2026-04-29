# MACSYMA Gap Analysis — Phases 18–25

> **Status**: Planning document. Phases 1–17 are complete. This spec covers
> everything needed to reach full parity with historical MACSYMA (1982–1994).

---

## Current state summary (Phases 1–17 complete)

| Area | Completeness |
|------|-------------|
| Integration (Risch + IBP + all hyperbolic powers) | ~85% |
| Differentiation (all elementary functions) | ~99% |
| Factoring (rational-root + Kronecker + BZH) | ~95% |
| Solving (deg 1–4 + linear systems) | ~60% |
| Simplification (Expand/Collect/Together/Apart/RatSimplify) | ~50% |
| Trig simplification | ~90% |
| Limits (direct substitution only) | ~25% |
| ODE solving (4 types) | ~35% |
| Linear algebra (basic operations) | ~65% |
| Fourier/Laplace transforms | ~80% |
| Number theory | ~90% |
| Complex numbers | ~90% |
| Pattern matching / user rules | 0% |
| Assumptions framework | 0% |
| Special functions (erf, Si, Ci, Li₂, Γ, B) | 0% |
| Symbolic summation | 0% |

**Estimated overall parity with historical MACSYMA: ~55–60%**

The biggest gaps — pattern matching, assumptions, special functions, richer
ODEs and limits — are precisely what gave MACSYMA its practical power.
Closing them brings the system to full parity.

---

## Phase sizing philosophy

Phases 14–17 were each ~100–700 lines — one algorithm per phase. Going
forward each phase covers a **complete capability cluster**: all the
algorithms, handlers, IR heads, tests, and MACSYMA surface syntax for a
coherent feature set in one PR. Target: **1 500–3 000 lines per PR**.

---

## Phase 18 — ODE completion (5 new types)

**Target version**: `symbolic-vm` 0.38.0, `cas-ode` 0.2.0

Historical MACSYMA's `ode2` handled ~12 classes. We have 4. This phase
adds the 5 most practically important missing ones, all in one shot.

### 18a — Bernoulli equations

```
y' + P(x)·y = Q(x)·y^n
```

Substitution `v = y^(1−n)` → linear in v.  Result: `y^(1−n)` expressed in
closed form via the integrating-factor formula.

### 18b — Exact equations

```
M(x,y) dx + N(x,y) dy = 0   where  ∂M/∂y = ∂N/∂x
```

Algorithm: verify exactness, integrate M w.r.t. x to get F, adjust with
function of y only (determined by differentiating F w.r.t. y and matching
N). Solution: `F(x,y) = C`.

### 18c — Homogeneous equations

```
y' = f(y/x)
```

Substitution `v = y/x`, `y = v·x`, `y' = v + x·v'` → separable in v.

### 18d — 2nd-order non-homogeneous (constant coefficients)

```
a·y'' + b·y' + c·y = f(x)
```

Method of undetermined coefficients for `f(x)` from the standard families:
- Polynomial: `P_n(x)`
- Exponential: `e^(kx)`
- Trig: `sin(kx)`, `cos(kx)`
- Products: `P_n(x)·e^(kx)`, `e^(kx)·sin/cos(kx)`

Resonance detection: multiply ansatz by `x` when the forcing frequency
matches a homogeneous solution.

### 18e — Reduction of order

When one solution `y₁(x)` to `y'' + P(x)·y' + Q(x)·y = 0` is known
(by inspection or from 18-homogeneous), find `y₂ = v(x)·y₁` via
Abel's formula:

```
v'(x) = exp(−∫P(x)dx) / y₁²(x)
```

### New module: `cas_ode/ode2.py` additions

- `solve_bernoulli(p, q, n, y, x)` — Bernoulli
- `solve_exact(M, N, y, x)` — exact
- `solve_homogeneous_type(f, y, x)` — homogeneous
- `solve_nonhomogeneous_cc(a, b, c, f, y, x)` — undetermined coefficients
- `solve_reduction_of_order(p, q, y1, y, x)` — reduction of order

### MACSYMA surface syntax

```
ode2(y' + x*y = x*y^3, y, x);          /* Bernoulli */
ode2(2*x*y + (x^2-y^2)*'diff(y,x)=0, y, x);   /* exact */
ode2('diff(y,x) = (y/x + 1)^2, y, x);  /* homogeneous */
ode2('diff(y,x,2) + 3*'diff(y,x) + 2*y = sin(x), y, x); /* non-hom */
```

### Tests

6 test classes × ~8 tests = ~50 tests.

---

## Phase 19 — Linear algebra completion

**Target version**: `symbolic-vm` 0.39.0, `cas-matrix` 0.3.0

### 19a — Eigenvalues

`eigenvalues(A)` → `[λ₁, λ₂, …]` (with multiplicity notation) via:
1. Characteristic polynomial `det(A − λ·I)` → polynomial in λ
2. `cas-solve` on the polynomial → roots

Return format: `List(List(λ₁, m₁), List(λ₂, m₂), …)` where `mᵢ` is
algebraic multiplicity.

### 19b — Eigenvectors

`eigenvectors(A)` → `List(List(λ, m, List(v₁, v₂, …)), …)` via:
1. For each eigenvalue λ: solve `(A − λI)v = 0` (null space)
2. Express solution as list of basis vectors

### 19c — LU decomposition

`lu(A)` → `List(L, U, P)` — Doolittle algorithm with partial pivoting.

### 19d — Null space / column space / row space

- `nullspace(A)` → basis of `ker(A)` (via `rowreduce`)
- `columnspace(A)` → basis of `col(A)` (pivot columns)
- `rowspace(A)` → basis of `row(A)` (pivot rows of RREF)

### 19e — Matrix norms and conditioning

- `norm(A, "frobenius")` → `sqrt(sum of squares of entries)`
- `norm(v)` for vectors → Euclidean norm
- `charpoly(A, lambda)` → characteristic polynomial as a symbolic expr

### New IR heads

`EIGENVALUES`, `EIGENVECTORS`, `LU`, `NULLSPACE`, `COLUMNSPACE`,
`CHARPOLY`, `NORM` added to `symbolic-ir`.

### MACSYMA surface syntax

```
eigenvalues(matrix([1,2],[2,1]));       /* [[-1,1],[1,1]] */
eigenvectors(matrix([1,2],[2,1]));
lu(matrix([2,1],[1,3]));
nullspace(matrix([1,2,3],[4,5,6]));
charpoly(matrix([1,2],[3,4]), lambda);
```

### Tests

5 test classes × ~8 tests = ~40 tests.

---

## Phase 20 — Limits: L'Hôpital, infinity, indeterminate forms

**Target version**: `symbolic-vm` 0.40.0, `cas-limit-series` 0.2.0

### 20a — L'Hôpital's rule (0/0 and ∞/∞)

Detect `f/g → 0/0` or `∞/∞` at limit point by evaluating numerator and
denominator. Differentiate both and retry (up to depth 8). Uses `_diff_ir`
already in the VM.

### 20b — Limits at ±∞

For rational functions: degree comparison gives `0`, `±∞`, or leading
coefficient ratio. For exponentials/logs: dominance ordering:

```
polynomial ≪ exp(x) ≪ x^x  (as x → ∞)
log(x) ≪ polynomial  (as x → ∞)
```

### 20c — All standard indeterminate forms

| Form | Reduction |
|------|-----------|
| `0/0` | L'Hôpital |
| `∞/∞` | L'Hôpital |
| `0·∞` | Rewrite as `0/(1/∞)` |
| `∞ − ∞` | Rationalise / common denominator |
| `1^∞` | `exp(∞ · log(1))` → L'Hôpital on `log(f)/g⁻¹` |
| `0^0` | `exp(0 · log(0))` → L'Hôpital |
| `∞^0` | `exp(0 · log(∞))` → L'Hôpital |

### 20d — One-sided limits

`limit(f, x, a, plus)` / `limit(f, x, a, minus)` via Taylor sign analysis.

### MACSYMA surface syntax

```
limit(sin(x)/x, x, 0);            /* 1 */
limit((1+1/x)^x, x, inf);         /* %e */
limit(x*log(x), x, 0, plus);      /* 0 */
limit((exp(x)-1)/x, x, 0);        /* 1 */
```

### Tests

4 test classes × ~10 tests = ~40 tests.

---

## Phase 21 — Simplification suite: `radcan`, `logcontract`, assumptions

**Target version**: `symbolic-vm` 0.41.0, `cas-simplify` 0.3.0

This phase covers three families of simplification that MACSYMA users reach
for constantly: radical canonicalization, log contraction/expansion, and
the sign-assumption framework that underpins both.

### 21a — `assume` / `forget` framework

```
assume(x > 0)       → records x ∈ (0, ∞)
assume(n, integer)  → records n ∈ ℤ
assume(a, positive) → records a > 0
forget(x > 0)       → removes the assumption
is(x > 0)           → True | False | Unknown
```

Stored in a per-VM `AssumptionContext`.  Affects:
- `|x|` → `x` when `x > 0`
- `√(x²)` → `x` when `x > 0` (otherwise `|x|`)
- `log(x^n)` → `n·log(x)` when `x > 0`
- Sign function `sign(x)` → `1 | -1 | 0`
- Integration: branch selection (e.g. `∫ 1/x dx` = `log(x)` when `x > 0`)

New IR heads: `ASSUME`, `FORGET`, `IS`, `SIGN`.

### 21b — `radcan` — radical canonicalization

Rules applied in order:
1. `√a · √b = √(ab)` (when a,b > 0 or under `assume`)
2. `√(a²·b) = a·√b` (when a > 0)
3. `a^(p/q) · b^(p/q) = (ab)^(p/q)` — collect identical rational exponents
4. `exp(log(x)) = x`, `log(exp(x)) = x`
5. Denesting: `√(a + b·√c)` → `√d + √e` when a²−b²c is a perfect square

### 21c — `logcontract` and `logexpand`

```
logcontract:
  log(a) + log(b)    → log(a·b)
  n·log(a)           → log(a^n)  (n rational or integer)
  log(a) - log(b)    → log(a/b)

logexpand:
  log(a·b)           → log(a) + log(b)  (when a,b > 0)
  log(a/b)           → log(a) - log(b)
  log(a^n)           → n·log(a)
```

### 21d — `exponentialize` and `demoivre`

```
exponentialize:
  sin(x)  → (exp(ix) - exp(-ix)) / (2i)
  cos(x)  → (exp(ix) + exp(-ix)) / 2
  sinh(x) → (exp(x) - exp(-x)) / 2
  cosh(x) → (exp(x) + exp(-x)) / 2

demoivre:
  exp(a + bi) → exp(a) · (cos(b) + i·sin(b))
```

### New IR heads

`ASSUME`, `FORGET`, `IS`, `SIGN`, `RADCAN`, `LOGCONTRACT`, `LOGEXPAND`,
`EXPONENTIALIZE`, `DEMOIVRE`.

### MACSYMA surface syntax

```
assume(x > 0);
radcan(sqrt(x^2*y));          /* x*sqrt(y) */
logcontract(log(a)+log(b));   /* log(a*b) */
logexpand(log(x^3));          /* 3*log(x) */
exponentialize(sin(x));
demoivre(exp(x+%i*y));
is(x > 0);                    /* true */
```

### Tests

6 test classes × ~10 tests = ~60 tests.

---

## Phase 22 — Pattern matching and user-defined rules

**Target version**: `symbolic-vm` 0.42.0, new package `cas-pattern-rules` 0.1.0

This is the macro-level power of MACSYMA — user-extensible algebra.

### 22a — `matchdeclare` — predicates for pattern variables

```
matchdeclare(a, numberp)    → a matches any number
matchdeclare(x, symbolp)    → x matches any symbol
matchdeclare(f, true)       → f matches anything
matchdeclare(n, integerp)   → n matches integers only
```

Implementation: a `PatternContext` dict mapping symbol names to predicate
functions. Pattern variables are distinguished from regular symbols.

### 22b — `defrule` — named rewrite rules

```
defrule(rule1, sin(x)^2 + cos(x)^2, 1);
defrule(rule2, a * (b + c), a*b + a*c);
```

Each rule is stored as `(lhs_pattern, rhs_template)`.

### 22c — `apply1` and `apply2`

```
apply1(expr, rule1, rule2, …)
  → apply each rule once at the top level of every subexpression (bottom-up)

apply2(expr, rule1, rule2, …)
  → apply until no more rules fire (fixpoint)
```

### 22d — `tellsimp` — automatic simplification rules

```
tellsimp(sin(x)^2 + cos(x)^2, 1);
```

Rules registered via `tellsimp` fire automatically inside `simplify`.

### 22e — Pattern matching engine

Full structural unification supporting:
- Literal matches: `sin(x)` matches `sin(x)` only
- Predicate-bound variables: `matchdeclare(a, numberp)` → `a` matches 2
- Sequence variables: `f(a__, b__)` matches any split of arguments
- `%%` for the entire matched expression

### New IR heads

`MATCH_DECLARE`, `DEF_RULE`, `APPLY1`, `APPLY2`, `TELL_SIMP`.

### MACSYMA surface syntax

```
matchdeclare(a, numberp, x, symbolp);
defrule(r1, a*log(x), log(x^a));
apply1(3*log(y), r1);              /* log(y^3) */
apply2(sin(t)^2 + cos(t)^2, r1);  /* 1 */
```

### Tests

5 test classes × ~10 tests = ~50 tests.

---

## Phase 23 — Special functions as integration fallback

**Target version**: `symbolic-vm` 0.43.0, `symbolic-ir` 0.9.0

When the Risch algorithm exhausts elementary representations, MACSYMA
returned answers in terms of named special functions rather than leaving
integrals unevaluated. This phase adds the most common ones.

### 23a — Error functions: erf, erfc, erfi

```
∫ exp(-x²) dx = √π/2 · erf(x)
∫ exp(x²) dx = √π/2 · erfi(x)    [imaginary error function]
∫ exp(-a²x²) dx = √π/(2a) · erf(ax)
```

**Differentiation**: `d/dx erf(x) = 2/√π · exp(-x²)`.

### 23b — Trig integrals: Si, Ci, Shi, Chi

```
∫ sin(x)/x dx = Si(x)
∫ cos(x)/x dx = Ci(x) + log(x)   [up to convention]
∫ sinh(x)/x dx = Shi(x)
∫ cosh(x)/x dx = Chi(x) + log(x)
```

**Differentiation**: `d/dx Si(x) = sin(x)/x`.

### 23c — Dilogarithm: Li₂

```
∫ log(t)/(1-t) dt = -Li₂(1-t)
∫ log(1-t)/t dt = -Li₂(t)
∫ x·tanh(x) dx = x·log(cosh(x)) - Li₂(-exp(-2x))/2 + ...
```

### 23d — Gamma and Beta functions

```
Gamma(n) = (n-1)!  for positive integer n
Gamma(1/2) = √π
Beta(a,b) = Gamma(a)·Gamma(b)/Gamma(a+b)

∫₀^∞ x^(n-1)·exp(-x) dx = Gamma(n)
∫₀^1 x^(a-1)·(1-x)^(b-1) dx = Beta(a,b)
```

Numeric evaluation for floating-point arguments via Lanczos approximation.

### 23e — Fresnel integrals: S(x), C(x)

```
∫₀^x sin(πt²/2) dt = S(x)
∫₀^x cos(πt²/2) dt = C(x)
```

### New IR heads (in `symbolic-ir` 0.9.0)

`ERF`, `ERFC`, `ERFI`, `SI`, `CI`, `SHI`, `CHI`, `LI2`, `GAMMA_FUNC`,
`BETA_FUNC`, `FRESNEL_S`, `FRESNEL_C`.

### Integration fallback wiring

`integrate.py` Phase 23 dispatch: after exhausting all Risch rules,
check if the integrand matches a known special-function pattern and
return the special-function form rather than unevaluated `Integrate`.

### MACSYMA surface syntax

```
integrate(exp(-x^2), x);           /* sqrt(%pi)/2 * erf(x) */
integrate(sin(x)/x, x);            /* Si(x) */
integrate(log(x)/(1-x), x);        /* -Li2(1-x) */
gamma(5);                          /* 24 */
beta(1/2, 1/2);                    /* %pi */
```

### Tests

6 test classes × ~10 tests = ~60 tests.

---

## Phase 24 — Transcendental equation solving

**Target version**: `symbolic-vm` 0.44.0, `cas-solve` 0.7.0

`solve` currently handles polynomial equations (deg 1–4) and linear
systems. This phase extends it to the most common transcendental families.

### 24a — Trigonometric equations

```
sin(x) = c  →  x = arcsin(c) + 2kπ  or  π - arcsin(c) + 2kπ
cos(x) = c  →  x = ±arccos(c) + 2kπ
tan(x) = c  →  x = arctan(c) + kπ
```

Return format: `List(Rule(x, expr1 + 2*%pi*%k), …)` where `%k` is a new
free integer constant symbol.

### 24b — Logarithmic equations

```
log(f(x)) = c  →  f(x) = exp(c)  then recurse
exp(f(x)) = c  →  f(x) = log(c)  then recurse
```

### 24c — Lambert W equations

```
f(x)·exp(f(x)) = c  →  f(x) = W(c)   [Lambert W function]
x·exp(x) = k       →  x = W(k)
x^x = k            →  x = exp(W(log(k)))
```

New IR head `LAMBERT_W` added to `symbolic-ir`.

### 24d — Hyperbolic equations

```
sinh(x) = c  →  x = asinh(c)
cosh(x) = c  →  x = ±acosh(c)  (c ≥ 1)
tanh(x) = c  →  x = atanh(c)   (|c| < 1)
```

### 24e — Compound forms

Single substitution reduction: `sin(x)^2 + sin(x) = 0` → quadratic in
`u = sin(x)`, then solve for x.

### New IR heads

`FREE_INTEGER` (for `%k` in trig solutions), `LAMBERT_W`.

### MACSYMA surface syntax

```
solve(sin(x) = 1/2, x);
solve(exp(2*x) - 3*exp(x) + 2 = 0, x);
solve(x*exp(x) = 1, x);
solve(log(x+1) = 2, x);
```

### Tests

5 test classes × ~10 tests = ~50 tests.

---

## Phase 25 — Symbolic summation

**Target version**: `symbolic-vm` 0.45.0, new package `cas-summation` 0.1.0

MACSYMA's `sum` could evaluate closed-form sums over many standard
families. This phase implements the most practically useful subset.

### 25a — Polynomial sums (Bernoulli polynomials)

```
sum(k^0, k, 1, n)  = n
sum(k^1, k, 1, n)  = n(n+1)/2
sum(k^2, k, 1, n)  = n(n+1)(2n+1)/6
sum(k^m, k, 1, n)  = Bernoulli polynomial formula  (m ≤ 10)
```

### 25b — Geometric series

```
sum(r^k, k, 0, n)   = (r^(n+1)-1)/(r-1)   (finite)
sum(r^k, k, 0, inf) = 1/(1-r)             (|r| < 1)
```

### 25c — Telescoping sums

```
sum(f(k+1)-f(k), k, a, b) = f(b+1) - f(a)
```

Detected by pattern: consecutive terms cancel.

### 25d — Exponential/factorial sums

```
sum(k/k!, k, 1, inf)   = e
sum(1/k!, k, 0, inf)   = e
sum(x^k/k!, k, 0, inf) = exp(x)   [formal; returns exp(x)]
```

### 25e — Classic convergent series

```
sum(1/k^2, k, 1, inf) = %pi^2/6    [Basel problem]
sum(1/k^4, k, 1, inf) = %pi^4/90
sum((-1)^k/(2k+1), k, 0, inf) = %pi/4  [Leibniz]
```

### 25f — `product` — finite products

```
product(k, k, 1, n)     = n!
product(1-x^2/k^2, k, 1, inf) = sin(%pi*x)/(%pi*x)  [Euler]
```

### New IR heads

`SUM`, `PRODUCT` (these replace the current unevaluated-only forms if any
exist). `BERNOULLI_B` for Bernoulli numbers.

### MACSYMA surface syntax

```
sum(k^2, k, 1, n);           /* n*(n+1)*(2*n+1)/6 */
sum(1/2^k, k, 0, inf);       /* 2 */
sum(1/k^2, k, 1, inf);       /* %pi^2/6 */
product(k, k, 1, n);         /* n! */
```

### Tests

6 test classes × ~10 tests = ~60 tests.

---

## Summary roadmap

| Phase | Feature cluster | New lines est. | `symbolic-vm` version |
|-------|----------------|---------------|----------------------|
| 18 | ODE: Bernoulli + Exact + Homogeneous + non-homogeneous 2nd-order + reduction of order | ~2 000 | 0.38.0 |
| 19 | Linear algebra: eigenvalues + eigenvectors + LU + null/col/row space + norms | ~1 800 | 0.39.0 |
| 20 | Limits: L'Hôpital + infinity + all indeterminate forms + one-sided | ~1 200 | 0.40.0 |
| 21 | Simplification: `assume`/`forget` + `radcan` + `logcontract`/`logexpand` + `exponentialize` | ~1 800 | 0.41.0 |
| 22 | Pattern matching: `matchdeclare` + `defrule` + `apply1`/`apply2` + `tellsimp` | ~1 500 | 0.42.0 |
| 23 | Special functions: erf + Si/Ci + Li₂ + Γ/B + Fresnel; integration fallbacks | ~2 200 | 0.43.0 |
| 24 | Transcendental solving: trig + log + Lambert W + hyperbolic + compound | ~1 200 | 0.44.0 |
| 25 | Symbolic summation: polynomial + geometric + telescoping + classic series + `product` | ~1 500 | 0.45.0 |

**Total: ~13 200 new lines across 8 phases → closes the MACSYMA gap to ~95%**

---

## What stays out of scope (MACSYMA had it; we won't)

These are either non-essential or belong to higher layers:

| Feature | Reason to defer |
|---------|----------------|
| `describe` / built-in help | Documentation layer, not computation |
| MACSYMA batch files / `loadfile` | I/O layer above the VM |
| FORTRAN/C code generation | Separate compiler pass |
| Plotted output (`plot2d`) | Rendering layer |
| Definite integrals to special constants | Covered by Phase 23 fallbacks |
| `residue` (complex residues) | Niche; needs complex plane arithmetic |
| Formal power series ring | Beyond Phases 1–25 scope |
| `asksign` interactive | Superseded by Phase 21's `assume` |

---

## After Phase 25: ~95% parity

The remaining 5% is deep specialised analysis (complex residues, formal
power series rings, non-elementary definite integral evaluation via
Meijer G-functions) that was rarely used in practice even by MACSYMA's
power users.
