# Changelog

## Unreleased

- Add normalized output-probe names to selected-run artifacts in
  `runDeckAnalysis` and render them in a stable `OutputProbeList` column from
  `formatDeckRunArtifactTable`, matching Python and Rust.
- Emit explicit policy diagnostics for selected `.control` block
  variable/state mutation commands, including `let`, `alter`, `alterparam`,
  `set`, and `unset`, in `analyzeDeckControls` and `resolveDeckSources`,
  matching Python and Rust. Accepted no-op `set` options still route as no-op
  markers.
- Emit explicit policy diagnostics for selected `.control` block control-flow
  commands, including `if`, `while`, `foreach`, and `repeat`, in
  `analyzeDeckControls` and `resolveDeckSources`, matching Python and Rust.
  Control-flow execution remains disabled by the deck execution policy.
- Emit explicit policy diagnostics for selected `.control` block `cd`
  working-directory mutation commands in `analyzeDeckControls` and
  `resolveDeckSources`, matching Python and Rust. Working-directory mutation
  remains disabled by the deck execution policy.
- Emit explicit policy diagnostics for selected `.control` block `source` and
  `shell` external script/shell commands in `analyzeDeckControls` and
  `resolveDeckSources`, matching Python and Rust. External script execution
  and shelling out remain disabled by the deck execution policy.
- Accept selected `.control` block read-only `echo`, `rusage`, and `where`
  console/debug commands as no-op control commands in `analyzeDeckControls`
  and `resolveDeckSources`, matching Python and Rust. Actual console/debug
  output remains out of scope for these markers.
- Accept selected `.control` block read-only `status`, `version`, and `help`
  UI introspection commands as no-op control commands in `analyzeDeckControls`
  and `resolveDeckSources`, matching Python and Rust. Actual console/help
  output remains out of scope for these markers.
- Accept selected `.control` block read-only `show` and `showmod`
  device/model inspection commands as no-op control commands in
  `analyzeDeckControls` and `resolveDeckSources`, matching Python and Rust.
  Actual console/model inspection output remains out of scope for these
  markers.
- Accept selected `.control` block read-only `display` and `listing`
  inspection commands as no-op control commands in `analyzeDeckControls` and
  `resolveDeckSources`, matching Python and Rust. Actual console/listing
  output remains out of scope for these markers.
- Accept selected `.control` block `wrdata <file> <probes...>` ASCII
  data-write markers as no-op control commands in `analyzeDeckControls` and
  `resolveDeckSources`, matching Python and Rust. Actual data-file
  serialization remains out of scope for this marker.
- Accept selected `.control` block `write <rawfile> [probes...]` rawfile-write
  markers as no-op control commands in `analyzeDeckControls` and
  `resolveDeckSources`, matching Python and Rust. Rawfile serialization remains
  out of scope for this marker.
- Accept selected `.control` block `set appendwrite` rawfile append-write
  options as no-op control commands in `analyzeDeckControls` and
  `resolveDeckSources`, matching Python and Rust.
- Accept selected `.control` block `set wr_vecnames` and `set wr_singlescale`
  rawfile output toggles as no-op control commands in `analyzeDeckControls`
  and `resolveDeckSources`, matching Python and Rust.
- Accept selected `.control` block `set filetype=ascii` output-format options
  as no-op control commands in `analyzeDeckControls` and `resolveDeckSources`,
  matching Python and Rust.
- Accept selected `.control` block `reset` session-reset markers as no-op
  control commands in `analyzeDeckControls` and `resolveDeckSources`, matching
  Python and Rust.
- Accept selected `.control` block `set noaskquit` UI options as no-op control
  commands in `analyzeDeckControls` and `resolveDeckSources`, matching Python
  and Rust.
- Accept selected `.control` block `quit` interpreter-exit markers as no-op
  control commands in `analyzeDeckControls` and `resolveDeckSources`, matching
  Python and Rust.
- Accept selected `.control` block `run` execution markers as no-op control
  commands in `analyzeDeckControls` and `resolveDeckSources`, matching Python
  and Rust.
- Add selected `.control` block `four` and `fourier` command routing to
  `analyzeDeckControls` and `resolveDeckSources`; the commands are normalized
  into `.four` deck cards, matching Python and Rust.
