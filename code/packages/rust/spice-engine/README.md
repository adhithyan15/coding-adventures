# spice-engine

`spice-engine` provides SPICE-style circuit analysis primitives for Rust.

The initial slices implement:

- DC operating-point analysis for linear circuits using modified nodal analysis
  (MNA), with stable `DcResult::diagnostics` metadata for matrix size, selected
  real solver path, tolerance, convergence aid, final Newton delta, and a
  nested solver profile for backend, structural nonzeros, density, and fill-in.
  Nonlinear operating points can bound Newton updates with
  `DcOpOptions::newton_step_limit`, and diagnostics report limiter activity.
- DC operating-point sweeps across explicit analysis temperatures, including
  named corner sweeps and order-preserving parallel named corner DC sweeps.
- DC source sweeps over independent voltage and current sources, including
  order-preserving parallel named corner source sweeps.
- DC sensitivity analysis for output-node voltage changes with respect to
  resistor and independent source parameters, including order-preserving
  parallel named corner sweeps.
- Seeded DC Monte Carlo analysis for linear element tolerances with Gaussian
  and uniform distributions, including order-preserving parallel named corner
  sweeps.
- AC noise analysis with resistor thermal-noise contributions, input-referred
  PSD, and order-preserving parallel named corner sweeps.
- AC frequency sweeps, including order-preserving parallel named corner sweeps.
- DC small-signal transfer-function (`.tf`) analysis with input and output
  impedance estimates, including order-preserving parallel named corner sweeps.
- AC small-signal frequency sweeps for linear RC/RL circuits and explicit AC
  source phasors with DC-bias operating-point linearization for nonlinear
  devices.
- S-parameter extraction for two-port networks, including order-preserving
  parallel named corner sweeps.
- Backward-Euler, trapezoidal, Gear-2, and adaptive transient analysis for
  linear RC/RL circuits, including named corner sweeps for fixed-step and
  adaptive runs.
- Periodic steady-state analysis for periodic source circuits, including named
  corner sweeps.
- Time-varying transient source waveforms: PWL, SIN, PULSE, and EXP.
- Mixed-signal boundary helpers that map binary digital event timelines and
  named event streams to finite-edge PWL voltage sources, run SPICE-side
  fixed-step and adaptive digital-input transient bridge fixtures, and sample
  transient probes back into thresholded digital events, including multi-probe
  named event streams and named-corner digital-input bridge runs with event
  stream table output plus bridge breakpoint schedules and deterministic VCD
  correlation output.
- A Rust-native custom-model foothold with `CustomModel`,
  `CustomModelKind::LinearConductance`, `CustomModelEvaluation`, and
  `analyze_custom_model_source` for the first two-terminal residual/Jacobian
  hook and diagnostic-only Verilog-A subset.
- Fourier post-processing for transient output, including DC, harmonic
  magnitude/phase, THD results, and named corner sweeps.
- Transient-to-distortion projection through the Fourier extraction path,
  including named corner sweeps.
- Constrained RC and RLC low-pass/high-pass/band-pass/notch pole-zero helpers,
  including named corner sweeps.
- Stable text output tables for selected node voltages, branch currents,
  sampled digital event streams and named multi-probe digital event streams,
  mixed-signal bridge breakpoint schedules, cornered and adaptive mixed-signal
  bridge output streams,
  VCD mixed-signal bridge correlation output,
  DC operating-point temperature sweeps, cornered DC operating-point
  temperature sweeps, transient samples, adaptive transient samples, cornered
  transient samples, cornered adaptive transient samples, AC phasors, cornered DC
  operating points, cornered AC phasors, PSS steady-state periods,
  sensitivity entries, Fourier harmonics, cornered Fourier harmonics,
  transfer-function results, cornered transfer-function results, S-parameter
  entries, cornered S-parameter entries, pole-zero entries, noise PSD
  contributions, cornered noise PSD contributions, cornered sensitivity
  entries, cornered PSS steady-state periods, cornered pole-zero entries,
  distortion harmonics, and cornered distortion harmonics, Monte Carlo trials,
  cornered Monte Carlo trials, DC source sweeps, and cornered DC source
  sweeps.
