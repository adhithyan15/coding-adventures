# spice-engine

`spice-engine` provides SPICE-style circuit analysis primitives for Rust.

The initial slices implement:

- DC operating-point analysis for linear circuits using modified nodal analysis
  (MNA), with stable `DcResult::diagnostics` metadata for matrix size, selected
  real solver path, tolerance, convergence aid, and final Newton delta.
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

The package supports resistors, capacitors, inductors, diodes, BJTs,
independent current sources, independent voltage sources, voltage-controlled
current sources, optional AC source phasors, optional source waveforms, ground
aliases, node voltages, and voltage source branch currents.
Large real DC and complex AC matrix solves use sparse-row solver paths when the
matrix size reaches the package threshold.

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
`device_model_audit_fixtures` returns the canonical cross-language fixture
cards used to keep the Rust, Python, and TypeScript ports aligned.

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
stable table's text, CSV, compact JSON, and header-keyed records. Selected
`.tran` plans route
`START` output filtering, `.tran TSTEP` as the output print grid, `MAXSTEP` as
an internal fixed-step cap, and `UIC` initial-condition intent through that
stable transient table surface. They also return selected `.four` harmonic
results and a stable Fourier table. Executions also include selected-run
artifact summaries plus `format_deck_run_artifact_table` and
`format_deck_run_artifact_csv` / `format_deck_run_artifact_json` output for
stable result-row, table, analysis-directive, output-probe, output-directive,
measurement, Fourier, control-command, and diagnostic count/name lists.
Normalized accepted `.control` commands are surfaced separately in
`ControlLines` / `ControlLineList` artifact fields. Existing `.control` body
policy diagnostics flow into those selected-run artifact `Diagnostics` /
`DiagnosticCodeList` fields and through the same run-artifact table, CSV, JSON,
and `table_artifacts` records. `format_deck_table_csv` also converts any stable
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
selected-run artifact summaries with table, analysis-directive, output-probe, output-directive,
measurement, Fourier probe, and `.control` command inventories, `.ac LIN`, `.ac DEC`, `.ac OCT`
frequency grids, and `.tran`
`START` / print-step `TSTEP` / `MAXSTEP` / `UIC` controls.
