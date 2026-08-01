# Changelog

- Reject non-positive and non-finite Level-1 MOS model-card `W` values before
  channel-current, capacitance, and noise calculations.

- Reject negative and non-finite Level-1 MOS model-card `JS` values before
  bulk-junction leakage-density and temperature preprocessing.

- Reject non-positive and non-finite Level-1 MOS model-card `IS` values before
  bulk-junction leakage and temperature preprocessing.

- Reject negative and non-finite Level-1 MOS model-card `CGBO` values before
  gate-bulk overlap-capacitance stamping.

- Reject negative and non-finite Level-1 MOS model-card `CGDO` values before
  gate-drain overlap-capacitance stamping.

- Reject negative and non-finite Level-1 MOS model-card `CGSO` values before
  gate-source overlap-capacitance stamping.

- Reject negative and non-finite Level-1 MOS model-card `CBD` / `CJD` values
  before drain-bulk capacitance shaping.

- Reject negative and non-finite Level-1 MOS model-card `CBS` / `CJS` values
  before source-bulk capacitance shaping.

- Reject negative and non-finite Level-1 MOS model-card `CJSW` values before
  sidewall-junction capacitance shaping.

- Reject negative and non-finite Level-1 MOS model-card `CJ` values before
  bottom-junction capacitance shaping.

- Reject negative or non-finite Level-1 MOS model-card `MJSW` values with a
  stable diagnostic before sidewall depletion-capacitance shaping.
- Reject Level-1 MOS model-card `FC` values outside `[0, 1)` or non-finite
  values with a stable diagnostic before depletion-capacitance shaping.
- Reject negative or non-finite Level-1 MOS model-card `MJ` values with a
  stable diagnostic before depletion-capacitance shaping.
- Reject non-positive or non-finite Level-1 MOS model-card `PB` values with a
  stable diagnostic before depletion and temperature preprocessing.
- Reject negative or non-finite Level-1 MOS model-card `GAMMA` values with a
  stable diagnostic before body-effect and temperature preprocessing.
- Reject non-positive or non-finite Level-1 MOS model-card `PHI` values with a
  stable diagnostic before electrostatic and temperature preprocessing.
- Reject non-finite Level-1 MOS model-card `LAMBDA` / `LAM` values with a
  stable diagnostic before current evaluation.
- Reject non-finite Level-1 MOS model-card `VT0` / `VTO` / `VTH` values with a
  stable diagnostic before threshold preprocessing.
- Reject non-positive or non-finite Level-1 MOS model-card `KP` values with a
  stable diagnostic before temperature scaling and current evaluation.
- Reject negative or non-finite Level-1 MOS model-card `U0` / `UO` values with
  a stable diagnostic before mobility preprocessing.
- Reject non-positive or non-finite Level-1 MOS model-card `TOX` values with a
  stable diagnostic before mobility and electrostatic preprocessing.
- Reject non-positive or non-finite Level-1 MOS model-card `NSUB` values with a
  stable diagnostic before process-parameter preprocessing.
- Reject non-positive or non-finite Level-1 MOS model-card `TNOM` values with a
  stable diagnostic before temperature and electrostatic preprocessing.
- Reject negative or non-finite Level-1 MOS model-card `NSS` surface-state
  densities with a stable diagnostic before threshold preprocessing.
- Accept Berkeley Level-1 MOS model-card `TPG` gate-material selectors and use
  their `-1`, `0`, or `1` work-function branches when deriving `VT0`, while
  preserving explicit threshold-voltage precedence.
- Accept Level-1 MOS model-card `NSS` surface-state density and include its
  oxide-charge flat-band shift when deriving `VT0` from process parameters,
  while preserving explicit threshold-voltage precedence.
- Derive Level-1 MOS `VT0` from explicit model-card `NSUB` plus `TOX` using
  Berkeley MOS1 default gate-work-function and surface-state assumptions,
  preserving explicit `VTO` / `VT0` precedence for NMOS and PMOS.
- Derive Level-1 MOS `PHI` and `GAMMA` from explicit model-card `NSUB` plus
  `TOX`, preserving explicit electrostatic parameters and rejecting substrate
  doping at or below the intrinsic carrier density.
- Prefer a non-default Level-1 MOS model-card `TNOM` over the circuit nominal
  temperature when applying Berkeley temperature preprocessing.
- Scale Level-1 MOS zero-bias bottom-junction `CJ`, source/drain `CBS`/`CBD`,
  and sidewall `CJSW` capacitances with temperature using Berkeley MOS1
  `MJ`/`MJSW` grading paths.
- Apply Berkeley MOS1 temperature preprocessing to the Level-1 MOS
  bulk-junction potential `PB`.