- Add selected `.control` block `measure` and `meas` command routing to
  `analyzeDeckControls` and `resolveDeckSources`; the commands are normalized
  into `.measure` and `.meas` deck cards, matching Python and Rust.
- Add selected `.control` block `save` and `probe` command routing to
  `analyzeDeckControls` and `resolveDeckSources`; the commands are normalized
  into `.save` and `.probe` deck cards, matching Python and Rust.
- Add selected `.control` block command routing to `analyzeDeckControls` and
  `resolveDeckSources`; analysis/output commands (`op`, `dc`, `ac`, `tran`,
  `save`, `probe`, `print`, and `plot`) are normalized into dotted deck cards,
  matching Python and Rust.
- Add control-block exclusion diagnostics to `analyzeDeckControls` and
  `resolveDeckSources`; unsupported `.control` / `.endc` block markers and
  unrecognized body commands are no longer forwarded as active deck lines and
  emit stable diagnostics, matching Python and Rust.
- Add parsed `.plot <analysis> ...` output routing to `resolveDeckOutputs`,
  `selectDeckOutputProbes`, and deck table formatters, matching Python and
  Rust.
- Add parsed `.print <analysis> ...` output routing to `resolveDeckOutputs`,
  `selectDeckOutputProbes`, and deck table formatters, matching Python and
  Rust.
- Add selected-run artifact summaries to `runDeckAnalysis`; executions now
  return stable result-row, output-probe, measurement, and Fourier counts plus
  a run-artifact table, matching Python and Rust.
- Add selected Fourier artifacts to `runDeckAnalysis`; selected `.tran`
  executions now return parsed `.four` harmonic results and a stable Fourier
  table alongside the selected plan, solver result, output probes, and
  measurement artifacts, matching Python and Rust.
- Add selected measurement artifacts to `runDeckAnalysis`; selected `.dc`,
  `.ac`, and `.tran` executions now return parsed `.measure` / `.meas` results
  and a stable measurement table alongside the selected plan, solver result,
  output probes, and output table, matching Python and Rust.
- Add selected-output probe artifacts to `runDeckAnalysis`; callers now receive
  the normalized deck-selected output probes alongside each selected plan,
  solver result, and stable table, matching Python and Rust.
- Add `.tran` print-step output routing to `runDeckAnalysis`; deck transient
  plans now keep `.tran TSTEP` as the stable output print grid while `MAXSTEP`
  caps internal solver stepping, matching Python and Rust.
- Add `.tran START/MAXSTEP/UIC` selected-plan execution routing to
  `runDeckAnalysis`; deck transient plans now apply `START` output filtering,
  `MAXSTEP` fixed-step caps, and `UIC` initial-condition intent through stable
  deck-selected transient tables, matching Python and Rust.
- Add `.ac LIN` and `.ac OCT` selected-plan execution routing to
  `runDeckAnalysis`; deck AC plans now execute SPICE-style linear,
  points-per-decade, and points-per-octave grids, matching Python and Rust.
- Add `runDeckAnalysis` so callers can select one deck `.op`, `.dc`,
  `.ac DEC`, or `.tran` plan, execute the matching solver, and receive the
  selected plan, solver result, and deck-selected output table, matching
  Python and Rust.
- Add `selectDeckAnalysisPlan` so callers can choose one explicit or implicit
  deck analysis plan with stable ambiguity and invalid-card errors, matching
  Python and Rust.
- Add `resolveDeckAnalyses` so `.op`, `.dc`, `.ac`, and `.tran` analysis
  cards are extracted before `.end` into stable metadata with shared
  diagnostics, matching Python and Rust.
- Add `resolveDeckOutputs`, `selectDeckOutputProbes`, and the
  `formatDeck*Table` helpers so parsed `.save` / `.probe` cards route into
  stable operating-point, DC sweep, AC sweep, and transient tables, matching
  Python and Rust.
- Add `resolveDeckFourier`, `fourierTransientCards`, and
  `fourierTransientDeck` so parsed `.four` / `.FOUR` deck cards can route
  transient samples into SPICE-style Fourier harmonic results with optional
  `HARMONICS=` and `FROM=` controls, matching Python and Rust.