- Parsed transient and DC sweep `.measure` / `.meas` card extraction and
  execution helpers that route deck cards into stable scalar measurement rows.
- Source-order `run_deck` whole-deck execution for parsed `.op`, `.dc`, `.ac`,
  `.tran`, `.tf`, `.sens`, and `.noise` cards with aggregate run-artifact
  table, CSV, compact JSON, and header-keyed record exports.

The package supports resistors, capacitors, inductors, diodes, BJTs,
independent current sources, independent voltage sources, voltage-controlled
current sources, optional AC source phasors, optional source waveforms, ground
aliases, node voltages, and voltage source branch currents.
Large real DC and complex AC matrix solves use native sparse-row solver paths
when the matrix size reaches the package threshold. DC diagnostics expose the
actual real-solver backend and matrix sparsity profile for production audits.
For nonlinear DC solves, `DcOpOptions::newton_step_limit` bounds each Newton
update per unknown. `DcResult::diagnostics` reports the active limit, clipped
step count, and minimum damping factor; set the option to `None` to disable
the limiter.

```rust
use spice_engine::{
    transient_adaptive, AdaptiveTransientOptions, Circuit, Element, PwlWaveform, Resistor,
    TransientMethod, VoltageSource, Waveform,
};

let mut circuit = Circuit::new();
circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
    "Vin",
    "in",
    "0",
    0.0,
    Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (1.0e-9, 1.8)])),
)));
circuit.add(Element::Resistor(Resistor::new("Rload", "in", "0", 1_000.0)));
let result = transient_adaptive(
    &circuit,
    0.5e-9,
    1.0e-9,
    AdaptiveTransientOptions {
        method: TransientMethod::Gear2,
        ..Default::default()
    },
)?;
```

`diode_at_temperature`, `bjt_at_temperature`, `mosfet_at_temperature`, and
`circuit_at_temperature` provide operating-temperature footholds for diode,
BJT, and Level-1 MOSFET models before running an analysis.

