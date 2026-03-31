# Mobile 03 — Watch Local Storage

## Overview

The third app in the mobile learning series. A standalone Apple Watch app
that logs water intake and persists it locally on the Watch using SwiftData.
No iPhone required. No sync. The Watch is a first-class, independent device.

This stage teaches watchOS app development, SwiftData on watchOS, haptic
feedback, and the Action Button on Apple Watch Ultra.

Sync between Watch and iPhone is introduced in Stage 05, after both devices
have their own local persistence (Stage 04 adds it to iPhone and Android).

---

## Why Local-First on the Watch

The Apple Watch Ultra is designed to work without a phone — outdoor athletes,
swimmers, and ultramarathon runners use it phone-free for hours. A water
logging app that requires a phone nearby fails exactly when it's most needed.

By giving the Watch its own SwiftData store, the app works correctly in every
scenario:

- Phone at home, Watch on wrist → logs saved on Watch
- Phone in range → same behaviour (sync added in Stage 05)
- Watch restarted → data survives (SwiftData is persistent)
- New calendar day → counter resets (same date-filter logic as Stage 04 iOS)

---

## Learning Goals

- Create a standalone watchOS app target in Xcode (no iOS companion required)
- Use SwiftData on watchOS (same `@Model` API as iOS — one mental model)
- Build a compact watchOS UI with SwiftUI that respects watch screen constraints
- Deliver haptic feedback with `WKInterfaceDevice.current().play(.success)`
- Register the app as the Action Button handler on Apple Watch Ultra

---

## Project Structure

```
code/programs/swift/03-watch-local/
├── project.yml                        ← xcodegen config (watchOS target only)
├── Sources/
│   └── WaterWatch/
│       ├── WaterWatchApp.swift        ← @main, ModelContainer setup
│       ├── ContentView.swift          ← counter display + Log button
│       ├── WaterEntry.swift           ← @Model (identical to Stage 04 iOS)
│       └── ActionButtonHandler.swift  ← Ultra Action Button registration
├── README.md
└── CHANGELOG.md
```

---

## Data Model

Identical to Stage 04 (iOS). Using the same `@Model` class on both platforms
means the sync layer in Stage 05 has a clean, compatible schema to work with.

```swift
// WaterEntry.swift
import SwiftData
import Foundation

/// A single logged drink, persisted in the Watch's local SwiftData store.
///
/// The Watch has its own independent database — no iPhone needed to write
/// or read entries. The schema matches Stage 04 (iOS) exactly so Stage 05
/// can reconcile the two stores without schema translation.
@Model
final class WaterEntry {
    var id: UUID
    var timestamp: Date
    var amountMl: Int

    init(amountMl: Int = 250) {
        self.id = UUID()
        self.timestamp = Date()
        self.amountMl = amountMl
    }
}
```

---

## Watch UI

The Apple Watch Ultra has a 49mm display — the largest watch face Apple makes.
Even so, watchOS UI must be designed for glanceability: one action, one number,
done in under three seconds.

```
┌─────────────────────┐
│                     │
│   💧  1,250 ml      │
│   of 2,000 ml       │
│                     │
│  ┌───────────────┐  │
│  │  Log a Drink  │  │  ← full-width button (48pt minimum tap height)
│  └───────────────┘  │
│                     │
└─────────────────────┘
```

**Design rules for watchOS:**
- Font: `.title3` for the total, `.caption2` for the goal — these scale
  correctly across 40mm, 44mm, and 49mm (Ultra) displays
- Button: full-width, minimum 48pt height — small tap targets are the
  most common watchOS UI mistake
- No navigation stack for this stage — single screen only
- No images or icons that require asset catalogues — SF Symbols only

---

## App Setup

```swift
// WaterWatchApp.swift
import SwiftUI
import SwiftData

/// Entry point for the standalone Watch app.
///
/// The ModelContainer is created here and injected into the environment.
/// SwiftData creates the SQLite file on the Watch's local storage
/// automatically — no configuration needed beyond passing the model type.
@main
struct WaterWatchApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .modelContainer(for: WaterEntry.self)
    }
}
```

---

## ContentView