- Add `measureTransientDelayBetweenProbes` and parsed transient
  `.measure ... TRIG ... TARG ...` routing so deck measurements can report
  trigger-to-target delays with counted crossing controls, matching Python and
  Rust.
- Add `measureTransientWhenProbeCounted` and parsed transient
  `.measure ... WHEN probe=target RISE|FALL|CROSS=n` routing so deck
  measurements can report counted threshold occurrences over optional
  `FROM=` / `TO=` windows, matching Python and Rust.
- Add `measureTransientWhenProbe` and parsed transient
  `.measure ... WHEN probe=target` routing so deck measurements can report the
  first crossing time over optional `FROM=` / `TO=` windows, matching Python
  and Rust.
- Add `measureTransientFindAtProbe` and parsed transient
  `.measure ... FIND ... AT=` routing so deck measurements can sample or
  linearly interpolate a probe value at one scalar time, matching Python and
  Rust.
- Add `measureAcSweepProbe`, `measureAcSweepCards`, and
  `measureAcSweepDeck` so parsed `.measure ac` / `.meas ac` cards can route
  AC sweep probe magnitudes into the shared scalar measurement table surface,
  matching Python and Rust.
- Add `measureDcSweepProbe`, `measureDcSweepCards`, and
  `measureDcSweepDeck` so parsed `.measure dc` / `.meas dc` cards can route
  DC sweep probe samples into the shared scalar measurement table surface,
  matching Python and Rust.
- Add `resolveDeckMeasurements`, `measureTransientCards`, and
  `measureTransientDeck` for parsed transient `.measure` / `.meas` card
  routing into stable scalar measurement rows, matching Python and Rust.
- Add `measureTransientProbe` and `formatMeasurementTable` for a shared
  `.MEASURE`-style scalar transient output surface with MAX, MIN, AVG, RMS,
  peak-to-peak, and final-value probe measurements, matching Python and Rust.
- Add `dcInitialVectorFromConditions`, `dcOpWithInitialConditions`, and
  `dcOpWithInitialVector` so parsed `.ic` / `.nodeset` node-voltage hints can
  seed DC operating-point Newton solves as MNA warm-start vectors, with `.ic`
  values taking precedence over `.nodeset`, matching Python and Rust.
- Add scalar `.func` call evaluation to `resolveDeckParameters`: definitions
  are collected before `.end`, calls can appear in `.param` assignments and
  braced or quoted active-line expressions, and unknown functions, bad arity,
  and recursive calls produce stable diagnostics, matching Python and Rust.
- Add `resolveDeckFunctions` for scalar `.func name(args) expression`
  definition extraction before `.end`, braced or quoted expression delimiter
  stripping, and stable diagnostics for malformed signatures, arguments,
  duplicate arguments, and empty expressions, matching Python and Rust.
- Add `resolveDeckInitialConditions` for scalar `.ic` and `.nodeset`
  `V(node)=value` hint extraction before `.end`, numeric SPICE
  suffix/arithmetic expression evaluation, and stable diagnostics for malformed
  targets and unresolved values, matching Python and Rust.
- Add `resolveDeckParameters` for scalar whitespace-tokenized `.param`
  assignment evaluation, braced and quoted active-line expression rewriting,
  and stable diagnostics for unresolved expressions, matching Python and Rust.
- Add `resolveDeckSources` for map-backed `.include` and selected
  `.lib path section` expansion with stable diagnostics for missing sources,
  missing or unterminated library sections, cycles, and still-unsupported
  `.control` blocks, matching Python and Rust.
- Add `analyzeDeckControls` for shared deck-control boundary diagnostics:
  active pre-`.end` lines plus stable unsupported-feature diagnostics for
  `.include`, `.lib`, and `.control`, matching Python and Rust.
- Add `formatDcSweepTable`, `formatCornerDcSweepTable`,
  `formatCornerAcTable`, and `formatCornerTfTable` to close the remaining
  Rust-first `.DC`, `.AC`, and `.TF` named-corner table helper gaps in the
  native web package.
- Add `formatCornerDcTable`, `dcTemperatureSweep`,
  `dcTemperatureSweepCorners`, `formatTemperatureDcTable`, and
  `formatCornerTemperatureDcTable` for Rust-matching named-corner and
  `.temp`-style DC operating-point snapshots with stable table columns in the
  native web package.
