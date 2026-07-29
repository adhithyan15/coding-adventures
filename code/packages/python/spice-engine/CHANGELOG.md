# Changelog

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

### Added

- **BJT reverse high-current beta roll-off** — BJT model cards now accept
  `IKR` and apply reverse base-charge modulation in DC, transient, AC,
  transfer-function, temperature, and noise paths, matching Rust and
  TypeScript. The supported-parameter catalog now contains 110 canonical rows.

- **BJT reverse current gain** — BJT model cards now accept `BR`/`BETA_R` and
  apply the reverse base-current branch in DC, transient, AC,
  transfer-function, temperature, and noise paths, matching Rust and
  TypeScript. `XTB` now scales both forward and reverse beta. The
  supported-parameter catalog now contains 108 canonical rows.

- **BJT forward-beta temperature exponent** — BJT model cards now accept `XTB`
  and scale forward beta by the analysis-to-nominal absolute temperature ratio,
  matching Rust and TypeScript. The supported-parameter catalog now contains
  106 canonical rows.

- **BJT base-collector leakage** — BJT model cards now accept `ISC`/`NC` and
  apply the leakage branch in DC, transient, AC, transfer-function,
  temperature, and noise paths, matching Rust and TypeScript. The
  supported-parameter catalog now contains 104 canonical rows.

- **BJT base-emitter leakage** — BJT model cards now accept `ISE`/`NE` and
  apply the leakage branch in DC, transient, AC, transfer-function,
  temperature, and noise paths, matching Rust and TypeScript. The
  supported-parameter catalog now contains 100 canonical rows.

- **BJT forward high-current beta roll-off** — BJT model cards now accept
  `IKF`/`IK` and apply shared base-charge modulation in DC, transient, AC,
  transfer-function, and noise paths, matching Rust and TypeScript. The
  supported-parameter catalog now contains 96 canonical rows.

- **BJT reverse Early voltage** — BJT model cards now accept `VAR`/`VB` and
  apply reverse Early-effect base-charge modulation in DC, transient, AC,
  transfer-function, and noise paths, matching Rust and TypeScript. The
  supported-parameter catalog now contains 94 canonical rows.

- **BJT forward-bias depletion coefficient** — BJT model cards now accept `FC`
  and apply it to the shared `CJE` and `CJC` Berkeley continuation law in AC and
  transient analysis, matching Rust and TypeScript. The supported-parameter
  catalog now contains 92 canonical rows.

- **BJT base-collector depletion shaping** — BJT model cards now accept
  `VJC`/`PC` junction potential and `MJC`/`MC` grading coefficient parameters
  and apply them to `CJC` in AC and transient analysis, matching Rust and
  TypeScript. The supported-parameter catalog now contains 90 canonical rows.

- **BJT base-emitter depletion shaping** — BJT model cards now accept
  `VJE`/`PE` junction potential and `MJE`/`ME` grading coefficient parameters
  and apply them to `CJE` in AC and transient analysis, matching Rust and
  TypeScript. The supported-parameter catalog now contains 86 canonical rows.

- **BJT reverse emission coefficient** — BJT model cards now accept `NR` and
  apply it to reverse base-collector diffusion charge in AC and transient
  analysis, matching Rust and TypeScript. The supported-parameter catalog now
  contains 82 canonical rows.

- **BJT forward emission coefficient** — BJT model cards now accept `NF` and
  apply it to DC, transient charge, AC, transfer-function, and noise paths,
  matching Rust and TypeScript. The supported-parameter catalog now contains
  80 canonical rows.

- **BJT forward Early voltage** — BJT model cards now accept `VAF`/`VA` and
  apply collector-voltage modulation in DC, transient, AC, transfer-function,
  and noise paths, matching Rust and TypeScript. The supported-parameter
  catalog now contains 78 canonical rows.

- **BJT energy gap** — BJT model cards now accept `EG`, apply each model's
  energy gap to saturation-current temperature scaling, and preserve it through
  subcircuit expansion, matching Rust and TypeScript. The supported-parameter
  catalog now contains 76 canonical rows.

- **BJT saturation-current temperature exponent** — BJT model cards now accept
  `XTI`, apply it to saturation-current temperature scaling, and preserve it
  through subcircuit expansion, matching Rust and TypeScript. The supported-
  parameter catalog now contains 74 canonical rows.

- **Diode energy gap** — diode model cards now accept `EG`, apply it to
  saturation-current temperature scaling, and preserve it through subcircuit
  expansion, matching Rust and TypeScript.

- **Diode saturation-current temperature exponent** — diode model cards now
  accept `XTI`, apply it to saturation-current temperature scaling, and
  preserve it through subcircuit expansion, matching Rust and TypeScript.

- **Diode forward-bias depletion coefficient** — diode model cards now accept
  `FC` and apply the continuous Berkeley piecewise depletion-capacitance law in
  AC and transient analysis, matching Rust and TypeScript.

- **Bias-shaped diode depletion capacitance** — diode model cards now accept
  `VJ`/`PB` junction potential and `M`/`MJ` grading coefficient parameters and
  apply them to depletion capacitance in AC and transient analysis, matching
  Rust and TypeScript.

- **Model-card supported-parameter coverage dashboard** —
  `model_card_supported_parameter_coverage_dashboard()`,
  `format_model_card_supported_parameter_coverage_dashboard_table()`,
  `model_card_supported_parameter_coverage_dashboard_records()`,
  `format_model_card_supported_parameter_coverage_dashboard_csv()`, and
  `format_model_card_supported_parameter_coverage_dashboard_json()` now expose
  per-kind actual versus expected coverage counts plus gate issue fields for
  Mosaic/browser dashboards, matching Rust and TypeScript.

- **Model-card supported-parameter coverage gate** —
  `model_card_supported_parameter_coverage_gate()`,
  `format_model_card_supported_parameter_coverage_gate_report()`,
  `format_model_card_supported_parameter_coverage_gate_issue_table()`,
  `model_card_supported_parameter_coverage_gate_issue_records()`,
  `format_model_card_supported_parameter_coverage_gate_issue_csv()`, and
  `format_model_card_supported_parameter_coverage_gate_issue_json()` now
  validate the expected seven-kind, 72-row supported-parameter catalog and
  expose stable issue rows for release automation, matching Rust and
  TypeScript.

- **Model-card supported-parameter coverage summaries** —
  `model_card_supported_parameter_coverage_summary()`,
  `format_model_card_supported_parameter_coverage_summary_table()`,
  `model_card_supported_parameter_coverage_summary_records()`,
  `format_model_card_supported_parameter_coverage_summary_csv()`, and
  `format_model_card_supported_parameter_coverage_summary_json()` now expose
  per-model-kind supported parameter alias counts as stable compact exports,
  matching Rust and TypeScript.

- **Model-card supported-parameter coverage** —
  `model_card_supported_parameter_coverage()`,
  `format_model_card_supported_parameter_coverage_table()`,
  `model_card_supported_parameter_coverage_records()`,
  `format_model_card_supported_parameter_coverage_csv()`, and
  `format_model_card_supported_parameter_coverage_json()` now expose the
  supported canonical model-card parameters and accepted aliases as stable
  catalog exports, matching Rust and TypeScript.

- **Model-card unsupported-parameter diagnostics** —
  `model_card_unsupported_parameter_issues()`,
  `format_model_card_unsupported_parameter_issue_table()`,
  `model_card_unsupported_parameter_issue_records()`,
  `format_model_card_unsupported_parameter_issue_csv()`, and
  `format_model_card_unsupported_parameter_issue_json()` now expose retained
  unsupported model-card parameters as stable diagnostics, matching Rust and
  TypeScript.

- **Device model reference-deck audit gate coverage digest** —
  `device_model_reference_deck_audit_gate_coverage_digest()`,
  `format_device_model_reference_deck_audit_gate_coverage_digest_table()`,
  `device_model_reference_deck_audit_gate_coverage_digest_records()`,
  `format_device_model_reference_deck_audit_gate_coverage_digest_csv()`, and
  `format_device_model_reference_deck_audit_gate_coverage_digest_json()` now
  expose one-row coverage health exports for release dashboards, matching Rust
  and TypeScript.

- **Device model reference-deck audit gate issue summaries** —
  `device_model_reference_deck_audit_gate_issue_summary()`,
  `format_device_model_reference_deck_audit_gate_issue_summary_table()`,
  `device_model_reference_deck_audit_gate_issue_summary_records()`,
  `format_device_model_reference_deck_audit_gate_issue_summary_csv()`, and
  `format_device_model_reference_deck_audit_gate_issue_summary_json()` now
  expose grouped gate issue counts for reference-deck audit dashboards,
  matching Rust and TypeScript.

- **Berkeley SPICE syntax facade** —
  `parse_berkeley_syntax()` now mirrors the Rust Berkeley logical-card parser
  contract with embedded grammar metadata, normalized continuation cards,
  source spans, token streams, stable diagnostics, and analysis inventory for
  frontend and parser-tooling consumers.

- **Device model reference-deck audit gate issue exports** —
  `format_device_model_reference_deck_audit_gate_issue_table()`,
  `device_model_reference_deck_audit_gate_issue_records()`,
  `format_device_model_reference_deck_audit_gate_issue_csv()`, and
  `format_device_model_reference_deck_audit_gate_issue_json()` now expose
  stable machine-readable issue rows from the reference-deck audit release
  gate, matching Rust and TypeScript.

- **Device model reference-deck audit matrix dashboards** —
  `device_model_reference_deck_audit_matrix()`,
  `format_device_model_reference_deck_audit_matrix_table()`,
  `device_model_reference_deck_audit_matrix_records()`,
  `format_device_model_reference_deck_audit_matrix_csv()`, and
  `format_device_model_reference_deck_audit_matrix_json()` now expose stable
  per-model-family audit dashboard rows with explicit OP, temperature, AC,
  noise, and transient fixture columns, matching Rust and TypeScript.

- **Device model reference-deck audit analysis summaries** —
  `device_model_reference_deck_audit_analysis_summary()`,
  `format_device_model_reference_deck_audit_analysis_summary_table()`,
  `device_model_reference_deck_audit_analysis_summary_records()`,
  `format_device_model_reference_deck_audit_analysis_summary_csv()`, and
  `format_device_model_reference_deck_audit_analysis_summary_json()` now expose
  stable per-analysis coverage summaries for the reference-deck audit matrix,
  matching Rust and TypeScript.

- **Device model reference-deck audit summaries** —
  `device_model_reference_deck_audit_summary()`,
  `format_device_model_reference_deck_audit_summary_table()`,
  `device_model_reference_deck_audit_summary_records()`,
  `format_device_model_reference_deck_audit_summary_csv()`, and
  `format_device_model_reference_deck_audit_summary_json()` now expose stable
  per-kind coverage summaries for the reference-deck audit matrix, matching
  Rust and TypeScript.

- **Device model reference-deck audit record exports** —
  `device_model_reference_deck_audit_records()`,
  `format_device_model_reference_deck_audit_csv()`, and
  `format_device_model_reference_deck_audit_json()` now expose the audit
  matrix as stable header-keyed records plus CSV/JSON outputs, matching Rust
  and TypeScript.

- **Device model reference-deck audit release gate** —
  `device_model_reference_deck_audit_gate()` and
  `format_device_model_reference_deck_audit_gate_report()` now validate the
  required kind-by-analysis coverage matrix and emit a stable pass/fail gate
  report, matching Rust and TypeScript.

- **Device model reference-deck audit table** —
  `format_device_model_reference_deck_audit_table()` now emits a stable
  tab-separated summary for the device-model reference-deck audit matrix,
  matching Rust and TypeScript.

- **Device model reference-deck audit fixtures** —
  `device_model_reference_deck_audit_fixtures()` now exposes a stable
  reference coverage matrix across DC, temperature, AC, noise, and transient
  model-depth fixtures for diode, BJT, JFET, and Level-1 MOS families,
  matching Rust and TypeScript.

- **MOS Level-1 bulk-junction depletion charge shaping** —
  Level-1 MOS `PB`/`MJ` model-card parameters now shape reverse-biased
  `CBS`/`CBD` bulk-junction capacitance for AC operating-point capacitance
  reports and transient source-body / drain-body charge companions, matching
  Rust and TypeScript, with regression coverage for reverse-biased drain-step
  delay.

