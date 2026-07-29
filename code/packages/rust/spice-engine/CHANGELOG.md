# Changelog

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
- Add Level-1 MOS model-card `U0` / `UO` surface mobility with the Berkeley
  default of 600 cm^2/V/s. When `TOX` is explicit and `KP` is omitted, derive
  `KP = U0 * Cox * 1e-4`; explicit `KP` retains precedence.
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
  Python and TypeScript. The supported-parameter catalog now contains 110
  canonical rows.
- Add BJT model-card `BR`/`BETA_R` reverse-current-gain support in DC,
  transient, AC, transfer-function, temperature, and noise paths, matching
  Python and TypeScript. `XTB` now scales both forward and reverse beta. The
  supported-parameter catalog now contains 108 canonical rows.
- Add BJT model-card `XTB` forward-beta temperature-exponent support, scaling
  forward beta by the analysis-to-nominal absolute temperature ratio, matching
  Python and TypeScript. The supported-parameter catalog now contains 106
  canonical rows.
- Add BJT model-card `ISC`/`NC` base-collector leakage support in DC,
  transient, AC, transfer-function, temperature, and noise paths, matching
  Python and TypeScript. The supported-parameter catalog now contains 104
  canonical rows.
- Add BJT model-card `ISE`/`NE` base-emitter leakage support in DC, transient,
  AC, transfer-function, temperature, and noise paths, matching Python and
  TypeScript. The supported-parameter catalog now contains 100 canonical rows.
- Add BJT model-card `IKF`/`IK` forward high-current beta roll-off support with
  shared base-charge modulation in DC, transient, AC, transfer-function, and
  noise paths, matching Python and TypeScript. The supported-parameter catalog
  now contains 96 canonical rows.
- Add BJT model-card `VAR`/`VB` reverse Early-voltage support with base-charge
  modulation in DC, transient, AC, transfer-function, and noise paths, matching
  Python and TypeScript. The supported-parameter catalog now contains 94
  canonical rows.
- Add BJT model-card `FC` forward-bias depletion-coefficient support for the
  shared `CJE` and `CJC` Berkeley continuation law in AC and transient analysis,
  matching Python and TypeScript. The supported-parameter catalog now contains
  92 canonical rows.
- Add BJT model-card `VJC`/`PC` base-collector junction-potential and `MJC`/`MC`
  grading-coefficient support with bias-shaped `CJC` depletion capacitance in
  AC and transient analysis, matching Python and TypeScript. The supported-
  parameter catalog now contains 90 canonical rows.
- Add BJT model-card `VJE`/`PE` base-emitter junction-potential and `MJE`/`ME`
  grading-coefficient support with bias-shaped `CJE` depletion capacitance in
  AC and transient analysis, matching Python and TypeScript. The supported-
  parameter catalog now contains 86 canonical rows.
- Add BJT model-card `NR` reverse emission-coefficient support to reverse
  base-collector diffusion charge in AC and transient analysis, matching Python
  and TypeScript. The supported-parameter catalog now contains 82 canonical rows.
- Add BJT model-card `NF` forward emission-coefficient support across DC,
  transient charge, AC, transfer-function, and noise paths, matching Python and
  TypeScript. The supported-parameter catalog now contains 80 canonical rows.
- Add BJT model-card `VAF`/`VA` forward Early-voltage support with
  collector-voltage modulation in DC, transient, AC, transfer-function, and
  noise paths, matching Python and TypeScript. The supported-parameter catalog
  now contains 78 canonical rows.
- Add BJT model-card `EG` energy-gap support to model-specific temperature
  scaling, preserving it through subcircuit expansion and matching Python and
  TypeScript. The supported-parameter catalog now contains 76 canonical rows.
- Add BJT model-card `XTI` saturation-current temperature-exponent support,
  preserving it through subcircuit expansion and matching Python and
  TypeScript. The supported-parameter catalog now contains 74 canonical rows.
- Add diode model-card `EG` energy-gap support to temperature scaling,
  preserving it through subcircuit expansion and matching Python and
  TypeScript.
- Add diode model-card `XTI` saturation-current temperature-exponent support,
  preserving it through subcircuit expansion and matching Python and
  TypeScript.
- Add diode model-card `FC` forward-bias depletion coefficient support and a
  continuous piecewise depletion-capacitance law, matching Python and
  TypeScript.
- Shape diode depletion capacitance from the operating-point bias with model-card
  `VJ`/`PB` junction-potential and `M`/`MJ` grading-coefficient parameters in AC
  and transient analysis, matching Python and TypeScript.
- Add `model_card_supported_parameter_coverage_dashboard`,
  `format_model_card_supported_parameter_coverage_dashboard_table`,
  `model_card_supported_parameter_coverage_dashboard_records`,
  `format_model_card_supported_parameter_coverage_dashboard_csv`, and
  `format_model_card_supported_parameter_coverage_dashboard_json`, stable
  per-kind supported-parameter coverage dashboard rows with actual versus
  expected counts plus gate issue fields, matching Python and TypeScript.
- Add `model_card_supported_parameter_coverage_gate`,
  `format_model_card_supported_parameter_coverage_gate_report`,
  `format_model_card_supported_parameter_coverage_gate_issue_table`,
  `model_card_supported_parameter_coverage_gate_issue_records`,
  `format_model_card_supported_parameter_coverage_gate_issue_csv`, and
  `format_model_card_supported_parameter_coverage_gate_issue_json`, stable
  release-gate checks and issue exports for the seven-kind, 72-row supported
  model-card parameter catalog, matching Python and TypeScript.
- Add `model_card_supported_parameter_coverage_summary`,
  `format_model_card_supported_parameter_coverage_summary_table`,
  `model_card_supported_parameter_coverage_summary_records`,
  `format_model_card_supported_parameter_coverage_summary_csv`, and
  `format_model_card_supported_parameter_coverage_summary_json`, stable
  per-model-kind summaries of supported model-card parameter alias coverage,
  matching Python and TypeScript.
