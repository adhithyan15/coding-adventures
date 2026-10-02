# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- **Windows build.** New `BUILD_windows` running the same `dotnet test` without the POSIX
  `mkdir -p` / `VAR=value` setup, which `cmd /C` cannot run (it read the first variable name as
  the command). Flagged by the build tool's new Windows env-assignment check.

## [0.1.0] - 2026-04-16

### Added

- Typed `JsonValue` hierarchy for all six JSON value kinds
- JSON parsing via `JsonValue.Parse` plus native conversion helpers
- Strict native conversion rules for unsupported runtime types