`normalize_model_card`, `diode_from_model_card`, `bjt_from_model_card`,
`jfet_from_model_card`, and `mosfet_from_model_card` provide the shared
`.model` alias surface for diode, BJT, JFET, and Level-1 MOS cards.
Diode cards accept `VJ`/`PB` junction potential, `M`/`MJ` grading coefficient,
and `FC` forward-bias depletion coefficient. AC and transient analyses use them
to shape `CJO` depletion capacitance continuously around the `FC * VJ`
transition.
Diode cards also accept `XTI` (default `3`) and `EG` (default `1.11 eV`) to
control saturation-current temperature scaling.
`model_card_unsupported_parameter_issues`,
`format_model_card_unsupported_parameter_issue_table`,
`model_card_unsupported_parameter_issue_records`,
`format_model_card_unsupported_parameter_issue_csv`, and
`format_model_card_unsupported_parameter_issue_json` expose retained
unsupported model-card keys as stable diagnostics for parser and UI surfaces.
`model_card_supported_parameter_coverage`,
`format_model_card_supported_parameter_coverage_table`,
`model_card_supported_parameter_coverage_records`,
`format_model_card_supported_parameter_coverage_csv`, and
`format_model_card_supported_parameter_coverage_json` expose the supported
canonical model-card parameters and accepted aliases for D, BJT, JFET, and
Level-1 MOS cards as stable dashboard/export rows.
`model_card_supported_parameter_coverage_summary`,
`format_model_card_supported_parameter_coverage_summary_table`,
`model_card_supported_parameter_coverage_summary_records`,
`format_model_card_supported_parameter_coverage_summary_csv`, and
`format_model_card_supported_parameter_coverage_summary_json` condense that
catalog by model kind for compact release dashboards and Mosaic UI inventories.
`model_card_supported_parameter_coverage_gate`,
`format_model_card_supported_parameter_coverage_gate_report`,
`format_model_card_supported_parameter_coverage_gate_issue_table`,
`model_card_supported_parameter_coverage_gate_issue_records`,
`format_model_card_supported_parameter_coverage_gate_issue_csv`, and
`format_model_card_supported_parameter_coverage_gate_issue_json` validate the
expected seven-kind, 72-row supported-parameter catalog and expose stable issue
rows for release automation.
`model_card_supported_parameter_coverage_dashboard`,
`format_model_card_supported_parameter_coverage_dashboard_table`,
`model_card_supported_parameter_coverage_dashboard_records`,
`format_model_card_supported_parameter_coverage_dashboard_csv`, and
`format_model_card_supported_parameter_coverage_dashboard_json` combine the
per-kind summary counts with gate issue fields for Mosaic/browser dashboards.
`device_model_audit_fixtures` returns the canonical cross-language fixture
cards used to keep the Rust, Python, and TypeScript ports aligned.
`device_model_behavior_audit_fixtures` extends those cards into runnable
one-device DC bias fixtures with reference deck lines and stable expected
probe-voltage windows for diode, BJT, JFET, and Level-1 MOS model-depth audits.
`device_model_temperature_audit_fixtures` adds matching `.temp`
reference-deck metadata and stable per-temperature probe windows for those same
fixture circuits. `device_model_capacitance_audit_fixtures` adds matching
`.ac` reference-deck metadata and stable high-frequency probe magnitude windows
for diode, BJT, JFET `CGS`/`CGD`, and Level-1 MOS capacitance audits.
`device_model_noise_audit_fixtures` adds matching `.noise` reference-deck
metadata and stable source/output PSD windows for diode and BJT shot noise plus
JFET and Level-1 MOS channel thermal noise audits.
`device_model_charge_audit_fixtures` adds matching `.tran` reference-deck
metadata, explicit terminal storage capacitance metadata, stable first/final
probe-voltage windows, and charge-behavior notes for diode, BJT, JFET, and
Level-1 MOS charge audits. Diode `junction_capacitance` / `transit_time` and
BJT `base_emitter_capacitance` / `base_collector_capacitance` /
`forward_transit_time` / `reverse_transit_time`, and JFET
`gate_source_capacitance` / `gate_drain_capacitance` plus Level-1 MOS
`gate_source_overlap_capacitance`, `gate_drain_overlap_capacitance`,
`gate_bulk_overlap_capacitance`, and bulk-junction
`source_bulk_capacitance` / `drain_bulk_capacitance` model-card parameters
also stamp transient storage, with MOS `bulk_junction_potential` /
`bulk_junction_grading_coefficient` shaping reverse-biased source-body and
drain-body capacitance to match their small-signal AC semantics.
`device_model_reference_deck_audit_fixtures` flattens those DC, temperature,
AC, noise, and transient fixture families into a stable reference-deck
coverage matrix for each supported diode, BJT, JFET, and Level-1 MOS model
family.
`format_device_model_reference_deck_audit_table` renders that matrix as a
stable tab-separated audit table for release and reference-deck comparisons.
`device_model_reference_deck_audit_records`,
`format_device_model_reference_deck_audit_csv`, and
`format_device_model_reference_deck_audit_json` expose the same matrix as
header-keyed records and browser/release-friendly CSV or compact JSON.
`device_model_reference_deck_audit_summary`,
`format_device_model_reference_deck_audit_summary_table`,
`device_model_reference_deck_audit_summary_records`,
`format_device_model_reference_deck_audit_summary_csv`, and
`format_device_model_reference_deck_audit_summary_json` expose stable per-kind
coverage summaries with missing-analysis and deck-line totals.
`device_model_reference_deck_audit_analysis_summary`,
`format_device_model_reference_deck_audit_analysis_summary_table`,
`device_model_reference_deck_audit_analysis_summary_records`,
`format_device_model_reference_deck_audit_analysis_summary_csv`, and
`format_device_model_reference_deck_audit_analysis_summary_json` expose the
same audit matrix grouped by analysis kind, with missing-model-family and
deck-line totals for release dashboards.
`device_model_reference_deck_audit_matrix`,
`format_device_model_reference_deck_audit_matrix_table`,
`device_model_reference_deck_audit_matrix_records`,
`format_device_model_reference_deck_audit_matrix_csv`, and
`format_device_model_reference_deck_audit_matrix_json` expose one stable
dashboard row per model family with explicit OP, temperature, AC, noise, and
transient fixture columns plus missing/extra-analysis inventories.
`device_model_reference_deck_audit_gate` and
`format_device_model_reference_deck_audit_gate_report` validate the required
kind-by-analysis coverage matrix and emit a stable pass/fail gate report.
`device_model_reference_deck_audit_gate_coverage_digest`,
`format_device_model_reference_deck_audit_gate_coverage_digest_table`,
`device_model_reference_deck_audit_gate_coverage_digest_records`,
`format_device_model_reference_deck_audit_gate_coverage_digest_csv`, and
`format_device_model_reference_deck_audit_gate_coverage_digest_json` expose a
one-row release-dashboard digest with expected, covered, and missing pair
counts plus issue-field inventories.
`format_device_model_reference_deck_audit_gate_issue_table`,
`device_model_reference_deck_audit_gate_issue_records`,
`format_device_model_reference_deck_audit_gate_issue_csv`, and
`format_device_model_reference_deck_audit_gate_issue_json` expose the gate's
issue rows as stable table, record, CSV, and compact JSON payloads for release
dashboards.
`device_model_reference_deck_audit_gate_issue_summary`,
`format_device_model_reference_deck_audit_gate_issue_summary_table`,
`device_model_reference_deck_audit_gate_issue_summary_records`,
`format_device_model_reference_deck_audit_gate_issue_summary_csv`, and
`format_device_model_reference_deck_audit_gate_issue_summary_json` aggregate
those gate issues by field with issue counts, affected fixtures, and messages
for compact CI dashboards.

