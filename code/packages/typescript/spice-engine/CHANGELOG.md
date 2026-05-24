# Changelog

## Unreleased

- Add `diodeAtTemperature` and `circuitAtTemperature` helpers, which adjust
  diode thermal voltage and saturation current for an operating temperature
  using a SPICE-style silicon energy-gap foothold.
- Add `bjtAtTemperature` and extend `circuitAtTemperature` to adjust BJT
  thermal voltage and saturation current with the same silicon energy-gap
  foothold.
- Add `mosfetAtTemperature` and extend `circuitAtTemperature` to adjust
  Level-1 MOSFET threshold voltage, transconductance parameter, and nominal
  temperature.
- Add `formatDcTable` and `formatTransientTable` for stable tab-separated
  node-voltage and branch-current text output snapshots.
- Add `fourier`, which computes SPICE-style DC, harmonic sine/cosine
  coefficients, magnitudes, phases, and THD from transient samples for
  `V(node)` and `I(source)` probes.
- Add `distortionFromTransient`, which runs the Fourier extraction path and
  returns the Phase-8 distortion result shape directly from transient samples.
- Add MOS Level-1 capacitance support through `CGSO`, `CGDO`, `CGBO`, `CBS`,
  and `CBD`, contributing small-signal AC susceptance.
- Add MOSFET channel thermal noise to `.NOISE` via the long-channel `4kTγgm`
  model and per-element `M` device contributions.
- Add diode emission coefficient support through `emissionCoefficient`, scaling
  the effective thermal voltage in DC and small-signal diode conductance.
- Add diode breakdown support through `breakdownVoltage` / `breakdownCurrent`,
  adding a bounded reverse-breakdown current and conductance foothold.
- Add diode junction capacitance support through `junctionCapacitance`,
  contributing small-signal AC susceptance in parallel with the linearized
  diode conductance.
- Add diode transit-time support through `transitTime`, contributing
  forward-bias diffusion capacitance to small-signal AC admittance.
- Add BJT capacitance support through `baseEmitterCapacitance` /
  `baseCollectorCapacitance`, contributing small-signal AC susceptance.
- Add BJT transit-time support through `forwardTransitTime`, contributing
  forward-bias diffusion capacitance to small-signal AC admittance.
- Add BJT reverse transit-time support through `reverseTransitTime`,
  contributing base-collector diffusion capacitance to small-signal AC
  admittance.
- Add pseudo-transient DC continuation as a final bounded convergence aid after
  Newton, Gmin stepping, and source stepping; successful fallback results
  report `convergenceAid: "pseudo_transient"`.
- Add `DcResult.convergenceAid`, reporting whether the DC operating point came
  from plain Newton, Gmin stepping, source stepping, or no successful
  convergence aid.
- Add `transientAdaptive`, an LTE-controlled transient surface with bounded
  step growth/shrinkage and `euler` / `trap` / `gear2` method routing.
- Add trapezoidal transient integration parity for capacitors and inductors,
  enabling LC damping comparisons against Gear-2.
- Add Gear-2 transient integration with BDF2 capacitor/inductor companion
  histories after bootstrapping with one backward-Euler step.
- Add transient analysis stamping for `TransmissionLine` using a lossless
  Bergeron delay-line companion model, including matched-load delayed step
  behavior.
- Add AC analysis stamping for `TransmissionLine` using the lossless two-port
  admittance matrix, including matched-load phase-delay behavior.
- Add a public `TransmissionLine` element and `transmissionLine` factory as
  the parser-facing SPICE `T` card foothold for future AC/transient delay-line
  stamping.
- Add transient analysis stamping for `MutualInductor` by coupling referenced
  inductor pairs through a two-winding companion conductance matrix.
- Add AC analysis stamping for `MutualInductor` by coupling referenced
  inductor pairs through the inverted two-winding inductance matrix.
- Add a public `MutualInductor` element and `mutualInductor` factory as the
  parser-facing SPICE `K` card foothold.
- Add JFET nonlinear DC operating-point stamping and AC small-signal analysis
  from the solved DC bias point.
- Add a public `Jfet` element and `jfet` factory as the parser-facing
  three-terminal SPICE `J` card foothold; nonlinear analysis stamping follows
  in a later compatibility slice.
- Add `pss`, which runs the bounded shooting-Newton solve and returns one
  steady-state transient period from the solved circuit.
- Add `pssNewtonSolve`, which runs bounded accepted Newton iterations until
  residual convergence, no improvement, or the iteration cap.
- Add `pssNewtonIteration`, which runs one candidate update, accepts it only
  when the residual L2 norm does not increase, and reports the retained
  circuit/state for the next shooting step.
- Add `pssNewtonCandidate`, which applies one least-squares Newton update to
  reactive initial conditions and reports the candidate circuit plus its
  refreshed one-period residual.
- Add `pssNewtonUpdate`, a least-squares Newton correction helper from the
  finite-difference residual Jacobian to reactive initial-condition updates.
- Add `pssResidualJacobian`, a forward finite-difference Jacobian from
  reactive initial conditions to the ordered PSS residual vector for future
  shooting-Newton updates.
- Add L2 and RMS norms over the ordered PSS residual vector for future
  shooting-Newton convergence checks.
- Add a stable node-then-branch residual vector to `pssResidual` as the next
  state-vector foothold for shooting-Newton PSS solves.
- Add branch-current closure residuals to `pssResidual` results alongside
  node-voltage residuals.
- Add tolerance-aware PSS residual convergence reporting through
  `pssResidual`, including `residualTolerance` and `withinTolerance`.
- Add PSS period-closure residual reporting with `pssResidual`, which runs one
  estimated source period and returns node-voltage closure residuals as the
  next foothold for shooting-Newton periodic steady-state analysis.
- Add PSS source-period estimation with `waveformPeriod` for periodic `SIN` and
  `PULSE` source forms plus `estimatePeriod` for deriving a harmonic common
  independent-source period.
- Add multi-corner transfer-function analysis with `tfCorners`, returning the
  same `.TF` query evaluated under each named corner.
- Add multi-corner AC frequency sweeps with `acSweepCorners`, returning the
  same frequency grid evaluated under each named corner.
- Add multi-corner DC source sweeps with `dcSweepCorners`, returning the same
  source-value trace evaluated under each named corner.
- Add multi-corner DC operating point sweeps with named corner specs and
  element-parameter overrides for core linear parameters.
- Add two-port S-parameter extraction from named AC voltage-source ports,
  returning S11/S21/S12/S22 for a configurable reference impedance.
- Add a sparse-row real linear solver path for large DC / real small-signal
  matrices while keeping the dense solver for small systems.
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
