# SPICE Engine & MACSYMA Pipeline — Status and Pending Work

> **Living document.** Updated each time a PR lands or new work is planned.
> Last updated: 2026-05-16. Sprint complete: TypeScript 0.2.0 releases (PR #3170 ✅
> merged), Rust 0.2.0 releases (PR #3171 ✅ merged), Python EllipticE/Pi —
> `symbolic-vm` 0.55.0 (PR #3173 ✅ merged), TypeScript EllipticE/Pi —
> `symbolic-vm` 0.3.0 (PR #3179 ✅ merged), Rust EllipticE/Pi —
> `symbolic-vm` 0.3.0 (PR #3178 ✅ merged), Python `macsyma-runtime` 1.26.0,
> `spice-engine` 0.13.0 (inductor IC, AcSource phasors, waveforms already implemented),
> `rust/macsyma-runtime` 0.3.0 (EllipticE/Pi pipeline tests),
> Phase 21 named variable-coefficient ODEs Python (PR #3360 ✅ merged),
> Phase 21 TypeScript + Rust ports (PR #3369 ✅ merged),
> version housekeeping chore (PR #3354 ✅ merged),
> Phase 26 log-power IBP Python (PR #3372 ✅ merged),
> Phase 27 trig-of-log integration Python `symbolic-vm` 0.57.0 (PR #3373 ✅ merged).

This document is the canonical reference for resuming work on either project.
It records exactly what is on `main`, what is in flight, and what has not been
built yet — with enough context to pick up any item cold.

---

## Architecture overview

### SPICE Engine

```
Python API (Circuit, elements)
       ↓
MNA matrix assembly (_stamp_* functions)
       ↓
Dense LU solver (numpy) / Newton-Raphson loop
       ↓
Analysis result (DcResult, TransientResult, AcResult, …)
```

Package: `code/packages/python/spice-engine`
PyPI name: `coding-adventures-spice-engine`
Spec: `code/specs/spice-engine.md`

### MACSYMA Pipeline

```
MACSYMA surface syntax
    ↓  macsyma-lexer      (grammar-driven tokenizer)
    ↓  macsyma-parser     (grammar → AST)
    ↓  macsyma-compiler   (AST → IR; 60+ name-table entries)
    ↓  symbolic-ir        (IRNode types)
    ↓  symbolic-vm        (SymbolicBackend + all CAS handlers)
    ↓  MacsymaBackend     (history, $/; terminators, kill, ev)
    →  result (symbolic IR or numeric value)
```

Key rule: **new CAS operations always go in `SymbolicBackend`**, never in
`MacsymaBackend`. That way a future Maple or Mathematica frontend shares the
same handlers without touching each other's code.

Spec: `code/specs/macsyma-completion.md`, `code/specs/macsyma-runtime.md`,
`code/specs/symbolic-computation.md`, and the individual `code/specs/cas-*.md`
files.

---

## SPICE Engine

### What is on `main`

| Version | PR | What shipped |
|---|---|---|
| v0.1.0 | #1353 | MNA core, DC operating point (Newton-Raphson), forward-Euler transient, `Diode`, `Mosfet` |
| v0.10.0 | #2901 ✅ | All four SPICE controlled sources: `VCVS` (E), `VCCS` (G), `CCCS` (F), `CCVS` (H). Correct MNA stamps (DC + AC). `TfResult.gain`. Fixed CCCS stamp sign. |
| v0.11.0 | — | Time-varying source waveforms: `PwlWaveform`, `SinWaveform`, `PulseWaveform`, `ExpWaveform`; waveform evaluation at each transient timestep |
| v0.12.0 | — | DC convergence aids: Gmin stepping + source stepping fallback chain; `_dc_newton` / `_x_from_result` helpers |
| v0.13.0 | — | Inductor `initial_current` for transient seeding; explicit AC source phasors (`AcSource`) on `VoltageSource`/`CurrentSource` |
| v0.14.0 | #3384 ✅ | Behavioral B sources across Python, TypeScript, and Rust: DC behavioral current/voltage expressions over constants and `V(node)` / `V(node1,node2)` references. |
| v0.2.0 | #2339 | Trapezoidal + Backward Euler integration, LTE-based adaptive timestep |
| v0.3.0 | #2342 | `BJT` element (Ebers-Moll, NPN/PNP) |
| v0.4.0 | #2344 | AC small-signal frequency sweep (`.AC`) |
| v0.5.0 | #2348 | DC transfer function (`.TF`) — transfer ratio, input/output impedance |
| v0.6.0 | #2353 | DC parameter sweep (`.DC`) |
| v0.7.0 | #2357 | DC sensitivity analysis (`.SENS`) — ∂V/∂P per element |
| v0.8.0 | #2359 | Monte Carlo (`.MC`) — Gaussian + uniform tolerance distributions |
| v0.9.0 | #2370 | Noise analysis (`.NOISE`) — Johnson-Nyquist + shot noise, adjoint method |

**Elements on main:** `Resistor`, `Capacitor`, `Inductor`, `VoltageSource`,
`CurrentSource`, `BSource`, `Diode`, `Mosfet`, `BJT`

**Analyses on main:** `dc_op`, `transient`, `ac_sweep`, `tf`, `dc_sweep`,
`sens_dc`, `mc_dc`, `noise_ac`

**Waveforms on main:** `PwlWaveform`, `SinWaveform`, `PulseWaveform`, `ExpWaveform`

**Netlist parser on main:** `spice-netlist-parser` 0.2.0 — parses R, C, L, V,
I, M (MOSFET), D, Q (BJT), E/G/F/H (controlled sources), `.subckt` / X instances,
`.tran`, `.dc`, `.ac`, `.op`, `.tf`, `.sens`, `.mc`, `.noise`, `.model` cards;
IC parameters for C/L; AC phasor specs for V/I sources.

---

### What is in flight

- Sparse real solver path across Python, TypeScript, and Rust SPICE engines:
  large DC / real small-signal matrices route through sparse-row Gaussian
  elimination while small systems keep the dense solver.

---

### What is not yet built

Items are listed in priority order within each group.

#### Group 1 — High value, well-scoped

All Group 1 items have shipped. See "What is on main" above.

#### Group 2 — Medium value

| Feature | Why it matters | Design notes |
|---|---|---|
| **Sub-circuit support (`.subckt` / X-element)** | Essential for hierarchical design — reusing a cell (e.g. an inverter) multiple times with different parameter values. | Add an `XInstance` element class. At circuit build time, expand each X-element by cloning its `.subckt` definition with renamed nodes (prefix with instance name). Parameters propagate via a dict of `{param: value}` substitutions. |
| **Sparse matrix solver** | Dense LU is O(n³). At ~100 nodes it becomes the bottleneck. SPICE netlists for a small IC cell can have 300+ nodes. | Replace `numpy.linalg.solve` with `scipy.sparse.linalg.splu` (already a dependency). MNA matrices are naturally sparse (each element touches only 2–4 nodes). Keep dense path as fallback for small circuits (< 30 nodes). |

#### Group 3 — Lower priority / longer horizon

| Feature | Design notes |
|---|---|
| **S-parameter extraction** | Two-port network characterisation. Run AC sweep, compute Y-parameters from node voltages and port currents, convert to S-parameters. |
| **Periodic steady-state (PSS)** | RF / oscillator analysis. Shooting-Newton method: find the initial condition `x(0)` such that `x(T) = x(0)`. Requires a good oscillation-period estimator. |
| **Multi-corner parallel sweep** | Run the same analysis at N PVT corners in parallel goroutines / subprocesses. Mostly an orchestration problem once the core engine is solid. |
| **Mixed-signal coupling with `hardware-vm`** | AMS simulation — digital events feed into analog SPICE nodes and vice versa. Long-range project; `hardware-vm.md` spec describes the interface. |
| **Verilog-A compact models** | Custom device models. Requires a Verilog-A parser (`code/specs/verilog-a-parser.md` is referenced but not written). |

---

## MACSYMA Pipeline

### What is on `main` (MACSYMA pipeline after PR #3141)

Every item below is wired end-to-end: surface syntax → compile → VM → correct result.

#### Core infrastructure (Python)

| Package | Version | What it provides |
|---|---|---|
| `symbolic-ir` | latest | `IRSymbol`, `IRInteger`, `IRRational`, `IRFloat`, `IRString`, `IRApply` node types |
| `symbolic-vm` | 0.57.0 | Pluggable VM, `SymbolicBackend`, arithmetic, Risch integration (phases 1–14+, Phase 25 EllipticF/K/E/Pi, Phase 26 log-power IBP, Phase 27 trig-of-log), numeric folding, 100+ handlers |
| `macsyma-lexer` | 0.1.0 | Grammar-driven tokenizer |
| `macsyma-parser` | 0.1.0 | Grammar-driven parser |
| `macsyma-compiler` | 0.9.0 | AST → IR; 60+ MACSYMA identifier mappings in name table |
| `macsyma-runtime` | 1.26.0 | History (`%`, `%i1`, `%o1`, …), `;`/`$` terminators, `kill`, `ev`, `MacsymaBackend` |

#### TypeScript port version releases (PR #3170 ✅ merged)

The TypeScript port was at v0.1.0 with all implemented features in an "Unreleased"
CHANGELOG section. PR #3170 cut proper 0.2.0 releases.

| Package | Version | Notes |
|---|---|---|
| `typescript/symbolic-vm` | 0.2.0 | EllipticF/K, multivariate Factor footholds, D derivative handler, reciprocal hyperbolic |
| `typescript/macsyma-runtime` | 0.2.0 | Elliptic pipeline, multivariate factor, `?` help, assume/declare, ev display2d, list/solve wiring |

#### TypeScript EllipticE/Pi integration recognition (PR #3179 ✅ merged)

| Package | Version | Notes |
|---|---|---|
| `typescript/symbolic-vm` | 0.3.0 | EllipticE (complete + incomplete), EllipticPi (complete) pattern recognition |

#### Rust port version releases (PR #3171 ✅ merged)

The Rust port was at v0.1.x with all implemented features in an "Unreleased"
CHANGELOG section. PR #3171 cut proper 0.2.0 releases and created the missing
`cas-ode` CHANGELOG.

| Package | Version | Notes |
|---|---|---|
| `rust/symbolic-vm` | 0.2.0 | Same feature set as TypeScript 0.2.0 |
| `rust/macsyma-runtime` | 0.2.0 | Same feature set as TypeScript 0.2.0 |
| `rust/cas-ode` | 0.1.0 | CHANGELOG.md created — documents all 9 ODE types (Phase 18–20) |

#### Rust EllipticE/Pi integration recognition (PR #3178 ✅ merged)

| Package | Version | Notes |
|---|---|---|
| `rust/symbolic-vm` | 0.3.0 | EllipticE (complete + incomplete), EllipticPi (complete) pattern recognition |

#### Phase 26 + Phase 27: log-power IBP and trig-of-log — TypeScript + Rust ports

| Package | Version | Notes |
|---|---|---|
| `typescript/symbolic-vm` | 0.4.0 | Phase 26 `∫ Q(x)·log(x)^n dx` + Phase 27 `∫ xᵏ·trig(log(x)) dx` via u=log(x) substitution |
| `rust/symbolic-vm` | 0.4.0 | Same as TypeScript 0.4.0 |

#### Phase 21 — named variable-coefficient ODE recognition (all three languages)

Python `cas-ode` 0.6.0 (PR #3360 ✅ merged), TypeScript `cas-ode` 0.2.0 and
Rust `cas-ode` 0.2.0 (PR #3369 ✅ merged). Also in PR #3360: `symbolic-ir`
0.14.0 (Python) and `spice-netlist-parser` 0.2.0.

`ode2` now numerically identifies four classical families of
variable-coefficient second-order ODEs and returns closed-form solutions in
terms of named special functions.  Uses test-point evaluation (x ∈ {0.3, 0.6,
−0.25, 0.85}) to match coefficient patterns without symbolic manipulation.

| ODE family | Standard form | Returned solution |
|---|---|---|
| **Legendre** | `(1−x²)y''−2xy'+n(n+1)y=0` | `EllipticF(n,x)` with `LegendreP(n,x)` and `LegendreQ(n,x)` |
| **Bessel** | `x²y''+xy'+(x²−ν²)y=0` | `BesselJ(ν,x)` and `BesselY(ν,x)` |
| **Hermite** | `y''−2xy'+2ny=0` | `HermiteH(n,x)` and `HermiteH2(n,x)` |
| **Chebyshev** | `(1−x²)y''−xy'+n²y=0` | `ChebyshevT(n,x)` and `ChebyshevU(n,x)` |

Dispatch order: Chebyshev → Legendre → Bessel → Hermite (Chebyshev must come
before Legendre since both have leading coefficient P ≈ 1−x²; the Q coefficient
distinguishes them: −x for Chebyshev, −2x for Legendre).

Eight new head symbols added to `symbolic-ir` in all three languages:
`LEGENDRE_P`, `LEGENDRE_Q`, `BESSEL_J`, `BESSEL_Y`, `HERMITE_H`,
`HERMITE_H2`, `CHEBYSHEV_T`, `CHEBYSHEV_U`.

| Package | Python | TypeScript | Rust |
|---|---|---|---|
| `symbolic-ir` | 0.14.0 (8 new Phase 27 heads) | 0.2.0 (8 new Phase 27 heads) | 0.2.0 (8 new Phase 27 heads) |
| `cas-ode` | 0.6.0 | 0.2.0 | 0.2.0 |

#### CAS substrate packages — all fully wired

| Package | Version | MACSYMA names |
|---|---|---|
| `cas-simplify` | 0.3.0 | `simplify`, `expand`, `collect`, `together`, `ratsimp`, `partfrac` |
| `cas-factor` | 0.3.0 | `factor` — univariate integer polynomial (rational-root + Kronecker + BZH) |
| `cas-solve` | 0.8.0 | `solve` deg 1–4; `nsolve` (Durand-Kerner); linear systems; polynomial inequalities; transcendental equations |
| `cas-substitution` | 0.1.0 | `subst(value, var, expr)` |
| `cas-list-operations` | 0.1.0 | `length`, `first`, `rest`, `last`, `append`, `reverse`, `range`, `map`, `apply`, `select`, `sort`, `part`, `flatten`, `join`, `makelist` |
| `cas-matrix` | 0.3.0 | Basic: `matrix`, `transpose`, `determinant`, `invert`. Advanced: `dot`, `mattrace`, `matrix_size`, `ident`, `zeromatrix`, `rank`, `rowreduce`, `eigenvalues`, `eigenvectors`, `charpoly`, `nullspace`, `columnspace`, `rowspace`, `norm`, `lu` |
| `cas-limit-series` | 0.2.0 | `limit` (direct substitution + L'Hôpital), `taylor` |
| `cas-ode` | Python 0.6.0, TS/Rust 0.2.0 | `ode2` — first-order linear, separable, Bernoulli, exact, homogeneous-type, 2nd-order constant-coefficient homogeneous/non-homogeneous, Euler-Cauchy, variation-of-parameters fallback, and Phase 21 named variable-coefficient ODEs (Legendre, Bessel, Hermite, Chebyshev) |
| `cas-laplace` | latest | `laplace`, `ilt` (inverse Laplace), `dirac_delta`, `unit_step` |
| `cas-trig` | 0.1.0 | `trigsimp`, `trigexpand`, `trigreduce` |
| `cas-complex` | 0.1.0 | `re`, `im`, `conjugate`, `cabs`, `carg`, `rectform`, `polarform`; `%i` pre-bound; `%i²→-1` fires automatically |
| `cas-number-theory` | 0.1.0 | `primep`, `next_prime`, `prev_prime`, `ifactor`, `divisors`, `totient`, `moebius`, `jacobi`, `chinese`, `numdigits` |
| `cas-algebraic` | 0.1.0 | `algfactor(poly, sqrt(d))` — factoring over quadratic extensions Q[√d] |
| `cas-multivariate` | 0.1.0 | `groebner`, `poly_reduce`, `ideal_solve` — Buchberger's algorithm, Gröbner bases, polynomial system solving |
| `cas-pattern-matching` | latest | `matchdeclare`, `defrule`, `apply1`, `apply2`, `tellsimp` |
| `cas-pretty-printer` | 0.4.0 | `pretty(node, dialect)` — MACSYMA / Mathematica / Maple / Lisp dialects; 2D box-model layout (fraction bars, superscripts, √ radicals) |
| `cas-mnewton` | 0.1.0 | `mnewton(f, x, x0)` — Newton's method numeric root-finding |

#### VM-level operations (live in `symbolic-vm`, not separate packages)

**Integration** (Risch phases 1–27): polynomials, rational functions, trig,
exp, log, IBP, partial fractions, inverse-trig, hyperbolic powers,
exp×hyperbolic, sinh/cosh mixed powers, Rothstein-Trager for rational
functions, EllipticF/K/E/Pi, log-power IBP, trig-of-log.

**Calculus:** `diff` (all elementary functions).

**Symbolic summation:** `sum`, `product` — Faulhaber, geometric, special
closed forms.

**Transforms:** `fourier`, `ifourier`, `laplace`, `ilt`.

**Special functions:** `erf`, `erfc`, `erfi`, `gamma`, `beta`, `si`, `ci`,
`shi`, `chi`, `li2`, `fresnel_s`, `fresnel_c`, `lambert_w`.

**Numeric/arithmetic:** `abs`, `floor`, `ceiling`, `mod`, `gcd`, `lcm`,
`cbrt`, `float`, `radcan`, `logcontract`, `logexpand`, `exponentialize`,
`demoivre`.

**Assumptions and properties:** `assume(x > 0)`, `is(x > 0)`, `forget()`,
`forget(relation)`, `declare(x, positive)`, `properties(x)`, and `propvars()`
— shared VM/session state, persists across REPL statements.

**Session/runtime tooling:** batch `.mac` file execution in the Python REPL,
`ev(expr, display2d)` 2D presentation, `showtime:true` diagnostics, friendly
syntax errors, and `?` / `? topic` help are all present.

**Constants pre-bound:** `%pi`, `%e`, `%i`.

---

### What is not yet built

#### ODE status and remaining gap

Phase 18/20 ODE coverage (PR #3049–#3062) and Phase 21 named variable-coefficient
ODE recognition (PR #3360, PR #3369) have all landed across the Python,
TypeScript, and Rust `cas-ode` packages.

| ODE type | Identifying form | Algorithm |
|---|---|---|
| **First-order linear** | `P(x)·y' + Q(x) = 0` | ✅ Integrating factor `μ = e^(∫P dx)` |
| **Separable** | `g(y)·y' = h(x)` | ✅ Integrate both sides |
| **Bernoulli** | `y' + P(x)·y = Q(x)·yⁿ` | ✅ `v = y^(1−n)` reduction |
| **Exact** | `M(x,y) dx + N(x,y) dy = 0`, `∂M/∂y = ∂N/∂x` | ✅ Implicit potential construction |
| **Homogeneous type** | `y' = f(y/x)` | ✅ `v = y/x` reduction |
| **2nd-order const-coeff homogeneous** | `a·y'' + b·y' + c·y = 0` | ✅ Characteristic equation; real/repeated/complex roots |
| **2nd-order const-coeff non-homogeneous** | `a·y'' + b·y' + c·y = g(x)` | ✅ Undetermined coefficients (EPT family) |
| **Euler-Cauchy** | `ax²y'' + bxy' + cy = 0` | ✅ Characteristic equation on `x^r` |
| **Variation of parameters** | `a·y'' + b·y' + c·y = f(x)`, any `f` | ✅ Wronskian-based VoP fallback |
| **Legendre** | `(1−x²)y''−2xy'+n(n+1)y=0` | ✅ Phase 21 numerical pattern matching → `LegendreP/Q(n,x)` |
| **Bessel** | `x²y''+xy'+(x²−ν²)y=0` | ✅ Phase 21 numerical pattern matching → `BesselJ/Y(ν,x)` |
| **Hermite** | `y''−2xy'+2ny=0` | ✅ Phase 21 numerical pattern matching → `HermiteH/H2(n,x)` |
| **Chebyshev** | `(1−x²)y''−xy'+n²y=0` | ✅ Phase 21 numerical pattern matching → `ChebyshevT/U(n,x)` |
| **Variable-coefficient (Frobenius)** | `P(x)·y'' + Q(x)·y' + R(x)·y = 0` | Still open: power series / Frobenius method |

Next ODE work should focus only on the Frobenius power-series case (irregular
singular points and series solutions around regular singular points) unless a
parity audit finds a smaller gap.

#### Factoring gaps

The `Factor` handler now covers several structural multivariate patterns via
`symbolic-vm` footholds landed in PRs #3073–#3120:

| Pattern | Example | Status |
|---|---|---|
| Common symbolic factor | `factor(x^2*y - y)` → `y*(x-1)*(x+1)` | ✅ #3073 |
| Bivariate perfect square | `factor(x^2 + 2*x*y + y^2)` → `(x+y)^2` | ✅ #3083 |
| Bivariate difference of squares | `factor(x^2 - y^2)` → `(x-y)*(x+1)` | ✅ #3090 |
| Bivariate cubic identities | `factor(x^3 - y^3)`, `factor(x^3 + y^3)` | ✅ #3098 |
| Four-term bilinear grouping | `factor(x*y + x*z + y + z)` → `(x+1)*(y+z)` | ✅ #3106 |
| Perfect cube expansion | `factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3)` → `(x+y)^3` | ✅ #3116 |
| Integer content + common factor | `factor(2*x*y + 2*x*z)` → `2*x*(y+z)` | ✅ #3120 |

**Still open:** General multivariate factoring via Hensel lifting (e.g.
`factor(x^2 + x*y - 2*y^2)` → `(x+2y)*(x-y)` when no common factor exists).
The footholds above all work by recognising known algebraic identities; a
general-case algorithm needs variable-by-variable Kronecker/Cantor-Zassenhaus.
This is the main remaining factoring gap.

#### Integration gaps

The Risch integration suite is ~90% complete. Known remaining gaps:

| Case | Example | Status |
|---|---|---|
| Elliptic integrals — first-kind foothold | `∫ 1/√(1-k²sin²θ) dθ` | ✅ #3141: returns `EllipticF(θ,k)`; definite 0…π/2 returns `EllipticK(k)` |
| Elliptic integrals — second kind | `∫ √(1-k²sin²θ) dθ`, `∫₀^(π/2) √(1-k²sin²θ) dθ` | ✅ Python PR #3173, TypeScript PR #3179, Rust PR #3178 — all three languages at 0.3.0/0.54.0 |
| Elliptic integrals — third kind | `∫₀^(π/2) 1/((1+n·sin²θ)·√(1-k²sin²θ)) dθ` | ✅ Python PR #3173, TypeScript PR #3179, Rust PR #3178 — all three languages at 0.3.0/0.54.0 |
| `∫ log(ax+b)^n dx`, `∫ Q(x)·log(x)^n dx` (Phase 26 log-power IBP) | IBP reduction `F_n = (ax+b)/a·log(ax+b)^n − n·F_{n-1}` and term-by-term poly×log^n | ✅ Python PR #3372 `symbolic-vm` 0.56.0; TS + Rust `symbolic-vm` 0.4.0 (same PR as Phase 27) |
| `∫ sin(log(x)) dx`, `∫ cos(log(x)) dx`, `∫ xᵏ·sin/cos(log(x)) dx` (Phase 27 trig-of-log) | u=log(x) substitution converts to exp×trig form; closed form `x^(k+1)·((k+1)trig(log x)∓cotrig(log x))/((k+1)²+1)` | ✅ Python PR #3373 `symbolic-vm` 0.57.0; TS + Rust `symbolic-vm` 0.4.0 |
| `∫ f(x)·g(x)` where neither integrates alone | General IBP fallback missing; only specific matched patterns work | Open |

#### Completed REPL and session features

| Feature | Priority | Notes |
|---|---|---|
| **Batch / file execution** | Done | Python `macsyma-repl` supports `--file` / `-f` execution for `.mac` programs. |
| **`declare` with properties** | Done | Python, TypeScript, and Rust sessions support `declare`, and properties feed assumption queries. |
| **`display2d` flag in `ev`** | Done | Python, TypeScript, and Rust sessions route `ev(expr, display2d)` through the 2D box pretty-printer. |
| **`propvars` / `properties`** | Done | Python, TypeScript, and Rust sessions expose property queries for declared symbols. |

#### Remaining REPL and session features

| Feature | Priority | Notes |
|---|---|---|
| **MACSYMA package system** | Low | `:load`, `:algebraic`, `:orthopoly` etc. No design yet. |
| **Symbolic infinite sums** | Low | `sum` handles Faulhaber and geometric closed forms. Hypergeometric / telescoping detection not done. |

#### Diagnostic / tooling gaps

| Feature | Notes |
|---|---|
| **Friendly error messages** | Python, TypeScript, and Rust MACSYMA syntax failures now surface as `Incorrect syntax ...` diagnostics with line/column and a caret where parser token metadata is available. |
| **`?` help system** | Python, TypeScript, and Rust sessions support `?` and `? topic` with a small curated help catalog for core runtime/CAS topics. |
| **`showtime`** | Python, TypeScript, and Rust sessions support `showtime:true` / `showtime:false` wall-clock timing per expression, including suppressed statements. |

---

## How to add a new SPICE element

1. Add a frozen dataclass to `elements.py`; extend the `Element` union type.
2. Write `_stamp_<name>(G, b, node_to_idx, el)` for the DC case.
3. Write `_stamp_<name>_ac(G, b, node_to_idx, el, omega)` for AC (complex G/b).
4. If the element requires a branch unknown (like a voltage source), add it to
   `_branch_sources()` and handle the extra row/column in the MNA matrix.
5. Wire into `_build_mna`, `_build_mna_ac`, and any analysis functions that
   iterate over elements.
6. Add tests covering DC op, transient, AC, and `.TF` where applicable.
7. Export from `__init__.py`; bump version; update CHANGELOG and README.

## How to add a new CAS operation to the MACSYMA pipeline

1. Implement pure Python functions operating on `IRNode` in the relevant
   `cas-*` package (or `symbolic-vm` if it's a generic algebraic operation).
2. Write a `build_<name>_handler_table() -> dict[str, Handler]` factory.
3. Register in `SymbolicBackend.__init__()`:
   `self._handlers.update(build_<name>_handler_table())`
4. Add the MACSYMA surface name to `macsyma_runtime/name_table.py`:
   `NAME_TABLE["macsyma_name"] = IRSymbol("HeadName")`
5. Write tests covering the new operations through the full pipeline
   (source string → REPL eval → expected result).
6. Bump versions; update CHANGELOGs; export from `__init__.py` if needed.