- Add `model_card_supported_parameter_coverage`,
  `format_model_card_supported_parameter_coverage_table`,
  `model_card_supported_parameter_coverage_records`,
  `format_model_card_supported_parameter_coverage_csv`, and
  `format_model_card_supported_parameter_coverage_json`, stable supported
  model-card parameter and alias catalog exports, matching Python and
  TypeScript.
- Add `model_card_unsupported_parameter_issues`,
  `format_model_card_unsupported_parameter_issue_table`,
  `model_card_unsupported_parameter_issue_records`,
  `format_model_card_unsupported_parameter_issue_csv`, and
  `format_model_card_unsupported_parameter_issue_json`, stable diagnostics for
  retained unsupported model-card parameters, matching Python and TypeScript.
- Add `device_model_reference_deck_audit_gate_coverage_digest`,
  `format_device_model_reference_deck_audit_gate_coverage_digest_table`,
  `device_model_reference_deck_audit_gate_coverage_digest_records`,
  `format_device_model_reference_deck_audit_gate_coverage_digest_csv`, and
  `format_device_model_reference_deck_audit_gate_coverage_digest_json`, stable
  one-row audit gate coverage digest exports for release dashboards, matching
  Python and TypeScript.
- Add `device_model_reference_deck_audit_gate_issue_summary`,
  `format_device_model_reference_deck_audit_gate_issue_summary_table`,
  `device_model_reference_deck_audit_gate_issue_summary_records`,
  `format_device_model_reference_deck_audit_gate_issue_summary_csv`, and
  `format_device_model_reference_deck_audit_gate_issue_summary_json`, stable
  grouped issue exports for reference-deck audit gate dashboards, matching
  Python and TypeScript.
- Add `format_device_model_reference_deck_audit_gate_issue_table`,
  `device_model_reference_deck_audit_gate_issue_records`,
  `format_device_model_reference_deck_audit_gate_issue_csv`, and
  `format_device_model_reference_deck_audit_gate_issue_json`, stable
  machine-readable exports for reference-deck audit gate issue rows, matching
  Python and TypeScript.
- Add `device_model_reference_deck_audit_matrix`,
  `format_device_model_reference_deck_audit_matrix_table`,
  `device_model_reference_deck_audit_matrix_records`,
  `format_device_model_reference_deck_audit_matrix_csv`, and
  `format_device_model_reference_deck_audit_matrix_json`, stable
  per-model-family audit dashboard rows with explicit OP, temperature, AC,
  noise, and transient fixture columns, matching Python and TypeScript.
- Add `device_model_reference_deck_audit_analysis_summary`,
  `format_device_model_reference_deck_audit_analysis_summary_table`,
  `device_model_reference_deck_audit_analysis_summary_records`,
  `format_device_model_reference_deck_audit_analysis_summary_csv`, and
  `format_device_model_reference_deck_audit_analysis_summary_json`, stable
  per-analysis coverage summaries for the reference-deck audit matrix,
  matching Python and TypeScript.
- Add `device_model_reference_deck_audit_summary`,
  `format_device_model_reference_deck_audit_summary_table`,
  `device_model_reference_deck_audit_summary_records`,
  `format_device_model_reference_deck_audit_summary_csv`, and
  `format_device_model_reference_deck_audit_summary_json`, stable per-kind
  coverage summaries for the reference-deck audit matrix, matching Python and
  TypeScript.
- Add `device_model_reference_deck_audit_records`,
  `format_device_model_reference_deck_audit_csv`, and
  `format_device_model_reference_deck_audit_json`, stable record-oriented
  exports for the device-model reference-deck audit matrix, matching Python
  and TypeScript.
- Add `device_model_reference_deck_audit_gate` and
  `format_device_model_reference_deck_audit_gate_report`, a stable pass/fail
  gate for the required device-model reference-deck audit coverage matrix,
  matching Python and TypeScript.
- Add `format_device_model_reference_deck_audit_table`, a stable
  tab-separated summary for the device-model reference-deck audit matrix,
  matching Python and TypeScript.
- Add `device_model_reference_deck_audit_fixtures`, a stable reference coverage
  matrix across DC, temperature, AC, noise, and transient model-depth fixtures
  for diode, BJT, JFET, and Level-1 MOS families, matching Python and
  TypeScript.
- Shape Level-1 MOS reverse-biased bulk-junction capacitance with
  `bulk_junction_potential` and `bulk_junction_grading_coefficient`
  model-card parameters (`PB`/`MJ`) for AC operating-point capacitance reports
  and transient source-body / drain-body companions, matching Python and
  TypeScript, with regression coverage for reverse-biased drain-step delay.
- Stamp Level-1 MOS zero-bias bulk-junction
  `source_bulk_capacitance` and `drain_bulk_capacitance` model-card storage as
  transient source-body and drain-body companions, matching Python and
  TypeScript, with regression coverage for drain-step delay.
- Stamp Level-1 MOS `gate_source_overlap_capacitance`,
  `gate_drain_overlap_capacitance`, and `gate_bulk_overlap_capacitance`
  model-card storage as transient gate-source, gate-drain, and gate-body
  companions, matching Python and TypeScript, with regression coverage for
  gate-step delay.
- Stamp JFET `gate_source_capacitance` and `gate_drain_capacitance`
  model-card storage as transient gate-source and gate-drain companions and AC
  susceptance, matching Python and TypeScript, with regression coverage for
  gate-step delay and high-frequency gate-drive shunting.
- Stamp BJT `base_emitter_capacitance`, `base_collector_capacitance`,
  `forward_transit_time`, and `reverse_transit_time` model-card storage as
  transient base-emitter and base-collector companions, matching Python and
  TypeScript, with regression coverage for base current-step delay and forward
  transit-time turnoff charge.
- Stamp diode `junction_capacitance` and `transit_time` model-card storage as
  transient anode-cathode companions, matching Python and TypeScript, with
  regression coverage for current-step delay and turnoff charge retention.
