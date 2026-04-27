# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Pure F# document AST package covering shared document, inline, and GFM extension nodes
- xUnit coverage for node typing, core structure, and extension shapes
- BUILD scripts now use `dotnet test --artifacts-path .artifacts` so transitive .NET project builds do not collide under parallel CI
- Linux BUILD scripts pin both `HOME` and `DOTNET_CLI_HOME` to the package-local `.dotnet` directory so parallel CI avoids `.NET` first-run migration races
- `BUILD_windows` now uses defensive `set "VAR=value"` quoting so path metacharacters cannot alter the command stream
