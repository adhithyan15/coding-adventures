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

- Parser support and AC phase-shift behavior are on `main`; this slice adds
  transient delayed-step behavior across Python, TypeScript, and Rust.

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
- Selected `.options` keys are wired into engine-call helpers across Python,
  TypeScript, and Rust, covering DC solver tolerances/iteration limits and
  transient method/adaptive-step options.
- Deck-level `.temp` cards are resolved into Kelvin helper temperatures across
  Python, TypeScript, and Rust, and explicit `.noise temp=<kelvin>` values take
  precedence for noise-engine calls.
- `.disto` and `.pz` parser/control-card records plus first distortion and
  pole-zero result shapes are implemented across Python, TypeScript, and Rust,
  with smoke fixtures for nonlinear-device distortion output and a simple RC
  pole result.
- A constrained Phase 8 executable foothold is implemented across Python,
  TypeScript, and Rust: a simple grounded RC low-pass pole-zero helper and a
  Fourier-to-distortion projection helper.

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

## Initial Implementation Order

The next implementation slice is Phase 1, JFET.

Recommended subdivision:

1. Add cross-language JFET element data structures plus parser model/device
   support.
2. Add Python DC/AC engine behavior for JFET.
3. Port DC/AC behavior to TypeScript and Rust.
4. Add transient participation if not already covered by the shared nonlinear
   DC solve path.

This lets parser and public API compatibility land before the more delicate
nonlinear model math broadens.