- Add `device_model_charge_audit_fixtures` runnable one-device `.tran`
  fixtures with reference deck lines, explicit terminal storage capacitance
  metadata, stable first/final probe-voltage windows, and charge-behavior notes
  for diode, BJT, JFET, and Level-1 MOS audits, matching Python and
  TypeScript.
- Add `device_model_noise_audit_fixtures` runnable one-device `.noise`
  fixtures with reference deck lines and stable source/output PSD windows for
  diode and BJT shot noise plus JFET and Level-1 MOS channel thermal noise
  audits, matching Python and TypeScript.
- Add `device_model_capacitance_audit_fixtures` runnable one-device AC fixtures
  with `.ac` reference deck lines and stable high-frequency probe-magnitude
  windows for diode, BJT, JFET, and Level-1 MOS model-depth audits, matching
  Python and TypeScript.
- Add `device_model_temperature_audit_fixtures` runnable one-device DC
  temperature-sweep fixtures with `.temp` reference deck lines and stable
  probe-voltage windows for diode, BJT, JFET, and Level-1 MOS model-depth
  audits, matching Python and TypeScript.
- Add `device_model_behavior_audit_fixtures` runnable one-device DC bias
  fixtures with reference deck lines and stable probe-voltage windows for
  diode, BJT, JFET, and Level-1 MOS model-depth audits, matching Python and
  TypeScript.
- Add configurable nonlinear Newton damping through
  `DcOpOptions::newton_step_limit`, plus stable diagnostics for
  `newton_step_limit`, `limited_newton_steps`, and `minimum_damping_factor`,
  matching Python and TypeScript.
- Add `DcResult::diagnostics.solver_profile` with matrix size, solver kind,
  backend, structural nonzero count, density, peak fill-in, and fallback
  metadata for production sparse-solver audits, matching Python and TypeScript.
- Add `run_deck` whole-run execution for every parsed `.op`, `.dc`, `.ac`,
  `.tran`, `.tf`, `.sens`, and `.noise` card in source order, preserving
  duplicate analysis directives, defaulting analysis-less decks to an implicit
  `.op`, and returning aggregate run-artifact table, CSV, compact JSON, and
  header-keyed record exports, matching Python and TypeScript.
- Expose selected analysis sweep, frequency, transient timing, and `UIC`
  metadata in `run_deck_analysis` output-plan artifacts, with stable table,
  CSV, compact JSON, and header-keyed record exports, matching Python and
  TypeScript.
- Expose selected analysis output-node metadata in `run_deck_analysis`
  output-plan artifacts beside line/source metadata, with stable table, CSV,
  compact JSON, and header-keyed record exports, matching Python and
  TypeScript.
- Expose selected analysis line/source metadata in `run_deck_analysis`
  output-plan artifacts beside directive metadata, with stable table, CSV,
  compact JSON, and header-keyed record exports, matching Python and
  TypeScript.
- Expose selected result row counts in `run_deck_analysis` output-plan
  artifacts beside result-column inventories, with stable table, CSV, compact
  JSON, and header-keyed record exports, matching Python and TypeScript.
- Expose selected output probe source line inventories in `run_deck_analysis`
  output-plan artifacts aligned with selected output-probe inventories, with
  stable table, CSV, compact JSON, and header-keyed record exports, matching
  Python and TypeScript.
- Expose selected output directive source line inventories in
  `run_deck_analysis` output-plan artifacts beside directive scope
  inventories, with stable table, CSV, compact JSON, and header-keyed record
  exports, matching Python and TypeScript.
- Expose normalized selected output directive analysis scope inventories in
  `run_deck_analysis` output-plan artifacts beside directive kind inventories,
  distinguishing global `.save` / `.probe` selections from scoped `.probe`,
  `.print`, and `.plot` selections in stable table, CSV, compact JSON, and
  header-keyed record exports, matching Python and TypeScript.
- Expose normalized selected output directive kind inventories in
  `run_deck_analysis` output-plan artifacts beside the selected directive
  tokens, with stable table, CSV, compact JSON, and header-keyed record exports,
  matching Python and TypeScript.
- Include selected `run_deck_analysis` output-plan tables in execution
  `tables`, selected-run `TableList` metadata, and ordered `table_artifacts`
  with stable table, CSV, compact JSON, and header-keyed record payloads,
  matching Python and TypeScript.
- Expose selected `run_deck_analysis` output-plan inventories as
  `output_plan_artifacts` with stable result-column, output-probe,
  output-directive, and table lists plus table, CSV, compact JSON, and
  header-keyed record exports, matching Python and TypeScript.
- Include policy-blocked `.control` row and summary tables in selected
  `run_deck_analysis` execution `tables`, selected-run `TableList` metadata,
  and ordered `table_artifacts` as `control-policy` and
  `control-policy-summary` exports with stable table, CSV, JSON, and
  header-keyed records, matching Python and TypeScript.
- Carry policy-blocked `.control` command inventories through selected
  `run_deck_analysis` run artifacts as stable `ControlPolicyArtifacts`,
  `ControlPolicyCategoryList`, `ControlPolicyCodeList`, and
  `ControlPolicySeverityList` table, CSV/JSON, and `table_artifacts` fields,
  matching Python and TypeScript.
- Group policy-blocked `.control` command artifacts from selected
  `run_deck_analysis` execution results by category as
  `control_policy_summary_artifacts` with stable counts, line lists, command
  lists, code lists, severity lists, and table, CSV, compact JSON, and
  header-keyed record exports, matching Python and TypeScript.
- Expose policy-blocked `.control` commands from selected `run_deck_analysis`
  execution results as `control_policy_artifacts` with stable line, category,
  command, code, severity, and message metadata plus table, CSV, compact JSON,
  and header-keyed record exports, matching Python and TypeScript.
- Carry matched and unmatched `write <rawfile> <probes...>` probe inventories
  through rawfile artifact `MatchedProbes` / `MatchedProbeList` and
  `UnmatchedProbes` / `UnmatchedProbeList` summary columns, and keep only
  requested matching vector columns in deterministic in-memory rawfile output,
  matching Python and TypeScript.
