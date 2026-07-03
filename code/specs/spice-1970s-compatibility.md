# SPICE 1970s Compatibility Plan

## Overview

This plan defines the remaining work to make the local SPICE engines feel like
a credible Berkeley SPICE1/SPICE2-era simulator rather than a modern collection
of isolated analysis helpers.

The target is not HSPICE, ngspice, BSIM-era process accuracy, or full SPICE3
compatibility. The target is the practical 1970s core:

- nonlinear DC operating point and sweeps
- nonlinear transient analysis
- linear AC small-signal analysis
- classic passive, source, controlled-source, and semiconductor devices
- SPICE-style model cards, netlist cards, options, and text outputs
- the small-signal analyses and convergence aids that made SPICE2 useful on
  real transistor circuits

Historical references:

- Laurence W. Nagel, *SPICE2: A Computer Program to Simulate Semiconductor
  Circuits*, UCB/ERL M520, 1975.
- Ellis Cohen, *Program Reference for SPICE2*, UCB/ERL M592, 1976.
- SPICE Version 2G User's Guide, 1981, used only as a post-1970s compatibility
  cross-check because it documents the mature SPICE2 family surface.

## Current Baseline

The repo already has the essential solver spine:

- MNA-based DC operating point with Newton-Raphson.
- DC source sweep, transfer function, sensitivity, Monte Carlo, noise
  including MOSFET channel thermal noise, AC sweep, transient analysis, and
  recent PSS footholds.
- R, C, L, independent V/I sources, all four dependent sources, diode, BJT,
  MOSFET, behavioral sources, subcircuits, and sparse real solve support.
- Python, TypeScript, and Rust `spice-engine` packages.
- Python, TypeScript, and Rust `spice-netlist-parser` packages with main cards
  for `.op`, `.tran`, `.dc`, `.ac`, `.tf`, `.sens`, `.mc`, `.noise`,
  `.options`, `.model`, and `.subckt`.

The gaps below are the remaining compatibility work for a recognizable
1970s-SPICE profile.

## Compatibility Goal

A representative SPICE2-era deck should be able to:

1. Parse with familiar element and analysis cards.
2. Build equivalent circuit objects in Python, TypeScript, and Rust.
3. Run the relevant analysis with comparable result fields.
4. Exercise the same behavior through cross-language tests.
5. Document unsupported cards with explicit parser errors rather than silent
   misinterpretation.

## Work Queue

### Phase 1 - JFET Device and Model Cards

Add `JFET` as the missing SPICE2-era semiconductor family.

Scope:

- Python, TypeScript, and Rust `Jfet`/`JFET` element types.
- N-channel and P-channel Shichman-Hodges model foothold.
- DC operating-point stamping.
- AC small-signal stamping from the DC bias point.
- Transient participation through the existing nonlinear solve path where
  applicable.
- `.model <name> NJF(...)` / `.model <name> PJF(...)` parser support.
- `J` device-card parser support.

Acceptance:

- Cross-language DC tests for an N-channel source resistor bias circuit.
- Cross-language AC tests for a small-signal common-source JFET stage.
- Parser tests for model cards, device cards, and subcircuit terminal remapping.

Status:

- JFET element data structures, `.model <name> NJF(...)` /
  `.model <name> PJF(...)` cards, and `J` device cards are implemented across
  Python, TypeScript, and Rust.
- JFET nonlinear DC operating-point stamping and small-signal AC stamping are
  implemented across Python, TypeScript, and Rust.
- JFET transient participation through the shared nonlinear transient solve path
  is covered across Python, TypeScript, and Rust with source-follower output
  capacitor fixtures.
- Phase 1 is complete for the current compatibility target.

### Phase 2 - Mutual Inductors (`K` Cards)

Add classic coupled inductors.

Scope:

- Mutual-coupling element that references two named inductors and a coupling
  coefficient.
- DC behavior remains compatible with inductor shorts.
- AC and transient stamps include the mutual terms.
- Parser support for `Kname L1 L2 coefficient`.

Acceptance:

