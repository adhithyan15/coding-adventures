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
- DC small-signal transfer-function (`.tf`) analysis with input and output
  impedance estimates.
- AC small-signal frequency sweeps for linear RC/RL circuits.
- Backward-Euler transient analysis for linear RC/RL circuits.
- Time-varying transient source waveforms: PWL, SIN, PULSE, and EXP.

The package supports resistors, capacitors, inductors, independent current sources,
independent voltage sources, voltage-controlled current sources, optional source
waveforms, ground aliases, node voltages, and voltage source branch currents.

```rust
use spice_engine::{Circuit, Element, PwlWaveform, Resistor, VoltageSource, Waveform};

let mut circuit = Circuit::new();
circuit.add(Element::VoltageSource(VoltageSource::with_waveform(
    "Vin",
    "in",
    "0",
    0.0,
    Waveform::Pwl(PwlWaveform::new(vec![(0.0, 0.0), (1.0e-9, 1.8)])),
)));
circuit.add(Element::Resistor(Resistor::new("Rload", "in", "0", 1_000.0)));
```
