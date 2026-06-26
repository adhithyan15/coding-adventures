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
distortion harmonics, parsed `.save` / `.probe` / `.print` / `.plot`
deck-selected output tables, and backward-Euler reactive-element companion
models.

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
`measureTransientProbe`, `measureDcSweepProbe`, `measureAcSweepProbe`, and
`formatMeasurementTable`
provide the shared `.MEASURE`-style scalar output surface for MAX, MIN, AVG,
RMS, peak-to-peak, and final-value probe measurements. The AC helper measures
complex probe magnitudes over optional frequency windows.
`measureTransientFindAtProbe` and parsed transient `FIND ... AT=` cards sample
or linearly interpolate one probe value at a scalar time, while
`measureTransientWhenProbe`, `measureTransientWhenProbeCounted`, and parsed
transient `WHEN probe=target` cards report first or counted `RISE`, `FALL`, and
`CROSS` crossing times over optional `FROM=` / `TO=` windows.
`measureTransientDelayBetweenProbes` and parsed transient `TRIG ... TARG ...`
cards report trigger-to-target crossing delays. The deck helpers route parsed
transient, DC sweep, and AC sweep `.measure` / `.meas` cards into those stable
measurement rows.
`resolveDeckAnalyses` extracts `.op`, `.dc`, `.ac`, and `.tran` cards before
`.end`, keeps non-analysis active lines, and reports stable diagnostics for
malformed arguments, unsupported AC sweep modes, invalid sweep intervals, and
unresolved scalar expressions. `selectDeckAnalysisPlan` picks one explicit card
by analysis alias, defaults decks without analysis cards to an implicit `.op`,
and reports ambiguity before solver dispatch.
`runDeckAnalysis` executes one selected `.op`, `.dc`, `.ac LIN`, `.ac DEC`,
`.ac OCT`, or `.tran` plan against an existing `Circuit` and returns the plan,
solver result, deck-selected output table, and normalized analysis directive,
table count/name list, output probes, and output directives that produced the
table, plus selected `.measure` results and
a stable measurement table for `.dc`, `.ac`, and `.tran` executions.
Execution `tableArtifacts` preserve the same order as `tables` and carry each
stable table's text, CSV, compact JSON, and header-keyed records. Selected
`.tran` plans route
`START` output filtering, `.tran TSTEP` as the output print grid, `MAXSTEP` as
an internal fixed-step cap, and `UIC` initial-condition intent through that
stable transient table surface. They also return selected `.four` harmonic
results and a stable Fourier table. Executions also include selected-run
artifact summaries plus `formatDeckRunArtifactTable` and
`formatDeckRunArtifactCsv` / `formatDeckRunArtifactJson` output for stable
result-row, table, analysis-directive, output-probe, output-directive,
measurement, Fourier, control-command, write-marker, rawfile-option, and
diagnostic count/name lists.
Normalized accepted `.control` commands are surfaced separately in
`controlLineCount` / `controlLines` execution fields and in
`ControlLines` / `ControlLineList` artifact fields.
Accepted `.control` `write` / `wrdata` rawfile/data-write markers are surfaced
as `writeMarkerCount` / `writeMarkers` execution fields and as `WriteMarkers` /
`WriteMarkerList` artifact fields without serializing files.
Accepted `write <rawfile> ...` markers also produce deterministic in-memory
ASCII rawfile artifacts on `rawfileArtifacts`, with stable probe-aware table,
CSV, JSON, and header-keyed record summaries. Accepted `wrdata <file> ...` markers
produce deterministic in-memory ASCII data-file artifacts on
`wrdataArtifacts`, with stable table, CSV, JSON, and header-keyed record
summaries; filesystem writes remain metadata-only.
Accepted `.control` rawfile output options (`set filetype=ascii`,
`set wr_vecnames`, `set wr_singlescale`, and `set appendwrite`) are surfaced as
`rawfileOptionCount` / `rawfileOptions` execution fields and as
`RawfileOptions` / `RawfileOptionList` artifact fields. WRDATA artifacts also
carry the same option inventories and render `wr_vecnames` / `wr_singlescale`
intent as stable `VectorNames` / `Scale` metadata in the in-memory data file.
When a `write` or `wrdata` marker names probes, its in-memory artifact keeps
the scale column plus only the requested matching probe columns, while artifact
summaries keep matched and unmatched probe inventories in stable table, CSV,
JSON, and record exports.
Existing
`.control` body policy diagnostics flow into those selected-run artifact `Diagnostics` /
`DiagnosticCodeList` fields and through the same run-artifact table, CSV, JSON,
and `tableArtifacts` records. Policy-blocked `source` / `shell`, `cd`,
control-flow, and variable/state commands also populate
`controlPolicyArtifacts` with stable line, category, command, code, severity,
and message fields plus table, CSV, compact JSON, and header-keyed record
exports. The selected-run artifact row also carries
`ControlPolicyArtifacts`, `ControlPolicyCategoryList`,
`ControlPolicyCodeList`, and `ControlPolicySeverityList` inventory columns.
`controlPolicySummaryArtifacts` groups the same policy artifacts by
category with stable count, line-list, command-list, code-list, and severity-list
table, CSV, compact JSON, and record exports. The same policy row and summary
tables also appear as `control-policy` and `control-policy-summary` entries in
`tables`, selected-run `TableList` metadata, and ordered `tableArtifacts`.
`formatDeckTableCsv` also converts any stable tab-separated deck table to CSV,
`formatDeckTableJson` converts the same tables to compact JSON records, and
`deckTableRecords` returns header-keyed
native records for browser and host integrations.
`resolveDeckOutputs` and `selectDeckOutputProbes` extract `.save`, scoped or
global `.probe`, scoped `.print <analysis> ...`, and scoped
`.plot <analysis> ...` cards before `.end`, normalize and deduplicate output
probes in deck order, and feed
`formatDeckOpTable`,
`formatDeckDcSweepTable`, `formatDeckAcTable`, and
`formatDeckTransientTable` for stable deck-selected text output.
`resolveDeckFourier`, `fourierTransientCards`, `fourierTransientDeck`, and
`formatDeckFourierTable` extract parsed `.four` / `.FOUR` cards before `.end`
and route transient samples into the existing SPICE-style Fourier result shape
with optional `HARMONICS=` and `FROM=` controls.

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
`.include`, `.lib`, and `.control` directives. Inside `.control` blocks,
selected analysis/output commands (`op`, `dc`, `ac`, `tran`, `save`, `probe`,
`measure`, `meas`, `four`, `fourier`, `print`, and `plot`) are normalized into
dotted deck cards, while `run`, `reset`, `quit`, and the UI-only
`set noaskquit` option plus the ASCII rawfile-format `set filetype=ascii`
option, vector-name/single-scale rawfile output toggles (`set wr_vecnames`,
`set wr_singlescale`), and the append-write rawfile option (`set appendwrite`)
plus target-bearing rawfile-write markers (`write <rawfile> [probes...]`) and
ASCII data-write markers (`wrdata <file> <probes...>`) are accepted as no-op
control markers. Read-only control inspection commands (`display`, `listing`,
`show`, `showmod`, `status`, `version`, `help`, `echo`, `rusage`, and `where`)
are also accepted as no-op markers. External script and shell commands
(`source` and `shell`) emit explicit policy diagnostics and are not executed.
Working-directory mutation commands (`cd`) also emit explicit policy
diagnostics and are not executed. Control-flow commands (`if`, `while`,
`foreach`, and `repeat`) emit explicit policy diagnostics as well.
Variable/state mutation commands (`let`, `alter`, `alterparam`, `set`, and
`unset`) emit explicit policy diagnostics unless they are one of the accepted
no-op `set` options. Other unrecognized non-comment commands emit diagnostics
until a broader executed control subset is in scope.
`resolveDeckSources` is the first include/library resolution layer: callers
provide a source-content map, `.include` directives are expanded in place, and
`.lib path section` selects a named `.lib` / `.endl` section with stable
diagnostics for missing files, missing sections, unterminated sections, cycles,
and still-unsupported `.control` block commands that are not part of the
selected analysis/output subset.
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
`resolveDeckMeasurements` extracts transient, DC sweep, and AC sweep `.measure` / `.meas`
cards before `.end`, keeps non-measure active lines, evaluates optional
`FROM=` / `TO=` scalar time, source-value, or frequency windows plus transient
`FIND ... AT=` sample points and `WHEN probe=target` crossings with optional
`RISE`, `FALL`, or `CROSS` counters, and reports stable diagnostics for
unsupported analyses, modes, options, expressions, and invalid windows.
`resolveDeckOutputs` extracts `.save`, `.probe`, `.print`, and `.plot` cards
before `.end`, preserves non-output active lines, and reports stable
diagnostics for missing probe lists, unsupported scoped output analyses, or
malformed `V(node)` / `I(source)` probes.
`resolveDeckAnalyses` extracts `.op`, `.dc`, `.ac`, and `.tran` cards before
`.end`, preserves non-analysis active lines, and reports stable diagnostics for
malformed deck-level analysis controls.
`selectDeckAnalysisPlan` returns one selected or implicit plan for downstream
deck execution helpers.
`runDeckAnalysis` routes that selected plan into the matching solver and stable
deck-selected table output with normalized table-inventory, output-probe, and
output-directive artifacts, selected measurement artifacts, selected transient Fourier artifacts,
selected-run artifact summaries with table, analysis-directive, output-probe, output-directive,
measurement, Fourier probe, `.control` command, write-marker, rawfile-option, and
control-policy diagnostic inventories, `.ac LIN`, `.ac DEC`, `.ac OCT` frequency grids, and
`.tran` `START` / print-step `TSTEP` / `MAXSTEP` / `UIC` controls. Selected
execution fields expose `.control` command, write-marker, rawfile-option, and
diagnostic inventories directly for host integrations.
