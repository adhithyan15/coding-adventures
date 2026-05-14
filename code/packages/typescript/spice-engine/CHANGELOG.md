# Changelog

## Unreleased

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