- Apply Berkeley MOS1 electrostatic temperature preprocessing to Level-1 MOS
  `PHI` and polarity-aware `VT0` instead of a fixed threshold-voltage shift.
- Scale Level-1 MOS bulk-junction `IS` and `JS` with temperature using the
  silicon energy-gap law already shared by semiconductor junction models.
- Scale Level-1 MOS `U0` alongside `KP` by `(T / T_NOM)^-3/2` so temperature
  helpers preserve Berkeley surface-mobility state and their nominal ratio.
- Add Level-1 MOS model-card `U0` / `UO` surface mobility and derive `KP` from
  explicit `TOX` when `KP` is omitted, preserving explicit-`KP` precedence.
- Add Level-1 MOS model-card `JS` as bulk-junction saturation-current density,
  scaling source/drain leakage and shot noise by `AS` / `AD` when both areas
  are present and otherwise retaining Berkeley-compatible `IS` fallback.
- Apply Level-1 MOS model-card `IS` to the source-body and drain-body junctions
  in DC, transient, transfer-function, and AC analysis, and emit distinct
  `IBS` / `IBD` shot-noise sources.
- Add Level-1 MOS model-card `MJSW`, defaulting to `0.33`, to shape `CJSW`
  sidewall depletion independently from `MJ` bottom-junction capacitance.
- Add the Level-1 MOS `PS` instance parameter. With model-card `CJSW`, its
  product augments `CBS` in AC and transient source-body capacitance.
- Add the Level-1 MOS `PD` instance parameter and model-card `CJSW` density.
  Their product augments `CBD` in AC and transient drain-body capacitance.
- Add the Level-1 MOS `AS` instance parameter. With model-card `CJ`, its
  product augments `CBS` in AC and transient source-body capacitance.
- Add the Level-1 MOS `AD` instance parameter and model-card `CJ` density.
  Their product augments `CBD` in AC and transient drain-body capacitance.
- Add the Level-1 MOS `NRS` instance parameter. It defaults to one and scales
  the `RSH` source fallback while preserving explicit positive `RS` precedence,
  intrinsic-node stamping, and `:RS` Johnson noise.
- Add the Level-1 MOS `NRD` instance parameter. It defaults to one and scales
  the `RSH` drain fallback while preserving explicit positive `RD` precedence,
  intrinsic-node stamping, and `:RD` Johnson noise.
- Add Level-1 MOS model-card `RSH` support. A positive sheet resistance supplies
  the default one-square drain/source terminal resistances when `RD` / `RS`
  remain zero, reusing intrinsic-node stamping and terminal Johnson noise.
- Add Level-1 MOS model-card `RS` support. Nonzero values create an intrinsic
  source node, stamp the external resistor in DC, TF, AC, and transient
  analyses, route source-side capacitances through it, and emit `:RS` Johnson
  noise.
- Add Level-1 MOS model-card `RD` support. Nonzero values create an intrinsic
  drain node, stamp the external resistor in DC, TF, AC, and transient
  analyses, route drain-side capacitances through it, and emit `:RD` Johnson
  noise.
- Add Level-1 MOS model-card `TOX` support, defaulting to `100 nm` and deriving
  intrinsic Meyer gate capacitance from silicon-dioxide permittivity and oxide
  thickness.
- Add Level-1 MOS model-card `LD` support, using `L - 2*LD` for channel
  current and length-scaled intrinsic/`CGBO` capacitance.
- Add Level-1 MOS model-card `FC` support, applying the continuous Berkeley
  forward-bias continuation to `CBS` / `CBD` in AC and transient analysis.
- Add Level-1 MOS model-card `AF` support and apply the configurable
  drain-current exponent to MOS flicker-noise power spectral density.
- Add Level-1 MOS model-card `KF` support and emit drain-current-scaled
  inverse-frequency flicker noise alongside channel thermal noise.
- Add JFET model-card `NLEV` and `GDSNOI` support, preserving the legacy
  channel thermal-noise path below level 3 and adding the Berkeley
  linear-region equation at level 3 and above.
- Add JFET model-card `B` support with Parker-Skellern doping-tail shaping in
  linear and saturation operation, preserving Shichman-Hodges behavior at the
  Berkeley default of 1.
- Add JFET model-card `EG` support, replacing the fixed 1.11 eV silicon
  bandgap in gate saturation-current temperature scaling while preserving that
  value as the default.
- Add JFET model-card `XTI` support, scaling gate saturation current from
  `TNOM` with the standard silicon bandgap temperature law and a default
  exponent of 3.
- Add JFET model-card `VTOTC` support, applying the alternative
  `VTO(T) = VTO + VTOTC * (T - TNOM)` rule with precedence over `TCV` when
  explicitly present.
