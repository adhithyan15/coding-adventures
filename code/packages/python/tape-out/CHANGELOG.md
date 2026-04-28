# Changelog

## [0.1.0] — Unreleased

### Added
- `TapeoutMetadata`, `TapeoutBundle`, `PadLocation` data classes.
- `Shuttle` enum: chipIgnite open MPW, chipIgnite paid MPW, TinyTapeout.
- `write_bundle(bundle, out_dir)`: copies files, emits manifest.yaml + README.md.
- `validate_for_chipignite(bundle) -> ValidationReport`: checks required metadata, required files (gds/lef/def/verilog/drc_report/lvs_report), signoff (drc + lvs must be "clean"), pad locations.
- `REQUIRED_FILES` constant.

### Out of scope (v0.2.0)
- Caravel user-project wrapper integration.
- Automatic pad-ring generation.
- TinyTapeout-specific bundling rules.
- Other-PDK support.
