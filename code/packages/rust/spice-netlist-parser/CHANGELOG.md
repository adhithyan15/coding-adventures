# Changelog

## Unreleased

- Add Berkeley SPICE app-deck shell event summaries for Mosaic startup
  dashboards and gates. `BerkeleyAppDeck::app_shell_event_summary()`,
  `run_app_shell_event_summary()`, and their JSON helpers derive compact
  event-kind, severity, diagnostic, repaired-state, and capability counts from
  shell event logs.
- Add Berkeley SPICE app-deck shell event logs for Mosaic product-shell startup
  streams. `BerkeleyAppDeck::app_shell_event_log()`,
  `run_app_shell_event_log()`, and their JSON helpers derive stable status,
  route, primary-action, diagnostic, repaired-state, and capability events from
  shell handoffs.
- Add Berkeley SPICE app-deck shell telemetry for Mosaic startup metrics.
  `BerkeleyAppDeck::app_shell_telemetry()`, `run_app_shell_telemetry()`, and
  their JSON helpers derive compact route, entry-action, availability,
  diagnostic, repaired-state, and capability counts from the shell handoff.
- Add Berkeley SPICE app-deck shell statuses for Mosaic startup chrome and
  telemetry. `BerkeleyAppDeck::app_shell_status()`,
  `run_app_shell_status()`, and their JSON helpers derive a compact route,
  severity, message, entry action, and diagnostic counts from the shell handoff.
- Add Berkeley SPICE app-deck shell handoffs for Mosaic WebAssembly and
  product-shell startup. `BerkeleyAppDeck::app_shell_handoff()`,
  `run_app_shell_handoff()`, and their JSON helpers combine the package
  manifest, startup summary, launch plan, and readiness report into one compact
  startup envelope.
- Add Berkeley SPICE app-deck readiness reports for Mosaic product-shell
  telemetry and startup gates. `BerkeleyAppDeck::app_readiness_report()`,
  `run_app_readiness_report()`, and their JSON helpers summarize launch route,
  panel/action availability, diagnostic severity counts, repaired state, and
  blocking reasons from bootstrap snapshots.
- Add Berkeley SPICE app-deck launch plans for Mosaic product-shell startup
  routing. `BerkeleyAppDeck::app_launch_plan()`, `run_app_launch_plan()`, and
  their JSON helpers derive ready/blocked entry panels, route targets, and panel
  action descriptors from bootstrap snapshots.
- Add Berkeley SPICE app-deck persisted editor-state snapshots for Mosaic host
  restoration. `BerkeleyAppDeck::editor_state_snapshot()` and
  `run_editor_state_snapshot()` now resolve saved selected-card and
  active-command IDs against the current deck, including stale-state repair
  flags.
- Add Berkeley SPICE app-deck editor command plans for Mosaic host wiring.
  `BerkeleyAppDeck::editor_command_plan()` and `run_editor_command_plan()` now
  expose stable per-analysis command IDs, action kinds, targets, enabled states,
  and disabled reasons derived from editor controls.
- Add Berkeley SPICE app-deck editor controls for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::editor_controls()` and `run_editor_controls()`
  now expose stable per-analysis select/run/table/waveform actions, enabled
  states, and disabled reasons derived from the app session state.
- Add Berkeley SPICE app-deck session snapshots for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::session_state()` and `run_session_state()` now
  expose deterministic source fingerprints, selected-analysis state,
  run/blocked status, diagnostics, table columns, output probes, and selected
  waveform availability without requiring UI hosts to own simulator internals.
- Add Berkeley SPICE app-deck waveform inspection series for Mosaic-facing
  Rust UI substrates. Card-indexed analysis artifacts now expose numeric
  plot-ready series derived from stable result tables, including selected-card
  waveform access and probe-grouped AC magnitude/phase series.
