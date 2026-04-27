# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure F# `PixelContainer` with RGBA8 row-major storage
- Bounds-safe `GetPixel`, `SetPixel`, and `Fill` helpers
- `PixelContainers.create` and `PixelContainers.fromData` construction helpers
- `IImageCodec` interface for future encoder and decoder packages
- xUnit coverage for construction, bounds behavior, fill operations, and data reuse

### Changed

- Linux BUILD scripts now set package-local `TMPDIR`, `HOME`, and `DOTNET_CLI_HOME` so parallel CI avoids `.NET` first-run migration mutex collisions