- Carry matched and unmatched `wrdata <file> <probes...>` probe inventories
  through WRDATA artifact `MatchedProbes` / `MatchedProbeList` and
  `UnmatchedProbes` / `UnmatchedProbeList` summary columns, matching Python and
  TypeScript.
- Treat explicit `wrdata <file> <probes...>` probe lists as in-memory data-file
  column selectors in `format_deck_wrdata_ascii`, preserving the scale column
  plus requested matching probe columns in deterministic WRDATA output,
  matching Python and TypeScript.
- Carry accepted `.control` rawfile/data-write option inventories through
  WRDATA artifact `Options` / `RawfileOptionList` summary columns, and render
  `wr_vecnames` / `wr_singlescale` intent as deterministic `VectorNames` /
  `Scale` metadata in in-memory WRDATA data files, matching Python and
  TypeScript.
- Expose deterministic in-memory ASCII data-file artifacts for accepted
  `.control` `wrdata <file> ...` markers from selected `run_deck_analysis`
  execution results as `wrdata_artifact_count`, `wrdata_artifacts`,
  `wrdata_artifact_table`, `wrdata_artifact_csv`, `wrdata_artifact_json`, and
  `wrdata_artifact_records`, matching Python and TypeScript.
- Expose deterministic in-memory ASCII rawfile artifacts for accepted
  `.control` `write <rawfile> ...` markers from selected `run_deck_analysis`
  execution results as `rawfile_artifact_count`, `rawfile_artifacts`,
  `rawfile_artifact_table`, `rawfile_artifact_csv`, `rawfile_artifact_json`,
  and `rawfile_artifact_records`, matching Python and TypeScript.
- Expose accepted `.control` rawfile option inventories from
  `analyze_deck_controls` and selected `run_deck_analysis` execution results as
  `rawfile_option_count` / `rawfile_options`, and carry them through
  selected-run artifacts as stable `RawfileOptions` / `RawfileOptionList`
  table, CSV/JSON, and ordered `table_artifacts` fields, matching Python and
  TypeScript.
- Expose accepted `.control` `write` / `wrdata` marker inventories from
  `analyze_deck_controls` and selected `run_deck_analysis` execution results as
  `write_marker_count` / `write_markers`, and carry them through selected-run
  artifacts as stable `WriteMarkers` / `WriteMarkerList` table, CSV/JSON, and
  ordered `table_artifacts` fields, matching Python and TypeScript.
- Expose selected diagnostic inventories directly on selected
  `run_deck_analysis` execution results as `diagnostic_count` /
  `diagnostic_codes`, matching Python and TypeScript.
- Expose normalized `.control` command inventories directly on selected
  `run_deck_analysis` execution results as `control_line_count` /
  `control_lines`, matching Python and TypeScript.
- Add normalized `.control` command inventories to `analyze_deck_controls`
  separately from full active deck input, and carry those commands through
  selected `run_deck_analysis` run artifacts as stable `ControlLines` /
  `ControlLineList` table, CSV/JSON, and ordered `table_artifacts` fields,
  matching Python and TypeScript.
- Surface existing `.control` body policy diagnostic codes in selected
  `run_deck_analysis` run artifacts and propagate them through stable
  run-artifact tables, CSV/JSON helpers, and ordered `table_artifacts`,
  matching Python and TypeScript.
- Add ordered `table_artifacts` to selected `run_deck_analysis` execution
  results with each stable table's text, CSV, compact JSON, and header-keyed
  records beside the existing table inventory, matching Python and TypeScript.
- Add stable table count/name lists directly to selected `run_deck_analysis`
  execution results beside analysis directives, output probes, output
  directives, and selected-run artifacts, matching Python and TypeScript.
- Add stable table count/name lists to selected-run artifacts in
  `run_deck_analysis` and render them in a stable `TableList` column from
  `format_deck_run_artifact_table`, matching Python and TypeScript.
- Add selected analysis directives to `run_deck_analysis` results and selected-run
  artifacts, including a stable `AnalysisDirectiveList` column from
  `format_deck_run_artifact_table`, matching Python and TypeScript.
- Add selected output directives to `run_deck_analysis` results beside selected
  output probes, matching Python and TypeScript.
- Add `deck_table_records` for stable tab-separated deck output tables as
  header-keyed records for browser and host integrations, matching Python and
  TypeScript.
- Add `format_deck_table_json` for stable tab-separated deck output tables as
  compact JSON records keyed by the header row, matching Python and TypeScript.
- Add `format_deck_table_csv` for stable tab-separated deck output tables with
  the same deterministic CSV escaping as selected-run artifacts, matching
  Python and TypeScript.
- Add `format_deck_run_artifact_json` for selected-run artifacts with the same
  stable keys and normalized cell values as `format_deck_run_artifact_table`,
  matching Python and TypeScript.
- Add `format_deck_run_artifact_csv` for selected-run artifacts with the same
  stable columns as `format_deck_run_artifact_table` plus deterministic CSV
  escaping for browser and spreadsheet consumers, matching Python and
  TypeScript.
- Add selected Fourier probe names to selected-run artifacts in
  `run_deck_analysis` and render them in a stable `FourierList` column from
  `format_deck_run_artifact_table`, matching Python and TypeScript.
- Add selected measurement names to selected-run artifacts in
  `run_deck_analysis` and render them in a stable `MeasurementList` column
  from `format_deck_run_artifact_table`, matching Python and TypeScript.
- Add normalized output-probe names to selected-run artifacts in
  `run_deck_analysis` and render them in a stable `OutputProbeList` column
  from `format_deck_run_artifact_table`, matching Python and TypeScript.
- Emit explicit policy diagnostics for selected `.control` block
  variable/state mutation commands, including `let`, `alter`, `alterparam`,
  `set`, and `unset`, in `analyze_deck_controls` and `resolve_deck_sources`,
  matching Python and TypeScript. Accepted no-op `set` options still route as
  no-op markers.