- AC transformer ratio fixture with two coupled inductors.
- Transient coupled-inductor smoke test.
- Parser rejection for missing referenced inductors and non-finite coupling.

Status:

- `MutualInductor` / `mutualInductor` / `MutualInductor` element surfaces are
  implemented across Python, TypeScript, and Rust.
- `Kname L1 L2 coefficient` parser cards are implemented across Python,
  TypeScript, and Rust, including subcircuit reference remapping and rejection
  for missing referenced inductors or non-finite coupling.
- AC transformer-ratio fixtures and transient coupled-inductor smoke fixtures
  cover the mutual terms across Python, TypeScript, and Rust.
- Phase 2 is complete for the current compatibility target.

### Phase 3 - Ideal Transmission Lines (`T` Cards)

Add ideal lossless transmission-line support as the classic distributed element.

Scope:

- Four-terminal transmission-line element with characteristic impedance and
  delay.
- Parser support for a conservative `Tname n1 n2 n3 n4 Z0=<z> TD=<delay>`
  form first.
- Transient delay-line behavior.
- AC phase-shift behavior.

Acceptance:

- Transient delayed step fixture.
- AC phase-delay fixture.
- Parser tests for supported and unsupported line parameter forms.

Status:

- `TransmissionLine` / `transmissionLine` / `TransmissionLine` element
  surfaces are implemented across Python, TypeScript, and Rust.
- Conservative `Tname n1 n2 n3 n4 Z0=<z> TD=<delay>` parser cards are
  implemented across Python, TypeScript, and Rust, including subcircuit node
  remapping and rejection for unsupported positional and non-positive parameter
  forms.
- AC phase-delay fixtures and transient delayed-step fixtures cover the
  lossless line behavior across Python, TypeScript, and Rust.
- Phase 3 is complete for the current compatibility target.

### Phase 4 - Gear-2 / BDF2 Transient Integration

Complete the integration-method set expected by the spec.

Scope:

- `method="gear2"` / equivalent enum support across languages.
- Capacitor and inductor BDF2 companion histories.
- Adaptive-step interaction at the same surface as existing trap/euler.
- Netlist `.options method=gear2` or `.tran ... method=gear2` routing.

Acceptance:

- Cross-language RC and RL fixtures showing Gear-2 produces stable results.
- LC damped fixture showing Gear-2 suppresses trapezoidal ringing better than
  trap at the same coarse step.

Status:

- Fixed-step Gear-2 / BDF2 capacitor and inductor companions are implemented
  across Python, TypeScript, and Rust, with one backward-Euler bootstrap step
  before two-step history is available. TypeScript and Rust now also expose
  trapezoidal transient companions for parity with Python, and all three
  packages include a coarse LC fixture showing Gear-2 damps ringing more than
  trapezoidal integration. The parser packages now validate
  `.tran ... method=<euler|trap|gear2>`, preserve the method on transient
  analysis cards, and expose `.options method=<...>` as the fallback route.
  TypeScript and Rust now expose adaptive transient entry points with bounded
  LTE-based step growth/shrinkage and method routing for Euler, trapezoidal,
  and Gear-2. Phase 4 is complete for the current compatibility target.

### Phase 5 - Pseudo-Transient DC Continuation

Finish the convergence-aid chain named in `spice-engine.md`.

Scope:

- If regular DC, Gmin stepping, and source stepping fail, run a short transient
  continuation toward a DC solution.
- Preserve existing `DcResult` metadata so callers can see which aid converged.
- Options to cap pseudo-transient steps and tolerances.

Acceptance:

- Nonlinear diode/BJT/MOS bias fixtures that fail with aids disabled and
  converge through the full aid chain.
- Tests proving pseudo-transient does not run when earlier aids succeed.

Status:

- DC operating-point results now report which convergence path produced the
  result (`newton`, `gmin`, `source`, `pseudo_transient`, or `none`) across
  Python, TypeScript, and Rust.
- Python, TypeScript, and Rust now include a bounded pseudo-transient DC
  continuation fallback after Newton, Gmin stepping, and source stepping. The
  aid uses artificial backward-Euler node companions, has step/conductance and
  per-step Newton caps, and reports `pseudo_transient` when it converges.