- **MOS Level-1 transient bulk-junction charge stamping** —
  Level-1 MOS zero-bias bulk-junction `CBS`/`CBD` model-card storage now
  stamps transient source-body and drain-body companions, matching Rust and
  TypeScript, with regression coverage for drain-step delay.

- **MOS Level-1 transient overlap charge stamping** —
  Level-1 MOS `CGSO`/`CGDO`/`CGBO` model-card storage now stamps transient
  gate-source, gate-drain, and gate-body companions, matching Rust and
  TypeScript, with regression coverage for gate-step delay.

- **JFET transient charge stamping** —
  JFET `Cgs`/`Cgd` model-card storage now stamps transient gate-source and
  gate-drain companions and contributes AC susceptance, matching Rust and
  TypeScript, with regression coverage for gate-step delay and high-frequency
  gate-drive shunting.

- **BJT transient charge stamping** —
  BJT `Cje`/`Cjc`/`Tf`/`Tr` model-card storage now stamps transient
  base-emitter and base-collector companions, matching Rust and TypeScript,
  with regression coverage for base current-step delay and forward transit-time
  turnoff charge.

- **Diode transient charge stamping** —
  Diode `Cjo`/`Tt` model-card storage now stamps transient anode-cathode
  companions, matching Rust and TypeScript, with regression coverage for
  junction-capacitance current-step delay and transit-time turnoff charge.

- **Device model charge audit fixtures** —
  `device_model_charge_audit_fixtures()` now exposes runnable one-device
  `.tran` fixtures with reference deck lines, explicit terminal storage
  capacitance metadata, stable first/final probe-voltage windows, and
  charge-behavior notes for diode, BJT, JFET, and Level-1 MOS audits,
  matching Rust and TypeScript.

- **Device model noise audit fixtures** —
  `device_model_noise_audit_fixtures()` now exposes runnable one-device
  `.noise` fixtures with reference deck lines and stable source/output PSD
  windows for diode and BJT shot noise plus JFET and Level-1 MOS channel
  thermal noise audits, matching Rust and TypeScript.

- **Device model capacitance audit fixtures** —
  `device_model_capacitance_audit_fixtures()` now exposes runnable
  one-device AC fixtures with `.ac` reference deck lines and stable
  high-frequency probe-magnitude windows for diode, BJT, JFET, and Level-1 MOS
  model-depth audits, matching Rust and TypeScript.

- **Device model temperature audit fixtures** —
  `device_model_temperature_audit_fixtures()` now exposes runnable one-device
  DC temperature-sweep fixtures with `.temp` reference deck lines and stable
  probe-voltage windows for diode, BJT, JFET, and Level-1 MOS model-depth
  audits, matching Rust and TypeScript.

- **Device model behavior audit fixtures** —
  `device_model_behavior_audit_fixtures()` now exposes runnable one-device DC
  bias fixtures with reference deck lines and stable probe-voltage windows for
  diode, BJT, JFET, and Level-1 MOS model-depth audits, matching Rust and
  TypeScript.

- **Nonlinear Newton damping diagnostics** —
  `dc_op()` now applies a configurable `newton_step_limit` to nonlinear
  Newton updates and reports `newton_step_limit`, `limited_newton_steps`, and
  `minimum_damping_factor` in `DcResult.diagnostics`, matching Rust and
  TypeScript.

- **Production solver profiles** —
  `DcResult.diagnostics` now carries a nested `solver_profile` with matrix
  size, solver kind, backend, structural nonzero count, density, peak fill-in,
  and fallback metadata. Large real DC solves prefer an optional SciPy
  sparse-LU backend and fall back to the native sparse-row solver with a stable
  fallback reason, matching Rust and TypeScript profile surfaces.

- **Deck whole-run analysis execution** —
  `run_deck()` now executes every parsed `.op`, `.dc`, `.ac`, `.tran`,
  `.tf`, `.sens`, and `.noise` card in source order, preserves duplicate
  analysis directives, defaults analysis-less decks to an implicit `.op`, and
  returns aggregate run-artifact table, CSV, compact JSON, and header-keyed
  record exports, matching Rust and TypeScript.

- **Deck output-plan analysis sweep artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose selected
  sweep, frequency, transient timing, and `UIC` metadata in table, CSV, compact
  JSON, and header-keyed record exports, matching Rust and TypeScript.

- **Deck output-plan analysis output-node artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose the selected
  analysis output node beside line/source metadata in table, CSV, compact JSON,
  and header-keyed record exports, matching Rust and TypeScript.

- **Deck output-plan analysis source artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose the selected
  analysis line number and source name beside directive metadata in table, CSV,
  compact JSON, and header-keyed record exports, matching Rust and TypeScript.

- **Deck output-plan result row artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose selected
  result row counts beside result-column inventories in table, CSV, compact
  JSON, and header-keyed record exports, matching Rust and TypeScript.

- **Deck output-plan probe source line artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose selected
  output probe source line counts/lists aligned with the selected output-probe
  inventories in table, CSV, compact JSON, and header-keyed record exports,
  matching Rust and TypeScript.

- **Deck output-plan directive line artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose selected
  output directive source line counts/lists beside directive scope inventories
  in table, CSV, compact JSON, and header-keyed record exports, matching Rust
  and TypeScript.

- **Deck output-plan directive analysis-kind artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose normalized
  output directive analysis scope counts/lists beside directive kind
  inventories, distinguishing global `.save` / `.probe` selections from
  scoped `.probe`, `.print`, and `.plot` selections in table, CSV, compact
  JSON, and header-keyed record exports, matching Rust and TypeScript.

- **Deck output-plan directive-kind artifacts** —
  selected `run_deck_analysis()` output-plan artifacts now expose normalized
  output directive kind counts/lists beside the selected directive tokens in
  table, CSV, compact JSON, and header-keyed record exports, matching Rust and
  TypeScript.

- **Deck output-plan table export artifacts** —
  selected `run_deck_analysis()` executions now include `output-plan` in
  `tables`, selected-run `TableList` metadata, and ordered `table_artifacts`
  with stable table, CSV, compact JSON, and header-keyed record payloads,
  matching Rust and TypeScript.

- **Deck output-plan inventory artifacts** —
  selected `run_deck_analysis()` executions now expose
  `output_plan_artifacts` with stable result-column, output-probe,
  output-directive, and table inventories plus table, CSV, compact JSON, and
  header-keyed record exports, matching Rust and TypeScript.

- **Deck control policy table export artifacts** —
  selected `run_deck_analysis()` executions now include `control-policy` and
  `control-policy-summary` entries in `tables`, selected-run `TableList`
  metadata, and ordered `table_artifacts` with stable table, CSV, compact JSON,
  and header-keyed record payloads, matching Rust and TypeScript.

- **Deck control policy run-artifact inventories** —
  selected `run_deck_analysis()` run artifacts now carry
  `ControlPolicyArtifacts`, `ControlPolicyCategoryList`,
  `ControlPolicyCodeList`, and `ControlPolicySeverityList` summary fields so
  policy-blocked `.control` commands are visible in the stable run-artifact
  table, CSV, compact JSON, and header-keyed record exports, matching Rust and
  TypeScript.

- **Deck control policy summary artifacts** —
  selected `run_deck_analysis()` executions now group policy-blocked `.control`
  command artifacts by category as `control_policy_summary_artifacts` with
  stable counts, line lists, command lists, code lists, severity lists, and
  table, CSV, compact JSON, and header-keyed record exports, matching Rust and
  TypeScript.

- **Deck control policy diagnostic artifacts** —
  selected `run_deck_analysis()` executions now expose policy-blocked
  `.control` commands as `control_policy_artifacts` with stable line,
  category, command, code, severity, and message metadata plus table, CSV,
  compact JSON, and header-keyed record exports, matching Rust and TypeScript.

- **Deck rawfile probe inventory artifacts** —
  selected `run_deck_analysis()` rawfile artifact summaries now carry
  `MatchedProbes` / `MatchedProbeList` and `UnmatchedProbes` /
  `UnmatchedProbeList` columns, and `write <rawfile> <probes...>` artifacts now
  keep only requested matching vector columns in deterministic in-memory
  rawfile output, matching Rust and TypeScript.

- **Deck WRDATA unmatched probe artifacts** —
  selected `run_deck_analysis()` WRDATA artifact summaries now carry
  `MatchedProbes` / `MatchedProbeList` and `UnmatchedProbes` /
  `UnmatchedProbeList` columns so ignored `wrdata` probe names remain
  auditable in stable table, CSV, JSON, and record exports, matching Rust and
  TypeScript.

- **Deck WRDATA probe column artifacts** —
  `format_deck_wrdata_ascii()` now treats explicit `wrdata <file> <probes...>`
  probe lists as data-file column selectors, preserving the scale column plus
  requested matching probe columns in deterministic WRDATA output, matching
  Rust and TypeScript.

- **Deck WRDATA rawfile option rendering artifacts** —
  selected `run_deck_analysis()` WRDATA artifacts now carry accepted
  `.control` rawfile/data-write option inventories through stable
  `Options` / `RawfileOptionList` summary columns and render
  `wr_vecnames` / `wr_singlescale` intent as deterministic `VectorNames` /
  `Scale` metadata in the in-memory data file, matching Rust and TypeScript.

- **Deck WRDATA ASCII artifacts** —
  selected `run_deck_analysis()` executions now expose deterministic in-memory
  ASCII data-file artifacts for accepted `.control` `wrdata <file> ...`
  markers as `wrdata_artifact_count`, `wrdata_artifacts`,
  `wrdata_artifact_table`, `wrdata_artifact_csv`, `wrdata_artifact_json`, and
  `wrdata_artifact_records`, matching Rust and TypeScript.

- **Deck rawfile ASCII artifacts** —
  selected `run_deck_analysis()` executions now expose deterministic in-memory
  ASCII rawfile artifacts for accepted `.control` `write <rawfile> ...`
  markers as `rawfile_artifact_count`, `rawfile_artifacts`,
  `rawfile_artifact_table`, `rawfile_artifact_csv`, `rawfile_artifact_json`,
  and `rawfile_artifact_records`, matching Rust and TypeScript.

- **Deck rawfile option artifacts** —
  `analyze_deck_controls()` and selected `run_deck_analysis()` executions now
  expose normalized accepted `.control` rawfile option inventories as
  `rawfile_option_count` / `rawfile_options`, and selected-run artifacts carry
  stable `RawfileOptions` / `RawfileOptionList` columns through tables,
  CSV/JSON, and ordered `table_artifacts`, matching Rust and TypeScript.

- **Deck rawfile write marker artifacts** —
  `analyze_deck_controls()` and selected `run_deck_analysis()` executions now
  expose normalized accepted `.control` `write` / `wrdata` marker inventories
  as `write_marker_count` / `write_markers`, and selected-run artifacts carry
  stable `WriteMarkers` / `WriteMarkerList` columns through tables, CSV/JSON,
  and ordered `table_artifacts`, matching Rust and TypeScript.

- **Deck execution diagnostic artifacts** —
  `run_deck_analysis()` selected executions now expose selected diagnostic
  inventories directly as `diagnostic_count` / `diagnostic_codes` alongside
  control command, table, output, measurement, Fourier, and analysis-directive
  metadata, matching Rust and TypeScript.

- **Deck execution control command inventory artifacts** —
  `run_deck_analysis()` selected executions now expose normalized `.control`
  command inventories directly as `control_line_count` / `control_lines`
  alongside table, output, measurement, Fourier, and analysis-directive
  metadata, matching Rust and TypeScript.

- **Deck run control command inventory artifacts** —
  `analyze_deck_controls()` now exposes normalized `.control` command lines
  separately from full active deck input, and `run_deck_analysis()`
  selected-run artifacts carry those commands in `ControlLines` /
  `ControlLineList` across stable tables, CSV/JSON helpers, and ordered
  `table_artifacts`, matching Rust and TypeScript.

