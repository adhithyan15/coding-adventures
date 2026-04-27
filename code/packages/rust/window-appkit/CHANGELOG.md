# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `AppKitBackend` with AppKit-specific attribute validation.
- Surface selection rules for normal AppKit views versus Metal-layer hosts.
- Real `NSApplication` and `NSWindow` creation for the minimal macOS launch path.
- `AppKitWindow` implementing the shared `Window` trait with AppKit render-target handles.
- Auto-closing `launch_window` example for macOS smoke testing.
- Explicit rejection of browser and Windows-only surface requests.
- Unit tests for AppKit validation and render-target behavior.