- Emit explicit policy diagnostics for selected `.control` block control-flow
  commands, including `if`, `while`, `foreach`, and `repeat`, in
  `analyze_deck_controls` and `resolve_deck_sources`, matching Python and
  TypeScript. Control-flow execution remains disabled by the deck execution
  policy.
- Emit explicit policy diagnostics for selected `.control` block `cd`
  working-directory mutation commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript. Working-directory
  mutation remains disabled by the deck execution policy.
- Emit explicit policy diagnostics for selected `.control` block `source` and
  `shell` external script/shell commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript. External script
  execution and shelling out remain disabled by the deck execution policy.
- Accept selected `.control` block read-only `echo`, `rusage`, and `where`
  console/debug commands as no-op control commands in `analyze_deck_controls`
  and `resolve_deck_sources`, matching Python and TypeScript. Actual
  console/debug output remains out of scope for these markers.
- Accept selected `.control` block read-only `status`, `version`, and `help`
  UI introspection commands as no-op control commands in
  `analyze_deck_controls` and `resolve_deck_sources`, matching Python and
  TypeScript. Actual console/help output remains out of scope for these
  markers.
- Accept selected `.control` block read-only `show` and `showmod`
  device/model inspection commands as no-op control commands in
  `analyze_deck_controls` and `resolve_deck_sources`, matching Python and
  TypeScript. Actual console/model inspection output remains out of scope for
  these markers.
- Accept selected `.control` block read-only `display` and `listing`
  inspection commands as no-op control commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript. Actual
  console/listing output remains out of scope for these markers.
- Accept selected `.control` block `wrdata <file> <probes...>` ASCII
  data-write markers as no-op control commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript. Actual data-file
  serialization remains out of scope for this marker.
- Accept selected `.control` block `write <rawfile> [probes...]` rawfile-write
  markers as no-op control commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript. Rawfile
  serialization remains out of scope for this marker.
- Accept selected `.control` block `set appendwrite` rawfile append-write
  options as no-op control commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript.
- Accept selected `.control` block `set wr_vecnames` and `set wr_singlescale`
  rawfile output toggles as no-op control commands in `analyze_deck_controls`
  and `resolve_deck_sources`, matching Python and TypeScript.
- Accept selected `.control` block `set filetype=ascii` output-format options
  as no-op control commands in `analyze_deck_controls` and
  `resolve_deck_sources`, matching Python and TypeScript.
- Accept selected `.control` block `reset` session-reset markers as no-op
  control commands in `analyze_deck_controls` and `resolve_deck_sources`,
  matching Python and TypeScript.
- Accept selected `.control` block `set noaskquit` UI options as no-op control
  commands in `analyze_deck_controls` and `resolve_deck_sources`, matching
  Python and TypeScript.
- Accept selected `.control` block `quit` interpreter-exit markers as no-op
  control commands in `analyze_deck_controls` and `resolve_deck_sources`,
  matching Python and TypeScript.
- Accept selected `.control` block `run` execution markers as no-op control
  commands in `analyze_deck_controls` and `resolve_deck_sources`, matching
  Python and TypeScript.
- Add selected `.control` block `four` and `fourier` command routing to
  `analyze_deck_controls` and `resolve_deck_sources`; the commands are
  normalized into `.four` deck cards, matching Python and TypeScript.
- Add selected `.control` block `measure` and `meas` command routing to
  `analyze_deck_controls` and `resolve_deck_sources`; the commands are
  normalized into `.measure` and `.meas` deck cards, matching Python and
  TypeScript.
- Add selected `.control` block `save` and `probe` command routing to
  `analyze_deck_controls` and `resolve_deck_sources`; the commands are
  normalized into `.save` and `.probe` deck cards, matching Python and
  TypeScript.
- Add selected `.control` block command routing to `analyze_deck_controls` and
  `resolve_deck_sources`; analysis/output commands (`op`, `dc`, `ac`, `tran`,
  `save`, `probe`, `print`, and `plot`) are normalized into dotted deck cards,
  matching Python and TypeScript.
- Add control-block exclusion diagnostics to `analyze_deck_controls` and
  `resolve_deck_sources`; unsupported `.control` / `.endc` block markers and
  unrecognized body commands are no longer forwarded as active deck lines and
  emit stable diagnostics, matching Python and TypeScript.
- Add parsed `.plot <analysis> ...` output routing to `resolve_deck_outputs`,
  `select_deck_output_probes`, and deck table formatters, matching Python and
  TypeScript.
- Add parsed `.print <analysis> ...` output routing to `resolve_deck_outputs`,
  `select_deck_output_probes`, and deck table formatters, matching Python and
  TypeScript.
- Add selected-run artifact summaries to `run_deck_analysis`; executions now
  return stable result-row, output-probe, measurement, and Fourier counts plus
  a run-artifact table, matching Python and TypeScript.
- Add selected Fourier artifacts to `run_deck_analysis`; selected `.tran`
  executions now return parsed `.four` harmonic results and a stable Fourier
  table alongside the selected plan, solver result, output probes, and
  measurement artifacts, matching Python and TypeScript.
- Add selected measurement artifacts to `run_deck_analysis`; selected `.dc`,
  `.ac`, and `.tran` executions now return parsed `.measure` / `.meas` results
  and a stable measurement table alongside the selected plan, solver result,
  output probes, and output table, matching Python and TypeScript.
- Add selected-output probe artifacts to `run_deck_analysis`; callers now
  receive the normalized deck-selected output probes alongside each selected
  plan, solver result, and stable table, matching Python and TypeScript.
- Add `.tran` print-step output routing to `run_deck_analysis`; deck transient
  plans now keep `.tran TSTEP` as the stable output print grid while `MAXSTEP`
  caps internal solver stepping, matching Python and TypeScript.
- Add `.tran START/MAXSTEP/UIC` selected-plan execution routing to
  `run_deck_analysis`; deck transient plans now apply `START` output filtering,
  `MAXSTEP` fixed-step caps, and `UIC` initial-condition intent through stable
  deck-selected transient tables, matching Python and TypeScript.
