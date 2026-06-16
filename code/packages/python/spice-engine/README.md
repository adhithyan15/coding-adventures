# spice-engine

SPICE-compatible analog circuit simulator. Modified Nodal Analysis (MNA) with
Newton-Raphson DC, trapezoidal transient, AC small-signal sweep, DC transfer
function (`.TF`), DC parameter sweep (`.DC`), sensitivity analysis (`.SENS`),
Monte Carlo (`.MC`), noise analysis (`.NOISE`), and all four SPICE controlled
sources (VCVS / VCCS / CCCS / CCVS).  Transient outputs can also be
post-processed with SPICE-style Fourier (`.FOUR`) measurements and projected
into a distortion result shape.  Constrained pole-zero helpers cover simple RC
low-pass/high-pass and series RLC low-pass/high-pass/band-pass/notch fixtures,
with stable AC, Fourier, transfer-function, pole-zero, and distortion text
output for `.AC` / `.FOUR` / `.TF` / `.PZ` / `.DISTO` snapshots.

See [`code/specs/spice-engine.md`](../../../specs/spice-engine.md).

## Quick start

```python
from spice_engine import Circuit, Resistor, VoltageSource, dc_op

# Voltage divider: V1 = 10V, R1 = 1k, R2 = 1k -> V_mid = 5V
circuit = Circuit()
circuit.add(VoltageSource("V1", "vin", "0", voltage=10.0))
circuit.add(Resistor("R1", "vin", "vmid", 1000.0))
circuit.add(Resistor("R2", "vmid", "0", 1000.0))

result = dc_op(circuit)
print(result.node_voltages)        # {"vin": 10.0, "vmid": 5.0}
print(result.branch_currents)      # {"I(V1)": -0.005}
print(result.converged)            # True
print(result.diagnostics.solver)   # "dense_real" or "sparse_real"
```

## Supported elements

| Class | SPICE | Description |
|-------|-------|-------------|
| `Resistor` | R | Ohmic resistor |
| `Capacitor` | C | Linear capacitor (with optional initial voltage) |
| `Inductor` | L | Linear inductor (with optional initial current) |
| `VoltageSource` | V | Independent voltage source |
| `CurrentSource` | I | Independent current source |
| `Diode` | D | Shockley diode model |
| `Mosfet` | M | MOSFET (uses `mosfet_models.MOSFET`) |
| `BJT` | Q | Bipolar transistor (simplified Ebers-Moll) |
| `CustomModel` | Verilog-A subset foothold | Two-terminal custom current residual/Jacobian hook |
| `VCVS` | E | Voltage-Controlled Voltage Source |
| `VCCS` | G | Voltage-Controlled Current Source |
| `CCCS` | F | Current-Controlled Current Source |
| `CCVS` | H | Current-Controlled Voltage Source |

## Supported analyses

| Function | SPICE | Description |
|----------|-------|-------------|
| `dc_op` | `.OP` | DC operating point (Newton-Raphson) |
| `transient` | `.TRAN` | Time-domain transient (trapezoidal/BE, adaptive timestep) |
| `ac_sweep` | `.AC` | Small-signal AC frequency sweep |
| `tf` | `.TF` | DC transfer function, input/output impedance |
| `dc_sweep` | `.DC` | DC parameter sweep |
| `resolve_deck_analyses` | `.OP` / `.DC` / `.AC` / `.TRAN` | Parsed deck analysis metadata |
| `sens_dc` | `.SENS` | DC sensitivity analysis |
| `mc_dc` | `.MC` | Monte Carlo DC analysis |
| `noise_ac` | `.NOISE` | Small-signal noise PSD (adjoint method) |
| `fourier` | `.FOUR` | Harmonic magnitudes/phases and THD from transient output |
| `format_dc_table`, `format_transient_table` | `.PRINT` / `.PLOT` output | Stable tabular node voltages and branch currents |
| `resolve_deck_outputs`, `format_deck_*_table` | `.SAVE` / `.PROBE` / `.PRINT` / `.PLOT` output | Parsed deck-selected OP, DC sweep, AC, and transient tables |
| `format_deck_run_artifact_table` | Deck execution artifact | Stable selected-run row, output-probe, measurement, and Fourier counts |
| `measure_transient_probe`, `measure_dc_sweep_probe`, `measure_ac_sweep_probe`, `format_measurement_table` | `.MEASURE` output | Stable scalar transient, DC sweep, and AC sweep probe measurements |

