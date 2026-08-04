# SPICE Engine & MACSYMA Pipeline — Status and Pending Work

> **🎉 MACSYMA truly-finish plan closed (2026-05-29).** All eight
> tracks of `code/specs/macsyma-truly-finish-plan.md` (F through N)
> have landed across Python, TypeScript, and Rust.  Every CHANGELOG
> `Unreleased` section in the MACSYMA pipeline is empty.  MACSYMA is
> considered feature-complete for mainstream Maxima 5.x parity; new
> work is feature-driven (e.g. Maple frontend) rather than gap-driven.

> **🎉 MACSYMA finish-plan closed (2026-05-28).** All five tracks of
> `code/specs/macsyma-finish-plan.md` (ten sub-tracks total) have
> shipped across all three languages.  Sub-track PR map:
>
> | Sub-track | PR | Description |
> |---|---|---|
> | A1 | ✅ #4552 | Phase 86 generic recogniser port (TS + Rust) |
> | A2 | ✅ #4557 | Delete 42 redundant grid helpers (all 3 langs) |
> | B1 | ✅ #4558 | Apart simple-roots port (TS + Rust) |
> | B2 | ✅ #4559 | Apart-retry telescope chain port (TS + Rust) |
> | B3 | ✅ #4560 | Apart repeated-linear-factors port (TS + Rust) |
> | C1 | ✅ #4561 | Frobenius / power-series ODE (Python) |
> | C2 | ✅ #4562 | Frobenius port (TS + Rust) |
> | D1 | ✅ #4563 | Bivariate Hensel lifting (Python `cas-factor`) |
> | D2 | ✅ #4564 | Bivariate Hensel port (TS + Rust) |
> | E1 | ✅ #4569 | Generic tabular IBP fallback (Python `symbolic-vm`) |
> | E2 | ✅ this PR | Generic IBP port (TS + Rust `symbolic-vm` 0.16.0) |
>
> The MACSYMA pipeline is now considered done for the purposes of this
> repo.  New work is feature-driven (e.g. Maple frontend) rather than
> gap-driven.

> **MACSYMA parity audit follow-up (2026-06-06).** A fresh `Apart`
> audit found one post-finish residual gap: mixed rational-root plus
> irreducible-denominator factors still returned unevaluated
> `Apart(...)` even though pure irreducible and fully split repeated
> rational factors were handled. This PR closes that gap across Python,
> TypeScript, and Rust by subtracting rational-pole terms and emitting
> the remaining proper irreducible residual.

> **MACSYMA summation audit follow-up (2026-06-06).** Infinite telescope
> limits now recognise direct decaying exponential terms, not only rational
> quotients with diverging denominators. Across Python, TypeScript, and Rust,
> `exp(-k)` and `b^(-k)` with `|b| > 1` are treated as vanishing at infinity,
> closing structurally telescoping exponential tails while preserving
> conservative fallthrough for growing or sign-ambiguous exponentials.

> **MACSYMA limit audit follow-up (2026-06-06).** Advanced limits at infinity
> now recognise bounded numerators over diverging denominators across Python,
> TypeScript, and Rust. This closes `limit(sin(x)/x, x, inf)` and
> `limit(cos(x)/(x^2+1), x, minf)` to exact `0`, rather than accepting the
> oscillatory numerator's direct `sin(inf)` / `cos(inf)` classification as an
> unevaluated `Limit(...)`.

> **SPICE completion follow-up inventory (2026-06-06).** The 2026-06-05
> release cut closed the planned SPICE 1970s compatibility slice, but a live
> follow-up audit found one real parity gap: Rust had advanced named-corner
> wrappers and stable tables that Python and TypeScript still lacked for
> `.MC`, `.SENS`, `.NOISE`, and two-port S-parameters. That gap is now closed
> in the Python and TypeScript packages with native wrappers and stable
> tab-separated direct/corner table output. The remaining work is not a blocker
> for the current compatibility slice, but it is still real project work:
> Python/TypeScript parity for the wider Rust named-corner family
> (fixed/adaptive transient samples, PSS, Fourier, distortion, constrained
> pole-zero, and temperature operating-point corners), broader ordered parallel
> corner orchestration outside the current Rust helpers, full `hardware-vm`
> scheduler integration, Verilog-A/custom compact models, production sparse/KLU
> and SPICE3-era raw/control/BSIM surfaces, and richer nonlinear distortion
> accuracy beyond the Phase 8 executable footholds.