### Phase 6 - 1970s Model-Card Depth

Deepen semiconductor models enough for SPICE2-style examples.

Scope:

- Diode junction capacitance, transit time, emission coefficient, breakdown
  footholds, and temperature scaling.
- BJT charge-storage and capacitance footholds beyond current Ebers-Moll-like
  DC behavior.
- MOS Level-1 parameter coverage audit and missing parser/model aliases.
- Temperature directive plumbing into analyses.

Acceptance:

- Parser/model-card tests for common SPICE2 parameters.
- DC/AC/transient fixtures that prove capacitance and temperature parameters
  change results in expected directions.

Status:

- Diode `.model ... D(... N=<emission coefficient>)` parsing and engine
  plumbing are implemented across Python, TypeScript, and Rust. DC and
  small-signal AC diode conductance now use `N * Vt`, with subcircuit remapping
  preserving the parameter.
- Diode `.model ... D(... BV=<voltage> IBV=<current>)` parsing and reverse
  breakdown current/conductance footholds are implemented across Python,
  TypeScript, and Rust.
- Diode `.model ... D(... CJO=<capacitance>)` / `CJ0` parsing and AC
  junction-capacitance admittance are implemented across Python, TypeScript,
  and Rust.
- Diode `.model ... D(... TT=<time>)` parsing and forward-bias diffusion
  capacitance are implemented in AC analysis across Python, TypeScript, and
  Rust.
- Diode operating-temperature helpers are implemented across Python,
  TypeScript, and Rust. They scale thermal voltage and saturation current with
  absolute temperature before an analysis, with fixed-current fixtures proving
  hotter silicon lowers forward voltage.
- BJT operating-temperature helpers are implemented across Python, TypeScript,
  and Rust. They scale thermal voltage and saturation current with absolute
  temperature before an analysis, with fixed-base emitter-follower fixtures
  proving hotter silicon lowers forward drop.
- BJT `.model ... NPN|PNP(... CJE=<capacitance> CJC=<capacitance>)` parsing
  and base-emitter/base-collector AC capacitance stamping are implemented
  across Python, TypeScript, and Rust.
- BJT `.model ... NPN|PNP(... TF=<time>)` parsing and forward-bias diffusion
  capacitance are implemented in AC analysis across Python, TypeScript, and
  Rust.
- BJT `.model ... NPN|PNP(... TR=<time>)` parsing and reverse/base-collector
  diffusion capacitance are implemented in AC analysis across Python,
  TypeScript, and Rust.
- MOS Level-1 `.model ... NMOS|PMOS(... CGSO=<capacitance> CGDO=<capacitance>
  CGBO=<capacitance> CBS=<capacitance> CBD=<capacitance>)` parsing and
  small-signal AC capacitance stamping are implemented across Python,
  TypeScript, and Rust.
- MOS Level-1 operating-temperature helpers are implemented across Python,
  TypeScript, and Rust. They shift threshold voltage, scale the
  transconductance parameter, and update nominal model temperature before an
  analysis, with common-source fixtures proving hotter silicon pulls the drain
  lower.

### Phase 7 - Classic Text Output and Control Cards

Make deck-level output feel like SPICE rather than only package APIs.

Scope:

- `.print` and `.plot` parse records.
- `.four` Fourier analysis over transient output.
- `.temp` directive.
- More `.options` keys wired into engine calls.
- Text output helpers for tabular node voltages and branch currents.

Acceptance:

- Parser fixtures for `.print`, `.plot`, `.four`, `.temp`, and selected
  `.options`.
- Fourier fixture for a sinusoidal transient output.
- Text-output snapshot tests with stable ordering.

Status:

- `.temp <celsius> [celsius ...]` parser/control-card records are implemented
  across Python, TypeScript, and Rust.
- `.print <analysis> <V(node)|I(source)>...` and
  `.plot <analysis> <V(node)|I(source)>...` parser/control-card records are
  implemented across Python, TypeScript, and Rust.
