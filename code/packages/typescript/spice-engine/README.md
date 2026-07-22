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
deck-selected output tables, source-order `runDeck` whole-deck execution for
parsed `.op`, `.dc`, `.ac`, `.tran`, `.tf`, `.sens`, and `.noise` cards, and
backward-Euler reactive-element companion models.

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
matrix size, selected real solver path, tolerance, convergence aid, final
Newton delta, and a nested `solverProfile` with backend, structural nonzero
count, density, fill-in, and fallback metadata. Large real DC and complex AC
matrix solves use native sparse-row solver paths when the matrix size reaches
the package threshold.
For nonlinear operating points, `dcOp(circuit, { newtonStepLimit })` bounds
each Newton update per unknown. Diagnostics report the active limit, how many
steps were clipped, and the minimum damping factor; pass
`newtonStepLimit: null` to disable the limiter.

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
`parseBerkeleySyntax` exposes the shared Berkeley SPICE logical-card parser
contract for editors and Mosaic-style app shells: it normalizes leading `+`
continuations, removes inline semicolon comments, preserves source spans and
token streams, reports stable syntax diagnostics, embeds the checked grammar
metadata, and returns an analysis inventory without requiring solver dispatch.
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
Execution `outputPlanArtifacts` summarize the selected result columns, output
probes, selected analysis line/source/output-node and sweep/time/frequency
metadata, selected output probe source lines, output directives, normalized output
directive kinds, normalized directive analysis scopes, selected output directive
source lines, selected result row counts, and stable table names with table, CSV,
compact JSON, and header-keyed record exports.
Execution `tableArtifacts` preserve the same order as `tables` and carry each
stable table's text, CSV, compact JSON, and header-keyed records.
The `output-plan` entry in `tableArtifacts` carries the same
`outputPlanArtifactTable` data, CSV, compact JSON, and header-keyed record
exports.
Selected `.tran` plans route
`START` output filtering, `.tran TSTEP` as the output print grid, `MAXSTEP` as
an internal fixed-step cap, and `UIC` initial-condition intent through that
stable transient table surface. They also return selected `.four` harmonic
results and a stable Fourier table. Executions also include selected-run
artifact summaries plus `formatDeckRunArtifactTable` and
`formatDeckRunArtifactCsv` / `formatDeckRunArtifactJson` output for stable
result-row, table, analysis-directive, output-probe, output-directive,
measurement, Fourier, control-command, write-marker, rawfile-option, and
diagnostic count/name lists.
`runDeck` executes every parsed `.op`, `.dc`, `.ac`, `.tran`, `.tf`, `.sens`,
and `.noise` card in source order, preserves duplicate analysis directives,
and defaults analysis-less decks to an implicit `.op`. Its whole-run result
returns ordered selected executions plus aggregate selected-run artifact table,
CSV, compact JSON, and header-keyed records, and each selected-run artifact
carries the deck-wide analysis kind/directive inventories beside the selected
analysis directive metadata.
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
Diode cards accept `VJ`/`PB` junction potential, `M`/`MJ` grading coefficient,
and `FC` forward-bias depletion coefficient. AC and transient analyses use them
to shape `CJO` depletion capacitance continuously around the `FC * VJ`
transition.
Diode cards also accept `XTI` (default `3`) and `EG` (default `1.11 eV`) to
control saturation-current temperature scaling.
BJT cards accept `XTI` (default `3`) and `EG` (default `1.11` eV) for
model-specific saturation-current temperature scaling, `VAF`/`VA` (default
`0`, meaning infinite) for forward Early-effect modulation, `VAR`/`VB`
(default `0`, meaning infinite) for reverse Early-effect base-charge
modulation, and `NF` (default
`1`) for forward-junction emission shaping. `NR` (default `1`) shapes reverse
base-collector diffusion capacitance in AC and transient analysis. `VJE`/`PE`
(default `0.75 V`) and `MJE`/`ME` (default `0.33`) shape `CJE` base-emitter
depletion capacitance. `VJC`/`PC` (default `0.75 V`) and `MJC`/`MC` (default
`0.33`) likewise shape `CJC` base-collector depletion capacitance. `FC`
(default `0.5`) selects the shared Berkeley forward-bias continuation point for
both junctions.
`modelCardUnsupportedParameterIssues`,
`formatModelCardUnsupportedParameterIssueTable`,
`modelCardUnsupportedParameterIssueRecords`,
`formatModelCardUnsupportedParameterIssueCsv`, and
`formatModelCardUnsupportedParameterIssueJson` expose retained unsupported
model-card keys as stable diagnostics for parser and UI surfaces.
`modelCardSupportedParameterCoverage`,
`formatModelCardSupportedParameterCoverageTable`,
`modelCardSupportedParameterCoverageRecords`,
`formatModelCardSupportedParameterCoverageCsv`, and
`formatModelCardSupportedParameterCoverageJson` expose the supported canonical
model-card parameters and accepted aliases for D, BJT, JFET, and Level-1 MOS
cards as stable dashboard/export rows.
`modelCardSupportedParameterCoverageSummary`,
`formatModelCardSupportedParameterCoverageSummaryTable`,
`modelCardSupportedParameterCoverageSummaryRecords`,
`formatModelCardSupportedParameterCoverageSummaryCsv`, and
`formatModelCardSupportedParameterCoverageSummaryJson` condense that catalog by
model kind for compact release dashboards and Mosaic UI inventories.
`modelCardSupportedParameterCoverageGate`,
`formatModelCardSupportedParameterCoverageGateReport`,
`formatModelCardSupportedParameterCoverageGateIssueTable`,
`modelCardSupportedParameterCoverageGateIssueRecords`,
`formatModelCardSupportedParameterCoverageGateIssueCsv`, and
`formatModelCardSupportedParameterCoverageGateIssueJson` validate the expected
seven-kind, 74-row supported-parameter catalog and expose stable issue rows for
release automation.
`modelCardSupportedParameterCoverageDashboard`,
`formatModelCardSupportedParameterCoverageDashboardTable`,
`modelCardSupportedParameterCoverageDashboardRecords`,
`formatModelCardSupportedParameterCoverageDashboardCsv`, and
`formatModelCardSupportedParameterCoverageDashboardJson` combine the per-kind
summary counts with gate issue fields for Mosaic/browser dashboards.
`deviceModelAuditFixtures` returns the canonical cross-language fixture cards
used to keep the TypeScript, Python, and Rust ports aligned.
`deviceModelBehaviorAuditFixtures` extends those cards into runnable one-device
DC bias fixtures with reference deck lines and stable expected probe-voltage
windows for diode, BJT, JFET, and Level-1 MOS model-depth audits.
`deviceModelTemperatureAuditFixtures` adds matching `.temp` reference-deck
metadata and stable per-temperature probe windows for those same fixture
circuits. `deviceModelCapacitanceAuditFixtures` adds matching `.ac`
reference-deck metadata and stable high-frequency probe magnitude windows for
diode, BJT, JFET `CGS`/`CGD`, and Level-1 MOS capacitance audits.
`deviceModelNoiseAuditFixtures` adds matching `.noise` reference-deck metadata
and stable source/output PSD windows for diode and BJT shot noise plus JFET
and Level-1 MOS channel thermal noise audits.
`deviceModelChargeAuditFixtures` adds matching `.tran` reference-deck metadata,
explicit terminal storage capacitance metadata, stable first/final
probe-voltage windows, and charge-behavior notes for diode, BJT, JFET, and
Level-1 MOS charge audits. Diode `junctionCapacitance` / `transitTime`, BJT
`baseEmitterCapacitance` / `baseCollectorCapacitance` / `forwardTransitTime` /
`reverseTransitTime`, and JFET `gateSourceCapacitance` /
`gateDrainCapacitance` plus Level-1 MOS `CGSO` / `CGDO` / `CGBO` model-card
parameters plus bulk-junction `CBS` / `CBD` model-card parameters also stamp
transient storage, with MOS `PB` / `MJ` shaping reverse-biased source-body and
drain-body capacitance to match their small-signal AC semantics.
`deviceModelReferenceDeckAuditFixtures` flattens those DC, temperature, AC,
noise, and transient fixture families into a stable reference-deck coverage
matrix for each supported diode, BJT, JFET, and Level-1 MOS model family.
`formatDeviceModelReferenceDeckAuditTable` renders that matrix as a stable
tab-separated audit table for release and reference-deck comparisons.
`deviceModelReferenceDeckAuditRecords`,
`formatDeviceModelReferenceDeckAuditCsv`, and
`formatDeviceModelReferenceDeckAuditJson` expose the same matrix as
header-keyed records and browser/release-friendly CSV or compact JSON.
`deviceModelReferenceDeckAuditSummary`,
`formatDeviceModelReferenceDeckAuditSummaryTable`,
`deviceModelReferenceDeckAuditSummaryRecords`,
`formatDeviceModelReferenceDeckAuditSummaryCsv`, and
`formatDeviceModelReferenceDeckAuditSummaryJson` expose stable per-kind
coverage summaries with missing-analysis and deck-line totals.
`deviceModelReferenceDeckAuditAnalysisSummary`,
`formatDeviceModelReferenceDeckAuditAnalysisSummaryTable`,
`deviceModelReferenceDeckAuditAnalysisSummaryRecords`,
`formatDeviceModelReferenceDeckAuditAnalysisSummaryCsv`, and
`formatDeviceModelReferenceDeckAuditAnalysisSummaryJson` expose the same audit
matrix grouped by analysis kind, with missing-model-family and deck-line
totals for release dashboards.
`deviceModelReferenceDeckAuditMatrix`,
`formatDeviceModelReferenceDeckAuditMatrixTable`,
`deviceModelReferenceDeckAuditMatrixRecords`,
`formatDeviceModelReferenceDeckAuditMatrixCsv`, and
`formatDeviceModelReferenceDeckAuditMatrixJson` expose one stable dashboard row
per model family with explicit OP, temperature, AC, noise, and transient
fixture columns plus missing/extra-analysis inventories.
`deviceModelReferenceDeckAuditGate` and
`formatDeviceModelReferenceDeckAuditGateReport` validate the required
kind-by-analysis coverage matrix and emit a stable pass/fail gate report.
`deviceModelReferenceDeckAuditGateCoverageDigest`,
`formatDeviceModelReferenceDeckAuditGateCoverageDigestTable`,
`deviceModelReferenceDeckAuditGateCoverageDigestRecords`,
`formatDeviceModelReferenceDeckAuditGateCoverageDigestCsv`, and
`formatDeviceModelReferenceDeckAuditGateCoverageDigestJson` expose a one-row
release-dashboard digest with expected, covered, and missing pair counts plus
issue-field inventories.
`formatDeviceModelReferenceDeckAuditGateIssueTable`,
`deviceModelReferenceDeckAuditGateIssueRecords`,
`formatDeviceModelReferenceDeckAuditGateIssueCsv`, and
`formatDeviceModelReferenceDeckAuditGateIssueJson` expose the gate's issue
rows as stable table, record, CSV, and compact JSON payloads for release
dashboards.
`deviceModelReferenceDeckAuditGateIssueSummary`,
`formatDeviceModelReferenceDeckAuditGateIssueSummaryTable`,
`deviceModelReferenceDeckAuditGateIssueSummaryRecords`,
`formatDeviceModelReferenceDeckAuditGateIssueSummaryCsv`, and
`formatDeviceModelReferenceDeckAuditGateIssueSummaryJson` aggregate those gate
issues by field with issue counts, affected fixtures, and messages for compact
CI dashboards.

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
output-directive artifacts, output-plan inventory artifacts, selected measurement artifacts, selected transient Fourier artifacts,
selected-run artifact summaries with table, analysis-directive, output-probe, output-directive,
measurement, Fourier probe, `.control` command, write-marker, rawfile-option, and
control-policy diagnostic inventories, `.ac LIN`, `.ac DEC`, `.ac OCT` frequency grids, and
`.tran` `START` / print-step `TSTEP` / `MAXSTEP` / `UIC` controls. Selected
execution fields expose `.control` command, write-marker, rawfile-option, and
diagnostic inventories directly for host integrations.
