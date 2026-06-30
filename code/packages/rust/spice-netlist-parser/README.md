# spice-netlist-parser

Small SPICE3 netlist parser that builds `spice_engine::Circuit` values.

```rust
use spice_netlist_parser::parse_netlist;

let parsed = parse_netlist(r#"
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n method=gear2
.end
"#)?;

assert_eq!(parsed.tran_cards().len(), 1);
```

`parse_netlist` lowers decks through the Berkeley SPICE logical-card syntax
facade, so normal simulator parsing honors leading `+` continuations, strips
inline comments from normalized cards, and reports stable syntax diagnostics
before semantic lowering starts.

For editor, Mosaic, and parser-generator frontends, the crate also exposes the
same facade directly:

```rust
use spice_netlist_parser::{
    berkeley_app_package_manifest_json, parse_berkeley_app_deck,
    BerkeleyAppPersistedEditorState, BerkeleyCardKind,
};

let deck = parse_berkeley_app_deck(r#"
* divider
V1 in 0 DC 1
R1 in out 1k
R2 out 0 1k
.op
.end
"#);

assert!(!deck.has_errors());
assert_eq!(deck.analysis_inventory()[0].analysis, "op");
assert_eq!(deck.syntax.cards[1].kind, BerkeleyCardKind::Element);

let execution = deck.run_artifacts()?;
assert_eq!(execution.analyses[0].syntax_card_index, Some(3));
assert!(execution.analyses[0].table_columns.contains(&"Index".to_string()));
assert_eq!(execution.analyses[0].waveform_series[0].x_column, "Index");

let session = deck.run_session_state(Some(3))?;
assert!(session.execution_available);
assert_eq!(session.selected_waveform_series_count, Some(1));

let controls = deck.run_editor_controls(Some(3))?;
assert!(controls.selected_control.unwrap().waveform_available);

let command_plan = deck.run_editor_command_plan(Some(3))?;
assert!(command_plan
    .commands
    .iter()
    .any(|command| command.id == "analysis.3.inspect-waveform" && command.enabled));

let view = deck.run_editor_state_snapshot(Default::default())?;
assert_eq!(view.resolved_state.selected_syntax_card_index, Some(3));

let host = deck.run_host_surface(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert_eq!(host.active_panel.unwrap().id, "waveform");

let host_wire_json = deck.run_host_surface_wire_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(host_wire_json.contains(r#""activePanelId":"waveform""#));

let package_manifest_json = berkeley_app_package_manifest_json();
assert!(package_manifest_json.contains(r#""packageName":"berkeley-spice-mosaic-app""#));

let bootstrap_json = deck.run_app_bootstrap_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(bootstrap_json.contains(r#""packageManifest":{"#));
assert!(bootstrap_json.contains(r#""hostSurface":{"#));

let startup_summary_json = deck.run_app_startup_summary_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(startup_summary_json.contains(r#""ready":true"#));
```

The facade preserves normalized logical cards, source spans, token names
aligned with `code/grammars/spice/berkeley.tokens`, stable syntax diagnostics,
analysis inventory, source-order execution through the existing parser, and
card-indexed app artifacts with stable result tables, output-plan artifacts,
run-artifact summaries, rawfile / wrdata artifact metadata from the engine deck
execution layer, and numeric waveform series derived from result tables for
Mosaic plot surfaces. It also exposes app session snapshots with deterministic
source fingerprints, selected-analysis state, run/blocked status, diagnostics,
table columns, output probes, and selected waveform availability so Mosaic UI
hosts can render editor state without owning simulator internals. It also
derives stable per-analysis editor controls for selecting, running, and
inspecting tables or waveforms with explicit disabled reasons, plus command
plans with stable command IDs and target names for host menu or button wiring.
Persisted editor-state snapshots resolve saved selection and active-command IDs
against the current deck, repairing stale UI state after source edits. Host
surfaces turn those snapshots into stable source, diagnostics, analysis, table,
and waveform panel descriptors with explicit targets, enabled states, active
state, and disabled reasons for Mosaic shell integration. Host-surface wire
exports flatten the same contract into schema-versioned JSON with repaired
selection metadata and lower-case panel / diagnostic kinds for WebAssembly and
product-shell embedding. The static package manifest advertises the Berkeley
grammar version, host-surface wire schema, panel kinds, command targets,
runnable analysis directives, and artifact capabilities so Mosaic and
WebAssembly hosts can negotiate the app package before opening a deck. It is the
Rust app/runtime entrypoint for Mosaic-backed UI work. App bootstrap snapshots
combine that manifest with the schema-versioned host-surface wire export so
WebAssembly and product shells can load one startup payload with package
capabilities, repaired editor-state metadata, active panels, and diagnostics
before taking ownership of the UI. Startup summaries derive a compact ready /
blocked route from the same bootstrap payload, including source fingerprint,
active panel, repaired editor-state IDs, stale-state flags, diagnostic count,
and blocking reason. The grammar-backed parser generator and Python/TypeScript
parity surfaces continue to mature.

This parser supports `R`, `C`, `L`, `V`, `I`, `D`, `Q`, `M`, `G`, `E`, `F`, and
`H` elements, `.model <name> D(...)` diode cards with `IS` and `VT`
parameters, `.model <name> NPN|PNP(...)` BJT cards with `IS`, `BF` /
`BETA_F`, `VT`, `CJE`, `CJC`, `TF`, and `TR` parameters,
`.model <name> NMOS|PMOS(...)` Level-1 MOSFET
cards with common SPICE aliases (`VT0` / `VTO`, `KP`, `LAMBDA`, `GAMMA`, `PHI`,
`W`, `L`, `IS`, `N_SUB` / `NSUB`, `T_NOM` / `TNOM`, `CGSO`, `CGDO`, `CGBO`,
`CBS`, and `CBD`), SPICE engineering
suffixes, capacitor `IC=<voltage>` and inductor `IC=<current>` initial
conditions, independent-source `AC <magnitude> [phase]` forms,
PWL/PULSE/SIN/EXP source forms, comments, `.end`, `.subckt` / `X` instance
expansion, and `.op`, `.tran`, `.dc`, `.ac`, `.tf`, `.sens`, `.mc`, `.noise`,
`.temp`, `.print`, `.plot`, `.save`, `.probe`, `.measure`, `.four`, and
`.options` analysis cards.
Transient cards can carry `method=euler|trap|gear2`; when omitted,
`parsed.transient_method(None)?` falls back to `.options method=<...>` if
present.
Selected `.options` keys can also be turned into engine-call options with
`parsed.dc_op_options()?` and `parsed.adaptive_transient_options(None)?`.
Runnable `.op`, `.dc`, `.ac dec` / `.ac log`, and `.tran` cards can be planned
and executed directly with `parsed.analysis_plan()`, `parsed.run_analysis_plan()`,
or `run_netlist(deck)`.
`.save`, `.probe`, `.print`, and `.plot` cards can be applied to executed
analysis results with `parsed.select_outputs(&results)?`. Supported `.measure`
cards can be evaluated with `parsed.measure_results(&results)?`; the first
execution subset supports `FIND ... AT=<value>` plus `MAX`, `MIN`, `AVG`, and
`RMS` over optional `FROM=<value>` / `TO=<value>` ranges.
Deck-level `.temp` cards can be resolved into Kelvin with
`parsed.operating_temperature_kelvin(0, 300.0)?`, and
`parsed.noise_temperature_kelvin(Some(noise_card), 0, 300.0)?` applies the
SPICE precedence where an explicit `.noise temp=<kelvin>` overrides the deck
operating temperature.