- `.four <frequency> <V(node)|I(source)>...` parser/control-card records are
  implemented across Python, TypeScript, and Rust.
- Fourier post-processing over transient output is implemented across Python,
  TypeScript, and Rust, including DC, harmonic sine/cosine coefficients,
  magnitudes, phases, and THD for `V(node)` and `I(source)` probes.
- Stable text-output helpers for DC operating points and transient samples are
  implemented across Python, TypeScript, and Rust, with snapshot tests covering
  node-voltage and branch-current ordering.
- Named-corner transient samples can now be evaluated and rendered as stable
  tab-separated text tables in the live Rust SPICE package, preserving corner
  order and covering selected voltage/current probes.
- Adaptive transient samples can now be rendered as stable tab-separated text
  tables in the live Rust SPICE package, preserving integration method,
  rejected-step count, convergence state, sample time, and selected
  voltage/current probes for direct and named-corner runs.
- Named-corner DC operating-point results can now be rendered as stable
  tab-separated text tables in the live Rust SPICE package, preserving corner
  order and covering selected voltage/current probes.
- `.DC` source-sweep results can now be rendered as stable tab-separated text
  tables in the live Rust SPICE package for direct and named-corner runs,
  covering source values and selected voltage/current probes.
- `.AC` phasor results can now be rendered as stable tab-separated text tables
  across Python, TypeScript, and Rust, covering real, imaginary, magnitude,
  and phase rows for selected voltage/current probes.
- Named-corner `.AC` phasor results can now be rendered as stable
  tab-separated text tables in the live Rust SPICE package, preserving corner
  order and covering real, imaginary, magnitude, and phase rows for selected
  voltage/current probes.
- `.FOUR` Fourier results can now be rendered as stable tab-separated text
  tables across Python, TypeScript, and Rust, covering harmonic coefficients,
  magnitude, phase, DC, and THD rows.
- Named-corner `.FOUR` Fourier results can now be evaluated and rendered as
  stable tab-separated text tables in the live Rust SPICE package, preserving
  corner order and covering harmonic coefficients, magnitude, phase, DC, and
  THD rows.
- `.TF` transfer-function results can now be rendered as stable tab-separated
  text tables across Python, TypeScript, and Rust, covering gain and
  input/output impedance rows.
- Named-corner `.TF` transfer-function results can now be rendered as stable
  tab-separated text tables in the live Rust SPICE package, preserving corner
  order and covering gain and input/output impedance rows.
- Two-port S-parameter results can now be rendered as stable tab-separated
  text tables in the live Rust SPICE package, covering real, imaginary,
  magnitude, and phase rows for S11/S21/S12/S22 entries.
- `.NOISE` results can now be rendered as stable tab-separated text tables in
  the live Rust SPICE package, covering total output noise, input-referred
  noise, and per-source PSD contribution rows.
- `.SENS` results can now be rendered as stable tab-separated text tables in
  the live Rust SPICE package, covering nominal values, absolute
  sensitivities, and relative sensitivities.
- Direct PSS results can now be rendered as stable tab-separated text tables
  in the live Rust SPICE package, covering period, time step, convergence,
  Newton iteration count, final residual norm, steady-state time, and selected
  voltage/current probes.
- Monte Carlo DC analysis can now be evaluated across named corners in the
  live Rust SPICE package, reusing the existing corner override surface with a
  shared seeded tolerance trial configuration.
- `.MC` Monte Carlo DC results can now be rendered as stable tab-separated
  text tables in the live Rust SPICE package for direct and named-corner runs,
  covering output-node trial values, summary statistics, and convergence.
- DC sensitivity analysis can now be evaluated across named corners in the
  live Rust SPICE package, reusing the existing corner override surface for
  the same output-node query.
- Named-corner `.SENS` results can now be rendered as stable tab-separated
  text tables in the live Rust SPICE package, preserving corner order and
  covering nominal values, absolute sensitivities, and relative sensitivities.
- AC noise analysis can now be evaluated across named corners in the live Rust
  SPICE package, reusing the existing corner override surface for the same
  output/input query and frequency grid.