- Add JFET model-card `BETATCE` support, applying the alternative
  `BETA(T) = BETA * 1.01^(BETATCE * (T - TNOM))` rule with precedence over
  `BEX` when explicitly present.
- Add JFET model-card `BEX` support, applying
  `BETA(T) = BETA * (T / TNOM)^BEX` with a temperature-invariant zero default.
- Add JFET model-card `TNOM` / `T_NOM` and `TCV` support, converting nominal
  Celsius values to Kelvin and applying Berkeley threshold-voltage temperature
  scaling while preserving invariant behavior when `TCV` is omitted.
- Add JFET model-card `RS` source-resistance support with an intrinsic source
  node across DC, transient, AC, transfer-function, and noise analysis, plus
  hierarchy preservation, validation, and a distinct thermal-noise source.
- Add JFET model-card `RD` drain-resistance support with an intrinsic drain
  node across DC, transient, AC, transfer-function, and noise analysis, plus
  hierarchy preservation, validation, and a distinct thermal-noise source.
- Add JFET model-card `IS` gate-junction saturation-current support,
  defaulting to `1e-14`, stamping `IGS`/`IGD` leakage in DC, transient, AC,
  and transfer-function analyses, and emitting distinct shot-noise sources.
- Add JFET model-card `FC` forward-bias depletion coefficient support,
  defaulting to `0.5` and shaping `CGS`/`CGD` depletion capacitance in AC and
  transient analysis with hierarchy preservation and range validation.
- Add JFET model-card `PB` gate-junction-potential support with `VJ` as an
  alias, bias-dependent `CGS`/`CGD` depletion capacitance in AC and transient
  analysis, hierarchy preservation, and finite-positive validation.
- Add JFET model-card `AF` flicker-noise current-exponent support, defaulting
  to `1` and applying `KF * abs(Id)^AF / frequency` across noise analysis.
- Add JFET model-card `KF` flicker-noise support with a distinct drain-current
  `flicker` contribution and Berkeley-default inverse-frequency scaling.
- Add diode model-card `AF` flicker-noise current-exponent support, defaulting
  to `1` and applying `KF * abs(Id)^AF / frequency` across noise analysis.
- Add diode model-card `KF` flicker-noise support with a distinct diode-current
  `flicker` contribution and Berkeley-default inverse-frequency scaling.
- Add legacy SPICE2 BJT model-card `C2` and `C4` leakage-ratio support,
  deriving `ISE` and `ISC` from `IS` when explicit leakage currents are absent.
- Add BJT model-card `XCJC` support to partition base-collector depletion
  capacitance between intrinsic and external base nodes in AC and transient
  analysis.
- Add BJT model-card `RBM` and `IRB` support for Berkeley bias-dependent base
  resistance across DC, AC, transient, transfer-function, and noise analysis.
- Add BJT model-card `RB` base-resistance support with an intrinsic base node
  in DC, AC, and transient analysis plus thermal noise in noise analysis.
- Add BJT model-card `RC` collector-resistance support with an intrinsic
  collector node in DC, AC, and transient analysis plus thermal noise in noise
  analysis.
- Add BJT model-card `RE` emitter-resistance support with an intrinsic emitter
  node in DC, AC, and transient analysis plus thermal noise in noise analysis.
- Add BJT model-card `VTF` forward transit-time voltage-scale support, applying
  `exp(Vbc / (1.44 * VTF))` to the `XTF` AC and transient storage enhancement.

- Add BJT model-card `ITF` forward transit-time current-scale support, applying
  `(If / (If + ITF))^2` to the `XTF` AC and transient storage enhancement.
- Add BJT model-card `XTF` forward transit-time bias-coefficient support,
  scaling forward diffusion capacitance and transient stored charge by
  `TF * (1 + XTF)` with the Berkeley default of zero.
- Add BJT model-card `PTF` forward excess-phase support, rotating AC forward
  transconductance by the configured phase at `1 / (2*pi*TF)`.
- Add BJT model-card `AF` flicker-noise exponent support, defaulting to `1`
  and applying `KF * abs(Ib)^AF / frequency` across noise analysis.
- Add BJT model-card `KF` flicker-noise support with a distinct `flicker`
  contribution and Berkeley-default inverse-frequency scaling.
- Add BJT model-card `TNOM` / `T_NOM` nominal-temperature support, with
  Berkeley Celsius card values converted to Kelvin for model-owned temperature
  scaling and inherited circuit defaults when absent.

## Unreleased

- Add BJT model-card `IKR` reverse high-current beta roll-off support in DC,
  transient, AC, transfer-function, temperature, and noise paths, matching
  Rust and Python. The supported-parameter catalog now contains 110 canonical
  rows.
