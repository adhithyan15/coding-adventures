# 03 — Watch Local Storage

Part of the [mobile learning series](../../README.md). A standalone Apple Watch
app that logs water intake and persists it locally on the Watch using SwiftData.

**No iPhone required.** Logs saved on the Watch survive restarts, work offline,
and reset automatically at midnight. Sync with the iPhone is added in Stage 05.

## What this stage teaches

- SwiftData on watchOS (`@Model`, `@Query`, `ModelContainer`)
- Compact watchOS UI that works on 40mm through 49mm (Ultra) displays
- Haptic feedback via `WKInterfaceDevice.current().play(.success)`
- The Action Button on Apple Watch Ultra

## Running locally

```bash
cd code/programs/swift/03-watch-local
mise exec -- xcodegen generate
open WaterWatch.xcodeproj
# Select the Apple Watch Ultra simulator, press Run
```

## Project structure

```
Sources/WaterWatch/
├── WaterWatchApp.swift      Entry point, ModelContainer setup
├── ContentView.swift        Counter display + Log button
├── WaterEntry.swift         @Model — identical schema to Stage 04 iOS
└── Info.plist
```

## Series

| Stage | What it builds |
|-------|---------------|
| [01](../01-hello-world/) | Hello World — iPhone + Watch targets |
| [02](../02-water-counter/) | Ephemeral water counter |
| **03** | **Watch — local storage (this app)** |
| [04](../04-persistence/) | iPhone + Android — local persistence |
| 05 | Watch ↔ iPhone sync with offline queue |