- Add `.ac LIN` and `.ac OCT` selected-plan execution routing to
  `run_deck_analysis`; deck AC plans now execute SPICE-style linear,
  points-per-decade, and points-per-octave grids, matching Python and
  TypeScript.
- Add `run_deck_analysis` so callers can select one deck `.op`, `.dc`,
  `.ac DEC`, or `.tran` plan, execute the matching solver, and receive the
  selected plan, solver result, and deck-selected output table, matching Python
  and TypeScript.
- Add `select_deck_analysis_plan` so callers can choose one explicit or
  implicit deck analysis plan with stable ambiguity and invalid-card errors,
  matching Python and TypeScript.
- Add `resolve_deck_analyses` so `.op`, `.dc`, `.ac`, and `.tran` analysis
  cards are extracted before `.end` into stable metadata with shared
  diagnostics, matching Python and TypeScript.
- Add `resolve_deck_fourier`, `fourier_transient_cards`, and
  `fourier_transient_deck` so parsed `.four` / `.FOUR` deck cards can route
  transient samples into SPICE-style Fourier harmonic results with optional
  `HARMONICS=` and `FROM=` controls, matching Python and TypeScript.
- Add `measure_transient_delay_between_probes` and parsed transient
  `.measure ... TRIG ... TARG ...` routing so deck measurements can report
  trigger-to-target delays with counted crossing controls, matching Python and
  TypeScript.
- Add `measure_transient_when_probe_counted` and parsed transient
  `.measure ... WHEN probe=target RISE|FALL|CROSS=n` routing so deck
  measurements can report counted threshold occurrences over optional
  `FROM=` / `TO=` windows, matching Python and TypeScript.
- Add `measure_transient_when_probe` and parsed transient
  `.measure ... WHEN probe=target` routing so deck measurements can report the
  first crossing time over optional `FROM=` / `TO=` windows, matching Python
  and TypeScript.
- Add `measure_transient_find_at_probe` and parsed transient
  `.measure ... FIND ... AT=` routing so deck measurements can sample or
  linearly interpolate a probe value at one scalar time, matching Python and
  TypeScript.
- Add `measure_ac_sweep_probe`, `measure_ac_sweep_cards`, and
  `measure_ac_sweep_deck` so parsed `.measure ac` / `.meas ac` cards can route
  AC sweep probe magnitudes into the shared scalar measurement table surface,
  matching Python and TypeScript.
- Add `measure_dc_sweep_probe`, `measure_dc_sweep_cards`, and
  `measure_dc_sweep_deck` so parsed `.measure dc` / `.meas dc` cards can route
  DC sweep probe samples into the shared scalar measurement table surface,
  matching Python and TypeScript.
- Add `resolve_deck_outputs`, `select_deck_output_probes`, and
  `format_deck_*_table` helpers so parsed `.save` and `.probe` deck cards can
  drive stable operating-point, DC sweep, AC sweep, and transient table output
  in the live Rust package.
- Add `resolve_deck_measurements`, `measure_transient_cards`, and
  `measure_transient_deck` for parsed transient `.measure` / `.meas` card
  routing into stable scalar measurement rows, matching Python and TypeScript.
- Add `measure_transient_probe` and `format_measurement_table` for a shared
  `.MEASURE`-style scalar transient output surface with MAX, MIN, AVG, RMS,
  peak-to-peak, and final-value probe measurements, matching Python and
  TypeScript.
- Add `dc_initial_vector_from_conditions`,
  `dc_op_with_initial_conditions`, and `dc_op_with_initial_vector` so parsed
  `.ic` / `.nodeset` node-voltage hints can seed DC operating-point Newton
  solves as MNA warm-start vectors, with `.ic` values taking precedence over
  `.nodeset`, matching Python and TypeScript.
- Add scalar `.func` call evaluation to `resolve_deck_parameters`: definitions
  are collected before `.end`, calls can appear in `.param` assignments and
  braced or quoted active-line expressions, and unknown functions, bad arity,
  and recursive calls produce stable diagnostics, matching Python and
  TypeScript.
- Add `resolve_deck_functions` for scalar `.func name(args) expression`
  definition extraction before `.end`, braced or quoted expression delimiter
  stripping, and stable diagnostics for malformed signatures, arguments,
  duplicate arguments, and empty expressions, matching Python and TypeScript.
- Add `resolve_deck_initial_conditions` for scalar `.ic` and `.nodeset`
  `V(node)=value` hint extraction before `.end`, numeric SPICE
  suffix/arithmetic expression evaluation, and stable diagnostics for malformed
  targets and unresolved values, matching Python and TypeScript.
- Add `resolve_deck_parameters` for scalar whitespace-tokenized `.param`
  assignment evaluation, braced and quoted active-line expression rewriting,
  and stable diagnostics for unresolved expressions, matching Python and
  TypeScript.
- Add `resolve_deck_sources` for map-backed `.include` and selected
  `.lib path section` expansion with stable diagnostics for missing sources,
  missing or unterminated library sections, cycles, and still-unsupported
  `.control` blocks, matching Python and TypeScript.
- Add `analyze_deck_controls` for shared deck-control boundary diagnostics:
  active pre-`.end` lines plus stable unsupported-feature diagnostics for
  `.include`, `.lib`, and `.control`, matching Python and TypeScript.
- Add `compatibility_corpus`, `release_readiness_gates`,
  `format_compatibility_corpus_table`, and `format_release_readiness_report`
  for the first oracle-backed compatibility deck corpus with golden tolerances
  and known incompatibility notes shared with Python and TypeScript.
- Add `CustomModel`, `CustomModelKind`, `CustomModelEvaluation`, and
  `analyze_custom_model_source` for the first Rust-native two-terminal
  residual/Jacobian custom-model fast path and Verilog-A subset diagnostics
  shared with Python and TypeScript.