- **Deck run control diagnostic artifacts** —
  `run_deck_analysis()` selected-run artifacts now include existing `.control`
  body policy diagnostic codes in `Diagnostics` / `DiagnosticCodeList`, and
  those codes flow through stable run-artifact tables, CSV/JSON helpers, and
  ordered `table_artifacts`, matching Rust and TypeScript.

- **Deck execution table export artifacts** —
  `run_deck_analysis()` selected executions now expose ordered
  `table_artifacts` with each stable table's text, CSV, compact JSON, and
  header-keyed records beside the existing table inventory, matching Rust and
  TypeScript.

- **Deck execution table inventory** —
  `run_deck_analysis()` selected executions now expose stable table count/name
  lists beside analysis directives, output probes, output directives, and
  selected-run artifacts, matching Rust and TypeScript.

- **Deck run table artifacts** —
  `run_deck_analysis()` selected-run artifacts now include stable table
  count/name lists and `format_deck_run_artifact_table()` renders `TableList`,
  matching Rust and TypeScript.

- **Deck execution analysis directives** —
  `run_deck_analysis()` now returns the selected analysis directive beside
  selected output probes and output directives, and selected-run artifacts
  include stable `AnalysisDirectiveList` metadata, matching Rust and
  TypeScript.

- **Deck execution output directives** —
  `run_deck_analysis()` now returns selected output directives beside selected
  output probes so callers can inspect the deck output plan without reparsing
  run-artifact tables, matching Rust and TypeScript.

- **Deck table records** —
  `deck_table_records()` now parses stable tab-separated deck output tables into
  header-keyed records for browser and host integrations, matching Rust and
  TypeScript.

- **Deck table JSON format** —
  `format_deck_table_json()` now converts stable tab-separated deck output
  tables into compact JSON records keyed by the header row, matching Rust and
  TypeScript.

- **Deck table CSV format** —
  `format_deck_table_csv()` now converts stable tab-separated deck output
  tables into deterministic CSV using the same escaping rules as selected-run
  artifacts, matching Rust and TypeScript.

- **Deck run artifact JSON format** —
  `format_deck_run_artifact_json()` now renders selected-run artifacts as
  compact JSON records with the same stable keys and normalized cell values as
  `format_deck_run_artifact_table()`, matching Rust and TypeScript.

- **Deck run artifact CSV format** —
  `format_deck_run_artifact_csv()` now renders selected-run artifacts with the
  same stable columns as `format_deck_run_artifact_table()`, using deterministic
  CSV escaping for browser and spreadsheet consumers, matching Rust and
  TypeScript.

- **Deck run Fourier artifact probes** —
  `run_deck_analysis()` selected-run artifacts now include selected Fourier
  probe names alongside the Fourier result count, and
  `format_deck_run_artifact_table()` renders a stable `FourierList` column,
  matching Rust and TypeScript.

- **Deck run measurement artifact names** —
  `run_deck_analysis()` selected-run artifacts now include selected
  measurement names alongside the measurement count, and
  `format_deck_run_artifact_table()` renders a stable `MeasurementList`
  column, matching Rust and TypeScript.

- **Deck run output-probe artifact names** —
  `run_deck_analysis()` selected-run artifacts now include the normalized
  output-probe names alongside the output-probe count, and
  `format_deck_run_artifact_table()` renders a stable `OutputProbeList`
  column, matching Rust and TypeScript.

- **Deck control variable policy diagnostics** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now emit explicit
  policy diagnostics for selected `.control` block variable/state mutation
  commands, including `let`, `alter`, `alterparam`, `set`, and `unset`,
  instead of generic unsupported-command diagnostics, matching Rust and
  TypeScript. Accepted no-op `set` options still route as no-op markers.

- **Deck control-flow policy diagnostics** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now emit explicit
  policy diagnostics for selected `.control` block control-flow commands,
  including `if`, `while`, `foreach`, and `repeat`, instead of generic
  unsupported-command diagnostics, matching Rust and TypeScript. Control-flow
  execution remains disabled by the deck execution policy.

- **Deck control working-directory policy diagnostics** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now emit explicit
  policy diagnostics for selected `.control` block `cd` working-directory
  mutation commands instead of generic unsupported-command diagnostics,
  matching Rust and TypeScript. Working-directory mutation remains disabled by
  the deck execution policy.

- **Deck control script policy diagnostics** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now emit explicit
  policy diagnostics for selected `.control` block `source` and `shell`
  external script/shell commands instead of generic unsupported-command
  diagnostics, matching Rust and TypeScript. External script execution and
  shelling out remain disabled by the deck execution policy.

- **Deck control console marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block read-only `echo`, `rusage`, and `where` console/debug
  commands as no-op control commands instead of reporting unsupported-command
  diagnostics, matching Rust and TypeScript. Actual console/debug output
  remains out of scope for these markers.

- **Deck control introspection marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block read-only `status`, `version`, and `help` UI introspection
  commands as no-op control commands instead of reporting unsupported-command
  diagnostics, matching Rust and TypeScript. Actual console/help output remains
  out of scope for these markers.

- **Deck control show marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block read-only `show` and `showmod` device/model inspection
  commands as no-op control commands instead of reporting unsupported-command
  diagnostics, matching Rust and TypeScript. Actual console/model inspection
  output remains out of scope for these markers.

- **Deck control inspection marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block read-only `display` and `listing` inspection commands as
  no-op control commands instead of reporting unsupported-command diagnostics,
  matching Rust and TypeScript. Actual console/listing output remains out of
  scope for these markers.

- **Deck control WRDATA marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `wrdata <file> <probes...>` ASCII data-write markers as
  no-op control commands instead of reporting unsupported-command diagnostics,
  matching Rust and TypeScript. Actual data-file serialization remains out of
  scope for this marker.

- **Deck control rawfile write marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `write <rawfile> [probes...]` rawfile-write markers as
  no-op control commands instead of reporting unsupported-command diagnostics,
  matching Rust and TypeScript. Rawfile serialization remains out of scope for
  this marker.

- **Deck control appendwrite option routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `set appendwrite` rawfile append-write options as no-op
  control commands instead of reporting unsupported-command diagnostics,
  matching Rust and TypeScript.

- **Deck control rawfile output option routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `set wr_vecnames` and `set wr_singlescale` rawfile output
  toggles as no-op control commands instead of reporting unsupported-command
  diagnostics, matching Rust and TypeScript.

- **Deck control ASCII filetype option routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `set filetype=ascii` output-format options as no-op control
  commands instead of reporting unsupported-command diagnostics, matching Rust
  and TypeScript.

- **Deck control reset marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `reset` session-reset markers as no-op control commands
  instead of reporting unsupported-command diagnostics, matching Rust and
  TypeScript.

- **Deck control noaskquit option routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `set noaskquit` UI options as no-op control commands instead
  of reporting unsupported-command diagnostics, matching Rust and TypeScript.

- **Deck control quit marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `quit` interpreter-exit markers as no-op control commands
  instead of reporting unsupported-command diagnostics, matching Rust and
  TypeScript.

- **Deck control run marker routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now accept selected
  `.control` block `run` execution markers as no-op control commands instead
  of reporting unsupported-command diagnostics, matching Rust and TypeScript.

- **Deck control Fourier routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now normalize
  selected `.control` block `four` and `fourier` harmonic output commands into
  `.four` deck cards, matching Rust and TypeScript.

- **Deck control measurement routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now normalize
  selected `.control` block `measure` and `meas` measurement commands into
  `.measure` and `.meas` deck cards, matching Rust and TypeScript.

- **Deck control save/probe routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now normalize
  selected `.control` block `save` and `probe` output commands into `.save` and
  `.probe` deck cards, matching Rust and TypeScript.

- **Deck control-command routing** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now normalize
  selected `.control` block analysis/output commands (`op`, `dc`, `ac`,
  `tran`, `save`, `probe`, `print`, and `plot`) into dotted deck cards,
  matching Rust and TypeScript.

- **Deck control-block exclusion diagnostics** —
  `analyze_deck_controls()` and `resolve_deck_sources()` now exclude
  unsupported `.control` / `.endc` block markers and unrecognized body
  commands from active deck lines while reporting stable command diagnostics,
  matching Rust and TypeScript.

- **Parsed plot output routing** —
  `resolve_deck_outputs()`, `select_deck_output_probes()`, and
  `format_deck_*_table()` now route scoped `.plot <analysis> ...` output
  cards alongside `.save`, `.probe`, and `.print`, matching Rust and
  TypeScript.

- **Parsed print output routing** —
  `resolve_deck_outputs()`, `select_deck_output_probes()`, and
  `format_deck_*_table()` now route scoped `.print <analysis> ...` output
  cards alongside `.save` and `.probe`, matching Rust and TypeScript.

- **Deck run artifact metadata** —
  `run_deck_analysis()` now returns selected-run artifact summaries and a
  stable run-artifact table with result-row, output-probe, measurement, and
  Fourier counts, matching Rust and TypeScript.

- **Deck Fourier artifact routing** —
  `run_deck_analysis()` now returns selected transient `.four` harmonic
  results and a stable Fourier table alongside the selected plan, solver
  result, output probes, and measurement artifacts, matching Rust and
  TypeScript.

- **Deck measurement artifact routing** —
  `run_deck_analysis()` now returns selected `.measure` / `.meas` results and
  a stable measurement table for selected `.dc`, `.ac`, and `.tran` executions,
  matching Rust and TypeScript.

- **Deck selected-output artifact metadata** —
  `run_deck_analysis()` now returns the normalized deck-selected output probes
  alongside each selected plan, solver result, and stable table, matching Rust
  and TypeScript.

- **Deck transient print-step output routing** —
  `run_deck_analysis()` now keeps `.tran TSTEP` as the stable deck output
  print grid while `MAXSTEP` caps internal solver stepping, matching Rust and
  TypeScript.

- **Deck transient START/MAXSTEP/UIC routing** —
  `run_deck_analysis()` now routes selected `.tran` `START` output filtering,
  `MAXSTEP` fixed-step caps, and `UIC` initial-condition intent through stable
  deck-selected transient tables, matching Rust and TypeScript.

- **Deck AC LIN/OCT execution routing** —
  `run_deck_analysis()` now executes selected `.ac LIN`, `.ac DEC`, and
  `.ac OCT` plans with SPICE-style linear, points-per-decade, and
  points-per-octave frequency grids, matching Rust and TypeScript.

- **Deck analysis execution routing** —
  `run_deck_analysis()` now selects one deck `.op`, `.dc`, `.ac DEC`, or
  `.tran` plan, dispatches it into the matching solver, and returns the
  selected plan, solver result, and deck-selected output table, matching Rust
  and TypeScript.

- **Deck analysis-plan selector** —
  `select_deck_analysis_plan()` now resolves one explicit or implicit deck
  analysis plan with stable ambiguity and invalid-card errors, matching Rust
  and TypeScript.

- **Deck analysis-plan resolver** —
  `resolve_deck_analyses()` now extracts `.op`, `.dc`, `.ac`, and `.tran`
  analysis cards before `.end` into stable metadata with shared diagnostics,
  matching Rust and TypeScript.

- **Parsed save/probe output parity** —
  `resolve_deck_outputs()`, `select_deck_output_probes()`, and the
  `format_deck_*_table()` helpers now route parsed `.save` / `.probe` cards
  into stable operating-point, DC sweep, AC sweep, and transient tables,
  matching Rust and TypeScript.

- **Transient Fourier deck-card routing** —
  `resolve_deck_fourier()`, `fourier_transient_cards()`, and
  `fourier_transient_deck()` now route parsed `.four` / `.FOUR` deck cards
  into SPICE-style Fourier harmonic results with optional `HARMONICS=` and
  `FROM=` controls, matching Rust and TypeScript.

- **Transient TRIG/TARG delay measurement routing** —
  `measure_transient_delay_between_probes()` and parsed transient
  `.measure ... TRIG ... TARG ...` cards now report trigger-to-target delays
  with counted crossing controls, matching Rust and TypeScript.

- **Transient WHEN crossing counters** —
  `measure_transient_when_probe_counted()` and parsed transient
  `.measure ... WHEN probe=target RISE|FALL|CROSS=n` cards now report counted
  threshold occurrences over optional `FROM=` / `TO=` windows, matching Rust
  and TypeScript.