`diode_at_temperature()`, `bjt_at_temperature()`, `mosfet_at_temperature()`,
and `circuit_at_temperature()` provide operating-temperature footholds for
diode, BJT, and Level-1 MOSFET models before running an analysis.
`dc_temperature_sweep()` and `dc_temperature_sweep_corners()` run
`.temp`-style DC operating-point snapshots across explicit analysis
temperatures, with stable nominal and named-corner table helpers for
cross-language comparison. `format_corner_dc_table()` also renders named-corner
DC operating-point snapshots with the Rust-matching `Corner` / `Index` columns.
`format_dc_sweep_table()`, `format_corner_dc_sweep_table()`,
`format_corner_ac_table()`, and `format_corner_tf_table()` provide the matching
stable `.DC`, `.AC`, and `.TF` sweep/corner text surfaces.
`measure_transient_probe()`, `measure_transient_deck()`,
`measure_dc_sweep_probe()`, `measure_dc_sweep_deck()`,
`measure_ac_sweep_probe()`, `measure_ac_sweep_deck()`, and
`format_measurement_table()` provide the shared `.MEASURE`-style scalar output
surface for MAX, MIN, AVG, RMS, peak-to-peak, and final-value probe
measurements. The AC helper measures complex probe magnitudes over optional
frequency windows. `measure_transient_find_at_probe()` and parsed transient
`FIND ... AT=` cards sample or linearly interpolate one probe value at a scalar
time, while `measure_transient_when_probe()`,
`measure_transient_when_probe_counted()`, and parsed transient
`WHEN probe=target` cards report first or counted `RISE`, `FALL`, and `CROSS`
crossing times over optional `FROM=` / `TO=` windows.
`measure_transient_delay_between_probes()` and parsed transient `TRIG ...
TARG ...` cards report trigger-to-target crossing delays. The deck helpers
route parsed transient, DC sweep, and AC sweep `.measure` / `.meas` cards into
those stable measurement rows.
`resolve_deck_analyses()` extracts `.op`, `.dc`, `.ac`, and `.tran` cards
before `.end`, keeps non-analysis active lines, and reports stable diagnostics
for malformed arguments, unsupported AC sweep modes, invalid sweep intervals,
and unresolved scalar expressions. `select_deck_analysis_plan()` picks one
explicit card by analysis alias, defaults decks without analysis cards to an
implicit `.op`, and reports ambiguity before solver dispatch.
`run_deck_analysis()` executes one selected `.op`, `.dc`, `.ac LIN`,
`.ac DEC`, `.ac OCT`, or `.tran` plan against an existing `Circuit` and
returns the plan, solver result, deck-selected output table, and normalized
output probes that produced the table, plus selected `.measure` results and a
stable measurement table for `.dc`, `.ac`, and `.tran` executions. Selected
`.tran` plans also return selected `.four` harmonic results and a stable
Fourier table. Executions also include a selected-run artifact summary plus
`format_deck_run_artifact_table()` output for stable result-row,
output-probe, measurement, and Fourier counts. They route `START` output
filtering, use `.tran TSTEP` as the output print grid, apply `MAXSTEP` as an
internal fixed-step cap, and carry `UIC` initial-condition intent through that
stable transient table surface.
`resolve_deck_outputs()` and `select_deck_output_probes()` extract `.save`,
scoped or global `.probe`, scoped `.print <analysis> ...`, and scoped
`.plot <analysis> ...` cards before `.end`, normalize and deduplicate output
probes in deck order, and feed
`format_deck_op_table()`,
`format_deck_dc_sweep_table()`, `format_deck_ac_table()`, and
`format_deck_transient_table()` for stable deck-selected text output.
`resolve_deck_fourier()`, `fourier_transient_cards()`,
`fourier_transient_deck()`, and `format_deck_fourier_table()` extract parsed
`.four` / `.FOUR` cards before `.end` and route transient samples into the
existing SPICE-style Fourier result shape with optional `HARMONICS=` and
`FROM=` controls.