- Add `format_digital_event_stream_vcd` and
  `format_digital_event_stream_vcd_with_options` for deterministic VCD
  correlation output from SPICE-side mixed-signal digital event streams.
- Add `normalize_model_card`, typed model-card builders, and
  `device_model_audit_fixtures` for cross-language diode, BJT, JFET, and
  Level-1 MOS `.model` alias compatibility fixtures.
- Add `DcResult::diagnostics` with stable matrix size, solver kind, tolerance,
  convergence aid, and final Newton delta metadata; large AC complex systems
  now route through the sparse-row complex solver path.

## 0.14.0 — 2026-06-05

- Add `s_parameters_corners_parallel` for order-preserving parallel Rust
  S-parameter extraction across named PVT corners.
- Add `noise_ac_corners_parallel` for order-preserving parallel Rust `.NOISE`
  evaluation across named PVT corners.
- Add `sens_dc_corners_parallel` for order-preserving parallel Rust DC
  sensitivity evaluation across named PVT corners.
- Add `mc_dc_corners_parallel` for order-preserving parallel Rust Monte Carlo
  DC evaluation across named PVT corners.
- Add `tf_corners_parallel` for order-preserving parallel Rust `.TF`
  transfer-function evaluation across named PVT corners.
- Add `ac_sweep_corners_parallel` for order-preserving parallel Rust `.AC`
  frequency-sweep evaluation across named PVT corners.
- Add `dc_sweep_corners_parallel` for order-preserving parallel Rust `.DC`
  source-sweep evaluation across named PVT corners.
- Add `dc_corners_parallel` for order-preserving parallel Rust DC
  operating-point evaluation across named PVT corners.
- Add `digital_event_streams_to_bridge_schedule` and
  `format_digital_bridge_schedule_table` for stable SPICE-side mixed-signal
  bridge breakpoint schedules over digital event starts and finite-edge
  transition endpoints.
- Add `transient_adaptive_with_digital_event_streams`,
  `transient_adaptive_with_digital_event_streams_corners`, and adaptive
  digital event stream table formatters for SPICE-side mixed-signal bridge
  snapshots that carry method, rejected-step, and convergence metadata.
- Add `transient_with_digital_event_streams_corners` and
  `format_corner_digital_event_stream_table` for stable named-corner
  mixed-signal transient bridge output stream snapshots.
- Add `transient_with_digital_event_streams` for a SPICE-side mixed-signal
  transient bridge from named digital input streams to sampled output streams.
- Add `digital_event_streams_to_voltage_sources` for converting named
  mixed-signal event streams into finite-edge PWL voltage sources.
- Add `sample_transient_probes_as_digital_event_streams` for collecting
  multiple thresholded transient probes as named mixed-signal event streams.
- Add `DigitalEventStream` and `format_digital_event_stream_table` for stable
  tab-separated named mixed-signal digital event stream snapshots.
- Add `format_digital_event_table` for stable tab-separated mixed-signal
  digital event stream snapshots.
- Add binary mixed-signal boundary helpers that convert digital event timelines
  and named event streams into finite-edge PWL voltage sources and threshold
  transient probes back into digital events.
- Add `dc_temperature_sweep_corners` and
  `format_corner_temperature_dc_table` for stable tab-separated named-corner
  DC operating-point snapshots across explicit `.temp`-style analysis
  temperatures.
- Add `dc_temperature_sweep` and `format_temperature_dc_table` for stable
  tab-separated DC operating-point snapshots across explicit `.temp`-style
  analysis temperatures.
- Add `format_adaptive_transient_table`, `transient_adaptive_corners`, and
  `format_corner_adaptive_transient_table` for stable tab-separated adaptive
  transient sample text output snapshots, including method, rejected-step, and
  convergence metadata.
- Add `fourier_corners`, `fourier_corners_with_start_time`, and
  `format_corner_fourier_table` for stable tab-separated named-corner `.FOUR`
  harmonic coefficient, magnitude, phase, DC, and THD text output snapshots.
- Add `transient_corners`, `transient_corners_with_method`, and
  `format_corner_transient_table` for stable tab-separated named-corner
  transient sample text output snapshots.
- Add `format_corner_dc_table` for stable tab-separated named-corner DC
  operating-point voltage and current text output snapshots.
- Add `format_corner_tf_table` for stable tab-separated named-corner `.TF`
  gain and impedance text output snapshots.
- Add `format_corner_ac_table` for stable tab-separated named-corner `.AC`
  real, imaginary, magnitude, and phase text output snapshots.
- Add `format_dc_sweep_table` and `format_corner_dc_sweep_table` for stable
  tab-separated `.DC` source-sweep value and selected-probe snapshots.
- Add `format_mc_table` and `format_corner_mc_table` for stable tab-separated
  `.MC` output-node trial, mean, standard-deviation, and convergence snapshots.
- Add `format_corner_distortion_table` for stable tab-separated named-corner
  `.DISTO` harmonic magnitude, phase, and THD text output snapshots.
- Add `format_corner_pole_zero_table` for stable tab-separated named-corner
  `.PZ` pole-zero text output snapshots.
- Add `format_corner_pss_table` for stable tab-separated named-corner PSS
  steady-state period, convergence, residual, and probe text output snapshots.
- Add `format_corner_sens_table` for stable tab-separated named-corner `.SENS`
  nominal, absolute-sensitivity, and relative-sensitivity text output snapshots.
- Add `format_corner_noise_table` for stable tab-separated named-corner
  `.NOISE` total and per-source PSD text output snapshots.
- Add `format_corner_s_parameter_table` for stable tab-separated named-corner
  S-parameter real, imaginary, magnitude, and phase text output snapshots.
- Add `format_sens_table` for stable tab-separated `.SENS` nominal,
  absolute-sensitivity, and relative-sensitivity text output snapshots.
- Add multi-corner DC sensitivity analysis with `sens_dc_corners`, returning
  the same `.SENS` output-node query evaluated under each named corner.
- Add `format_noise_table` for stable tab-separated `.NOISE` total and
  per-source PSD text output snapshots.
