# Changelog

## [1.0.0] — 2026-03-30

### Added
- Standalone watchOS app with SwiftData local persistence
- `WaterEntry` `@Model` — id, timestamp, amountMl (250ml default)
- `ContentView` — water drop icon, today's total, goal progress bar, Log button
- Midnight reset via date-filtered `@Query` computed property
- Haptic feedback (`.success`) on every drink log
- `project.yml` for xcodegen — watchOS 11.0 deployment target
- README.md and CHANGELOG.md
