# Changelog

## Unreleased

- Add transient analysis stamping for `MutualInductor` by coupling referenced
  inductor pairs through a two-winding companion conductance matrix.
- Add AC analysis stamping for `MutualInductor` by coupling referenced
  inductor pairs through the inverted two-winding inductance matrix.
- Add a public `MutualInductor` element as the parser-facing SPICE `K` card
  foothold.
- Add JFET nonlinear DC operating-point stamping and AC small-signal analysis
  from the solved DC bias point.
- Add a public `Jfet` element and `JfetPolarity` as the parser-facing
  three-terminal SPICE `J` card foothold; nonlinear analysis stamping follows
  in a later compatibility slice.
- Add `pss`, which runs the bounded shooting-Newton solve and returns one
  steady-state transient period from the solved circuit.
- Add `pss_newton_solve`, which runs bounded accepted Newton iterations until
  residual convergence, no improvement, or the iteration cap.
- Add `pss_newton_iteration`, which runs one candidate update, accepts it only
  when the residual L2 norm does not increase, and reports the retained
  circuit/state for the next shooting step.
- Add `pss_newton_candidate`, which applies one least-squares Newton update to
  reactive initial conditions and reports the candidate circuit plus its
  refreshed one-period residual.
- Add `pss_newton_update`, a least-squares Newton correction helper from the
  finite-difference residual Jacobian to reactive initial-condition updates.
- Add `pss_residual_jacobian`, a forward finite-difference Jacobian from
  reactive initial conditions to the ordered PSS residual vector for future
  shooting-Newton updates.
- Add L2 and RMS norms over the ordered PSS residual vector for future
  shooting-Newton convergence checks.
- Add a stable node-then-branch residual vector to `PssResidualResult` as the
  next state-vector foothold for shooting-Newton PSS solves.
- Add branch-current closure residuals to `PssResidualResult` alongside
  node-voltage residuals.
- Add tolerance-aware PSS residual convergence reporting through
  `pss_residual_with_tolerance`, including `residual_tolerance` and
  `within_tolerance` on `PssResidualResult`.
- Add PSS period-closure residual reporting with `pss_residual` and
  `PssResidualResult`, which runs one estimated source period and returns
  node-voltage closure residuals as the next foothold for shooting-Newton
  periodic steady-state analysis.
- Add PSS source-period estimation with `Waveform::period_seconds`,
  `estimate_period`, and `estimate_period_with_tolerance` for deriving a
  harmonic common independent-source period.
- Add multi-corner transfer-function analysis with `tf_corners`, returning the
  same `.TF` query evaluated under each named corner.
- Add multi-corner AC frequency sweeps with `ac_sweep_corners`, returning the
  same frequency grid evaluated under each named corner.
- Add multi-corner DC source sweeps with `dc_sweep_corners`, returning the same
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
- Add DC operating-point convergence metadata and configurable Newton controls,
  with nonlinear Gmin/source stepping fallback aids for difficult bias points.

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
- Add AC small-signal frequency sweeps for linear RC/RL circuits, explicit AC
  source phasors, and DC-bias operating-point linearization for nonlinear
  devices when AC source specs are present.
- Add backward-Euler transient analysis for linear RC circuits.
- Add ideal-short DC and backward-Euler transient support for inductors.
- Add transient source waveforms for independent voltage and current sources:
  PWL, SIN, PULSE, and EXP.
