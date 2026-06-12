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
`.include`, `.lib`, and `.control` directives before future include/library
resolution and control-block execution are in scope.
`resolve_deck_sources` is the first include/library resolution layer: callers
provide a source-content map, `.include` directives are expanded in place, and
`.lib path section` selects a named `.lib` / `.endl` section with stable
diagnostics for missing files, missing sections, unterminated sections, cycles,
and still-unsupported `.control` blocks.
