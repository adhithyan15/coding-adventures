# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# paint IR for scenes, shapes, groups, layers, clips, gradients, and images
- Path command and filter effect variants with stable `Kind` tags
- Builder helpers for creating paint scenes without hand-populating every record field
- Support for URI-backed and `PixelContainer`-backed paint images
- xUnit coverage for builder behavior, option application, and instruction tagging

### Changed

- Linux BUILD scripts now set package-local `TMPDIR`, `HOME`, and `DOTNET_CLI_HOME` so parallel CI avoids `.NET` first-run migration mutex collisions
