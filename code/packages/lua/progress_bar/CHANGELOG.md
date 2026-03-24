# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full Tracker implementation ported from Go progress-bar package
- Event types: STARTED, FINISHED, SKIPPED (string constants)
- `progress.new(total, writer, label)` constructor with metatable OOP
- `Tracker:start()` to initialize timing
- `Tracker:send(event)` for synchronous event processing and redraw
- `Tracker:stop()` to finalize output with trailing newline
- `Tracker:child(total, label)` for hierarchical (nested) progress bars
- `Tracker:finish()` to complete a child and advance the parent
- Unicode progress bar rendering (U+2588 filled, U+2591 empty, 20 chars wide)
- Activity formatting: "Building: a, b, c", "waiting...", "done", "+N more" truncation
- Three display modes: flat, labeled, hierarchical
- Carriage return line overwriting for terminal animation
- Literate programming style with inline documentation
- Comprehensive busted test suite (50+ test cases)
