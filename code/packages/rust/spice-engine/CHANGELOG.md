# Changelog

## 0.1.0

- Add a DC modified nodal analysis solver for resistors, independent voltage
  sources, and independent current sources.
- Add Shockley diode elements with Newton-linearized DC operating-point support
  and zero-bias small-signal conductance for AC/transfer analysis.
- Add BJT elements with NPN/PNP polarity, Newton-linearized DC operating-point
  support, and zero-bias small-signal transconductance for AC/transfer
  analysis.
- Add Level-1 NMOS/PMOS MOSFET elements with body-effect parameters,
  Newton-linearized DC operating-point support, and zero-bias small-signal
  conductance/transconductance for AC/transfer analysis.
- Add voltage-controlled current sources (VCCS) for linear transconductance
  stages.
- Add voltage-controlled voltage sources (VCVS) across DC, AC, transfer
  function, sensitivity, Monte Carlo, and transient analyses.
- Add current-controlled current sources (CCCS) across DC, AC, transfer
  function, sensitivity, Monte Carlo, and transient analyses.
- Add current-controlled voltage sources (CCVS) across DC, AC, transfer
  function, sensitivity, Monte Carlo, and transient analyses.
- Add DC source sweeps for independent voltage and current sources.
- Add DC sensitivity analysis for resistor and independent source parameters.
- Add seeded DC Monte Carlo analysis for linear element parameters with
  Gaussian and uniform tolerance distributions.
- Add AC noise analysis with resistor Johnson-Nyquist source PSDs, adjoint
  output contributions, input-referred PSD, and default log sweeps.
- Add DC small-signal transfer-function analysis with input/output impedance.
- Add AC small-signal frequency sweeps for linear RC/RL circuits.
- Add backward-Euler transient analysis for linear RC circuits.
- Add ideal-short DC and backward-Euler transient support for inductors.
- Add transient source waveforms for independent voltage and current sources:
  PWL, SIN, PULSE, and EXP.