- Add BJT model-card `BR`/`BETA_R` reverse-current-gain support in DC,
  transient, AC, transfer-function, temperature, and noise paths, matching
  Rust and Python. `XTB` now scales both forward and reverse beta. The
  supported-parameter catalog now contains 108 canonical rows.
- Add BJT model-card `XTB` forward-beta temperature-exponent support, scaling
  forward beta by the analysis-to-nominal absolute temperature ratio, matching
  Rust and Python. The supported-parameter catalog now contains 106 canonical
  rows.
- Add BJT model-card `ISC`/`NC` base-collector leakage support in DC,
  transient, AC, transfer-function, temperature, and noise paths, matching
  Rust and Python. The supported-parameter catalog now contains 104 canonical
  rows.
- Add BJT model-card `ISE`/`NE` base-emitter leakage support in DC, transient,
  AC, transfer-function, temperature, and noise paths, matching Rust and
  Python. The supported-parameter catalog now contains 100 canonical rows.
- Add BJT model-card `IKF`/`IK` forward high-current beta roll-off support with
  shared base-charge modulation in DC, transient, AC, transfer-function, and
  noise paths, matching Python and Rust. The supported-parameter catalog now
  contains 96 canonical rows.
- Add BJT model-card `VAR`/`VB` reverse Early-voltage support with base-charge
  modulation in DC, transient, AC, transfer-function, and noise paths, matching
  Python and Rust. The supported-parameter catalog now contains 94 canonical
  rows.
- Add BJT model-card `FC` forward-bias depletion-coefficient support for the
  shared `CJE` and `CJC` Berkeley continuation law in AC and transient analysis,
  matching Python and Rust. The supported-parameter catalog now contains 92
  canonical rows.
- Add BJT model-card `VJC`/`PC` base-collector junction-potential and `MJC`/`MC`
  grading-coefficient support with bias-shaped `CJC` depletion capacitance in
  AC and transient analysis, matching Python and Rust. The supported-parameter
  catalog now contains 90 canonical rows.
- Add BJT model-card `VJE`/`PE` base-emitter junction-potential and `MJE`/`ME`
  grading-coefficient support with bias-shaped `CJE` depletion capacitance in
  AC and transient analysis, matching Python and Rust. The supported-parameter
  catalog now contains 86 canonical rows.
- Add BJT model-card `NR` reverse emission-coefficient support to reverse
  base-collector diffusion charge in AC and transient analysis, matching Python
  and Rust. The supported-parameter catalog now contains 82 canonical rows.
- Add BJT model-card `NF` forward emission-coefficient support across DC,
  transient charge, AC, transfer-function, and noise paths, matching Python and
  Rust. The supported-parameter catalog now contains 80 canonical rows.
- Add BJT model-card `VAF`/`VA` forward Early-voltage support with
  collector-voltage modulation in DC, transient, AC, transfer-function, and
  noise paths, matching Python and Rust. The supported-parameter catalog now
  contains 78 canonical rows.
- Add BJT model-card `EG` energy-gap support to model-specific temperature
  scaling, preserving it through subcircuit expansion and matching Python and
  Rust. The supported-parameter catalog now contains 76 canonical rows.
- Add BJT model-card `XTI` saturation-current temperature-exponent support,
  preserving it through subcircuit expansion and matching Python and Rust. The
  supported-parameter catalog now contains 74 canonical rows.
- Add diode model-card `EG` energy-gap support to temperature scaling,
  preserving it through subcircuit expansion and matching Python and Rust.
- Add diode model-card `XTI` saturation-current temperature-exponent support,
  preserving it through subcircuit expansion and matching Python and Rust.
- Add diode model-card `FC` forward-bias depletion coefficient support and a
  continuous piecewise depletion-capacitance law, matching Python and Rust.
- Shape diode depletion capacitance from the operating-point bias with model-card
  `VJ`/`PB` junction-potential and `M`/`MJ` grading-coefficient parameters in AC
  and transient analysis, matching Python and Rust.
- Add `modelCardSupportedParameterCoverageDashboard`,
  `formatModelCardSupportedParameterCoverageDashboardTable`,
  `modelCardSupportedParameterCoverageDashboardRecords`,
  `formatModelCardSupportedParameterCoverageDashboardCsv`, and
  `formatModelCardSupportedParameterCoverageDashboardJson`, stable per-kind
  supported-parameter coverage dashboard rows with actual versus expected
  counts plus gate issue fields, matching Python and Rust.