- **Transient WHEN measurement routing** —
  `measure_transient_when_probe()` and parsed transient
  `.measure ... WHEN probe=target` cards now report the first crossing time
  over optional `FROM=` / `TO=` windows, matching Rust and TypeScript.

- **Transient FIND/AT measurement routing** —
  `measure_transient_find_at_probe()` and parsed transient
  `.measure ... FIND ... AT=` cards now sample or linearly interpolate a probe
  value at one scalar time, matching Rust and TypeScript.

- **AC sweep measurement card routing** —
  `measure_ac_sweep_probe()`, `measure_ac_sweep_cards()`, and
  `measure_ac_sweep_deck()` now route direct or parsed `.measure ac` /
  `.meas ac` cards into the shared scalar measurement table surface using
  complex probe magnitudes, matching Rust and TypeScript.

- **DC sweep measurement card routing** —
  `measure_dc_sweep_probe()`, `measure_dc_sweep_cards()`, and
  `measure_dc_sweep_deck()` now route direct or parsed `.measure dc` /
  `.meas dc` cards into the shared scalar measurement table surface, matching
  Rust and TypeScript.

- **Parsed transient measurement card routing** —
  `resolve_deck_measurements()`, `measure_transient_cards()`, and
  `measure_transient_deck()` now extract transient `.measure` / `.meas` cards
  before `.end` and route MAX, MIN, AVG, RMS, peak-to-peak, and final-value
  probe measurements into stable measurement rows, matching Rust and
  TypeScript.

- **Transient measurement output expansion** — `measure_transient_probe()` and
  `format_measurement_table()` provide a shared `.MEASURE`-style scalar
  transient output surface with MAX, MIN, AVG, RMS, peak-to-peak, and
  final-value probe measurements, matching Rust and TypeScript.

- **Initial-condition execution aids** — `dc_initial_vector_from_conditions()`
  maps parsed `.ic` / `.nodeset` node-voltage hints into the DC solver's MNA
  warm-start vector, and `dc_op_with_initial_conditions()` applies those hints
  to operating-point solves with `.ic` values taking precedence over
  `.nodeset`, matching Rust and TypeScript.

- **Deck function-call expression resolution** — `resolve_deck_parameters()`
  now collects scalar `.func` definitions before `.end` and evaluates scalar
  function calls in `.param` assignments plus braced or quoted active-line
  expressions, with stable diagnostics for unknown functions, bad arity, and
  recursive calls, matching Rust and TypeScript.

- **Deck function definition resolution** — `resolve_deck_functions()` now
  extracts scalar `.func name(args) expression` definitions before `.end`,
  strips braced or quoted expression delimiters, and reports stable diagnostics
  for malformed signatures, arguments, duplicate arguments, and empty
  expressions, matching Rust and TypeScript.

- **Deck initial-condition resolution** — `resolve_deck_initial_conditions()`
  now extracts scalar `.ic` and `.nodeset` `V(node)=value` hints before `.end`,
  evaluates numeric SPICE suffix/arithmetic expressions, and reports stable
  diagnostics for malformed targets and unresolved values, matching Rust and
  TypeScript.

- **Deck parameter resolution** — `resolve_deck_parameters()` now evaluates
  scalar whitespace-tokenized `.param` assignments, rewrites braced and quoted
  active-line expressions, and reports stable diagnostics for unresolved
  expressions, matching Rust and TypeScript.

- **Deck source resolution** — `resolve_deck_sources()` now expands
  map-provided `.include` files and selected `.lib path section` library
  sections with stable diagnostics for missing sources, bad sections, cycles,
  and still-unsupported `.control` blocks, matching Rust and TypeScript.

- **Deck boundary diagnostics** — `analyze_deck_controls()` now reports the
  active pre-`.end` deck lines and stable unsupported-feature diagnostics for
  `.include`, `.lib`, and `.control` directives, matching Rust and TypeScript.

- **Remaining stable table parity** — `format_dc_sweep_table()`,
  `format_corner_dc_sweep_table()`, `format_corner_ac_table()`, and
  `format_corner_tf_table()` now close the remaining Rust-first `.DC`, `.AC`,
  and `.TF` named-corner table helper gaps in the Python package.

- **DC corner and temperature parity** — `format_corner_dc_table()`,
  `dc_temperature_sweep()`, `dc_temperature_sweep_corners()`,
  `format_temperature_dc_table()`, and `format_corner_temperature_dc_table()`
  now expose Rust-matching named-corner and `.temp`-style DC operating-point
  snapshots with stable table columns.

- **Compatibility corpus release gates** — `compatibility_corpus()`,
  `release_readiness_gates()`, `format_compatibility_corpus_table()`, and
  `format_release_readiness_report()` expose the first oracle-backed deck
  corpus with golden tolerances and known incompatibility notes shared with
  Rust and TypeScript.

- **Custom-model foothold** — `CustomModel`, `CustomModelEvaluation`,
  `custom_linear_conductance_model()`, and
  `analyze_custom_model_source()` add the first portable two-terminal
  residual/Jacobian hook and Verilog-A subset diagnostics shared with Rust and
  TypeScript.

- **Mixed-signal bridge helpers** — `DigitalEvent`, `DigitalEventStream`,
  `DigitalLogicLevels`, `DigitalThresholds`, digital-stream PWL voltage source
  conversion, fixed/adaptive digital transient bridge runners, named-corner
  bridge wrappers, stable event/schedule tables, and deterministic VCD output
  now match the Rust and TypeScript SPICE bridge surface.

- **Model-card alias normalization** — `normalize_model_card()`,
  `diode_from_model_card()`, `bjt_from_model_card()`,
  `jfet_from_model_card()`, `mosfet_from_model_card()`, and
  `device_model_audit_fixtures()` provide cross-language diode, BJT, JFET, and
  Level-1 MOS `.model` alias fixtures for future deck parsing.

- **Solver diagnostics and sparse complex solves** — `DcResult.diagnostics`
  now reports stable matrix size, solver kind, tolerance, convergence aid, and
  final Newton delta metadata; large AC complex systems now route through the
  sparse-row complex solver path.

- **Distortion and pole-zero named-corner wrappers** —
  `distortion_from_transient_corners()`, `pole_zero_corners()`,
  `format_corner_distortion_table()`, and `format_corner_pole_zero_table()` now
  expose Rust-matching named-corner analysis output for these SPICE helpers.

- **Fourier named-corner wrappers** — `fourier_corners()` and
  `format_corner_fourier_table()` now run `.FOUR`-style harmonic analysis across
  named corner specs, matching Rust output columns for cross-language parity.

- **PSS text output and named-corner wrappers** — `format_pss_table()`,
  `pss_corners()`, and `format_corner_pss_table()` now expose stable
  periodic-steady-state output and named-corner PSS parity with the Rust engine.

- **Transient named-corner wrappers** — `transient_corners()` and
  `transient_adaptive_corners()` now run fixed-step and LTE-adaptive transient
  analyses across named corner specs, with matching stable
  `format_corner_transient_table()` and
  `format_corner_adaptive_transient_table()` output helpers.

- **Multi-corner advanced analysis wrappers** — `mc_dc_corners()`,
  `sens_dc_corners()`, `noise_ac_corners()`, and `s_parameters_corners()` now
  run the corresponding analyses across named corner specs, matching the Rust
  engine surface for these SPICE outputs.

- **Advanced analysis text output tables** — `format_mc_table()`,
  `format_sens_table()`, `format_noise_table()`, and
  `format_s_parameter_table()` now emit stable tab-separated results, with
  matching `format_corner_*` variants for named-corner output.

## [0.14.0] — 2026-06-05

### Added

- **Diode temperature scaling helpers** — `diode_at_temperature()` and
  `circuit_at_temperature()` adjust diode thermal voltage and saturation
  current for an operating temperature using a SPICE-style silicon energy-gap
  foothold.

- **BJT temperature scaling helpers** — `bjt_at_temperature()` and
  `circuit_at_temperature()` adjust BJT thermal voltage and saturation current
  for an operating temperature using the same silicon energy-gap foothold.

- **MOSFET temperature scaling helpers** — `mosfet_at_temperature()` and
  `circuit_at_temperature()` adjust Level-1 MOSFET threshold voltage,
  transconductance parameter, and nominal temperature for an operating
  temperature.

- **Classic text output tables** — `format_dc_table()` and
  `format_transient_table()` now emit stable tab-separated node-voltage and
  branch-current tables for `.OP` / `.TRAN` style snapshots.

- **Pole-zero text output table** — `format_pole_zero_table()` now emits a
  stable tab-separated row set for `.PZ` poles and zeros.

- **Distortion text output table** — `format_distortion_table()` now emits a
  stable tab-separated row set for `.DISTO` harmonic magnitude, phase, and THD
  snapshots.

- **Fourier text output table** — `format_fourier_table()` now emits a stable
  tab-separated row set for `.FOUR` harmonic coefficients, magnitude, phase,
  DC, and THD snapshots.

- **AC text output table** — `format_ac_table()` now emits stable
  tab-separated real, imaginary, magnitude, and phase rows for `.AC` phasor
  snapshots.

- **Transfer-function text output table** — `format_tf_table()` now emits a
  stable tab-separated row for `.TF` gain and impedance snapshots.

- **JFET transient coverage** — source-follower transient fixtures now cover
  JFET participation in nonlinear companion-model solves.

- **Fourier transient analysis** — `fourier()` now computes SPICE-style DC,
  harmonic sine/cosine coefficients, magnitudes, phases, and THD from
  transient samples for `V(node)` and `I(source)` probes.

- **RC high-pass pole-zero helper** — `pole_zero_rc_highpass()` now returns the
  origin zero and RC pole for a constrained first-order high-pass fixture.

- **RLC low-pass pole-zero helper** — `pole_zero_rlc_lowpass()` now returns the
  second-order pole pair for a constrained series R-L / shunt-C low-pass
  fixture.

- **RLC high-pass pole-zero helper** — `pole_zero_rlc_highpass()` now returns
  the double origin zero plus second-order pole pair for a constrained series
  R-C / shunt-L high-pass fixture.

- **RLC band-pass pole-zero helper** — `pole_zero_rlc_bandpass()` now returns
  the origin zero plus second-order pole pair for a constrained series L-C /
  shunt-R band-pass fixture.

- **RLC notch pole-zero helper** — `pole_zero_rlc_notch()` now returns the
  imaginary-axis zero pair plus second-order pole pair for a constrained
  series-R / shunt-series-L-C notch fixture.

- **Transient distortion helper** — `distortion_from_transient()` now runs the
  Fourier extraction path and returns the Phase-8 distortion result shape
  directly from transient samples.

- **MOS Level-1 capacitance models** — `CGSO`, `CGDO`, `CGBO`, `CBS`, and
  `CBD` now contribute MOSFET small-signal AC susceptance.

- **MOSFET channel thermal noise** — `.NOISE` now includes long-channel
  `4kTγgm` channel noise for biased MOSFETs in the per-element breakdown.

- **Diode emission coefficient models** — `Diode.N` now scales the effective
  thermal voltage in DC and small-signal diode conductance calculations.

- **Diode breakdown models** — `Diode.BV` / `Diode.IBV` now add a bounded
  reverse-breakdown current and conductance foothold.

- **Diode junction capacitance models** — `Diode.Cjo` now contributes a
  small-signal AC susceptance in parallel with the linearized diode
  conductance.

- **Diode transit-time models** — `Diode.Tt` now contributes forward-bias
  diffusion capacitance to small-signal AC admittance.

- **BJT capacitance models** — `BJT.Cje` / `BJT.Cjc` now contribute
  base-emitter and base-collector small-signal AC susceptance.

- **BJT transit-time models** — `BJT.Tf` now contributes forward-bias
  diffusion capacitance to small-signal AC admittance.

- **BJT reverse transit-time models** — `BJT.Tr` now contributes
  base-collector diffusion capacitance to small-signal AC admittance.

- **Pseudo-transient DC continuation** — `dc_op` now has a final bounded
  artificial backward-Euler continuation aid after Newton, Gmin stepping, and
  source stepping; successful fallback results report
  `convergence_aid="pseudo_transient"`.