`DcResult.diagnostics` reports stable solve metadata, including the MNA matrix
size, selected real solver path, tolerance, convergence aid, and final Newton
delta.  Large real DC and complex AC matrix solves use sparse-row solver paths
when the matrix size reaches the package threshold.

`normalize_model_card()`, `diode_from_model_card()`,
`bjt_from_model_card()`, `jfet_from_model_card()`, and
`mosfet_from_model_card()` provide the shared `.model` alias surface for diode,
BJT, JFET, and Level-1 MOS cards. `device_model_audit_fixtures()` returns the
canonical cross-language fixture cards used to keep the Python, Rust, and
TypeScript ports aligned.

`DigitalEventStream`, `DigitalLogicLevels`, and `DigitalThresholds` provide the
first mixed-signal bridge surface: digital event streams can drive finite-edge
PWL voltage sources, fixed/adaptive transient outputs can be sampled back into
thresholded event streams, and stable event, bridge-schedule, corner, adaptive,
and VCD text outputs let hardware-VM traces correlate with SPICE probes.

`CustomModel`, `CustomModelEvaluation`, `custom_linear_conductance_model()`, and
`analyze_custom_model_source()` provide the first custom-model foothold. The
accepted source subset is a diagnostic-only two-terminal `I(p,n) <+ ...`
module shape; it deliberately rejects dynamic/event/system constructs until a
full Verilog-A compiler is in scope.

`compatibility_corpus()` exposes the first release-readiness deck corpus for
`.op`, `.dc`, `.ac`, `.tran`, and `.tf` coverage. Each fixture carries a
documented oracle, golden values with tolerances, and known incompatibility
notes. `release_readiness_gates()` validates the corpus metadata, while
`format_compatibility_corpus_table()` and `format_release_readiness_report()`
provide stable tab-separated summaries for package checks.
`analyze_deck_controls()` provides the shared deck-control boundary foothold:
it returns active lines before `.end` and stable diagnostics for unsupported
`.include`, `.lib`, and `.control` directives. Inside `.control` blocks,
selected analysis/output commands (`op`, `dc`, `ac`, `tran`, `save`, `probe`,
`measure`, `meas`, `four`, `fourier`, `print`, and `plot`) are normalized into
dotted deck cards, while `run`, `reset`, `quit`, and the UI-only
`set noaskquit` option plus the ASCII rawfile-format `set filetype=ascii`
option are accepted as no-op control markers. Other unrecognized non-comment
commands emit diagnostics until a broader executed control subset is in scope.
`resolve_deck_sources()` is the first include/library resolution layer: callers
provide a source-content map, `.include` directives are expanded in place, and
`.lib path section` selects a named `.lib` / `.endl` section with stable
diagnostics for missing files, missing sections, unterminated sections, cycles,
and still-unsupported `.control` block commands that are not part of the
selected analysis/output subset.
`resolve_deck_parameters()` evaluates scalar whitespace-tokenized `.param`
assignments, collects scalar `.func` definitions before `.end`, preserves
parameter order, rewrites braced and quoted active-line expressions, and emits
stable diagnostics for unresolved expressions, bad function arity, unknown
functions, and recursive function calls.
`resolve_deck_initial_conditions()` extracts scalar `.ic` and `.nodeset`
`V(node)=value` hints before `.end`, keeps non-condition active lines, evaluates
numeric SPICE suffix/arithmetic expressions, and reports stable diagnostics for
malformed targets or unresolved values. `dc_initial_vector_from_conditions()`
maps those parsed node-voltage hints into the DC solver's MNA warm-start vector,
and `dc_op_with_initial_conditions()` applies that vector to the operating-point
solve with `.ic` values taking precedence over `.nodeset` values.
`resolve_deck_functions()` extracts scalar `.func name(args) expression`
definitions before `.end`, preserves non-function active lines, strips braced
or quoted expression delimiters, and reports stable diagnostics for malformed
signatures, arguments, or empty expressions.
`resolve_deck_measurements()` extracts transient, DC sweep, and AC sweep `.measure` /
`.meas` cards before `.end`, keeps non-measure active lines, evaluates optional
`FROM=` / `TO=` scalar time, source-value, or frequency windows plus transient
`FIND ... AT=` sample points and `WHEN probe=target` crossings with optional
`RISE`, `FALL`, or `CROSS` counters, and reports stable diagnostics for
unsupported analyses, modes, options, expressions, and invalid windows.
`resolve_deck_outputs()` extracts `.save`, `.probe`, `.print`, and `.plot`
cards before `.end`, preserves non-output active lines, and reports stable
diagnostics for missing probe lists, unsupported scoped output analyses, or
malformed `V(node)` / `I(source)` probes.
`resolve_deck_analyses()` extracts `.op`, `.dc`, `.ac`, and `.tran` cards
before `.end`, preserves non-analysis active lines, and reports stable
diagnostics for malformed deck-level analysis controls.
`select_deck_analysis_plan()` returns one selected or implicit plan for
downstream deck execution helpers.
`run_deck_analysis()` routes that selected plan into the matching solver and
stable deck-selected table output with normalized output-probe artifacts,
selected measurement artifacts, selected transient Fourier artifacts,
selected-run artifact summaries, `.ac LIN`, `.ac DEC`, `.ac OCT` frequency
grids, and `.tran` `START` / print-step `TSTEP` / `MAXSTEP` / `UIC` controls.

