# SPICE Engine & MACSYMA Pipeline — Status and Pending Work

> **Living document.** Updated each time a PR lands or new work is planned.
> Last updated: 2026-05-12. Branch point: spice-engine v0.10.0 (PR #2901),
> macsyma-runtime v1.25.0 (PR #2379).

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
| v0.2.0 | #2339 | Trapezoidal + Backward Euler integration, LTE-based adaptive timestep |
| v0.3.0 | #2342 | `BJT` element (Ebers-Moll, NPN/PNP) |
| v0.4.0 | #2344 | AC small-signal frequency sweep (`.AC`) |
| v0.5.0 | #2348 | DC transfer function (`.TF`) — transfer ratio, input/output impedance |
| v0.6.0 | #2353 | DC parameter sweep (`.DC`) |
| v0.7.0 | #2357 | DC sensitivity analysis (`.SENS`) — ∂V/∂P per element |
| v0.8.0 | #2359 | Monte Carlo (`.MC`) — Gaussian + uniform tolerance distributions |
| v0.9.0 | #2370 | Noise analysis (`.NOISE`) — Johnson-Nyquist + shot noise, adjoint method |

**Elements on main:** `Resistor`, `Capacitor`, `Inductor`, `VoltageSource`,
`CurrentSource`, `Diode`, `Mosfet`, `BJT`

**Analyses on main:** `dc_op`, `transient`, `ac_sweep`, `tf`, `dc_sweep`,
`sens_dc`, `mc_dc`, `noise_ac`

---

### What is in flight

| Version | PR | Status | What it adds |
|---|---|---|---|
| **v0.10.0** | **#2901** | Open — all CI green, pending merge | All four SPICE controlled sources: `VCVS` (E-element), `VCCS` (G-element), `CCCS` (F-element), `CCVS` (H-element). Correct MNA stamps for all four (DC + AC). `TfResult.gain` convenience property. Fix to CCCS stamp sign convention (was reversed; now matches SPICE F-element convention where positive current exits `n_plus`). 287 tests, 85% coverage. |

---

### What is not yet built

Items are listed in priority order within each group.

#### Group 1 — High value, well-scoped

| Feature | Why it matters | Design notes |
|---|---|---|
| **Time-varying source waveforms** | Every realistic transient simulation — step response, oscillator startup, filter characterisation — needs non-static sources. Currently `VoltageSource` and `CurrentSource` are DC-constant only. | Add a `waveform` field (optional) to `VoltageSource` / `CurrentSource` accepting a callable `(t: float) -> float` or one of `PwlWaveform`, `SinWaveform`, `PulseWaveform`, `ExpWaveform`. The transient engine already calls `_stamp_vsource` at each timestep; it just needs to pass the current `t`. SPICE spec in `spice-engine.md` section "Source Waveforms" (conformance target: PWL, PULSE, SIN, EXP). |
| **SPICE3 netlist parser** | Lets you run existing `.cir` / `.sp` files directly. Huge usability leap — you can grab any NGSPICE example and feed it in. | New package `spice-netlist-parser`. Grammar-driven (reuse `grammar-tools`). Conformance matrix in `spice-engine.md`: R, C, L, V, I, M (MOSFET), D, Q (BJT), E/G/F/H (controlled sources), X (subcircuit), `.tran`, `.dc`, `.ac`, `.op`, `.include`, `.subckt`, `.model`, `.param`. Returns a `Circuit` object identical to what you'd build in Python. |
| **Convergence aids** | Necessary for power electronics, startup transients, and any circuit with strongly coupled nonlinear devices (e.g. bandgap references). Newton-Raphson alone fails on these. | Three strategies described in `spice-engine.md`: (1) **Gmin stepping** — add small conductance across every node to ground, gradually reduce to zero. (2) **Source stepping** — scale all independent source voltages from 0 to full value. (3) **Pseudo-transient continuation** — add capacitors to ground and integrate to DC. Try in order; each is a fallback. |

#### Group 2 — Medium value

| Feature | Why it matters | Design notes |
|---|---|---|
| **Sub-circuit support (`.subckt` / X-element)** | Essential for hierarchical design — reusing a cell (e.g. an inverter) multiple times with different parameter values. | Add an `XInstance` element class. At circuit build time, expand each X-element by cloning its `.subckt` definition with renamed nodes (prefix with instance name). Parameters propagate via a dict of `{param: value}` substitutions. |
| **Behavioral modeling (B-element)** | Enables arbitrary nonlinear voltage/current sources defined by algebraic expressions — useful for behavioral models and controlled oscillators. | `BSource` element with `voltage_expr: str` or `current_expr: str`. At stamp time, parse and evaluate the expression numerically; linearise around the operating point for Newton. |
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

### What is on `main` (macsyma-runtime v1.25.0, Phase 34)

Every item below is wired end-to-end: surface syntax → compile → VM → correct result.

#### Core infrastructure

| Package | Version | What it provides |
|---|---|---|
| `symbolic-ir` | latest | `IRSymbol`, `IRInteger`, `IRRational`, `IRFloat`, `IRString`, `IRApply` node types |
| `symbolic-vm` | latest | Pluggable VM, `SymbolicBackend`, arithmetic, Risch integration (phases 1–14+), numeric folding, 100+ handlers |
| `macsyma-lexer` | 0.1.0 | Grammar-driven tokenizer |
| `macsyma-parser` | 0.1.0 | Grammar-driven parser |
| `macsyma-compiler` | 0.9.0 | AST → IR; 60+ MACSYMA identifier mappings in name table |
| `macsyma-runtime` | 1.25.0 | History (`%`, `%i1`, `%o1`, …), `;`/`$` terminators, `kill`, `ev`, `MacsymaBackend` |

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
| `cas-ode` | 0.1.0 | `ode2` — first-order linear, separable, 2nd-order constant-coefficient (all 3 characteristic-root cases) |
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

**Integration** (Risch phases 1–14+): polynomials, rational functions, trig,
exp, log, IBP, partial fractions, inverse-trig, hyperbolic powers,
exp×hyperbolic, Rothstein-Trager for rational functions.

**Calculus:** `diff` (all elementary functions).

**Symbolic summation:** `sum`, `product` — Faulhaber, geometric, special
closed forms.

**Transforms:** `fourier`, `ifourier`, `laplace`, `ilt`.

**Special functions:** `erf`, `erfc`, `erfi`, `gamma`, `beta`, `si`, `ci`,
`shi`, `chi`, `li2`, `fresnel_s`, `fresnel_c`, `lambert_w`.

**Numeric/arithmetic:** `abs`, `floor`, `ceiling`, `mod`, `gcd`, `lcm`,
`cbrt`, `float`, `radcan`, `logcontract`, `logexpand`, `exponentialize`,
`demoivre`.

**Assumptions:** `assume(x > 0)`, `is(x > 0)`, `forget()`, `forget(relation)`
— shared VM state, persists across REPL statements.

**Constants pre-bound:** `%pi`, `%e`, `%i`.

---

### What is not yet built

#### ODE gaps (in `cas-ode` spec; `ode2` currently handles 4 types)

The following five ODE classes are specified in `code/specs/phase18-ode-completion.md`
but not yet implemented. They cover the bulk of remaining textbook ODE problems.

| ODE type | Identifying form | Algorithm |
|---|---|---|
| **Bernoulli** | `y' + P(x)·y = Q(x)·yⁿ` | Substitution `v = y^(1−n)` reduces to linear |
| **Exact** | `M(x,y) dx + N(x,y) dy = 0`, `∂M/∂y = ∂N/∂x` | Integrate M w.r.t. x, match N to find integration function |
| **Homogeneous** | `y' = f(y/x)` | Substitution `v = y/x` reduces to separable |
| **2nd-order non-homogeneous, constant coefficients** | `a·y'' + b·y' + c·y = g(x)` | Method of undetermined coefficients or variation of parameters |
| **Variable-coefficient 2nd-order** | `P(x)·y'' + Q(x)·y' + R(x)·y = 0` | Power series / Frobenius method |

All five are in `code/specs/phase18-ode-completion.md`. Target: `cas-ode` 0.2.0,
new `ode2` type-dispatch branch for each form.

#### Factoring gaps

`cas-factor` is **univariate only**. Multivariate factoring over Z is not
implemented. `cas-multivariate` has Gröbner bases but not a factoring front-end.

Example that currently returns unevaluated:
```
factor(x^2*y - y)       → should give y*(x-1)*(x+1)
factor(x^2 + 2*x*y + y^2) → should give (x+y)^2
```

Design: add a `factor_multivariate` function to `cas-factor` (or a new
`cas-multivariate-factor` package) using the square-free decomposition +
variable-by-variable Hensel lifting approach. Wire as an extended case in the
existing `factor_handler`.

#### Integration gaps

The Risch integration suite is ~85% complete. Known remaining gaps:

| Case | Example | Status |
|---|---|---|
| `sinh^m · cosh^n`, both `m, n ≥ 2` | `∫ sinh²(x)·cosh²(x) dx` | Returns unevaluated. Use double-angle reduction first. |
| Elliptic integrals | `∫ 1/√(1-k²sin²θ) dθ` | Non-elementary; should return unevaluated with a recognised `EllipticK(k)` form |
| `∫ f(x)·g(x)` where neither factor integrates alone | General product rule fallback missing; currently only specific matched patterns work |

#### REPL and session features

| Feature | Priority | Notes |
|---|---|---|
| **Batch / file execution** | High | No way to run a `.mac` file non-interactively. Add a `macsyma_run_file(path)` entry point to `macsyma-runtime` and a `--file` flag to `macsyma-repl`. |
| **`declare` with properties** | Medium | `declare(n, integer)`, `declare(f, antisymmetric)`, `declare(x, positive)`. Properties feed into the assumption system for automatic simplification. |
| **`display2d` flag in `ev`** | Medium | `ev(expr, display2d)` should route output through `cas-pretty-printer` 2D layout. Currently the `pretty()` API exists but isn't triggered by `ev`. |
| **MACSYMA package system** | Low | `:load`, `:algebraic`, `:orthopoly` etc. No design yet. |
| **Symbolic infinite sums** | Low | `sum` handles Faulhaber and geometric closed forms. Hypergeometric / telescoping detection not done. |
| **`propvars` / `properties`** | Low | Querying what properties a symbol has been declared with. |

#### Diagnostic / tooling gaps

| Feature | Notes |
|---|---|
| **Friendly error messages** | Syntax errors from the parser surface as raw Python exceptions. Should produce MACSYMA-style messages: `Incorrect syntax: …` with the offending token highlighted. |
| **`?` help system** | No documentation lookup. A thin wrapper over docstrings would cover most cases. |
| **`showtime`** | Python REPL supports `showtime:true` / `showtime:false` wall-clock timing per expression. TypeScript and Rust runtime parity still pending. |

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
