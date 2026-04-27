# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `WindowId`, `LogicalSize`, `PhysicalSize`, and scale-conversion helpers.
- `WindowAttributes` and `WindowBuilder` with backend-neutral validation.
- Normalized `WindowEvent`, key, pointer, modifier, and render-target types.
- `Window` and `WindowBackend` traits for Rust native backends.
- Mock-backed unit tests covering builder validation and event-surface behavior.
