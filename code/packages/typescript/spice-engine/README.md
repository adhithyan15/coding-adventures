# @coding-adventures/spice-engine

`@coding-adventures/spice-engine` provides SPICE-style circuit analysis
primitives for TypeScript.

The current slices implement DC operating-point analysis, DC source sweeps, DC
sensitivity analysis, seeded DC Monte Carlo analysis, DC small-signal
transfer-function analysis, AC small-signal frequency sweeps, and fixed-step
AC noise analysis, and fixed/adaptive RC/RL transient analysis for linear circuits using
modified nodal analysis (MNA). The package supports
resistors, capacitors, inductors, diodes, BJTs, Level-1 MOSFETs, independent current sources,
independent voltage sources, voltage-controlled current sources (VCCS),
PWL/SIN/PULSE/EXP
source waveforms for transient analysis, ground aliases, node voltages,
voltage source branch currents, Fourier post-processing for transient output,
transient-to-distortion projection,
constrained RC and RLC low-pass/high-pass/band-pass/notch pole-zero helpers,
mixed-signal digital event stream helpers that bridge finite-edge PWL sources
to thresholded transient probe outputs with stable schedule tables and VCD
correlation output,
stable text tables for selected node voltages, branch currents, AC phasors,
Fourier harmonics, transfer-function results, pole-zero entries, and
distortion harmonics,
and backward-Euler reactive-element companion models.

```ts
import {
  Circuit,
  PwlWaveform,
  resistor,
  transient,
  transientAdaptive,
  voltageSourceWithWaveform,
} from "@coding-adventures/spice-engine";

const circuit = new Circuit();
circuit.add(
  voltageSourceWithWaveform(
    "Vin",
    "in",
    "0",
    0.0,
    new PwlWaveform([
      [0.0, 0.0],
      [1.0e-9, 1.8],
    ]),
  ),
);
circuit.add(resistor("Rload", "in", "0", 1_000.0));

const points = transient(circuit, 0.5e-9, 1.0e-9);
const adaptive = transientAdaptive(circuit, 0.5e-9, 1.0e-9, { method: "gear2" });
```

`dcOp(circuit).diagnostics` reports stable solve metadata, including the MNA
matrix size, selected real solver path, tolerance, convergence aid, and final
Newton delta. Large real DC and complex AC matrix solves use sparse-row solver
paths when the matrix size reaches the package threshold.

`diodeAtTemperature`, `bjtAtTemperature`, `mosfetAtTemperature`, and
`circuitAtTemperature` provide operating-temperature footholds for diode, BJT,
and Level-1 MOSFET models before running an analysis.
`dcTemperatureSweep` and `dcTemperatureSweepCorners` run `.temp`-style DC
operating-point snapshots across explicit analysis temperatures, with stable
nominal and named-corner table helpers for cross-language comparison.
`formatCornerDcTable` also renders named-corner DC operating-point snapshots
with the Rust-matching `Corner` / `Index` columns.
`formatDcSweepTable`, `formatCornerDcSweepTable`, `formatCornerAcTable`, and
`formatCornerTfTable` provide the matching stable `.DC`, `.AC`, and `.TF`
sweep/corner text surfaces.

`normalizeModelCard`, `diodeFromModelCard`, `bjtFromModelCard`,
`jfetFromModelCard`, and `mosfetFromModelCard` provide the shared `.model`
alias surface for diode, BJT, JFET, and Level-1 MOS cards.
`deviceModelAuditFixtures` returns the canonical cross-language fixture cards
used to keep the TypeScript, Python, and Rust ports aligned.

`CustomModel`, `CustomModelEvaluation`, `customLinearConductanceModel`, and
`analyzeCustomModelSource` provide the first native-web custom-model foothold.
The accepted source subset is diagnostic-only and limited to a two-terminal
`I(p,n) <+ ...` module shape; it does not compile or evaluate source strings in
the browser sandbox.

`compatibilityCorpus` exposes the first release-readiness deck corpus for
`.op`, `.dc`, `.ac`, `.tran`, and `.tf` coverage. Each fixture carries a
documented oracle, golden values with tolerances, and known incompatibility
notes. `releaseReadinessGates` validates the corpus metadata, while
`formatCompatibilityCorpusTable` and `formatReleaseReadinessReport` provide
stable tab-separated summaries for package checks.
`analyzeDeckControls` provides the shared deck-control boundary foothold: it
returns active lines before `.end` and stable diagnostics for unsupported
`.include`, `.lib`, and `.control` directives before future include/library
resolution and control-block execution are in scope.
`resolveDeckSources` is the first include/library resolution layer: callers
provide a source-content map, `.include` directives are expanded in place, and
`.lib path section` selects a named `.lib` / `.endl` section with stable
diagnostics for missing files, missing sections, unterminated sections, cycles,
and still-unsupported `.control` blocks.
`resolveDeckParameters` evaluates scalar whitespace-tokenized `.param`
assignments, collects scalar `.func` definitions before `.end`, preserves
parameter order, rewrites braced and quoted active-line expressions, and emits
stable diagnostics for unresolved expressions, bad function arity, unknown
functions, and recursive function calls.
`resolveDeckInitialConditions` extracts scalar `.ic` and `.nodeset`
`V(node)=value` hints before `.end`, keeps non-condition active lines, evaluates
numeric SPICE suffix/arithmetic expressions, and reports stable diagnostics for
malformed targets or unresolved values. `dcInitialVectorFromConditions` maps
those parsed node-voltage hints into the DC solver's MNA warm-start vector, and
`dcOpWithInitialConditions` applies that vector to the operating-point solve
with `.ic` values taking precedence over `.nodeset` values.
`resolveDeckFunctions` extracts scalar `.func name(args) expression`
definitions before `.end`, preserves non-function active lines, strips braced
or quoted expression delimiters, and reports stable diagnostics for malformed
signatures, arguments, or empty expressions.