- **DC convergence-aid metadata** — `DcResult` now reports whether the
  operating point came from plain Newton, Gmin stepping, source stepping, or no
  successful convergence aid.

- **Gear-2 damping fixture** — transient tests now cover a coarse LC oscillator
  where Gear-2 damps numerical ringing more aggressively than trapezoidal
  integration.

- **Gear-2 transient companions** — transient analysis now accepts
  `method="gear2"` and uses BDF2 capacitor/inductor companion histories after
  bootstrapping with one backward-Euler step.

- **Transmission-line transient stamping** — `TransmissionLine` now participates
  in transient analysis with a lossless Bergeron delay-line companion model,
  including matched-load delayed step behavior.

- **Transmission-line AC stamping** — `TransmissionLine` now contributes the
  lossless two-port admittance matrix in AC analysis, including matched-load
  phase-delay behavior.

- **Transmission-line element foothold** — `TransmissionLine` now exposes the
  public `T`-card four-terminal delay-line shape for parser and future
  AC/transient stamping work.

- **Mutual-inductor transient stamping** — `MutualInductor` now couples
  referenced inductor pairs during transient analysis with a two-winding
  companion conductance matrix.

- **Mutual-inductor AC stamping** — `MutualInductor` now couples referenced
  inductor pairs in AC analysis using the inverted two-winding inductance
  matrix.

- **Mutual-inductor element foothold** — `MutualInductor` now exposes the
  public `K`-card coupling shape for future coupled-inductor AC/transient
  stamping.

- **JFET DC/AC analysis foothold** — `JFET` now participates in nonlinear DC
  operating-point solves and AC small-signal analysis using the DC bias point.

- **JFET element foothold** — `JFET` now exposes the public three-terminal
  device shape needed by SPICE `J` cards; nonlinear analysis stamping follows
  in a later compatibility slice.

- **PSS analysis foothold** — `pss` now runs the bounded shooting-Newton solve
  and returns one steady-state transient period from the solved circuit.

- **PSS Newton solve foothold** — `pss_newton_solve` now runs bounded accepted
  Newton iterations until residual convergence, no improvement, or the
  iteration cap.

- **PSS Newton iteration foothold** — `pss_newton_iteration` now runs one
  candidate update, accepts it only when the residual L2 norm does not
  increase, and reports the retained circuit/state for the next shooting step.

- **PSS Newton candidate foothold** — `pss_newton_candidate` now applies one
  least-squares Newton update to reactive initial conditions and reports the
  candidate circuit plus its refreshed one-period residual.

- **PSS Newton update foothold** — `pss_newton_update` now solves a
  least-squares Newton correction from the finite-difference residual
  Jacobian for reactive initial-condition updates.

- **PSS residual Jacobian foothold** — `pss_residual_jacobian` now reports a
  forward finite-difference Jacobian from reactive initial conditions to the
  ordered residual vector for future shooting-Newton updates.

- **PSS residual vector norms** — `pss_residual` now reports L2 and RMS
  norms over the ordered node-then-branch residual vector for future
  shooting-Newton convergence checks.

- **PSS ordered residual vector** — `pss_residual` now exposes a stable
  node-then-branch residual vector for future shooting-Newton solves.

- **PSS branch-current residuals** — `pss_residual` now includes one-period
  branch-current closure alongside node-voltage closure.

- **PSS residual convergence flag** — `pss_residual` now accepts a residual
  tolerance and reports whether one-period node closure is within tolerance.

- **PSS period-closure residual** — `pss_residual` runs one estimated source
  period and reports node-voltage closure residuals as the next foothold for
  shooting-Newton periodic steady-state analysis.

- **PSS source-period estimation** — `waveform_period` reports periodic `SIN`
  and `PULSE` source periods, and `estimate_period` derives a harmonic common
  period across independent source waveforms as a foothold for shooting-Newton
  periodic steady-state analysis.

- **Multi-corner transfer-function analysis** — `tf_corners` runs the same
  `.TF` query at each named corner and returns per-corner gain and impedance
  values.

- **Multi-corner AC frequency sweeps** — `ac_sweep_corners` runs the same AC
  frequency grid at each named corner and returns per-corner phasor responses.

- **Multi-corner DC source sweeps** — `dc_sweep_corners` runs the same
  independent-source sweep at each named corner and returns per-corner
  `.DC` traces.

- **Multi-corner DC operating point sweeps** — `dc_corners` runs named corner
  specs with element-parameter overrides for core linear parameters.

- **Two-port S-parameter extraction** — `s_parameters` derives a two-port
  Y-parameter matrix from named AC voltage-source ports and converts it to
  S11/S21/S12/S22 for a configurable reference impedance.

- **Sparse real solver path** — large DC / real small-signal matrices now route
  through a sparse-row Gaussian elimination path while small systems keep the
  dense solver.

- **Programmatic subcircuits** — `SubcircuitDefinition` plus `XInstance`
  let callers define reusable cells and expand each instance into namespaced
  primitive elements before simulation.

## [0.13.0] — 2026-05-16

### Added

- **Behavioral B sources** — `BSource` adds DC behavioral current and voltage
  sources with arithmetic expressions over constants and node-voltage
  references `V(node)` / `V(node1,node2)`.

- **Inductor initial current** — `Inductor` now accepts an `initial_current`
  value that seeds transient analysis companion models.

- **Explicit AC source phasors** — `VoltageSource` and `CurrentSource` now
  accept an optional `AcSource(magnitude, phase_degrees=0.0)` value.  AC
  analysis uses those phasors independently from the DC bias value, and zeros
  unspecified independent sources once any explicit AC source is present.

## [0.12.0] — 2026-05-13

### Added

- **DC convergence aids** — `dc_op` now implements a three-stage fallback chain
  that mirrors the SPICE3 approach for hard-to-converge circuits:

  1. **Plain Newton-Raphson** (existing behaviour, always tried first).
  2. **Gmin stepping** (`_dc_gmin_step`) — adds a small conductance `gmin` from
     every non-ground node to ground, logarithmically sweeping `gmin` from
     `1e-3 S` down to `1e-12 S` then removing it entirely.  Each step
     warm-starts from the previous converged solution.  The regularisation
     prevents the MNA matrix from becoming singular during early Newton
     iterations on strongly nonlinear circuits.
  3. **Source stepping** (`_dc_source_step`) — scales every independent voltage
     source and current source from `0` to their full value in `n_steps`
     equal increments.  Warm-starts each step from the previous converged
     solution.  Effective when Gmin stepping alone fails.

  Both aids are transparent for circuits that already converge with plain Newton
  (the result is identical).  The fallback chain is entered only when the
  previous stage diverges.

  **New `convergence_aids` parameter on `dc_op`:**
  ```python
  dc_op(circuit, convergence_aids=True)   # default — enable full chain
  dc_op(circuit, convergence_aids=False)  # raw Newton only (previous behaviour)
  ```

- **`_dc_newton` private helper** — the Newton-Raphson inner loop is now
  exposed as `_dc_newton(circuit, *, max_iterations, tol, x_init=None)`.
  The optional `x_init` argument warm-starts the solver from a previously
  converged state, enabling efficient multi-step convergence aids.

- **`_x_from_result` private helper** — reconstructs the raw MNA `x` vector
  (node voltages + branch currents) from a `DcResult`, so it can be passed
  as `x_init` to the next `_dc_newton` call.

### Implementation notes

- Gmin resistors reference only existing circuit nodes; the MNA variable
  ordering is identical between the original and augmented circuits, so no
  index remapping is needed when warm-starting across Gmin steps.
- Source stepping skips `waveform`-bearing sources (DC/AC analysis ignores
  waveforms) and also skips controlled sources (VCVS, VCCS, CCCS, CCVS).
- `iterations` field of `DcResult` now reflects the Newton iterations used
  in the *final* successful stage (plain Newton, Gmin, or source step).

---

## [0.11.0] — 2026-05-13

### Added

- **Time-varying source waveforms** — four SPICE3 transient source forms are
  now first-class elements, usable with both `VoltageSource` and
  `CurrentSource` via the new optional `waveform` field:

  | Class | SPICE keyword | Description |
  |-------|--------------|-------------|
  | `PwlWaveform` | `PWL` | Piecewise-linear; linearly interpolates between `(time, value)` breakpoints |
  | `SinWaveform` | `SIN` | Sinusoidal with optional DC offset, frequency, delay, and exponential damping |
  | `PulseWaveform` | `PULSE` | Trapezoidal pulse train with configurable delay, rise/fall times, pulse width, and period |
  | `ExpWaveform` | `EXP` | Double-exponential (rising then falling) with independent rise/fall delays and time constants |

  All four waveform classes are frozen dataclasses whose `__call__(t)` method
  returns the waveform value at simulation time `t`.  A `Waveform` union type
  alias is also exported for type annotations.

  Usage:
  ```python
  from spice_engine import (
      Circuit, VoltageSource, CurrentSource, Resistor, Capacitor,
      SinWaveform, PulseWaveform, PwlWaveform, ExpWaveform,
      transient,
  )

  # 1 V sinusoidal source at 1 kHz driving an RC filter
  c = Circuit([
      VoltageSource("Vin", "in", "0", voltage=0.0,
                    waveform=SinWaveform(amplitude=1.0, frequency=1e3)),
      Resistor("R1", "in", "out", 1e3),
      Capacitor("C1", "out", "0", 100e-9),
  ])
  result = transient(c, t_stop=2e-3, t_step=1e-6)
  ```

- **Engine: time-aware companion circuit construction** — `_build_transient_companions`
  now accepts a `t: float` parameter (current simulation time, default `0.0`).
  At each timestep, any `VoltageSource` or `CurrentSource` with a non-`None`
  `waveform` is replaced in the companion circuit with a static copy whose
  `voltage`/`current` is evaluated at `t`.  Sources without a waveform are
  unchanged.

- **Engine: waveform evaluation at t = 0** — the initial-condition circuit
  (used to establish the DC operating point at `t = 0`) also evaluates
  waveforms at `t = 0`, so the initial bias correctly reflects the waveform
  value at simulation start.

## [0.10.0] — 2026-05-12

### Added

- **Four controlled (dependent) source elements** — the full set of SPICE
  E/G/F/H elements is now available:

  | Class | SPICE letter | Type | Controlling quantity |
  |-------|-------------|------|----------------------|
  | `VCVS` | E | Voltage-Controlled Voltage Source | `V(ctrl+) − V(ctrl−)` |
  | `VCCS` | G | Voltage-Controlled Current Source | `V(ctrl+) − V(ctrl−)` |
  | `CCCS` | F | Current-Controlled Current Source | `I(ctrl_source)` |
  | `CCVS` | H | Current-Controlled Voltage Source | `I(ctrl_source)` |

  All four sources work across every analysis type: DC operating point,
  DC sweep, AC sweep, transient, `.TF`, `.SENS`, `.MC`, and `.NOISE`.

  **VCVS** (`E` element) — voltage follower / amplifier:
  ```
  V(n_plus, n_minus) = gain × [V(ctrl_plus) − V(ctrl_minus)]
  ```
  Introduces a new MNA branch unknown (like `VoltageSource`).  Used to
  model op-amps, buffers, and any ideal voltage amplifier.

  **VCCS** (`G` element) — transconductance amplifier:
  ```
  I(n_plus → n_minus) = gm × [V(ctrl_plus) − V(ctrl_minus)]
  ```
  No branch unknown needed; stamps off-diagonal entries in the MNA G
  matrix.  Identical primitive to the internal `gm` stamp used for
  MOSFETs and BJTs.  Used for op-amp macromodels, FET small-signal
  models, etc.

  **CCCS** (`F` element) — current mirror / current amplifier:
  ```
  I(n_plus → n_minus) = beta × I(ctrl_source)
  ```
  The controlling current is the branch current of a named
  `VoltageSource` (use a 0 V source as an ideal ammeter).  No new
  branch unknown.  Node convention: positive current flows FROM
  `n_plus` through the external circuit TO `n_minus` (SPICE standard).

  **CCVS** (`H` element) — transresistance amplifier:
  ```
  V(n_plus, n_minus) = transresistance × I(ctrl_source)
  ```
  Introduces a new MNA branch unknown.  Used to model transimpedance
  amplifiers, current-to-voltage converters, etc.

