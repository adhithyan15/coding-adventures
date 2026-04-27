# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-04-24

### Added

- `PathCommand` struct with `Kind` (`"move_to"`, `"line_to"`, `"close"`), `X`, and `Y` fields.
- `PaintPathInstruction` struct (implements `PaintInstruction` with kind `"path"`).
- `PaintPath(commands []PathCommand, fill string, metadata Metadata) PaintPathInstruction`
  constructor — required by `barcode-2d` for rendering MaxiCode hex modules.

## [0.1.0] - 2026-04-12

### Added

- Initial `PaintScene` and rectangle instruction primitives