- Named-corner `.NOISE` results can now be rendered as stable tab-separated
  text tables in the live Rust SPICE package, preserving corner order and
  covering total output noise, input-referred noise, and per-source PSD
  contribution rows.
- Periodic steady-state analysis can now be evaluated across named corners in
  the live Rust SPICE package, reusing the existing corner override surface
  for the same shooting-Newton solve options.
- Named-corner PSS results can now be rendered as stable tab-separated text
  tables in the live Rust SPICE package, preserving corner order and covering
  period, time step, convergence, Newton iteration count, final residual norm,
  steady-state time, and selected voltage/current probes.
- Constrained pole-zero analysis can now be evaluated across named corners in
  the live Rust SPICE package, reusing the existing corner override surface
  for the selected Phase 8 topology.
- Named-corner `.PZ` results can now be rendered as stable tab-separated text
  tables in the live Rust SPICE package, preserving corner order and covering
  pole/zero kind, real and imaginary parts, frequency, and damping.
- Transient-to-distortion projection can now be evaluated across named corners
  in the live Rust SPICE package, reusing the existing corner override surface
  for the same transient sampling and `.DISTO` output-probe query.
- Named-corner `.DISTO` results can now be rendered as stable tab-separated
  text tables in the live Rust SPICE package, preserving corner order and
  covering harmonic magnitude, phase, and THD rows.
- Two-port S-parameter extraction can now be evaluated across named corners in
  the live Rust SPICE package, reusing the existing corner override surface
  for the same port pair, frequency grid, and reference impedance.
- Named-corner S-parameter results can now be rendered as stable tab-separated
  text tables in the live Rust SPICE package, preserving corner order and
  covering real, imaginary, magnitude, and phase rows for S11/S21/S12/S22
  entries.
- Mixed-signal bridge breakpoint schedules can now be derived and rendered as
  stable tab-separated text tables in the live Rust SPICE package, covering
  ordered digital event starts, finite-edge transition endpoints, and the
  resulting stop time for scheduler-facing SPICE bridge snapshots.
- DC operating-point named-corner sweeps can now be evaluated with an
  order-preserving parallel helper in the live Rust SPICE package, giving the
  multi-corner orchestration roadmap its first parallel execution foothold.
- `.DC` source-sweep named-corner runs can now be evaluated with an
  order-preserving parallel helper in the live Rust SPICE package, preserving
  source value traces and selected-probe table compatibility.
- `.AC` frequency-sweep named-corner runs can now be evaluated with an
  order-preserving parallel helper in the live Rust SPICE package, preserving
  frequency traces and selected complex-probe table compatibility.
- `.TF` transfer-function named-corner runs can now be evaluated with an
  order-preserving parallel helper in the live Rust SPICE package, preserving
  transfer ratio, input impedance, and output impedance table compatibility.
- Monte Carlo DC named-corner runs can now be evaluated with an
  order-preserving parallel helper in the live Rust SPICE package, preserving
  per-corner seeded trial rows, summary statistics, and convergence metadata.
- DC sensitivity named-corner runs can now be evaluated with an
  order-preserving parallel helper in the live Rust SPICE package, preserving
  per-corner nominal values, sensitivity entries, and relative sensitivity
  ordering.
- AC noise named-corner runs can now be evaluated with an order-preserving
  parallel helper in the live Rust SPICE package, preserving total output PSD,
  input-referred PSD, and per-source contribution ordering.
- S-parameter named-corner runs can now be evaluated with an order-preserving
  parallel helper in the live Rust SPICE package, preserving two-port
  frequency rows and S11/S21/S12/S22 ordering.
- Selected `.options` keys are wired into engine-call helpers across Python,
  TypeScript, and Rust, covering DC solver tolerances/iteration limits and
  transient method/adaptive-step options.
- Deck-level `.temp` cards are resolved into Kelvin helper temperatures across
  Python, TypeScript, and Rust, and explicit `.noise temp=<kelvin>` values take
  precedence for noise-engine calls.