- Add `modelCardSupportedParameterCoverageGate`,
  `formatModelCardSupportedParameterCoverageGateReport`,
  `formatModelCardSupportedParameterCoverageGateIssueTable`,
  `modelCardSupportedParameterCoverageGateIssueRecords`,
  `formatModelCardSupportedParameterCoverageGateIssueCsv`, and
  `formatModelCardSupportedParameterCoverageGateIssueJson`, stable release-gate
  checks and issue exports for the seven-kind, 72-row supported model-card
  parameter catalog, matching Python and Rust.
- Add `modelCardSupportedParameterCoverageSummary`,
  `formatModelCardSupportedParameterCoverageSummaryTable`,
  `modelCardSupportedParameterCoverageSummaryRecords`,
  `formatModelCardSupportedParameterCoverageSummaryCsv`, and
  `formatModelCardSupportedParameterCoverageSummaryJson`, stable
  per-model-kind summaries of supported model-card parameter alias coverage,
  matching Python and Rust.
- Add `modelCardSupportedParameterCoverage`,
  `formatModelCardSupportedParameterCoverageTable`,
  `modelCardSupportedParameterCoverageRecords`,
  `formatModelCardSupportedParameterCoverageCsv`, and
  `formatModelCardSupportedParameterCoverageJson`, stable supported
  model-card parameter and alias catalog exports, matching Python and Rust.
- Add `modelCardUnsupportedParameterIssues`,
  `formatModelCardUnsupportedParameterIssueTable`,
  `modelCardUnsupportedParameterIssueRecords`,
  `formatModelCardUnsupportedParameterIssueCsv`, and
  `formatModelCardUnsupportedParameterIssueJson`, stable diagnostics for
  retained unsupported model-card parameters, matching Python and Rust.
- Add `deviceModelReferenceDeckAuditGateCoverageDigest`,
  `formatDeviceModelReferenceDeckAuditGateCoverageDigestTable`,
  `deviceModelReferenceDeckAuditGateCoverageDigestRecords`,
  `formatDeviceModelReferenceDeckAuditGateCoverageDigestCsv`, and
  `formatDeviceModelReferenceDeckAuditGateCoverageDigestJson`, stable one-row
  audit gate coverage digest exports for release dashboards, matching Python
  and Rust.
- Add `deviceModelReferenceDeckAuditGateIssueSummary`,
  `formatDeviceModelReferenceDeckAuditGateIssueSummaryTable`,
  `deviceModelReferenceDeckAuditGateIssueSummaryRecords`,
  `formatDeviceModelReferenceDeckAuditGateIssueSummaryCsv`, and
  `formatDeviceModelReferenceDeckAuditGateIssueSummaryJson`, stable grouped
  issue exports for reference-deck audit gate dashboards, matching Python and
  Rust.
- Add `parseBerkeleySyntax`, a Berkeley SPICE logical-card facade with
  embedded grammar metadata, normalized continuation cards, source spans,
  token streams, stable diagnostics, and analysis inventory, matching Python
  and Rust for frontend/parser-tooling consumers.
- Add `formatDeviceModelReferenceDeckAuditGateIssueTable`,
  `deviceModelReferenceDeckAuditGateIssueRecords`,
  `formatDeviceModelReferenceDeckAuditGateIssueCsv`, and
  `formatDeviceModelReferenceDeckAuditGateIssueJson`, stable machine-readable
  exports for reference-deck audit gate issue rows, matching Python and Rust.
- Add `deviceModelReferenceDeckAuditMatrix`,
  `formatDeviceModelReferenceDeckAuditMatrixTable`,
  `deviceModelReferenceDeckAuditMatrixRecords`,
  `formatDeviceModelReferenceDeckAuditMatrixCsv`, and
  `formatDeviceModelReferenceDeckAuditMatrixJson`, stable per-model-family
  audit dashboard rows with explicit OP, temperature, AC, noise, and transient
  fixture columns, matching Python and Rust.
- Add `deviceModelReferenceDeckAuditAnalysisSummary`,
  `formatDeviceModelReferenceDeckAuditAnalysisSummaryTable`,
  `deviceModelReferenceDeckAuditAnalysisSummaryRecords`,
  `formatDeviceModelReferenceDeckAuditAnalysisSummaryCsv`, and
  `formatDeviceModelReferenceDeckAuditAnalysisSummaryJson`, stable
  per-analysis coverage summaries for the reference-deck audit matrix,
  matching Python and Rust.
- Add `deviceModelReferenceDeckAuditSummary`,
  `formatDeviceModelReferenceDeckAuditSummaryTable`,
  `deviceModelReferenceDeckAuditSummaryRecords`,
  `formatDeviceModelReferenceDeckAuditSummaryCsv`, and
  `formatDeviceModelReferenceDeckAuditSummaryJson`, stable per-kind coverage
  summaries for the reference-deck audit matrix, matching Python and Rust.
