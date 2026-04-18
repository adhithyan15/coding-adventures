# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# `PaintVM<TContext>` with per-kind dispatch registration
- Scene execution, patch callbacks, registered-kind reporting, and optional export
- Custom error types for duplicate handlers, unknown instructions, null contexts, and unsupported export
- Structural comparison support used by patch diffing
- xUnit coverage for dispatch, patching, export, and deep equality behavior

### Security

- Added cycle tracking to `DeepEqual` so cyclic object graphs cannot trigger unbounded recursion or stack exhaustion

### Changed

- Linux BUILD scripts now set package-local `TMPDIR`, `HOME`, and `DOTNET_CLI_HOME` so parallel CI avoids `.NET` first-run migration mutex collisions
