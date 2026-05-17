# Changelog

## Unreleased

- Add programmatic subcircuits with `SubcircuitDefinition` and `XInstance`
  expansion into namespaced primitive elements before simulation.
- Add behavioral B sources for DC current and voltage expressions over
  constants and `V(node)` / `V(node1,node2)` node-voltage references.
- Add independent-source AC phasors with separate DC bias for AC analysis.
  Voltage and current sources can now carry an explicit AC magnitude and
  phase; once any explicit AC source is present, other independent sources are
  treated as AC-zero bias sources.
- Add DC operating-point convergence metadata and opt-in controls, with
  nonlinear Gmin/source stepping fallback aids for difficult bias points.
- Add Level-1 NMOS/PMOS MOSFET elements with Newton-linearized DC operating
  point support and zero-bias small-signal AC/transfer participation.
- Add Ebers-Moll-style BJT elements with Newton-linearized DC operating-point
  support and zero-bias small-signal conductance/transconductance for AC and
  transfer analysis.
- Add Shockley diode elements with Newton-linearized DC operating-point support
  and zero-bias small-signal conductance for AC/transfer analysis.
- Add current-controlled voltage source support across DC, AC, transfer
  function, sensitivity, Monte Carlo, and transient analyses.
- Add current-controlled current source support across DC, AC, transfer
  function, sensitivity, Monte Carlo, and transient analyses.
- Add voltage-controlled voltage source support across DC, AC, transfer
  function, sensitivity, Monte Carlo, and transient analyses.
- Add AC noise analysis with resistor Johnson-Nyquist source PSDs,
  adjoint output contributions, input-referred PSD, and default log sweeps.
- Add seeded DC Monte Carlo analysis for linear element parameters with
  Gaussian and uniform tolerance distributions.
- Add PWL, SIN, PULSE, and EXP source waveforms for transient voltage and
  current sources while preserving static source values for DC, AC, transfer
  function, sensitivity, and sweep analyses.
- Add voltage-controlled current source support across DC, AC, transfer
  function, and sensitivity analyses.
- Add DC sensitivity analysis for resistor and independent source parameters.
- Add DC small-signal transfer-function analysis with input/output impedance.
- Add AC small-signal frequency sweeps for linear RC/RL circuits.
- Add DC source sweeps for independent voltage and current sources.
- Add capacitor support and backward-Euler RC transient analysis.
- Add ideal-short DC and backward-Euler transient support for inductors.

## 0.1.0

- Add a DC modified nodal analysis solver for resistors, independent voltage
  sources, and independent current sources.