- Add `compatibilityCorpus`, `releaseReadinessGates`,
  `formatCompatibilityCorpusTable`, and `formatReleaseReadinessReport` for the
  first oracle-backed compatibility deck corpus with golden tolerances and
  known incompatibility notes shared with Python and Rust.
- Add `CustomModel`, `CustomModelEvaluation`, `customLinearConductanceModel`,
  and `analyzeCustomModelSource` for the first sandbox-friendly two-terminal
  residual/Jacobian custom-model foothold shared with Python and Rust.
- Add `DigitalEventStream`, `DigitalLogicLevels`, `DigitalThresholds`, digital
  stream PWL voltage source conversion, fixed/adaptive digital transient bridge
  runners, named-corner bridge wrappers, stable event/schedule tables, and
  deterministic VCD output for native web mixed-signal SPICE/VM fixtures.
- Add `normalizeModelCard`, typed model-card builders, and
  `deviceModelAuditFixtures` for cross-language diode, BJT, JFET, and Level-1
  MOS `.model` alias compatibility fixtures.
- Add `DcResult.diagnostics` with stable matrix size, solver kind, tolerance,
  convergence aid, and final Newton delta metadata; large AC complex systems
  now route through the sparse-row complex solver path.
- Add `distortionFromTransientCorners`, `poleZeroCorners`,
  `formatCornerDistortionTable`, and `formatCornerPoleZeroTable` for
  named-corner distortion and pole-zero parity in the native web package.
- Add `fourierCorners` and `formatCornerFourierTable` for named-corner
  `.FOUR`-style harmonic analysis parity in the native web package.
- Add `formatPssTable`, `pssCorners`, and `formatCornerPssTable` for stable
  periodic-steady-state output and named-corner PSS parity in the native web
  package.
- Add `transientCorners` and `transientAdaptiveCorners` for named-corner
  fixed-step and LTE-adaptive transient analysis, plus
  `formatCornerTransientTable` and `formatCornerAdaptiveTransientTable` for
  stable tab-separated corner waveform output.
- Add multi-corner advanced analysis wrappers with `mcDcCorners`,
  `sensDcCorners`, `noiseAcCorners`, and `sParametersCorners`, matching the
  Rust engine surface for these SPICE outputs in the native web package.
- Add stable tab-separated text output helpers for Monte Carlo, sensitivity,
  noise, and S-parameter results, including named-corner table variants.

## 0.14.0 — 2026-06-05

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
- Add `formatPoleZeroTable` for stable tab-separated `.PZ` pole-zero text
  output snapshots.
- Add `formatDistortionTable` for stable tab-separated `.DISTO` harmonic
  magnitude, phase, and THD text output snapshots.
- Add `formatFourierTable` for stable tab-separated `.FOUR` harmonic
  coefficient, magnitude, phase, DC, and THD text output snapshots.
- Add `formatAcTable` for stable tab-separated `.AC` real, imaginary,
  magnitude, and phase text output snapshots.
- Add `formatTfTable` for stable tab-separated `.TF` gain and impedance text
  output snapshots.
- Add JFET source-follower transient fixtures covering nonlinear
  companion-model solves.
- Add `fourier`, which computes SPICE-style DC, harmonic sine/cosine
  coefficients, magnitudes, phases, and THD from transient samples for
  `V(node)` and `I(source)` probes.
- Add `distortionFromTransient`, which runs the Fourier extraction path and
  returns the Phase-8 distortion result shape directly from transient samples.
- Add `poleZeroRcHighpass`, which returns the origin zero and RC pole for a
  constrained first-order high-pass fixture.
- Add `poleZeroRlcLowpass`, which returns the second-order pole pair for a
  constrained series R-L / shunt-C low-pass fixture.
- Add `poleZeroRlcHighpass`, which returns the double origin zero plus
  second-order pole pair for a constrained series R-C / shunt-L high-pass
  fixture.
- Add `poleZeroRlcBandpass`, which returns the origin zero plus second-order
  pole pair for a constrained series L-C / shunt-R band-pass fixture.
- Add `poleZeroRlcNotch`, which returns the imaginary-axis zero pair plus
  second-order pole pair for a constrained series-R / shunt-series-L-C notch
  fixture.
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