## Controlled source examples

### VCVS — unity-gain buffer

```python
from spice_engine import Circuit, VoltageSource, VCVS, Resistor, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 5.0),
    VCVS("E1", "out", "0", ctrl_plus="in", ctrl_minus="0", gain=1.0),
    Resistor("Rload", "out", "0", 1000.0),
])
r = dc_op(c)
print(r.node_voltages["out"])   # 5.0 V — perfect buffer
```

### VCCS — transconductance amplifier

```python
from spice_engine import Circuit, VoltageSource, VCCS, Resistor, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 1.0),
    VCCS("G1", "out", "0", ctrl_plus="in", ctrl_minus="0", gm=0.01),
    Resistor("Rout", "out", "0", 1000.0),
])
r = dc_op(c)
print(r.node_voltages["out"])   # 10.0 V  (gm * Vin * Rout = 0.01 * 1 * 1000)
```

### CCCS — current mirror

```python
from spice_engine import Circuit, VoltageSource, Resistor, CCCS, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 1.0),
    Resistor("Rin", "in", "mid", 1000.0),
    VoltageSource("Vsense", "mid", "0", 0.0),   # 0 V ammeter
    CCCS("F1", "out", "0", ctrl_source="Vsense", beta=2.0),
    Resistor("Rload", "out", "0", 500.0),
])
r = dc_op(c)
# I_ctrl = 1V/1kΩ = 1mA; I_out = 2 * 1mA = 2mA; V_out = 2mA * 500Ω = 1V
print(r.node_voltages["out"])   # 1.0 V
```

### CCVS — transresistance amplifier

```python
from spice_engine import Circuit, VoltageSource, Resistor, CCVS, dc_op

c = Circuit([
    VoltageSource("Vin", "in", "0", 1.0),
    Resistor("Rin", "in", "mid", 1000.0),
    VoltageSource("Vsense", "mid", "0", 0.0),
    CCVS("H1", "out", "0", ctrl_source="Vsense", transresistance=500.0),
    Resistor("Rload", "out", "0", 100.0),
])
r = dc_op(c)
# V_out = rm * I_ctrl = 500 * 1mA = 0.5V
print(r.node_voltages["out"])   # 0.5 V
```

## Node conventions

- Ground is any of `"0"`, `"gnd"`, or `"GND"`.
- CCCS and CCVS use a `VoltageSource` named `ctrl_source` as an ideal ammeter
  (set its voltage to `0.0`).
- CCCS node convention: `F1 n+ n-` → positive current exits `n_plus` into the
  external circuit (same as SPICE F element).

MIT.
