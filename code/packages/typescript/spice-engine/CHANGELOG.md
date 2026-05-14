# Changelog

## Unreleased

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