- Add `deviceModelReferenceDeckAuditRecords`,
  `formatDeviceModelReferenceDeckAuditCsv`, and
  `formatDeviceModelReferenceDeckAuditJson`, stable record-oriented exports
  for the device-model reference-deck audit matrix, matching Python and Rust.
- Add `deviceModelReferenceDeckAuditGate` and
  `formatDeviceModelReferenceDeckAuditGateReport`, a stable pass/fail gate for
  the required device-model reference-deck audit coverage matrix, matching
  Python and Rust.
- Add `formatDeviceModelReferenceDeckAuditTable`, a stable tab-separated
  summary for the device-model reference-deck audit matrix, matching Python
  and Rust.
- Add `deviceModelReferenceDeckAuditFixtures`, a stable reference coverage
  matrix across DC, temperature, AC, noise, and transient model-depth fixtures
  for diode, BJT, JFET, and Level-1 MOS families, matching Python and Rust.
- Shape Level-1 MOS reverse-biased bulk-junction capacitance with `PB` and
  `MJ` model-card parameters for AC operating-point capacitance reports and
  transient source-body / drain-body companions, matching Python and Rust,
  with regression coverage for reverse-biased drain-step delay.
- Stamp Level-1 MOS zero-bias bulk-junction `CBS` and `CBD` model-card storage
  as transient source-body and drain-body companions, matching Python and
  Rust, with regression coverage for drain-step delay.
- Stamp Level-1 MOS `CGSO`, `CGDO`, and `CGBO` model-card storage as
  transient gate-source, gate-drain, and gate-body companions, matching Python
  and Rust, with regression coverage for gate-step delay.
- Stamp JFET `gateSourceCapacitance` and `gateDrainCapacitance` model-card
  storage as transient gate-source and gate-drain companions and AC
  susceptance, matching Python and Rust, with regression coverage for gate-step
  delay and high-frequency gate-drive shunting.
- Stamp BJT `baseEmitterCapacitance`, `baseCollectorCapacitance`,
  `forwardTransitTime`, and `reverseTransitTime` model-card storage as
  transient base-emitter and base-collector companions, matching Python and
  Rust, with regression coverage for base current-step delay and forward
  transit-time turnoff charge.
- Stamp diode `junctionCapacitance` and `transitTime` model-card storage as
  transient anode-cathode companions, matching Python and Rust, with regression
  coverage for current-step delay and turnoff charge retention.
- Add `deviceModelChargeAuditFixtures` runnable one-device `.tran` fixtures
  with reference deck lines, explicit terminal storage capacitance metadata,
  stable first/final probe-voltage windows, and charge-behavior notes for
  diode, BJT, JFET, and Level-1 MOS audits, matching Python and Rust.
- Add `deviceModelNoiseAuditFixtures` runnable one-device `.noise` fixtures
  with reference deck lines and stable source/output PSD windows for diode and
  BJT shot noise plus JFET and Level-1 MOS channel thermal noise audits,
  matching Python and Rust.
- Fix TypeScript BJT small-signal and AC stamping to use the converged
  operating-point junction voltage when deriving transconductance and diffusion
  capacitance, matching Python and Rust.
- Add `deviceModelCapacitanceAuditFixtures` runnable one-device AC fixtures
  with `.ac` reference deck lines and stable high-frequency probe-magnitude
  windows for diode, BJT, JFET, and Level-1 MOS model-depth audits, matching
  Python and Rust.
- Add `deviceModelTemperatureAuditFixtures` runnable one-device DC
  temperature-sweep fixtures with `.temp` reference deck lines and stable
  probe-voltage windows for diode, BJT, JFET, and Level-1 MOS model-depth
  audits, matching Python and Rust.
- Add `deviceModelBehaviorAuditFixtures` runnable one-device DC bias fixtures
  with reference deck lines and stable probe-voltage windows for diode, BJT,
  JFET, and Level-1 MOS model-depth audits, matching Python and Rust.
- Add configurable nonlinear Newton damping through
  `dcOp(..., { newtonStepLimit })`, plus stable diagnostics for
  `newtonStepLimit`, `limitedNewtonSteps`, and `minimumDampingFactor`,
  matching Python and Rust.
- Add `dcOp(...).diagnostics.solverProfile` with matrix size, solver kind,
  backend, structural nonzero count, density, peak fill-in, and fallback
  metadata for production sparse-solver audits, matching Python and Rust.
- Add `runDeck` whole-run execution for every parsed `.op`, `.dc`, `.ac`,
  `.tran`, `.tf`, `.sens`, and `.noise` card in source order, preserving
  duplicate analysis directives, defaulting analysis-less decks to an implicit
  `.op`, and returning aggregate run-artifact table, CSV, compact JSON, and
  header-keyed record exports, matching Python and Rust.
