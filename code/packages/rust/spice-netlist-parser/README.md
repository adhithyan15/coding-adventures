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
use spice_netlist_parser::{parse_berkeley_app_deck, BerkeleyCardKind};

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
hosts can render editor controls without owning simulator internals. It is the
Rust app/runtime entrypoint for Mosaic-backed UI work while the grammar-backed
parser generator and Python/TypeScript parity surfaces continue to mature.

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