`analyze_custom_model_source` accepts only a two-terminal `I(p,n) <+ ...`
module shape and rejects dynamic/event/system constructs; it is not a full
Verilog-A compiler.

`compatibility_corpus` exposes the first release-readiness deck corpus for
`.op`, `.dc`, `.ac`, `.tran`, and `.tf` coverage. Each fixture carries a
documented oracle, golden values with tolerances, and known incompatibility
notes. `release_readiness_gates` validates the corpus metadata, while
`format_compatibility_corpus_table` and `format_release_readiness_report`
provide stable tab-separated summaries for package checks.
`analyze_deck_controls` provides the shared deck-control boundary foothold: it
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
`resolve_deck_sources` is the first include/library resolution layer: callers
provide a source-content map, `.include` directives are expanded in place, and
`.lib path section` selects a named `.lib` / `.endl` section with stable
diagnostics for missing files, missing sections, unterminated sections, cycles,
and still-unsupported `.control` block commands that are not part of the
selected analysis/output subset.
`measure_transient_probe`, `measure_transient_deck`,
`measure_dc_sweep_probe`, `measure_dc_sweep_deck`,
`measure_ac_sweep_probe`, `measure_ac_sweep_deck`, and
`format_measurement_table` provide the shared `.MEASURE`-style scalar output
surface for MAX, MIN, AVG, RMS, peak-to-peak, and final-value probe
measurements. The AC helper measures complex probe magnitudes over optional
frequency windows. `measure_transient_find_at_probe` and parsed transient
`FIND ... AT=` cards sample or linearly interpolate one probe value at a scalar
time, while `measure_transient_when_probe`,
`measure_transient_when_probe_counted`, and parsed transient
`WHEN probe=target` cards report first or counted `RISE`, `FALL`, and `CROSS`
crossing times over optional `FROM=` / `TO=` windows.
`measure_transient_delay_between_probes` and parsed transient `TRIG ... TARG
...` cards report trigger-to-target crossing delays. The deck helpers route
parsed transient, DC sweep, and AC sweep `.measure` / `.meas` cards into those
stable measurement rows.
`resolve_deck_analyses` extracts `.op`, `.dc`, `.ac`, and `.tran` cards before
`.end`, keeps non-analysis active lines, and reports stable diagnostics for
malformed arguments, unsupported AC sweep modes, invalid sweep intervals, and
unresolved scalar expressions. `select_deck_analysis_plan` picks one explicit
card by analysis alias, defaults decks without analysis cards to an implicit
`.op`, and reports ambiguity before solver dispatch.
`run_deck_analysis` executes one selected `.op`, `.dc`, `.ac LIN`, `.ac DEC`,
`.ac OCT`, or `.tran` plan against an existing `Circuit` and returns the plan,
solver result, deck-selected output table, and normalized analysis directive,
table count/name list, output probes, and output directives that produced the
table, plus selected `.measure` results and
a stable measurement table for `.dc`, `.ac`, and `.tran` executions.
Execution `table_artifacts` preserve the same order as `tables` and carry each
stable table's text, CSV, compact JSON, and header-keyed records.
Execution `output_plan_artifacts` summarize the selected result row count,
result columns, selected analysis line/source/output-node and sweep/time/frequency
metadata, output probes, selected output probe source lines, output directives,
normalized output directive kinds, normalized directive analysis scopes, selected
output directive source lines, and stable table names, and the
`output-plan` entry in `table_artifacts` carries the same
table, CSV, compact JSON, and header-keyed record exports.
Selected `.tran` plans route
`START` output filtering, `.tran TSTEP` as the output print grid, `MAXSTEP` as
an internal fixed-step cap, and `UIC` initial-condition intent through that
stable transient table surface. They also return selected `.four` harmonic
results and a stable Fourier table. Executions also include selected-run
artifact summaries plus `format_deck_run_artifact_table` and
`format_deck_run_artifact_csv` / `format_deck_run_artifact_json` output for
stable result-row, table, analysis-directive, output-probe, output-directive,
measurement, Fourier, control-command, write-marker, rawfile-option, and
diagnostic count/name lists.
`run_deck` executes every parsed `.op`, `.dc`, `.ac`, `.tran`, `.tf`, `.sens`,
and `.noise` card in source order, preserves duplicate analysis directives,
and defaults analysis-less decks to an implicit `.op`. Its whole-run result
returns ordered selected executions plus aggregate selected-run artifact table,
CSV, compact JSON, and header-keyed records, and each selected-run artifact
carries the deck-wide analysis kind/directive inventories beside the selected
analysis directive metadata.
Normalized accepted `.control` commands are surfaced separately in
`control_line_count` / `control_lines` execution fields and in
`ControlLines` / `ControlLineList` artifact fields.
Accepted `.control` `write` / `wrdata` rawfile/data-write markers are surfaced
as `write_marker_count` / `write_markers` execution fields and as
`WriteMarkers` / `WriteMarkerList` artifact fields without serializing files.
Accepted `write <rawfile> ...` markers also produce deterministic in-memory
ASCII rawfile artifacts on `rawfile_artifacts`, with stable probe-aware table,
CSV, JSON, and header-keyed record summaries. Accepted `wrdata <file> ...` markers
produce deterministic in-memory ASCII data-file artifacts on
`wrdata_artifacts`, with stable table, CSV, JSON, and header-keyed record
summaries; filesystem writes remain metadata-only.
Accepted `.control` rawfile output options (`set filetype=ascii`,
`set wr_vecnames`, `set wr_singlescale`, and `set appendwrite`) are surfaced as
`rawfile_option_count` / `rawfile_options` execution fields and as
`RawfileOptions` / `RawfileOptionList` artifact fields. WRDATA artifacts also
carry the same option inventories and render `wr_vecnames` / `wr_singlescale`
intent as stable `VectorNames` / `Scale` metadata in the in-memory data file.
When a `write` or `wrdata` marker names probes, its in-memory artifact keeps
the scale column plus only the requested matching probe columns, while artifact
summaries keep matched and unmatched probe inventories in stable table, CSV,
JSON, and record exports.
Existing `.control` body policy diagnostics flow into those selected-run artifact `Diagnostics` /
`DiagnosticCodeList` fields and through the same run-artifact table, CSV, JSON,
and `table_artifacts` records. Policy-blocked `source` / `shell`, `cd`,
control-flow, and variable/state commands also populate
`control_policy_artifacts` with stable line, category, command, code, severity,
and message fields plus table, CSV, compact JSON, and header-keyed record
exports. `format_deck_table_csv` also converts any stable
tab-separated deck table to CSV, `format_deck_table_json` converts the same
tables to compact JSON records, and `deck_table_records` returns header-keyed
native records for host integrations.
`resolve_deck_fourier`, `fourier_transient_cards`,
`fourier_transient_deck`, and `format_deck_fourier_table` extract parsed
`.four` / `.FOUR` cards before `.end` and route transient samples into the
existing SPICE-style Fourier result shape with optional `HARMONICS=` and
`FROM=` controls.
`resolve_deck_parameters` evaluates scalar whitespace-tokenized `.param`
assignments, collects scalar `.func` definitions before `.end`, preserves
parameter order, rewrites braced and quoted active-line expressions, and emits
stable diagnostics for unresolved expressions, bad function arity, unknown
functions, and recursive function calls.
`resolve_deck_initial_conditions` extracts scalar `.ic` and `.nodeset`
`V(node)=value` hints before `.end`, keeps non-condition active lines, evaluates
numeric SPICE suffix/arithmetic expressions, and reports stable diagnostics for
malformed targets or unresolved values. `dc_initial_vector_from_conditions`
maps those parsed node-voltage hints into the DC solver's MNA warm-start vector,
and `dc_op_with_initial_conditions` applies that vector to the operating-point
solve with `.ic` values taking precedence over `.nodeset` values.
`resolve_deck_functions` extracts scalar `.func name(args) expression`
definitions before `.end`, preserves non-function active lines, strips braced
or quoted expression delimiters, and reports stable diagnostics for malformed
signatures, arguments, or empty expressions.
`resolve_deck_measurements` extracts transient, DC sweep, and AC sweep `.measure` /
`.meas` cards before `.end`, keeps non-measure active lines, evaluates optional
`FROM=` / `TO=` scalar time, source-value, or frequency windows plus transient
`FIND ... AT=` sample points and `WHEN probe=target` crossings with optional
`RISE`, `FALL`, or `CROSS` counters, and reports stable diagnostics for
unsupported analyses, modes, options, expressions, and invalid windows.
`resolve_deck_outputs` extracts `.save`, scoped or global `.probe`, scoped
`.print <analysis> ...`, and scoped `.plot <analysis> ...` cards before
`.end`, keeps non-output active lines, and reports stable diagnostics for
missing probe lists, unsupported scoped output analyses, or malformed output
probes. `select_deck_output_probes` deduplicates the selected probes for a
requested analysis, while `format_deck_op_table`,
`format_deck_dc_sweep_table`, `format_deck_ac_table`, and
`format_deck_transient_table` feed those deck-card selections into the stable
text table formatters.
`resolve_deck_analyses` extracts `.op`, `.dc`, `.ac`, and `.tran` cards before
`.end`, preserves non-analysis active lines, and reports stable diagnostics for
malformed deck-level analysis controls.
`select_deck_analysis_plan` returns one selected or implicit plan for
downstream deck execution helpers.
`run_deck_analysis` routes that selected plan into the matching solver and
stable deck-selected table output with normalized table-inventory, output-probe
and output-directive artifacts,
selected measurement artifacts, selected transient Fourier artifacts,
selected-run artifact summaries with table, analysis-directive, output-probe,
output-directive, measurement, Fourier probe, `.control` command, and
write-marker, rawfile-option, and diagnostic inventories, `.ac LIN`, `.ac DEC`,
`.ac OCT` frequency grids, and `.tran` `START` / print-step `TSTEP` /
`MAXSTEP` / `UIC` controls. Selected execution fields expose `.control`
command, write-marker, rawfile-option, and diagnostic inventories directly for
host integrations.