- DC operating points can now be evaluated and rendered across explicit
  `.temp`-style analysis temperatures in the live Rust SPICE package, covering
  temperature, node-voltage, and branch-current rows with stable ordering.
- Named-corner DC operating points can now be evaluated and rendered across
  explicit `.temp`-style analysis temperatures in the live Rust SPICE package,
  preserving corner order and selected voltage/current probes.
- `.disto` and `.pz` parser/control-card records plus first distortion and
  pole-zero result shapes are implemented across Python, TypeScript, and Rust,
  with smoke fixtures for nonlinear-device distortion output and a simple RC
  pole result.
- A constrained Phase 8 executable foothold is implemented across Python,
  TypeScript, and Rust: a simple grounded RC low-pass pole-zero helper and a
  Fourier-to-distortion projection helper.
- Distortion analysis can now be projected directly from transient samples
  across Python, TypeScript, and Rust by reusing the Fourier extraction path
  for a selected output probe.
- The constrained Phase 8 pole-zero foothold now covers both simple grounded RC
  low-pass and high-pass fixtures, with the high-pass helper returning the
  origin zero plus the shared RC pole.
- The constrained Phase 8 pole-zero foothold now includes a second-order
  series R-L / shunt-C low-pass helper across Python, TypeScript, and Rust,
  including underdamped complex-conjugate pole fixtures.
- The second-order Phase 8 pole-zero foothold now also covers a series R-C /
  shunt-L high-pass helper across Python, TypeScript, and Rust, returning the
  double origin zero plus the shared RLC pole pair.
- The second-order Phase 8 pole-zero foothold now also covers a series L-C /
  shunt-R band-pass helper across Python, TypeScript, and Rust, returning one
  origin zero plus the shared RLC pole pair.
- The second-order Phase 8 pole-zero foothold now also covers a series-R /
  shunt-series-L-C notch helper across Python, TypeScript, and Rust, returning
  the imaginary-axis zero pair plus the shared RLC pole pair.
- `.PZ` pole-zero results can now be rendered as stable tab-separated text
  tables across Python, TypeScript, and Rust.
- `.DISTO` distortion results can now be rendered as stable tab-separated text
  tables across Python, TypeScript, and Rust, covering harmonic magnitude,
  phase, and THD rows.

### Phase 8 - Small-Signal Distortion and Pole-Zero

Complete the lower-frequency SPICE2 analysis surface after the core device and
parser work is in place.

Scope:

- Distortion-card parser and a first small-signal distortion result shape.
- Pole-zero parser and analysis result shape.
- Start with constrained linearized examples; leave rich nonlinear distortion
  accuracy as follow-up if needed.

Acceptance:

- Parser and result-shape tests across languages.
- At least one simple RC pole fixture and one nonlinear-device distortion smoke
  fixture.

## Loop Policy

Each implementation phase should be handled as a focused PR:

1. Start from fresh `origin/main` in a sibling worktree.
2. Keep Python, TypeScript, and Rust behavior comparable unless a phase is
   explicitly parser-only or engine-only.
3. Update this plan or the SPICE pending-work spec as each phase lands.
4. Run focused validation for touched packages.
5. Push a PR and monitor it with `gh pr view` and `gh pr checks --watch=false`.
6. If CI fails or a merge conflict appears, fix it immediately, commit, push,
   and restart the monitor.
7. Once merged, delete the old monitor and move to the next phase.

## Current Wrap-up Status

As of 2026-06-05, Phases 1-8 are complete across the Python, TypeScript, and
Rust packages for the current 1970s compatibility target. New SPICE work should
be tracked as post-compatibility expansion instead of reopening the original
phase queue.

Remaining expansion belongs in the broader SPICE pending-work inventory:

1. Broader parallel corner orchestration beyond the current Rust helpers.
2. Full `hardware-vm` scheduler integration for mixed-signal simulation.
3. Verilog-A/custom compact-model support.
4. Production sparse/KLU and SPICE3-era raw/control/BSIM surfaces.
5. Richer nonlinear distortion accuracy beyond the constrained Phase 8
   executable footholds.