> **Living document.** Updated each time a PR lands or new work is planned.
> Last updated: 2026-06-06 (MACSYMA `Apart` mixed residual parity, SPICE
> completion follow-up inventory, Python and TypeScript advanced-corner
> parity, and explicit remaining roadmap).
>
> **Phase 48 — Apart for repeated linear factors (Python only, so far):**
> `symbolic-vm` 0.72.0 (PR #3927).  Extends ``Apart`` to decompose
> denominators of the form ``∏_r (x − r)^{m_r}`` where each ``r`` is
> rational and ``m_r ≥ 1``.  Algorithm: for each root r of
> multiplicity m, Taylor-expand ``P(r + t)`` and
> ``Q(r + t) = den(r + t) / t^m`` to order ``m − 1``, then power-
> series-divide to get ``φ(t) = P(r + t) / Q(r + t)``; the j-th
> Taylor coefficient is the residue ``A_{r, m − j}``.  Everything
> stays exact (``Fraction`` arithmetic).  Closes the last Phase 45-
> documented gap: ``∑_{k=1}^∞ (2k+1)/(k²(k+1)²) = 1``.  TS/Rust
> ports remain blocked on porting ``Apart`` itself.
>
> **Phase 47 — Nested-Add flattening (Python + TS + Rust):**
> `symbolic-vm` 0.71.0 (Python PR #3922), 0.12.0 (TS PR #3923),
> 0.12.0 (Rust PR #3923).  Add handler now flattens nested
> ``Add(Add(k, 1), 1) → Add(k, 2)`` trees so structural-equality
> consumers (e.g. the cas-summation telescope detector) see
> canonical forms.  Substrate fix — benefits any CAS module that
> pattern-matches ``Add(k, c)``.  Closes the shifted-denominator
> case ``∑ 1/((k+1)(k+2)) = 1/2``.
>
> **Phase 46 — Apart-retry constant-numerator widening (Python +
> TS + Rust):** `symbolic-vm` 0.70.0 (Python PR #3918), `cas-summation`
> 0.6.0 (TS+Rust PR #3920).  Phase 40's normaliser now recognises
> ``Div(c, d)`` with literal ``c < 0`` as a negation (in addition
> to top-level ``Neg(x)``), so summands like ``5/(k(k+1))`` whose
> Apart output folds the sign into the numerator (``Add(Div(-5,
> k+1), Div(5, k))``) close via the telescope chain.
>
> **Phase 45 — End-to-end integration tests (Python only):**
> `symbolic-vm` 0.69.0 (PR #3914).  Tests-only release pinning the
> Apart + telescope cross-phase behaviour.  Documented 3 gaps as
> ``pytest.skip`` markers; all three are now closed by Phases 46–48.
>
> **Phase 43+44 — Transcendental vanishing-at-infinity (Python +
> TS + Rust):** `cas-summation` 0.5.0–0.6.0.  Extended the recogniser
> for ``∑ [g(k+1) − g(k)]`` to handle ``Exp(h(k))``, ``Pow(b, h(k))``
> with ``b > 1``, ``Log(...)``, and ``Mul(...)`` factors that diverge.
> Sign-aware via ``_polynomial_leading_coeff_sign_in_k``.
>
> **Phase 42 — Degree-aware vanishing-at-infinity (Python only, so far):**
> `cas-summation` 0.4.0 + `symbolic-vm` 0.66.0 (PR #3887 ✅ merged).
> Widens Phase 41's narrow constant-numerator recogniser to handle any
> proper rational ``P(k)/Q(k)`` shape with ``deg(P) < deg(Q)``.  New
> ``_polynomial_degree_in_k`` helper returns the polynomial degree of an
> IR node in ``k`` (or ``None`` for non-polynomial shapes like Sin/Log/
> fractional-Pow).  Closes telescopes like
> ``∑_{k=1}^∞ [k/(k²+1) − (k+1)/((k+1)²+1)] = 1/2`` end-to-end.
> Transcendental limits (e.g. ``sin(k)/k²``, ``log(k)/k``) and TS/Rust
> ports still deferred.
>
> **Phase 41 — Limit-aware infinite telescope (Python only, so far):**
> `cas-summation` 0.3.0 + `symbolic-vm` 0.65.0 (PR #3880 ✅ merged).
> Extends the Phase 39 telescope detector to handle `hi = %inf` when
> `g(k)` provably vanishes at infinity.  The narrow vanishing-at-infinity
> recogniser handles `Div(constant-in-k, positive-degree-polynomial-in-k)`
> shapes — the family Apart produces from a rational summand whose
> denominator factors over ℚ.  Combined with the existing Phase 40
> Apart-retry path, `∑_{k=1}^∞ 1/(k(k+1))` closes in one dispatch to
> the scalar integer `1`.  TS / Rust ports blocked on porting `Apart`.
>
> **Phase 40 — Apart + telescope composition (Python only, so far):**
> `symbolic-vm` 0.64.0 (PR #3872 ✅ merged).  The `sum_handler` now
> composes the existing partial-fraction decomposition (`Apart`) with the
> Phase 39 telescoping detector, so classic sums like
> `∑ 1/(k·(k+1)) = 1 − 1/(N+1)` close in one step.  When the initial
> `evaluate_sum` returns unevaluated AND the summand has a `Div` head,
> the handler runs `Apart(f, k)` once, normalises the
> `Add(Neg(...), ...)` output into `Sub(...)` form, deep-canonicalises
> `Add` operand order, and re-runs `evaluate_sum`.  Irreducible
> denominators (e.g. `1/(k²+1)`) stay unevaluated.  TypeScript and Rust
> ports are blocked on first porting the `Apart` handler itself (those
> backends don't have partial-fraction decomposition yet).
>
> **Phases 37 + 38 + 39 sprint complete:**
> - **Phase 37** — Weierstrass log form cos branch covers `b < −|a|`: Python
>   `symbolic-vm` 0.62.0 (PR #3683 ✅ merged), TypeScript 0.10.0 (PR #3685 ✅
>   merged), Rust 0.10.0 (PR #3689 ✅ merged). Removed overly conservative
>   guards in `_try_weierstrass_log_form` cos branch — the `Abs` wrapping on
>   the log argument already handles the sign flip across `b = ±|a|`.
> - **Phase 38** — Weierstrass closed forms lifted to linear trig arguments
>   `sin(α·x + β)` / `cos(α·x + β)`: Python `symbolic-vm` 0.63.0 (PR #3690 ✅
>   merged), TypeScript 0.11.0 (PR #3691 ✅ merged), Rust 0.11.0 (PR #3692 ✅
>   merged). The inner change of variable `u = α·x + β` (with `du = α·dx`)
>   gives a `1/α` outer scaling and `tan((α·x+β)/2)` in place of `tan(x/2)`;
>   every existing branch (arctan / degenerate / log) inherits the
>   generalisation unchanged.
> - **Phase 39** — Telescoping sum recognition in `cas-summation`: Python
>   0.2.0 (PR #3706 ✅ merged), TypeScript 0.2.0 (PR #3720 ✅ merged), Rust
>   0.2.0 (PR #3724 ✅ merged). The dispatcher detects structurally
>   telescoping summands `f = g(k+1) − g(k)` (or the antisymmetric
>   `g(k) − g(k+1)`) and emits `g(hi+1) − g(lo)` (resp. `g(lo) − g(hi+1)`).
>   Pure structural detection — no partial-fraction expansion (`1/(k(k+1))`
>   needs an `Apart` step first, deferred to a follow-on phase).  Infinite
>   ranges fall through (limit-aware phase deferred).
>
> Prior sprints still on main: TypeScript 0.2.0 releases (PR #3170 ✅
> merged), Rust 0.2.0 releases (PR #3171 ✅ merged), Python EllipticE/Pi —
> `symbolic-vm` 0.55.0 (PR #3173 ✅ merged), TypeScript EllipticE/Pi —
> `symbolic-vm` 0.3.0 (PR #3179 ✅ merged), Rust EllipticE/Pi —
> `symbolic-vm` 0.3.0 (PR #3178 ✅ merged), Python `macsyma-runtime` 1.26.0,
> `spice-engine` 0.13.0, `rust/macsyma-runtime` 0.3.0,
> Phase 21 named variable-coefficient ODEs (PRs #3360, #3369 ✅ merged),
> version housekeeping chore (PR #3354 ✅ merged),
> Phase 26 log-power IBP Python (PR #3372 ✅ merged),
> Phase 27 trig-of-log integration Python `symbolic-vm` 0.57.0 (PR #3373 ✅ merged),
> Phase 28 general IBP Python `symbolic-vm` 0.58.0 (PR #3380 ✅ merged),
> Phase 28 TypeScript + Rust ports `symbolic-vm` 0.5.0 (PR #3381 ✅ merged),
> Phases 29–33 algebraic simplification (Abs/Sqrt/Log/Exp/Trig) — TypeScript +
> Rust `symbolic-vm` 0.6.0 (PR #3468 ✅ merged),
> Phase 34 Weierstrass substitution `∫ 1/(a + b·sin/cos x) dx` — Python
> `symbolic-vm` 0.59.0 (PR #3472), TypeScript 0.7.0 (PR #3473), Rust 0.7.0
> (PR #3475), Phase 35 degenerate `a² = b²` cases, Phase 36 log-form
> `a² < b²` sin branch + cos `b > |a|` — all merged.

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
| v0.9.0 | #2370 | Noise analysis (`.NOISE`) — Johnson-Nyquist, MOSFET channel thermal, and shot noise; adjoint method |

**Elements on main:** `Resistor`, `Capacitor`, `Inductor`, `MutualInductor`,
`TransmissionLine`, `VoltageSource`, `CurrentSource`, `BSource`, `Diode`,
`Mosfet`, `BJT`, `JFET`

**Analyses on main:** `dc_op`, `transient`, `ac_sweep`, `tf`, `dc_sweep`,
`sens_dc`, `mc_dc`, `noise_ac`

**Waveforms on main:** `PwlWaveform`, `SinWaveform`, `PulseWaveform`, `ExpWaveform`

**Netlist parser on main:** `spice-netlist-parser` 0.2.0 — parses R, C, L, V,
I, M (MOSFET), D, Q (BJT), J (JFET), K (mutual inductor), T (lossless
transmission line), E/G/F/H (controlled sources), `.subckt` / X instances,
`.tran`, `.dc`, `.ac`, `.op`, `.tf`, `.sens`, `.mc`, `.noise`, `.model` cards;
IC parameters for C/L; AC phasor specs for V/I sources.

---

### What is in flight

- 1970s SPICE compatibility planning: the active workstream is tracked in
  `spice-1970s-compatibility.md`. JFET device/model-card support, mutual
  inductors, ideal transmission lines, Gear-2 transient integration,
  pseudo-transient DC continuation, 1970s model-card depth, classic text output
  cards including direct and named-corner DC operating-point temperature
  tables, and the first Phase 8 small-signal distortion / pole-zero footholds
  are now reflected there with per-phase status.

---

### What is not yet built

Items are listed in priority order within each group.

#### Group 1 — High value, well-scoped

All Group 1 items have shipped. See "What is on main" above.

#### Group 2 — Medium value

All Group 2 items have shipped: programmatic subcircuits landed in PR #3389
and the sparse real solver path landed in PR #3391.

#### Group 3 — Lower priority / longer horizon

| Feature | Design notes |
|---|---|
| **S-parameter extraction** | Two-port network characterisation. Run AC sweep, compute Y-parameters from node voltages and port currents, convert to S-parameters. Shipped in PR #3490; direct and named-corner S-parameter text tables plus named S-parameter corners are now exposed in the live Rust SPICE package. |
| **Periodic steady-state (PSS)** | RF / oscillator analysis. Shooting-Newton method: find the initial condition `x(0)` such that `x(T) = x(0)`. Source-period estimation for periodic `SIN` / `PULSE` waveforms shipped in PR #3524; one-period node-closure residual helpers shipped in PR #3534; tolerance-aware residual closure shipped in PR #3540; branch-current residual closure shipped in PR #3553; ordered residual vectors shipped in PR #3560; residual vector norms shipped in PR #3566; finite-difference residual Jacobians shipped in PR #3570; Newton correction helpers shipped in PR #3578; Newton candidate helpers shipped in PR #3588; one-step Newton iteration acceptance shipped in PR #3770; bounded Newton solve shipped in PR #3776; direct PSS analysis output is now exposed in the live Rust SPICE package as a steady-state text table over selected voltage/current probes. PSS can also be evaluated and rendered as stable text tables across named corners in the live Rust SPICE package. |
| **Multi-corner parallel sweep** | Run the same analysis at N PVT corners in parallel goroutines / subprocesses. Mostly an orchestration problem once the core engine is solid. DC operating-point corners shipped in PR #3495; DC source sweep corners shipped in PR #3501; AC frequency sweep corners shipped in PR #3511; transfer-function corners shipped in PR #3516; Monte Carlo DC, DC sensitivity, AC noise, S-parameters, PSS, constrained pole-zero, fixed-step and adaptive transient samples, DC temperature operating points, Fourier post-processing, and transient-projected distortion corners are now exposed in the live Rust SPICE package; DC operating-point, AC frequency sweep, DC source sweep, transfer-function, Monte Carlo DC, DC sensitivity, AC noise, S-parameter, PSS, constrained pole-zero, fixed-step and adaptive transient sample, DC temperature operating-point, Fourier, and transient-projected distortion corner results also have stable text tables. Python and TypeScript now expose native sequential named-corner wrappers and stable direct/corner tables for Monte Carlo DC, DC sensitivity, AC noise, and S-parameters. Rust DC operating-point, `.DC` source-sweep, `.AC` frequency-sweep, `.TF` transfer-function, Monte Carlo DC, DC sensitivity, AC noise, and S-parameter corners can now also be evaluated through order-preserving parallel helpers; Python/TypeScript parity for the wider Rust corner family and broader parallel corner orchestration remain future work. |
| **Mixed-signal coupling with `hardware-vm`** | AMS simulation — digital events feed into analog SPICE nodes and vice versa. Long-range project; `hardware-vm.md` spec describes the interface. SPICE-side Rust footholds now expose binary digital event timelines and named digital event streams as finite-edge PWL voltage sources, derive bridge breakpoint schedules over event starts and finite-edge transition endpoints, run direct fixed-step, adaptive, and named-corner digital-input transient bridge fixtures, sample one or more transient probes back into thresholded named digital event streams, and render bridge schedules, single, named multi-signal, fixed-step bridge, adaptive bridge, and cornered bridge event streams as stable tab-separated text tables; full `hardware-vm` scheduler integration remains future work. |
| **Verilog-A compact models** | Custom device models. Requires a Verilog-A parser (`code/specs/verilog-a-parser.md` is referenced but not written). |

#### SPICE 1970s compatibility

The compatibility workstream is split into concrete phases in
`code/specs/spice-1970s-compatibility.md`. This is the plan to move the current
solver from a strong SPICE-compatible educational engine toward a recognizable
Berkeley SPICE1/SPICE2-era simulator surface. Phase 1 JFET support, Phase 2
mutual inductors, and Phase 3 ideal transmission lines are complete for the
current compatibility target.

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

#### Phase 28: general IBP for poly×log(Q) and poly×atan(Q) — TypeScript + Rust ports

| Package | Version | Notes |
|---|---|---|
| `typescript/symbolic-vm` | 0.5.0 | Phase 28 `∫ P(x)·log(Q(x)) dx` + `∫ P(x)·atan(Q(x)) dx` for non-linear Q; limited rational integrator (Cases A/B) |
| `rust/symbolic-vm` | 0.5.0 | Same as TypeScript 0.5.0; uses i128 RatPoly arithmetic |

#### Phases 29–33: algebraic simplification rules — TypeScript + Rust ports (PR #3468)

The Python `symbolic-vm` has long shipped algebraic rules for `Abs`, `Sqrt`,
`Log`, `Exp` and the trig/hyperbolic family.  PR #3468 closes the gap for the
TypeScript and Rust ports.  Five rule families fire on every re-evaluation of
the affected expressions and are guarded by the `simplify` flag (the strict
numeric backend is unaffected).

| Package | Version | Notes |
|---|---|---|
| `typescript/symbolic-vm` | 0.6.0 | Phases 29–33; 119 tests pass |
| `rust/symbolic-vm` | 0.6.0 | Phases 29–33; 153 tests pass |

| Phase | Rule family | Sample identity |
|---|---|---|
| **29** | `Abs` / `Sqrt` algebraic rules | `Abs(Abs(x))→Abs(x)`, `Abs(-x)→Abs(x)`, `Abs(x^(2k))→x^(2k)`, `sqrt(x²)→|x|`, `sqrt(x⁴)→x²` |
| **30** | `Log` / `Exp` cancellation | `log(exp(x))→x`, `exp(log(x))→x`, `exp(n·log(x))→x^n` |
| **31** | Trig / hyperbolic symmetry + arc-cancellation | `sin(-x)→-sin(x)`, `cos(-x)→cos(x)`, `sin(asin(x))→x` etc. |
| **32** | Inverse-trig odd symmetry + acos reflection | `asin(-x)→-asin(x)`, `atan(-x)→-atan(x)`, `acos(-x)→π-acos(x)` |
| **33** | Trig exact values at rational π multiples | `sin(π/6)→1/2`, `cos(π/4)→√2/2`, `tan(π/3)→√3` (16+16+7 entries) |

**Intentionally omitted** in the TS/Rust ports: `log(x^n)→n·log(x)` and
`sqrt(x²)→x` — both require an `x≥0` assumption context, and the TS/Rust
ports do not yet have an assumption system.  Python keeps those rules behind
`is_nonneg(x)` checks.

**Update (2026-06-06):** TS/Rust MACSYMA runtime sessions now feed their
assumption context into direct `abs`, `sqrt`, and `log` evaluation, matching
the Python reference for `assume(x >= 0); sqrt(x^2)`, `log(x^n)`, and
`abs(x)`. The lower-level TS/Rust `symbolic-vm` packages remain
assumption-free by design; the MACSYMA runtime layer owns session assumptions.

**Pi-multiple detection** (Phase 33) covers both numeric (`IRFloat ≈ q·π`,
denominators {1,2,3,4,6}) and structural IR (`%pi`, `Neg(%pi)`, `Mul(n,%pi)`,
`Div(%pi,n)`, `Div(Mul(n,%pi),d)`) shapes, keyed via a reduced-fraction string
into period-2 (sin/cos) and period-1 (tan) tables.

**Known regression** baked into the test suite: `D(x^x, x)` now returns
`x^x · (log(x) + 1)` instead of `exp(x·log(x)) · (log(x) + 1)` because the
new `exp(x·log(x)) → x^x` rule fires eagerly during the derivative reduction.
The simplified form is mathematically equivalent.

#### Phase 34: Weierstrass substitution for ∫ 1/(a + b·sin/cos x) dx — all three languages

The substitution `u = tan(x/2)` produces `sin(x) = 2u/(1+u²)`,
`cos(x) = (1−u²)/(1+u²)`, `dx = 2/(1+u²) du` and reduces both
canonical denominator shapes to rational functions of `u` whose closed
form is an arctan whenever `a² > b²` (the denominator never crosses
zero on ℝ).

Closed forms now produced by all three language ports:

    ∫ 1/(a + b·sin x) dx  =  (2/√(a²−b²)) · arctan((a·tan(x/2) + b)/√(a²−b²))
    ∫ 1/(a + b·cos x) dx  =  (2/√(a²−b²)) · arctan(√((a−b)/(a+b)) · tan(x/2))

Numerator constants `c` scale the result.  Each port accepts integer
and rational `a, b`; perfect-square discriminants (e.g. a=5, b=3 →
disc=16) collapse to integer scalars without leaving a `Sqrt` node.

| Package | Version | PR | Tests |
|---|---|---|---|
| `python/symbolic-vm` | 0.59.0 | #3472 | 14 new (mirrors below) |
| `typescript/symbolic-vm` | 0.7.0 | #3473 | 14 new |
| `rust/symbolic-vm` | 0.7.0 | #3475 | 14 new |

**Originally deferred** at Phase 34; status as of 2026-05-20:

- ~~`a² < b²` — log form on `(a·tan(x/2) + b ± √(b²−a²))`.~~
  **✅ closed by Phase 36 (sin branch + cos `b > |a|`) and Phase 37 (cos
  `b < −|a|`).** The same `log|(D + (b−a)·tan(arg/2)) / (D − (b−a)·tan(arg/2))|`
  formula covers both sign regimes thanks to the `Abs` wrapping.
- ~~`a² = b²` — degenerate, reduces to a rational in `tan(x/2)`.~~
  **✅ closed by Phase 35** in all four sign combinations.
- ~~`a ≤ 0` for the cos case~~ — **✅ closed by Phase 37**.
- ~~Non-bare trig arguments (e.g. `sin(2x)`)~~ — **✅ closed by Phase 38**
  via the inner substitution `u = α·x + β` (with `du = α·dx`); every
  closed-form branch inherits the lift unchanged.
- Symbolic `a` or `b` — still deferred. Discriminant sign undecidable
  without an assumption context; only the Python port has assumptions
  today and even there the Phase 34 helper requires `a, b ∈ ℚ`.
- Trig argument involving `x²` or other nonlinear forms — out of scope
  for Weierstrass; would need a separate substitution phase.

**Version sequencing**: the TypeScript and Rust ports both jump
`0.5.0 → 0.7.0` to leave `0.6.0` for the in-flight Phase 29-33
algebraic-rules port (PR #3468).  Final order: 0.5.0 → 0.6.0 → 0.7.0.

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

#### `Expand` has no handler — ✅ fixed (2026-07-03)

Macsyma's `expand(...)` surface function and `ev(expr, expand)` both route
through `apply(sym("Expand"), …)` (`macsyma-runtime/src/lib.rs`), but
`symbolic-vm`'s `build_handler_table` registered no handler under the string
`"Expand"` — verified empirically: `expand((x+1)^2)` returned the unevaluated
input, not the distributed polynomial a user would expect.

**Fix**: added `cas_simplify::expand` — a faithful port of the Python
reference's general recursive-distributor path (`_sym_expand`/
`_sym_expand_mul`/`_sym_expand_pow`), generalized to n-ary `Add`/`Mul` and
guarded against the doubly-exponential term-count blowup square-and-multiply
can hit on a multi-term base (`EXPAND_MAX_TERMS`, checked *before* every
distribution step, not after). Registered as the `Expand` handler in
`MacsymaBackend` (the same decorator-over-`build_handler_table` pattern
already used for `Simplify`/`RatSimplify`/`Radcan`/etc.) — the shared
`symbolic-vm` table itself is still unchanged, matching how every other
Macsyma-specific head is layered on. `expand((x+1)^2)` now correctly returns
`1 + x + x + x*x`.

**Known remaining gap, honestly scoped out of this fix** — ✅ closed
(2026-07-16): the fix above deliberately deferred like-term collection (the
two `x` terms staying separate rather than combining into `2*x`, and `x*x`
not folding into `x^2`) as a "real, separate, more involved" follow-up. That
follow-up landed as `cas_simplify::collect_terms` — a bottom-up pass that
flattens `Add`/`Sub` into signed terms, decomposes each into a
`(coefficient, monomial)` pair (reusing `numeric_fold`'s exact-rational
accumulator), groups by monomial (summing coefficients, dropping
exact-zero groups so real cancellations disappear), and rebuilds. `expand`
now runs this pass before its final `simplify` call, so `expand((x+1)^2)`
returns `1 + 2*x + x^2`, not the raw `1 + x + x + x*x`. Python's reference
only reaches that clean form via a *second*, single-variable
rational-polynomial fast path (`to_rational`/`from_polynomial`); this port
takes the general (any-number-of-variables) path and collects afterward,
rather than reproducing that fast path.

This also unblocks Wolfram's own `Expand[...]` (MA04 §24.2) via the identical
thin-wiring pattern used for `Simplify[...]`.

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
| `∫ P(x)·log(Q(x)) dx`, `∫ P(x)·atan(Q(x)) dx` for non-linear Q (Phase 28 general IBP) | IBP with residual integrated via polynomial long division + Case A (prop to D′) / Case B (const/quadratic) | ✅ Python PR #3380 `symbolic-vm` 0.58.0; TS + Rust `symbolic-vm` 0.5.0 (PR #3381) |
| `∫ exp(c·x²) dx` for exact rational `c ≠ 0` (Phase 23 error-function forms) | Emits `Sqrt(%pi)/(2*sqrt(|c|))*Erf(sqrt(|c|)*x)` for `c < 0` and the matching `Erfi` form for `c > 0`. | ✅ Python already complete; TS + Rust `symbolic-vm` 0.18.0 port the special-function fallback. |
| `∫ sin/cos(a·x²) dx` and `∫ sin/cos(q·π·x²) dx` (Phase 23 Fresnel forms) | Emits `FresnelS` / `FresnelC` with the same scaling conventions as the Python reference; the canonical `q=1/2` case returns `FresnelS(x)` / `FresnelC(x)`. | ✅ Python already complete; TS + Rust `symbolic-vm` 0.17.0 port the special-function fallback. |
| `∫ c / (a + b·sin/cos(α·x + β)) dx` for rational `a, b, α, β` with `α ≠ 0` (Phases 34–38 Weierstrass family) | `u = tan((α·x+β)/2)` substitution: arctan form (`a² > b²`), degenerate `a² = b²`, log form (`a² < b²`), and linear-argument lifting all wired. | ✅ All discriminant regimes and both `b > |a|` / `b < −|a|` cos branches closed across all three languages. Phase 34 (PRs #3472/#3473/#3475 — arctan), Phase 35 (degenerate), Phase 36 (log form, `b > |a|`), Phase 37 (cos `b < −|a|` — PRs #3683/#3685/#3689), Phase 38 (non-bare linear arguments — PRs #3690/#3691/#3692). Symbolic `a, b, α, β` still need an assumption context. |
| `∫ f(x)·g(x)` where neither integrates alone | Generic tabular IBP fallback — Python (#4569 `symbolic-vm` 0.73.0), TS + Rust (this PR `symbolic-vm` 0.16.0).  Bounded by 5 factors / poly degree 8. | ✅ Track E1 + E2 |

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
| **Symbolic infinite sums** | Low | `sum` handles Faulhaber, geometric, classic-series, and Phase 39 structural telescoping (finite range) across Python, TypeScript, and Rust (`cas-summation` 0.2.0 — PRs #3706 ✅, #3720 ✅, #3724 ✅). Phase 40 adds Apart + telescope composition for `1/(k(k+1))` style sums in Python (`symbolic-vm` 0.64.0 — PR #3872 ✅). Phase 41 adds limit-aware infinite telescopes (`cas-summation` 0.3.0 + `symbolic-vm` 0.65.0 — PR #3880 ✅) so `∑_{k=1}^∞ 1/(k(k+1)) = 1` closes end-to-end. Phase 42 widens the vanishing-at-infinity check to any proper rational `deg(P) < deg(Q)` (`cas-summation` 0.4.0 + `symbolic-vm` 0.66.0 — PR #3887 ✅) so `∑_{k=1}^∞ [k/(k²+1) − (k+1)/((k+1)²+1)] = 1/2` and similar close in one dispatch. TS / Rust ports blocked on porting `Apart`. Hypergeometric series and transcendental limits (e.g. `sin(k)/k²`, `1/exp(k)`) still open. |

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
