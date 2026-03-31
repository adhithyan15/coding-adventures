# 04 — Persistence (iOS)

Part of the [mobile learning series](../../README.md). Extends the water
counter with local persistence via SwiftData. Close the app, reopen it —
the counter is exactly where you left it.

## What this stage teaches

- SwiftData `@Model`, `@Query`, `ModelContainer`
- Inserting records with `context.insert(_:)` (no manual `save()` needed)
- Filtering by date to compute a daily total
- In-memory `ModelContainer` for fast unit tests

## Running locally

```bash
cd code/programs/swift/04-persistence
mise exec -- xcodegen generate
open WaterPersist.xcodeproj
# Run on iPhone simulator, log some drinks, kill the app, reopen — counter persists
```

## Testing persistence manually

```bash
# Log drinks, then kill the app entirely:
xcrun simctl terminate booted com.codingadventures.waterpersist
# Relaunch — counter should restore:
xcrun simctl launch booted com.codingadventures.waterpersist
```

## Series

| Stage | What it builds |
|-------|---------------|
| [01](../01-hello-world/) | Hello World |
| [02](../02-water-counter/) | Ephemeral counter |
| [03](../03-watch-local/) | Watch — local storage |
| **04** | **iPhone — local persistence (this app)** |
| 05 | Watch ↔ iPhone sync |