- **`TfResult.gain` property** — convenient alias for
  `TfResult.transfer_ratio` (dimensionless voltage gain when the input
  is a `VoltageSource`).  Both names are now valid; `transfer_ratio` is
  retained for backward compatibility.

### Fixed

- **CCCS MNA stamp sign** — the previous stamp had reversed polarity:
  it injected current at `n_minus` instead of `n_plus`, contradicting
  both the docstring (`I(n_plus → n_minus) = beta × I_ctrl`) and the
  SPICE `F` element convention.  The corrected stamp uses `−beta` at
  `n_plus` and `+beta` at `n_minus` so that current correctly exits
  `n_plus` into the external circuit.  Existing circuits that worked
  around the old sign by swapping n_plus/n_minus should be updated to
  use the standard SPICE node order.

---

## [0.9.0] — 2026-05-08

### Added

- **`noise_ac()` function** — Small-signal noise analysis (the SPICE `.NOISE` command).

  Computes the voltage noise power spectral density (PSD) at a chosen output
  node due to **thermal** (Johnson-Nyquist) and **shot** noise in every circuit
  element, at each frequency in a user-supplied or default sweep.  Also reports
  **input-referred noise** — the equivalent input noise that would produce the
  same output noise — for direct comparison to the signal level.

  **Physics modelled:**

  | Element | Noise model | PSD formula |
  |---|---|---|
  | `Resistor` | Thermal (Johnson-Nyquist) | `S_i = 4kT/R` A²/Hz |
  | `Diode` | Shot noise | `S_i = 2q|I_D|` A²/Hz |
  | `BJT` | Shot noise (B-E junction) | `S_i = 2q|I_C|` A²/Hz |
  | `Capacitor`, `Inductor`, `VoltageSource`, `CurrentSource`, `Mosfet` | Noiseless | — |

  **Algorithm — adjoint method (one solve per frequency, not N):**

  At each frequency ω = 2πf:
  1. Build complex AC MNA matrix G(jω) (identical to `ac_sweep` linearisation).
  2. Solve the *adjoint* system: G(jω)^T × **v** = **e**_out (unit vector at output node).
  3. For each noise source k between nodes (a, b):
     - Transfer impedance: H_k = v[a] − v[b]
     - Output contribution: S_out_k = |H_k|² × S_k
  4. Total: S_out = Σ_k S_out_k
  5. Input-referred: S_in = S_out / |H_signal|², where H_signal is the adjoint-
     derived transfer from `input_source` to `output_node`.

  The adjoint method is O(N) in the number of noise sources, versus O(N²) for
  direct injection of each source separately.  For a 10-resistor circuit with
  one matrix solve per frequency, the saving is 10×.

  **Signature:**
  ```python
  noise_ac(
      circuit: Circuit,
      output_node: str,
      input_source: str,
      freqs: list[float] | None = None,  # default: 50 log-pts, 1 Hz – 1 MHz
      *,
      temperature: float = 300.0,  # Kelvin
      max_iterations: int = 50,
      tol: float = 1e-6,
  ) -> NoiseResult
  ```

  **Example — RC filter noise:**
  ```python
  from spice_engine import Circuit, VoltageSource, Resistor, noise_ac
  import math

  c = Circuit()
  c.add(VoltageSource("Vin", "in", "0", 0.0))
  c.add(Resistor("R1", "in", "out", 1000.0))
  c.add(Resistor("R2", "out", "0", 1000.0))

  result = noise_ac(c, "out", "Vin", temperature=300.0)
  pt = result.points[0]
  print(f"Output noise: {math.sqrt(pt.output_psd)*1e9:.2f} nV/√Hz")
  # → ≈ 2.88 nV/√Hz  (= sqrt(4kT × 500 Ω) × 1e9)
  ```

