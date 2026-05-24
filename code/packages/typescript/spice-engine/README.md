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
stable text tables for selected node voltages and branch currents,
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

`diodeAtTemperature`, `bjtAtTemperature`, and `circuitAtTemperature` provide
operating-temperature footholds for diode and BJT models by scaling thermal
voltage and saturation current before running an analysis.
