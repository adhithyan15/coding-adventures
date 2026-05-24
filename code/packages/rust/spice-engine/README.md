# spice-engine

`spice-engine` provides SPICE-style circuit analysis primitives for Rust.

The initial slices implement:

- DC operating-point analysis for linear circuits using modified nodal analysis
  (MNA).
- DC source sweeps over independent voltage and current sources.
- DC sensitivity analysis for output-node voltage changes with respect to
  resistor and independent source parameters.
- Seeded DC Monte Carlo analysis for linear element tolerances with Gaussian
  and uniform distributions.
- AC noise analysis with resistor thermal-noise contributions and
  input-referred PSD.
- DC small-signal transfer-function (`.tf`) analysis with input and output
  impedance estimates.
- AC small-signal frequency sweeps for linear RC/RL circuits and explicit AC
  source phasors with DC-bias operating-point linearization for nonlinear
  devices.
- Backward-Euler, trapezoidal, Gear-2, and adaptive transient analysis for
  linear RC/RL circuits.
- Time-varying transient source waveforms: PWL, SIN, PULSE, and EXP.
- Fourier post-processing for transient output, including DC, harmonic
  magnitude/phase, and THD results.
- Stable text output tables for selected node voltages and branch currents.

The package supports resistors, capacitors, inductors, diodes, BJTs,
independent current sources, independent voltage sources, voltage-controlled
current sources, optional AC source phasors, optional source waveforms, ground
aliases, node voltages, and voltage source branch currents.

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