- Expose selected analysis sweep, frequency, transient timing, and `UIC`
  metadata in `runDeckAnalysis` output-plan artifacts, with stable table, CSV,
  compact JSON, and header-keyed record exports, matching Python and Rust.
- Expose selected analysis output-node metadata in `runDeckAnalysis`
  output-plan artifacts beside line/source metadata, with stable table, CSV,
  compact JSON, and header-keyed record exports, matching Python and Rust.
- Expose selected analysis line/source metadata in `runDeckAnalysis`
  output-plan artifacts beside directive metadata, with stable table, CSV,
  compact JSON, and header-keyed record exports, matching Python and Rust.
- Expose selected result row counts in `runDeckAnalysis` output-plan artifacts
  beside result-column inventories, with stable table, CSV, compact JSON, and
  header-keyed record exports, matching Python and Rust.
- Expose selected output probe source line inventories in `runDeckAnalysis`
  output-plan artifacts aligned with selected output-probe inventories, with
  stable table, CSV, compact JSON, and header-keyed record exports, matching
  Python and Rust.
- Expose selected output directive source line inventories in
  `runDeckAnalysis` output-plan artifacts beside directive scope inventories,
  with stable table, CSV, compact JSON, and header-keyed record exports,
  matching Python and Rust.
- Expose normalized selected output directive analysis scope inventories in
  `runDeckAnalysis` output-plan artifacts beside directive kind inventories,
  distinguishing global `.save` / `.probe` selections from scoped `.probe`,
  `.print`, and `.plot` selections in stable table, CSV, compact JSON, and
  header-keyed record exports, matching Python and Rust.
- Expose normalized selected output directive kind inventories in
  `runDeckAnalysis` output-plan artifacts beside the selected directive tokens,
  with stable table, CSV, compact JSON, and header-keyed record exports,
  matching Python and Rust.
- Include selected `runDeckAnalysis` output-plan tables in execution `tables`,
  selected-run `TableList` metadata, and ordered `tableArtifacts` with stable
  table, CSV, compact JSON, and header-keyed record payloads, matching Python
  and Rust.
- Expose selected `runDeckAnalysis` output-plan inventories as
  `outputPlanArtifacts` with stable result-column, output-probe,
  output-directive, and table lists plus table, CSV, compact JSON, and
  header-keyed record exports, matching Python and Rust.
- Include policy-blocked `.control` row and summary tables in selected
  `runDeckAnalysis` execution `tables`, selected-run `TableList` metadata, and
  ordered `tableArtifacts` as `control-policy` and `control-policy-summary`
  exports with stable table, CSV, JSON, and header-keyed records, matching
  Python and Rust.
- Carry policy-blocked `.control` command inventories through selected
  `runDeckAnalysis` run artifacts as stable `ControlPolicyArtifacts`,
  `ControlPolicyCategoryList`, `ControlPolicyCodeList`, and
  `ControlPolicySeverityList` table, CSV/JSON, and `tableArtifacts` fields,
  matching Python and Rust.
- Group policy-blocked `.control` command artifacts from selected
  `runDeckAnalysis` execution results by category as
  `controlPolicySummaryArtifacts` with stable counts, line lists, command lists,
  code lists, severity lists, and table, CSV, compact JSON, and header-keyed
  record exports, matching Python and Rust.
- Expose policy-blocked `.control` commands from selected `runDeckAnalysis`
  execution results as `controlPolicyArtifacts` with stable line, category,
  command, code, severity, and message metadata plus table, CSV, compact JSON,
  and header-keyed record exports, matching Python and Rust.
- Carry matched and unmatched `write <rawfile> <probes...>` probe inventories
  through rawfile artifact `MatchedProbes` / `MatchedProbeList` and
  `UnmatchedProbes` / `UnmatchedProbeList` summary columns, and keep only
  requested matching vector columns in deterministic in-memory rawfile output,
  matching Python and Rust.
- Carry matched and unmatched `wrdata <file> <probes...>` probe inventories
  through WRDATA artifact `MatchedProbes` / `MatchedProbeList` and
  `UnmatchedProbes` / `UnmatchedProbeList` summary columns, matching Python and
  Rust.
- Treat explicit `wrdata <file> <probes...>` probe lists as in-memory data-file
  column selectors in `formatDeckWrdataAscii`, preserving the scale column plus
  requested matching probe columns in deterministic WRDATA output, matching
  Python and Rust.
- Carry accepted `.control` rawfile/data-write option inventories through
  WRDATA artifact `Options` / `RawfileOptionList` summary columns, and render
  `wr_vecnames` / `wr_singlescale` intent as deterministic `VectorNames` /
  `Scale` metadata in in-memory WRDATA data files, matching Python and Rust.
