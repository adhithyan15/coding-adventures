# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure F# `PaintVM<'Context>` with per-kind dispatch registration
- Scene execution, patch callbacks, registered-kind reporting, and optional export
- Custom error types for duplicate handlers, unknown instructions, null contexts, and unsupported export
- Structural comparison support used by patch diffing
- xUnit coverage for dispatch, patching, export, and deep equality behavior

### Changed

- Linux BUILD scripts now set package-local `TMPDIR`, `HOME`, and `DOTNET_CLI_HOME` so parallel CI avoids `.NET` first-run migration mutex collisions