- Add multi-corner AC noise analysis with `noise_ac_corners`, returning the
  same `.NOISE` output/input query evaluated under each named corner.
- Add multi-corner periodic steady-state analysis with `pss_corners`, returning
  the same PSS solve evaluated under each named corner.
- Add multi-corner pole-zero analysis with `pole_zero_corners`, returning the
  selected constrained `.PZ` topology evaluated under each named corner.
- Add multi-corner distortion projection with
  `distortion_from_transient_corners`, returning the same transient-to-`.DISTO`
  query evaluated under each named corner.
- Add multi-corner S-parameter extraction with `s_parameters_corners`,
  returning the same two-port query evaluated under each named corner.
- Add `format_s_parameter_table` for stable tab-separated S-parameter real,
  imaginary, magnitude, and phase text output snapshots.
- Add `diode_at_temperature` and `circuit_at_temperature` helpers, which adjust
  diode thermal voltage and saturation current for an operating temperature
  using a SPICE-style silicon energy-gap foothold.
- Add `bjt_at_temperature` and extend `circuit_at_temperature` to adjust BJT
  thermal voltage and saturation current with the same silicon energy-gap
  foothold.
- Add `mosfet_at_temperature` and extend `circuit_at_temperature` to adjust
  Level-1 MOSFET threshold voltage, transconductance parameter, and nominal
  temperature.
- Add `format_dc_table` and `format_transient_table` for stable tab-separated
  node-voltage and branch-current text output snapshots.
- Add `format_pole_zero_table` for stable tab-separated `.PZ` pole-zero text
  output snapshots.
- Add `format_distortion_table` for stable tab-separated `.DISTO` harmonic
  magnitude, phase, and THD text output snapshots.
- Add `format_fourier_table` for stable tab-separated `.FOUR` harmonic
  coefficient, magnitude, phase, DC, and THD text output snapshots.
- Add `format_ac_table` for stable tab-separated `.AC` real, imaginary,
  magnitude, and phase text output snapshots.
- Add `format_tf_table` for stable tab-separated `.TF` gain and impedance text
  output snapshots.
- Add JFET source-follower transient fixtures covering nonlinear
  companion-model solves.
- Add `fourier`, which computes SPICE-style DC, harmonic sine/cosine
  coefficients, magnitudes, phases, and THD from transient samples for
  `V(node)` and `I(source)` probes.
- Add `distortion_from_transient`, which runs the Fourier extraction path and
  returns the Phase-8 distortion result shape directly from transient samples.
- Add `pole_zero_rc_highpass`, which returns the origin zero and RC pole for a
  constrained first-order high-pass fixture.
- Add `pole_zero_rlc_lowpass`, which returns the second-order pole pair for a
  constrained series R-L / shunt-C low-pass fixture.
- Add `pole_zero_rlc_highpass`, which returns the double origin zero plus
  second-order pole pair for a constrained series R-C / shunt-L high-pass
  fixture.
- Add `pole_zero_rlc_bandpass`, which returns the origin zero plus second-order
  pole pair for a constrained series L-C / shunt-R band-pass fixture.
- Add `pole_zero_rlc_notch`, which returns the imaginary-axis zero pair plus
  second-order pole pair for a constrained series-R / shunt-series-L-C notch
  fixture.
- Add MOS Level-1 capacitance support through `CGSO`, `CGDO`, `CGBO`, `CBS`,
  and `CBD`, contributing small-signal AC susceptance.
- Add MOSFET channel thermal noise to `.NOISE` via the long-channel `4kTγgm`
  model and per-element `M` device contributions.
- Add diode emission coefficient support through `emission_coefficient`,
  scaling the effective thermal voltage in DC and small-signal diode
  conductance.
- Add diode breakdown support through `breakdown_voltage` /
  `breakdown_current`, adding a bounded reverse-breakdown current and
  conductance foothold.
- Add diode junction capacitance support through `junction_capacitance`,
  contributing small-signal AC susceptance in parallel with the linearized
  diode conductance.
- Add diode transit-time support through `transit_time`, contributing
  forward-bias diffusion capacitance to small-signal AC admittance.
- Add BJT capacitance support through `base_emitter_capacitance` /
  `base_collector_capacitance`, contributing small-signal AC susceptance.
- Add BJT transit-time support through `forward_transit_time`, contributing
  forward-bias diffusion capacitance to small-signal AC admittance.
- Add BJT reverse transit-time support through `reverse_transit_time`,
  contributing base-collector diffusion capacitance to small-signal AC
  admittance.
- Add pseudo-transient DC continuation as a final bounded convergence aid after
  Newton, Gmin stepping, and source stepping; successful fallback results
  report `DcConvergenceAid::PseudoTransient`.
- Add `DcResult::convergence_aid`, reporting whether the DC operating point
  came from plain Newton, Gmin stepping, source stepping, or no successful
  convergence aid.
- Add `transient_adaptive`, an LTE-controlled transient surface with bounded
  step growth/shrinkage and `Euler` / `Trap` / `Gear2` method routing.
- Add trapezoidal transient integration parity for capacitors and inductors,
  enabling LC damping comparisons against Gear-2.
- Add Gear-2 transient integration with BDF2 capacitor/inductor companion
  histories after bootstrapping with one backward-Euler step.
- Add transient analysis stamping for `TransmissionLine` using a lossless
  Bergeron delay-line companion model, including matched-load delayed step
  behavior.
- Add AC analysis stamping for `TransmissionLine` using the lossless two-port
  admittance matrix, including matched-load phase-delay behavior.
- Add a public `TransmissionLine` element as the parser-facing SPICE `T` card
  foothold for future AC/transient delay-line stamping.
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
- Add `format_pss_table`, which renders the direct PSS result as a stable
  tab-separated steady-state table with period, step, convergence, iteration,
  residual, time, and selected voltage/current probes.
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
- Add multi-corner Monte Carlo DC analysis with `mc_dc_corners`, returning the
  same seeded tolerance trial set evaluated under each named corner.
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
