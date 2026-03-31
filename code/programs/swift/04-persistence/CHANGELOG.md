# Changelog

## [1.0.0] — 2026-03-30

### Added
- `WaterEntry` `@Model` — id (UUID), timestamp (Date), amountMl (Int, default 250)
- `WaterPersistApp` — `@main` with `.modelContainer(for: WaterEntry.self)`
- `ContentView` — `@Query` all entries, filters to today via `DayFilter`, animated counter, progress bar, "Saved locally" badge
- `DayFilter` — `startOfToday()`, `startOfDay(for:)`, `daysAgo(_:)` helpers
- 4 XCTest unit tests with in-memory `ModelContainer`
- `project.yml` for xcodegen — iOS 17.0 deployment target
- README.md and CHANGELOG.md
