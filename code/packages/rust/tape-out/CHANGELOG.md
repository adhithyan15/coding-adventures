# Changelog — tape-out

## [0.1.0] — 2026-06-13

### Added
- `Shuttle`, `PadLocation`, `TapeoutMetadata`, `TapeoutBundle`, `ValidationReport` — bundle data model.
- `TapeoutMetadata::default()` — sky130A PDK, Apache-2.0 license, chipIgnite open MPW, 1.8 V VDD.
- `validate_for_chipignite()` — checks required metadata fields, required file keys (gds/lef/def/verilog/drc_report/lvs_report), DRC/LVS signoff, pad location warning for open MPW.
- `render_manifest()` — emits manifest.yaml (hand-rolled YAML; no external dependency).
- `render_readme()` — emits README.md summary.
- 15 integration tests + 1 doc-test.