- Expose deterministic in-memory ASCII data-file artifacts for accepted
  `.control` `wrdata <file> ...` markers from selected `runDeckAnalysis`
  execution results as `wrdataArtifactCount`, `wrdataArtifacts`,
  `wrdataArtifactTable`, `wrdataArtifactCsv`, `wrdataArtifactJson`, and
  `wrdataArtifactRecords`, matching Python and Rust.
- Expose deterministic in-memory ASCII rawfile artifacts for accepted
  `.control` `write <rawfile> ...` markers from selected `runDeckAnalysis`
  execution results as `rawfileArtifactCount`, `rawfileArtifacts`,
  `rawfileArtifactTable`, `rawfileArtifactCsv`, `rawfileArtifactJson`, and
  `rawfileArtifactRecords`, matching Python and Rust.
- Expose accepted `.control` rawfile option inventories from
  `analyzeDeckControls` and selected `runDeckAnalysis` execution results as
  `rawfileOptionCount` / `rawfileOptions`, and carry them through selected-run
  artifacts as stable `RawfileOptions` / `RawfileOptionList` table, CSV/JSON,
  and ordered `tableArtifacts` fields, matching Python and Rust.
- Expose accepted `.control` `write` / `wrdata` marker inventories from
  `analyzeDeckControls` and selected `runDeckAnalysis` execution results as
  `writeMarkerCount` / `writeMarkers`, and carry them through selected-run
  artifacts as stable `WriteMarkers` / `WriteMarkerList` table, CSV/JSON, and
  ordered `tableArtifacts` fields, matching Python and Rust.
- Expose selected diagnostic inventories directly on selected
  `runDeckAnalysis` execution results as `diagnosticCount` /
  `diagnosticCodes`, matching Python and Rust.
- Expose normalized `.control` command inventories directly on selected
  `runDeckAnalysis` execution results as `controlLineCount` / `controlLines`,
  matching Python and Rust.
- Add normalized `.control` command inventories to `analyzeDeckControls`
  separately from full active deck input, and carry those commands through
  selected `runDeckAnalysis` run artifacts as stable `ControlLines` /
  `ControlLineList` table, CSV/JSON, and ordered `tableArtifacts` fields,
  matching Python and Rust.
- Surface existing `.control` body policy diagnostic codes in selected
  `runDeckAnalysis` run artifacts and propagate them through stable
  run-artifact tables, CSV/JSON helpers, and ordered `tableArtifacts`,
  matching Python and Rust.
- Add ordered `tableArtifacts` to selected `runDeckAnalysis` execution results
  with each stable table's text, CSV, compact JSON, and header-keyed records
  beside the existing table inventory, matching Python and Rust.
- Add stable table count/name lists directly to selected `runDeckAnalysis`
  execution results beside analysis directives, output probes, output
  directives, and selected-run artifacts, matching Python and Rust.
- Add stable table count/name lists to selected-run artifacts in
  `runDeckAnalysis` and render them in a stable `TableList` column from
  `formatDeckRunArtifactTable`, matching Python and Rust.
- Add selected analysis directives to `runDeckAnalysis` results and selected-run
  artifacts, including a stable `AnalysisDirectiveList` column from
  `formatDeckRunArtifactTable`, matching Python and Rust.
- Add selected output directives to `runDeckAnalysis` results beside selected
  output probes, matching Python and Rust.
- Add `deckTableRecords` for stable tab-separated deck output tables as
  header-keyed records for browser and host integrations, matching Python and
  Rust.
- Add `formatDeckTableJson` for stable tab-separated deck output tables as
  compact JSON records keyed by the header row, matching Python and Rust.
- Add `formatDeckTableCsv` for stable tab-separated deck output tables with the
  same deterministic CSV escaping as selected-run artifacts, matching Python
  and Rust.
- Add `formatDeckRunArtifactJson` for selected-run artifacts with the same
  stable keys and normalized cell values as `formatDeckRunArtifactTable`,
  matching Python and Rust.
- Add `formatDeckRunArtifactCsv` for selected-run artifacts with the same
  stable columns as `formatDeckRunArtifactTable` plus deterministic CSV
  escaping for browser and spreadsheet consumers, matching Python and Rust.
- Add selected Fourier probe names to selected-run artifacts in
  `runDeckAnalysis` and render them in a stable `FourierList` column from
  `formatDeckRunArtifactTable`, matching Python and Rust.
- Add selected measurement names to selected-run artifacts in
  `runDeckAnalysis` and render them in a stable `MeasurementList` column from
  `formatDeckRunArtifactTable`, matching Python and Rust.
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