- Add Berkeley SPICE app-deck result artifacts for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::run_artifacts()` now exposes normalized source,
  syntax-card-indexed result tables, output-plan artifacts, run-artifact
  summaries, and rawfile / wrdata artifact metadata backed by the engine deck
  execution layer.
- Route `parse_netlist` through the Berkeley SPICE logical-card syntax facade,
  so the default Rust parser consumes normalized cards, supports leading `+`
  continuations, and reports stable syntax diagnostics before semantic
  lowering.
- Add a Berkeley SPICE logical-card syntax facade for Rust/Mosaic app
  substrates. The new surface exposes grammar metadata, normalized logical
  cards, leading `+` continuation handling, source spans, grammar-token names,
  stable syntax diagnostics, analysis inventory, and an app-deck wrapper that
  can run source-order or selected runnable analyses through the existing
  parser.
- Parse `.save`, scoped or global `.probe`, and `.measure` / `.meas` cards,
  and expose `select_outputs()` / `measure_results()` helpers plus matching
  `ParsedNetlist` methods for analysis-plan results.
- Add a deck execution layer with `build_analysis_plan()`, `run_analysis_plan()`,
  `run_netlist()`, plus matching `ParsedNetlist` methods for runnable `.op`,
  `.dc`, `.ac dec` / `.ac log`, and `.tran` cards.

## 0.3.0 — 2026-06-05

- Resolve `.temp` cards into Kelvin engine-call temperatures and let explicit
  `.noise temp=<kelvin>` overrides win over deck-level operating temperatures.
- Route selected `.options` keys into engine-call helpers:
  `dc_op_options()` for DC Newton options and `adaptive_transient_options()`
  for adaptive transient options.
- Parse SPICE `.four <frequency> <V(node)|I(source)>...` Fourier-analysis
  cards.
- Parse SPICE `.print <analysis> <V(node)|I(source)>...` and
  `.plot <analysis> <V(node)|I(source)>...` output cards.
- Parse SPICE `.temp <celsius> [celsius ...]` operating-temperature cards.
- Parse MOS Level-1 capacitance parameters with `.model ... NMOS|PMOS(... CGSO=<c>
  CGDO=<c> CGBO=<c> CBS=<c> CBD=<c>)`.
- Parse diode model-card emission coefficients with `.model ... D(... N=<n>)`
  and pass them into Rust `Diode` elements.
- Parse diode model-card reverse-breakdown parameters with
  `.model ... D(... BV=<v> IBV=<i>)`.
- Parse diode model-card junction capacitance with
  `.model ... D(... CJO=<c>)` / `.model ... D(... CJ0=<c>)`.
- Parse diode model-card transit time with `.model ... D(... TT=<time>)`.
- Parse BJT model-card capacitances with `.model ... NPN|PNP(... CJE=<c>
  CJC=<c>)` and pass them into Rust `Bjt` elements.
- Parse BJT model-card forward transit time with
  `.model ... NPN|PNP(... TF=<time>)`.
- Parse BJT model-card reverse transit time with
  `.model ... NPN|PNP(... TR=<time>)`.
- Parse and validate transient integration methods from
  `.tran ... method=<euler|trap|gear2>`, and expose fallback routing from
  `.options method=<...>`.
- Parse conservative SPICE `T` transmission-line cards of the form
  `Tname n1 n2 n3 n4 Z0=<ohms> TD=<seconds>`, including subcircuit node
  remapping and validation for unsupported, missing, non-finite, and
  non-positive parameters.
- Reject SPICE `K` mutual-inductor cards that reference missing inductors or
  use non-finite coupling coefficients.
- Parse SPICE `K` mutual-inductor cards into `MutualInductor` elements,
  including subcircuit-local inductor reference remapping.
- Parse SPICE `J` JFET elements via `.model <name> NJF(...)` and
  `.model <name> PJF(...)` cards with `BETA` / `B`, `VTO`, and `LAMBDA`
  parameters, including subcircuit drain/gate/source remapping.
- Parse capacitor `IC=<voltage>` initial-voltage parameters.
- Parse inductor `IC=<current>` initial-current parameters.
- Parse SPICE `.tf V(output_node) input_source` transfer-function analysis
  cards.
- Parse SPICE `.sens V(output_node)` DC sensitivity analysis cards.
- Parse SPICE `.mc V(output_node) n_trials [tolerance] [distribution] [seed]`
  Monte Carlo DC analysis cards.
- Parse SPICE `.noise V(output_node) input_source [freq ...] [temp=<kelvin>]`
  AC noise analysis cards.
- Parse SPICE `.options key=value ...` simulator-options cards.

## 0.1.7

- Add independent-source `AC <magnitude> [phase]` parsing, including combined
  `DC <bias> AC <magnitude> [phase]` forms for AC analysis with separate DC
  bias and small-signal excitation.

## 0.1.6

- Add SPICE `M` MOSFET element parsing via `.model <name> NMOS|PMOS(...)`
  Level-1 cards, per-instance parameter overrides such as `W=...` and `L=...`,
  and subcircuit drain/gate/source/body terminal remapping.

## 0.1.5

- Add SPICE `Q` BJT element parsing via `.model <name> NPN|PNP(...)` cards
  with `IS`, `BF` / `BETA_F`, and `VT` parameters, including subcircuit
  terminal remapping.

## 0.1.4

- Add SPICE `D` diode element parsing via `.model <name> D(...)` cards with
  `IS` and `VT` parameters, including subcircuit terminal remapping.

## 0.1.3

- Add SPICE `H` / CCVS controlled-source parsing, including subcircuit
  controlling-source name remapping for expanded CCVS elements.

## 0.1.2

- Add SPICE `F` / CCCS controlled-source parsing, including subcircuit
  controlling-source name remapping for expanded CCCS elements.

## 0.1.1

- Add SPICE `E` / VCVS controlled-source parsing, including subcircuit node
  remapping for expanded VCVS elements.

## 0.1.0

- Add a first SPICE3 netlist parser slice for linear R/C/L circuits,
  independent V/I sources, VCCS elements, PWL/PULSE/SIN/EXP source waveforms,
  and `.op`, `.tran`, `.dc`, and `.ac` analysis cards.
- Add first `.subckt` / `X` instance expansion for hierarchical netlists made
  from supported primitive elements.