- **`NoiseEntry` frozen dataclass** — contribution from one element at one frequency:
  - `element_name: str`
  - `noise_type: str` — `"thermal"` or `"shot"`
  - `source_psd: float` — A²/Hz (the element's own current noise PSD)
  - `output_psd: float` — V²/Hz (contribution to output after transfer)

- **`NoisePoint` frozen dataclass** — noise result at one frequency:
  - `freq: float` — Hz
  - `output_psd: float` — V²/Hz (total output noise spectral density)
  - `input_referred_psd: float` — V²/Hz (input-referred noise)
  - `entries: tuple[NoiseEntry, ...]` — per-element breakdown, sorted by
    `output_psd` descending (loudest contributor first)

- **`NoiseResult` dataclass** — complete `.NOISE` analysis:
  - `output_node: str`
  - `input_source: str`
  - `temperature: float` — Kelvin
  - `points: list[NoisePoint]` — one per frequency, ascending order

### Changed

- `pyproject.toml` version bumped `0.8.0` → `0.9.0`.
- `__init__.py` description updated; `NoiseEntry`, `NoisePoint`, `NoiseResult`,
  `noise_ac` added to imports and `__all__`.

### Tests

- **Sections 46–52** added (50 new tests; 177 → 227 total):
  - Section 46: dataclass type checks, field values, noise_type strings.
  - Section 47: analytical Nyquist verification — `S_out = 4kT × R_eq` for a
    symmetric resistor divider; white spectrum (flat PSD at all frequencies
    for resistor-only circuits); temperature-linear scaling.
  - Section 48: per-element breakdown count, entries sorted loudest-first,
    sum of entries equals total PSD, symmetric equal contributions,
    asymmetric divider (larger R wins), 3-resistor T-network entry count.
  - Section 49: input-referred formula `S_in = S_out / |H|²`; attenuator
    produces `S_in > S_out`; unknown source gives 0 input-referred PSD;
    inverse gain dependence; CurrentSource input path.
  - Section 50: diode and BJT shot noise — type string, positive source_psd,
    monotone increase with bias current.
  - Section 51: default sweep (50 points, 1 Hz – 1 MHz, ascending); custom
    freq list respected; single-point sweep; all default PSDs positive.
  - Section 52: unknown/ground output node → empty points; empty freqs list;
    Capacitor/Inductor/VoltageSource noiseless; output_psd ≥ 0; frozen
    dataclass immutability.

- Coverage: 82.25% (v0.8.0) → **83.90%** (v0.9.0).

### Rationale for adjoint method

The forward approach would require one linear solve per noise source.  For a
circuit with N noisy elements and F frequency points, that is N × F solves.
The adjoint method solves G^T × **v** = **e**_out *once* per frequency and then
reads off all N transfer impedances as v[a] − v[b].  For a 20-element circuit
and 50 frequencies, the saving is 20× in linear-algebra work.

The adjoint relationship G × x = b ↔ G^T × v = e_out comes from the identity:
x[out] = e_out^T G^{-1} b = (G^{-T} e_out)^T b = v^T b for any b.
This holds for non-symmetric matrices (circuits with MOSFETs and BJTs).

## [0.8.0] — 2026-05-08

### Added

- **`mc_dc()` function** — Monte Carlo analysis (the SPICE `.MC` command).

  Runs N independent DC operating-point trials, randomly varying every
  element's tunable parameter by ±`tolerance` each trial.  Collects the
  distribution of the output-node voltage and reports sample statistics.

  Two sampling distributions are supported:

  | Distribution | Draw formula | σ equiv |
  |---|---|---|
  | `"gaussian"` (default) | `nominal × (1 + N(0, tol/3))` | `tol` = 3σ → 99.73% coverage |
  | `"uniform"` | `nominal × Uniform(1−tol, 1+tol)` | flat |

  **Signature:**
  ```python
  mc_dc(
      circuit: Circuit,
      output_node: str,
      n_trials: int = 100,
      *,
      tolerance: float = 0.05,          # fractional (0.05 = 5%)
      distribution: str = "gaussian",   # "gaussian" | "uniform"
      seed: int | None = None,          # RNG seed for reproducibility
      max_iterations: int = 50,
      tol: float = 1e-6,
  ) -> McResult
  ```

  **Elements varied per trial:**

  | Element | Parameter varied |
  |---------|-----------------|
  | `Resistor` | `resistance` (Ω) |
  | `VoltageSource` | `voltage` (V) |
  | `CurrentSource` | `current` (A) |
  | `Diode` | `Is` (A) |
  | `BJT` | `Is` (A) and `beta_f` (dimensionless) |
  | `Capacitor` | skipped — no DC effect |
  | `Inductor` | skipped — no DC effect |
  | `Mosfet` | skipped — model not exposed |

  **Example — resistor divider with 5% Gaussian variation:**
  ```python
  from spice_engine import Circuit, VoltageSource, Resistor, mc_dc

  c = Circuit()
  c.add(VoltageSource("Vin", "in", "0", 10.0))
  c.add(Resistor("R1", "in", "mid", 1000.0))
  c.add(Resistor("R2", "mid", "0", 1000.0))

  result = mc_dc(c, "mid", n_trials=1000, tolerance=0.05, seed=42)
  print(f"mean={result.mean:.3f} V, σ={result.std_dev:.4f} V")
  # typical output: mean=5.001 V, σ=0.0982 V
  ```

- **`McPoint` dataclass** — immutable snapshot for one Monte Carlo trial:
  - `trial: int` — 0-based trial index
  - `node_voltages: dict[str, float]` — all node voltages for this trial
  - `branch_currents: dict[str, float]` — all branch currents for this trial
  - `converged: bool`

- **`McResult` dataclass** — collection returned by `mc_dc`:
  - `output_node: str`
  - `points: list[McPoint]` — one entry per trial (including non-converged)
  - `n_trials: int`
  - `mean: float` — sample mean over converged trials
  - `std_dev: float` — sample standard deviation (N−1 denominator) over converged trials

### Changed

- `__version__` bumped `0.7.0` → `0.8.0`.
- Module docstring updated to include Monte Carlo analysis.
- `pyproject.toml` description updated to include `.MC`.

### Tests (23 new, Sections 40–45)

| Section | Description |
|---------|-------------|
| 40 | `McPoint` and `McResult` dataclass types and field names |
| 41 | Gaussian variation: `mean` near nominal for symmetric divider |
| 42 | `std_dev > 0` when tolerance > 0; `std_dev` scales with tolerance |
| 43 | Seed reproducibility: identical results for same seed |
| 44 | Uniform distribution: mean close to nominal; std_dev independent check |
| 45 | Error cases: invalid node, invalid distribution, `n_trials < 1`, `tolerance < 0` |

Total: 177 tests, 82% coverage.

---

## [0.7.0] — 2026-05-08

### Added

- **`sens_dc()` function** — DC sensitivity analysis (the SPICE `.SENS` command).

  Computes how sensitive the DC voltage at a named output node is to small
  changes in each element's tunable parameter, using forward finite differences.
  For each `(element, parameter)` pair:

  ```
  sensitivity     = ∂V_out / ∂P  ≈  (V_out(P + δ) − V_out(P)) / δ
  rel_sensitivity = (P / V_out) × ∂V_out/∂P   (dimensionless)
  ```

  **Signature:**
  ```python
  sens_dc(
      circuit: Circuit,
      output_node: str,
      *,
      max_iterations: int = 50,
      tol: float = 1e-6,
      perturbation: float = 1e-3,   # relative δ: δ = max(|P| × 0.001, 1e-10)
      abs_floor: float = 1e-10,
  ) -> SensResult
  ```

  **Parameters perturbed per element type:**

  | Element | Parameter |
  |---------|-----------|
  | `Resistor` | `resistance` (Ω) |
  | `VoltageSource` | `voltage` (V) |
  | `CurrentSource` | `current` (A) |
  | `Diode` | `Is` (A) — reverse saturation current |
  | `BJT` | `Is` (A) and `beta_f` (dimensionless) |
  | `Capacitor` | skipped — no DC effect |
  | `Inductor` | skipped — no DC effect |
  | `Mosfet` | skipped — model introspection not yet exposed |

  **Interpreting results:**
  - `rel_sensitivity = 0.5` → a 1% increase in P causes a 0.5% increase in V_out.
  - `rel_sensitivity = -1.0` → a 1% increase in P causes a 1% decrease in V_out.
  - Entries are sorted by `abs(rel_sensitivity)` descending so the most influential
    components appear first.

  **Example — resistor divider:**
  ```python
  from spice_engine import Circuit, VoltageSource, Resistor, sens_dc

  c = Circuit()
  c.add(VoltageSource("Vin", "in", "0", 10.0))
  c.add(Resistor("R1", "in", "mid", 1000.0))
  c.add(Resistor("R2", "mid", "0", 1000.0))

  result = sens_dc(c, "mid")
  # result.nominal_voltage = 5.0
  # Vin  rel_sensitivity ≈ +1.00  (V_mid tracks V_in)
  # R1   rel_sensitivity ≈ −0.50  (increasing R1 lowers V_mid)
  # R2   rel_sensitivity ≈ +0.50  (increasing R2 raises V_mid)
  ```

- **`SensEntry` dataclass** — immutable result for one `(element, parameter)` pair:
  - `element_name: str`
  - `parameter: str` — `"resistance"`, `"voltage"`, `"current"`, `"Is"`, `"beta_f"`
  - `nominal_value: float` — unperturbed parameter value
  - `sensitivity: float` — absolute ∂V_out/∂P in V/unit(P)
  - `rel_sensitivity: float` — dimensionless (P/V_out) × ∂V_out/∂P

- **`SensResult` dataclass** — collection returned by `sens_dc`:
  - `output_node: str`
  - `nominal_voltage: float`
  - `entries: list[SensEntry]` sorted by `|rel_sensitivity|` descending
  - `converged: bool`

### Changed

- `__version__` bumped `0.6.0` → `0.7.0`.
- Module docstring updated to include `.SENS` analysis.

### Tests (26 new, Section 33–39)

| Section | Description |
|---------|-------------|
| 33 | Dataclass types, nominal voltage, converged flag |
| 34 | Resistor-divider analytical verification (equal and asymmetric) |
| 35 | Voltage-source (rel = 1.0) and current-source sensitivities |
| 36 | Nonlinear Diode Is sensitivity (positive direction) |
| 37 | BJT Is and beta_f both present; beta negative on collector |
| 38 | Sorted-descending ordering; Vin dominates; nominal_value stored |
| 39 | Invalid node ValueError; ground alias; skipped C/L; clamped-node R |

Total: 154 tests, 82% coverage.

---

## [0.6.0] — 2026-05-08

### Added

- **`dc_sweep()` function** — DC parameter sweep analysis (the SPICE `.DC` command).

  Steps one independent source (`VoltageSource` or `CurrentSource`) through a
  user-specified range `[start, stop]` with increment `step` and records a full
  DC operating-point snapshot at each step.  This enables transfer-curve
  measurements, bias-point sensitivity analysis, and DC load-line characterisation.

  **Signature:**
  ```python
  dc_sweep(
      circuit: Circuit,
      source_name: str,
      start: float,
      stop: float,
      step: float,
      *,
      max_iterations: int = 50,
      tol: float = 1e-6,
  ) -> DcSweepResult
  ```

  **Algorithm:**
  1. Validate that `step != 0`; locate the named source element in the circuit.
  2. Build the sweep-value list using integer-counted steps to avoid floating-point
     drift (`start + i * step` for i = 0, 1, …); stop value is included within
     half-step tolerance.  Wrong-sign steps silently produce an empty result list.
  3. For each sweep value:
     a. Create a **new** source element with the swept value (frozen dataclasses
        cannot be mutated; the original circuit is never modified).
     b. Rebuild the circuit with the new element in place of the original.
     c. Call `dc_op` on the modified circuit.
     d. Append a `DcSweepPoint` capturing `source_value`, `node_voltages`,
        `branch_currents`, and `converged`.
  4. Return `DcSweepResult(points=[…], source_name=source_name)`.

  **Why integer-counted steps:**  Floating-point addition accumulates error.
  After 100 steps of 0.1 V, `0.1 * 100 == 10.0` exactly in IEEE-754, but
  `sum(0.1 for _ in range(100))` drifts to ~9.99999…  Integer multiplication is
  exact and avoids accumulating any ULP error.

  **Original circuit immutability:** All elements are `frozen=True` dataclasses.
  To "change" a value dc_sweep creates a new instance and rebuilds the element
  list; the caller's `Circuit` object remains unchanged.

- **`DcSweepPoint` dataclass** — frozen snapshot of one DC operating point during
  a parameter sweep.
  - `source_value: float` — swept source value at this step (V or A).
  - `node_voltages: dict[str, float]` — DC node voltages (V), ground excluded.
  - `branch_currents: dict[str, float]` — DC branch currents (A) for all
    voltage sources, keyed by source name.
  - `converged: bool` — `True` when Newton-Raphson converged.

- **`DcSweepResult` dataclass** — collected results from `dc_sweep()`.
  - `points: list[DcSweepPoint]` — one entry per evaluated step, in sweep order.
  - `source_name: str` — name of the swept source.

### Tests added (sections 28-32)

| Section | Coverage |
|---------|----------|
| 28 | `DcSweepPoint` / `DcSweepResult` dataclass fields, frozen semantics, public API export |
| 29 | Linear resistive circuits: voltage-divider ratio, step sequence, circuit immutability, descending sweep, wrong-sign empty result, single-step, branch current recording, three-node ladder |
| 30 | Nonlinear diode circuit: all-converged forward-bias sweep, monotone-increasing cathode voltage |
| 31 | Current-source sweeps: Ohm's law at each step, descending current sweep |
| 32 | Error cases: zero step, missing source name, resistor (not a source) |

### Changed

- `__version__` bumped from `0.5.0` → `0.6.0`.
- Module docstring updated to mention DC sweep.

---

## [0.5.0] — 2026-05-08

### Added

- **`tf()` function** — DC small-signal transfer function analysis (the SPICE `.TF` command).

  **What it computes:**
  Given a circuit, one driving independent source (`input_source`), and one
  output node (`output_node`), `.TF` returns three DC small-signal quantities:

  | Quantity | Symbol | Definition |
  |---|---|---|
  | Transfer ratio | H | V_output / V_input (VoltageSource) or V_output / I_input (CurrentSource, transimpedance in Ω) |
  | Input impedance | Z_in | Thevenin impedance looking into the input port (Ω) |
  | Output impedance | Z_out | Thevenin impedance looking back from the output node (Ω) |

  **Algorithm (four steps):**
  1. **DC operating point** — run `dc_op` to bias all nonlinear devices (Diode,
     MOSFET, BJT).  This gives the linearisation point for the small-signal matrix.
  2. **Small-signal conductance matrix G_ss** — build a real (ω = 0) MNA matrix
     via `_build_ss_matrix`.  Independent sources are zeroed (only structural
     KVL/KCL entries remain for `VoltageSource`; `CurrentSource` is skipped
     entirely).  Reactive elements: Capacitor → open; Inductor → near-short
     (G = 1e12 S).  Nonlinear devices → small-signal conductances at the DC OP.
  3. **Forward solve** — apply a unit excitation at the input source while all
     other sources are zeroed; solve `G_ss · x_fwd = b_fwd`:
     - `VoltageSource` input: `b_fwd[branch] = 1.0` (1 V excitation);
       `H = x_fwd[output_idx]`;  `Z_in = -1 / x_fwd[branch]` (branch current is
       negative when the source delivers current — MNA stamp convention).
     - `CurrentSource` input: `b_fwd[n_plus] -= 1`, `b_fwd[n_minus] += 1` (1 A);
       `H = x_fwd[output_idx]`;  `Z_in = V_n_minus − V_n_plus` (compliance voltage).
  4. **Output impedance solve** — same G_ss, inject 1 A at `output_node`:
     `b_test[output_idx] = 1.0`;  `Z_out = x_test[output_idx]` (V/A = Thevenin Ω).

  **Why branch current is negative for VoltageSource:**
  The MNA stamp `G[n_plus][branch] = 1` places `x[branch]` in the KCL row for
  n_plus with coefficient +1.  For a resistive load:
  `(1/R)·V_n_plus + x[branch] = 0` → `x[branch] = −I_delivered`.
  So `Z_in = V_in / I_delivered = 1 / (−x[branch])`.

- **`TfResult` dataclass** — frozen result type for `tf()`.
  - `transfer_ratio: float` — V_out/V_in or V_out/I_in (transimpedance).
  - `input_impedance: float` — Thevenin input impedance (Ω).
  - `output_impedance: float` — Thevenin output impedance (Ω).
  - `converged: bool` — mirrors the DC operating-point convergence flag.

- **`_build_ss_matrix` helper** — builds the real DC small-signal MNA matrix.
  Stamping rules per element type:

  | Element | Stamp |
  |---|---|
  | Resistor R | conductance G = 1/R |
  | Capacitor | open circuit (skipped) |
  | Inductor | near-short G = 1e12 S |
  | VoltageSource | KVL/KCL structural entries only (b not set) |
  | CurrentSource | skipped (independent source → zero) |
  | Diode | gd = (Is/Vt)·exp(Vd/Vt) at DC OP |
  | MOSFET | gds + gm VCCS at DC OP |
  | BJT | g_π + gm VCCS at DC OP |

### Changed

- `__version__` bumped from `0.4.0` to `0.5.0`.
- `pyproject.toml` description updated to include DC transfer function analysis.

### Tests

27 new tests across 5 sections (23–27):

- **Section 23** (4 tests) — `TfResult` dataclass: fields, frozen immutability,
  `converged=False`, package export.
- **Section 24** (4 tests) — `_build_ss_matrix` unit tests: single resistor,
  capacitor open, inductor near-short, current source skipped.
- **Section 25** (7 tests) — voltage-source input: symmetric voltage divider
  (H=0.5, Z_in=2kΩ, Z_out=500Ω), asymmetric divider, source-node output (H=1,
  Z_out=0), ground output (H=0), three-resistor ladder, inductor-short, diode
  linearisation.
- **Section 26** (3 tests) — current-source input: transimpedance into R,
  parallel R∥R, mixed source circuit (VoltageSource input with CurrentSource
  zeroed).
- **Section 27** (5 tests) — error cases: missing source name, non-source
  element, unknown output node, independence of source voltage, two-source circuit.

Total: 107 tests, 80.16% coverage, ruff clean.

---

## [0.4.0] — 2026-05-08

### Added

- **`ac_sweep` function** — Small-signal AC frequency sweep (the SPICE `.AC` analysis).

  **Algorithm:**
  1. Compute DC operating point via `dc_op` to obtain bias voltages for
     nonlinear device linearisation.
  2. Build a frequency grid (log-spaced or linear).
  3. For each frequency ω = 2πf: construct a complex MNA matrix G_c, stamp every
     element with its complex admittance or small-signal model, then solve
     `G_c · x_c = b_c` via complex Gaussian elimination.
  4. Return one `AcPoint` per frequency containing phasor node voltages.

  **Linear element AC admittances:**
  - Resistor: `Y = 1/R` (purely real, frequency-independent)
  - Capacitor: `Y_C = jωC` (open circuit at DC, purely imaginary)
  - Inductor: `Y_L = 1/(jωL)` (short circuit at DC → modelled as `G = 1e12 S`
    when `ω = 0` to keep the matrix non-singular)
  - VoltageSource: ideal AC source at its `voltage` amplitude; a 0 V source is a
    short circuit (correct for DC-bias sources in AC analysis)
  - CurrentSource: phasor current injection into the RHS vector

  **Nonlinear element small-signal models** (linearised at DC OP):
  - **Diode**: `gd = (Is/Vt) · exp(Vd/Vt)` — small-signal conductance between
    anode and cathode; no Norton offset (DC terms vanish in AC)
  - **MOSFET**: `gds` (output conductance, drain–source) + `gm` VCCS
    (gate–source controls drain–source current); same stamp pattern as the DC
    Newton stamp but in the complex domain
  - **BJT**: `g_π = gm/beta_f` (junction conductance, B–E for NPN, E–B for PNP)
    + `gm` VCCS (junction voltage controls collector current)

  **Robustness:** if the AC MNA matrix is singular at a particular frequency
  (e.g. a floating node), all node voltages for that frequency are set to zero
  and the sweep continues.

- **`AcPoint` dataclass** — Phasor voltages at one frequency.
  - Fields: `freq` (Hz), `node_voltages` (dict `str → complex`).
  - Use `abs(v)` for magnitude, `cmath.phase(v)` for phase in radians.

- **`AcResult` dataclass** — Frequency-sweep output.
  - Field: `points` (list of `AcPoint`, ascending by frequency, empty when
    `n_points < 1`).

- **`_solve_complex` helper** — Gaussian elimination with partial pivoting for
  complex-valued matrices.  Same algorithm as `_solve` but operates on
  `list[list[complex]]` and `list[complex]`; pivot selection uses `abs()` (complex
  modulus) to choose the largest-magnitude pivot.

- **`_stamp_g_c` helper** — Stamps a complex admittance between two nodes onto the
  complex MNA matrix.  Parallel to the real-valued `_stamp_g` used in DC analysis.

- **`_stamp_ac` helper** — Dispatches AC stamping for all supported element types.

### Changed

- `__init__.py` now exports `AcPoint`, `AcResult`, `ac_sweep`.
- Version bumped: `0.3.0` → `0.4.0`.
- Package description updated to mention AC analysis.

### Tests

- **Section 15 — Complex linear solver** (5 tests):
  - `test_solve_complex_2x2_real_system` — matches real solver output
  - `test_solve_complex_purely_imaginary_diagonal` — imaginary diagonal matrix
  - `test_solve_complex_empty` — empty system returns empty list
  - `test_solve_complex_singular_raises` — singular matrix raises `ZeroDivisionError`
  - `test_solve_complex_3x3` — verifies A·x = b for a 3×3 complex system

- **Section 16 — Data structures** (5 tests):
  - `test_acpoint_fields`, `test_acresult_fields` — field storage
  - `test_ac_sweep_returns_acresult` — return type check
  - `test_ac_sweep_point_count` — exactly n_points points returned
  - `test_ac_sweep_zero_points`, `test_ac_sweep_single_point` — edge cases
  - `test_ac_sweep_point_has_node_voltages` — node names present in each point
  - `test_ac_sweep_frequencies_ascending` — frequencies in order

- **Section 17 — Resistive circuits** (3 tests):
  - `test_ac_resistive_voltage_divider_real_valued` — gain=0.5, Im≈0
  - `test_ac_source_node_equals_source_voltage` — source node matches amplitude
  - `test_ac_unequal_resistive_divider` — R2/(R1+R2) gain at all frequencies

- **Section 18 — RC low-pass filter** (5 tests):
  - `test_ac_rc_lowpass_dc_gain_unity` — gain ≈ 1 at very low f
  - `test_ac_rc_lowpass_3db_at_cutoff` — |H| = 1/√2 at f_c = 1/(2πRC)
  - `test_ac_rc_lowpass_phase_at_cutoff` — phase ≈ −45° at f_c
  - `test_ac_rc_lowpass_rolloff_above_cutoff` — 20 dB/decade roll-off
  - `test_ac_rc_lowpass_gain_monotone_decreasing` — strict monotone decrease

- **Section 19 — RL high-pass filter** (2 tests):
  - `test_ac_rl_highpass_gain_increases_with_frequency` — monotone increasing
  - `test_ac_rl_highpass_3db_at_cutoff` — 1/√2 at f_c = R/(2πL)

- **Section 20 — Sweep modes** (4 tests):
  - Log/lin first and last frequency endpoints
  - Linear spacing uniformity
  - Log decade spacing ratio

- **Section 21 — Small-signal nonlinear elements** (3 tests):
  - `test_ac_diode_small_signal_forward_biased` — heavy shunting when forward-biased
  - `test_ac_diode_reverse_biased_acts_like_open` — voltage divider unchanged
  - `test_ac_bjt_npn_small_signal` — node presence and convergence

- **Section 22 — Current source injection** (3 tests):
  - `test_ac_current_source_into_resistor` — V = I×R at all frequencies
  - `test_ac_current_source_with_capacitor_shunt` — voltage decreases with frequency
  - `test_ac_inductor_acts_as_short_at_very_low_frequency` — near-unity gain at DC

---

## [0.3.0] — 2026-05-08

### Added

- **`BJT` element** — Bipolar Junction Transistor dataclass (NPN and PNP).
  - Parameters: `name`, `collector`, `base`, `emitter`, `polarity` ("NPN"/"PNP"),
    `Is` (saturation current, default 10 fA), `beta_f` (current gain hFE,
    default 100), `Vt` (thermal voltage, default 25.85 mV).
  - Frozen dataclass with `slots=True` matching the style of all other elements.

- **`_stamp_bjt` stamping function** — Newton-linearised Ebers-Moll forward-active model.

  **Simplified Ebers-Moll model:**
  The collector current in the forward-active region is modelled as::

      Ic = Is * (exp(Vjunc / Vt) - 1)

  where Vjunc is the controlling junction voltage:
  - **NPN**: Vjunc = Vbe = Vb − Ve
  - **PNP**: Vjunc = Veb = Ve − Vb

  **Newton linearisation** at operating point Vjunc0 (clamped to 0.7 V)::

      exp_term = exp(Vjunc0 / Vt)
      Ic0      = Is * (exp_term - 1)      # collector current at OP
      gm       = (Is / Vt) * exp_term     # transconductance
      gπ       = gm / beta_f              # junction conductance
      Ib0      = Ic0 / beta_f             # base current at OP

  **Two MNA stamps:**
  1. **Junction stamp** (gπ): conductance between the controlling junction
     terminals (B–E for NPN, E–B for PNP) modelling the base-emitter diode.
     Norton companion: `Ieq_junc = Ib0 − gπ * Vjunc0`.
  2. **VCCS stamp** (gm): voltage-controlled current source; controlling nodes
     are the junction pair, output nodes are the collector-emitter pair.
     - NPN: `G[C][B] += gm`, `G[C][E] -= gm`, `G[E][B] -= gm`, `G[E][E] += gm`
     - PNP: roles of E and C are swapped (emitter injects, collector collects).
     Norton companion: `Ieq_coll = Ic0 − gm * Vjunc0`.

  All ground-node aliases (``"0"``, ``"gnd"``, ``"GND"``) are handled correctly;
  any terminal can be ground.

- **`_element_nodes` update** — `BJT` returns `[collector, base, emitter]`.
- **`_stamp_dc` update** — dispatches to `_stamp_bjt` for `BJT` instances.
- **`__init__.py` update** — `BJT` is exported from the top-level package.

### Changed

- `Element` union type now includes `BJT`.
- Version bumped: `0.2.0` → `0.3.0`

### Tests

- **12 new tests** in `tests/test_engine.py` under section 14 "DC: BJT":
  - `test_bjt_dataclass_defaults` — field values and defaults
  - `test_bjt_pnp_dataclass` — polarity field stored correctly
  - `test_bjt_npn_off` — Vbe = 0 → Ic ≈ 0, Vcol ≈ Vcc
  - `test_bjt_npn_forward_active` — Vbe = 0.7 V clamped; Ic matches analytic
  - `test_bjt_npn_beta_ratio` — Ic/Ib = beta_f held internally
  - `test_bjt_pnp_forward_active` — PNP with Veb = 0.7 V; Ic flows into collector
  - `test_bjt_element_nodes` — all three terminals in node index
  - `test_bjt_stamp_matrix_shape` — no NaN/Inf after stamping at Vbe = 0
  - `test_bjt_npn_ground_emitter_no_crash` — ground alias "gnd" on emitter
  - `test_bjt_npn_vcc_emitter` — non-ground emitter; Ic consistent with Vbe
  - `test_bjt_in_element_union` — BJT exported correctly
  - `test_bjt_custom_parameters` — Is/beta_f/Vt stored correctly

---

## [0.2.0] — 2026-05-06

### Added

- **Trapezoidal integration method** (`method="trap"`, new default)
  - Capacitor companion: `G_eq = 2C/h`, `I_eq = G_eq·V_n + I_n` (Norton current
    injected into the positive plate).  Second-order accurate — `O(h²)` LTE vs
    `O(h)` for backward Euler.
  - Inductor companion: `G_eq = h/(2L)`, `I_eq = I_n + G_eq·V_n` (parallel Norton
    current flowing n+ → n−).  Correctly accumulates inductor flux across time.
  - Both capacitor and inductor companion histories are updated in
    `_update_reactive_state` after each accepted step.

- **Backward-Euler method** (`method="euler"`)
  - Capacitor companion: `G_eq = C/h`, `I_eq = G_eq·V_n`.
  - Inductor companion: `G_eq = h/L`, `I_eq = I_n`.
  - `method="euler"` is available as a fallback; `"trap"` is the default.

- **LTE-based adaptive timestepping** (`adaptive=True`)
  - Trapezoidal-specific LTE estimate: `lte ≈ |V_{n+1} − 2·V_n + V_{n-1}| / 2`
    (second finite difference of each capacitor voltage, normalised by 2).
  - Step accepted when `lte ≤ tol_lte`; rejected and halved when `lte > tol_lte`
    and `h > min_step`.
  - Step doubled (up to `max_step`) when `lte < tol_lte / 8`.
  - `TransientResult` now carries `method` (str) and `steps_rejected` (int).
  - `min_step` defaults to `t_step / 1000`; `max_step` defaults to `t_step × 10`.

- **Correct reactive-element initial conditions**
  - Capacitor: initial current `I_C(0)` is seeded from the branch current of the
    substitute voltage source in the t=0 DC solve.  This eliminates the large
    first-step error in trapezoidal that arises from assuming `I_0 = 0`.
  - Inductor: the t=0 DC solve now uses a backward-Euler companion resistor
    `R = L/h` instead of a 0 V voltage source.  A 0 V source forces the steady-
    state current at t=0; the companion resistor correctly models near-zero initial
    current with the full supply voltage appearing across the inductor.  The
    initial voltage `V_L(0)` is seeded from the DC OP into `ind_voltages` for the
    first trapezoidal step.

- **Helper functions** (`_build_transient_companions`, `_update_reactive_state`,
  `_lte_estimate`, `_node_voltage`)

- **`TransientResult` extended** — new fields `method: str = "trap"` and
  `steps_rejected: int = 0`.

### Changed

- `transient()` signature extended with `method`, `adaptive`, `tol_lte`,
  `min_step`, `max_step`.  Default `method="trap"` (was implicit forward Euler in
  0.1.0 — existing callers that did not pass `method` now use trapezoidal).
- Version bumped: `0.1.0` → `0.2.0`

### Tests

- **37 tests** (up from initial suite) covering:
  - Backward-Euler and trapezoidal RC charging accuracy
  - Trap accuracy vs Euler at same step size
  - Adaptive control: steps_rejected, steady-state convergence
  - Adaptive and fixed trapezoidal agree when h is locked
  - RL current build-up to steady state
  - Inductor initial condition (near-zero current at t=0)
  - LTE estimate for zero/curved signals and circuits without capacitors
  - TransientResult method + steps_rejected metadata fields
- Coverage: **88%**

---

## [0.1.0]

### Added
- Element classes: Resistor, Capacitor, Inductor, VoltageSource, CurrentSource, Diode (Shockley), Mosfet (mosfet-models-backed).
- `Circuit` container.
- MNA matrix construction with element-specific stamp functions.
- Gaussian elimination with partial pivoting (`_solve`).
- `dc_op(circuit, max_iterations=50, tol=1e-6)`: Newton-Raphson DC operating point. Returns DcResult with node_voltages + branch_currents + converged flag.
- `transient(circuit, t_stop, t_step)`: forward-Euler with capacitor companion model (g = C/h, I_eq = (C/h) × V(t_n)). Returns TransientResult with per-step TransientPoints.
- Diode linearization with V_d clamping to avoid exp overflow.
- MOSFET stamping uses mosfet_models.MOSFET.dc() for I_d, g_m, g_ds.
- Ground node aliases: '0', 'gnd', 'GND'.

### Out of scope (v0.2.0)
- AC analysis (.ac).
- Better integrators (backward Euler, trapezoidal, Gear-2).
- Adaptive timestep with LTE control.
- Convergence aids (Gmin stepping, source stepping, pseudo-transient).
- SPICE3 netlist parser.
- BJTs, JFETs, Verilog-A.
- Sparse matrix solver.
