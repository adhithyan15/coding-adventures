# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- **Windows build.** New `BUILD_windows` running the same `dotnet test` without the POSIX
  `mkdir -p` / `VAR=value` setup, which `cmd /C` cannot run (it read the first variable name as
  the command). Flagged by the build tool's new Windows env-assignment check.

## [0.1.0] - 2026-04-16

### Added

- Compact serializer for `JsonValue` trees
- Pretty serializer with indentation, key sorting, and trailing newline options
- Native `.NET` stringify helpers built on `json-value`