```swift
// ContentView.swift
import SwiftUI
import SwiftData
import WatchKit

struct ContentView: View {
    /// Fetches all WaterEntry records from the local SwiftData store.
    /// SwiftData re-runs this query automatically when new records are inserted,
    /// so the UI updates instantly without any manual refresh logic.
    @Query private var allEntries: [WaterEntry]
    @Environment(\.modelContext) private var context

    private let goalMl = 2000

    /// Filters allEntries to only those logged today (since midnight).
    ///
    /// "Today" is evaluated in the user's local timezone. Because this is a
    /// computed property — not a stored value — it automatically reflects the
    /// correct day even if the app was open at midnight.
    private var todayEntries: [WaterEntry] {
        let startOfDay = Calendar.current.startOfDay(for: Date())
        return allEntries.filter { $0.timestamp >= startOfDay }
    }

    private var todayTotalMl: Int {
        todayEntries.reduce(0) { $0 + $1.amountMl }
    }

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "drop.fill")
                .font(.title2)
                .foregroundStyle(.blue)

            Text("\(todayTotalMl) ml")
                .font(.title3.bold())

            Text("of \(goalMl) ml")
                .font(.caption2)
                .foregroundStyle(.secondary)

            Button(action: logDrink) {
                Text("Log a Drink")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .tint(.blue)
        }
        .padding()
    }

    private func logDrink() {
        // 1. Insert the entry — SwiftData persists it automatically
        let entry = WaterEntry(amountMl: 250)
        context.insert(entry)

        // 2. Play a success haptic so the user gets tactile confirmation
        //    even without looking at the screen
        WKInterfaceDevice.current().play(.success)
    }
}
```

---

## Action Button (Apple Watch Ultra)

The Action Button is the orange programmable button on the side of the Ultra.
When the user assigns Droplet/WaterWatch to it in Watch Settings → Action Button,
pressing it anywhere — watch face, other app, screen off — logs a drink
instantly with no UI required.

```swift
// ActionButtonHandler.swift
import WatchKit

/// Registers this app as a candidate for the Apple Watch Ultra Action Button.
///
/// The user assigns this in: Watch app on iPhone → Action Button → WaterWatch
///
/// When triggered, the handler logs a drink and plays a haptic pulse.
/// No UI is shown — the haptic is the only feedback, which is intentional:
/// the user should be able to log a drink mid-activity without stopping.
extension WaterWatchApp {
    // Action Button support is declared in the app's Info.plist:
    // WKSupportsLiveActivityLaunchAttributeTypes → not needed here
    //
    // For Action Button: add WKActionButtonType = "system" to Info.plist
    // and implement the handleActionButton(_:) scene phase modifier.
}
```

> **Note:** Full Action Button integration requires Info.plist configuration
> and a specific `scene` modifier. The implementation details are fleshed out
> during coding — the API changed between watchOS 10 and 11. The spec captures
> intent; the code captures the exact call site.

---

## Haptic Feedback

Every time a drink is logged — via button tap or Action Button — the Watch
delivers a single haptic pulse:

```swift
WKInterfaceDevice.current().play(.success)
// .success = a crisp, confident single tap
// Alternatives: .click (softer), .notification (two taps), .failure (negative)
```

The haptic confirms the log without requiring the user to look at the screen.
This matters especially for the Action Button flow where the screen may be off.

---

## Midnight Reset

Same logic as Stage 04 (iOS). The `todayEntries` computed property filters by
`timestamp >= startOfDay(for: Date())`. On a new calendar day this naturally
returns 0 — no stored reset date, no scheduled job, no background task needed.

---

## Testing

| Test | What it verifies |
|------|-----------------|
| Insert + query | Logged entry appears in `todayEntries` |
| Yesterday excluded | Entry timestamped 25 hours ago not counted |
| Sum correct | Three 250ml entries sum to 750ml |
| Empty day returns 0 | No crash or nil when no entries exist |

Tests use an in-memory `ModelContainer` (`isStoredInMemoryOnly: true`) so
they run fast without touching the real database.

---

## GitHub Actions CI

```yaml
- name: Build WaterWatch (watchOS)
  run: xcodebuild build
    -project code/programs/swift/03-watch-local/WaterWatch.xcodeproj
    -scheme WaterWatch
    -destination 'generic/platform=watchOS Simulator'
    CODE_SIGNING_ALLOWED=NO
```

---

## Out of Scope for This Stage

- iPhone companion app (Stage 04)
- WatchConnectivity sync (Stage 05)
- Offline queue / conflict resolution (Stage 05)
- Watch face complication (Stage 07)
- HealthKit (Stage 08)
- Plant mascot (Foveo)
