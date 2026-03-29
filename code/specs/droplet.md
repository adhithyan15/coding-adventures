# Droplet — Water Reminder iOS App

## Overview

Droplet is a privacy-first water reminder app for iPhone and Apple Watch Ultra.
It has no server, collects no user data, shows no ads, and is fully open source.
All state lives on the user's device, in their iCloud account, or in Apple Health.

The central metaphor is a **flowering plant** that thrives when you drink water
and wilts when you neglect it. This makes the goal self-explanatory — everyone
knows what a plant needs — and creates a nurturing instinct rather than guilt.

---

## Core Principles

1. **Zero data collection** — nothing leaves the device except to the user's own
   iCloud and Apple Health accounts. No analytics, no crash reporting, no ads.
2. **Offline first** — every feature works with no internet connection.
3. **Apple-native storage only** — SwiftData (local), CloudKit (user's iCloud),
   HealthKit (Apple Health). No third-party backends.
4. **Open source** — full source on GitHub. Anyone can build and sideload their
   own copy.
5. **App Store distributed** — published under the Apple Developer Program for
   users who don't want to build from source.

---

## The Plant Mascot

The plant is the emotional core of the app. Its state is a direct function of
the user's hydration over the past rolling 7 days.

```
State         Condition                        Visual
──────────────────────────────────────────────────────────────────
Blooming      Met goal today + 5+/7 days       Full bloom, bright green,
                                               petals open, gentle sway
Healthy       Met goal today + 3–4/7 days      Green, flower slightly closed
Neutral       Partial goal today               Upright, no flower open
Drooping      Missed today, ok streak          Leaves starting to droop,
                                               flower closed
Wilting       Missed today + 2–3 bad days      Stem bent, leaves hanging
Critical      Missed 4+ of last 7 days         Brown edges, mostly wilted
Recovering    First drink after wilting         Animated straightening,
                                               one petal slowly opens
```

Recovery is **gradual, not instant**. A single drink after days of neglect
begins the recovery animation but does not jump straight to Healthy. The plant
reflects cumulative care, not a single action.

The app icon mirrors the plant state using iOS alternate app icons. The user
sees at a glance from their home screen whether their plant needs attention.

---

## Data Model

### Water Log Entry
```
id          UUID
timestamp   Date
amount_ml   Int          (default: 250ml per drink)
source      Enum         (.manual, .actionButton, .notification, .siri)
```

### Daily Summary (derived, not stored)
```
date              Date
total_ml          Int      (sum of entries for that day)
goal_ml           Int      (user's daily goal, default 2000ml)
goal_met          Bool
```

### User Settings
```
daily_goal_ml     Int      default: 2000
reminder_start    Time     default: 08:00
reminder_end      Time     default: 22:00
reminder_interval Minutes  default: 90
health_sync       Bool     default: true
icloud_sync       Bool     default: true
```

---

## Storage Strategy

```
SwiftData (local)    Water log entries, user settings, notification schedule
CloudKit             Mirror of SwiftData via automatic sync (user's iCloud only)
HealthKit            Water intake written as HKQuantitySample (.dietaryWater)
                     Also reads back to avoid double-counting if user logs
                     water elsewhere (e.g. Cronometer, Streaks)
```

HealthKit integration requires explicit user permission. If denied, the app
falls back to SwiftData only — all features still work.

---

## Notifications

Each notification carries:
- **Title:** "Time to drink water 💧"
- **Body:** One health fact (see below), rotated sequentially, never repeating
  until all facts have been shown
- **Action button:** "Log a drink" — logs 250ml without opening the app
- **Category:** `WATER_REMINDER` with the action registered at app launch

Notification schedule is computed at app launch and whenever settings change.
The system schedules up to 64 notifications in advance (iOS limit). The app
re-schedules on foreground return if fewer than 32 remain.

### Health Facts

~50 curated facts bundled in the app. Tone: conversational, not clinical.
Examples:
- "Your brain is about 75% water. When you're dehydrated, it physically shrinks
  away from your skull — that's the headache you feel."
- "Water helps your kidneys flush waste. Without enough of it, waste crystalises
  into kidney stones. Drink up."
- "Every breath you exhale contains water vapour. You lose around 400ml of water
  per day just by breathing."
- "Mild dehydration (1–2% of body weight) measurably reduces concentration and
  short-term memory. You don't feel thirsty yet at that point."

Facts are stored as a static array in Swift — no network request, no CMS,
works forever.

---

## Apple Watch App

### Architecture

The Watch app is an independent watchOS target within the same Xcode project.
It communicates with the iPhone app via WatchConnectivity (`WCSession`).

```
iPhone app  ←──WCSession──→  Watch app
     ↕                            ↕
SwiftData                    UserDefaults
HealthKit                    (local cache)
CloudKit
```

The Watch app maintains a local cache of today's log count and goal so it can
display the complication without waking the iPhone app.

### Action Button (Apple Watch Ultra)

Apps can register as the Action Button handler in watchOS. When Droplet is
selected:

1. User presses the orange Action Button
2. watchOS calls `ActionButtonSession` in the Watch app
3. Watch app logs 250ml locally + sends message to iPhone via WCSession
4. iPhone app writes to HealthKit + SwiftData
5. Watch delivers a single haptic pulse (`.success`)
6. No UI shown — the press is enough

If the iPhone is unreachable (out of range, dead), the Watch queues the log
and syncs when connectivity returns.

### Watch Face Complication

A circular complication showing:
- A small plant icon (state-aware — bloomed vs wilted)
- Today's progress (e.g. "5/8")

Tapping the complication opens the Watch app's main view.

### Watch App Main View

```
┌─────────────────────┐
│   💧  5 of 8 today  │
│                     │
│   ┌─────────────┐   │
│   │  Log Drink  │   │  ← large tap target, full width
│   └─────────────┘   │
│                     │
│  Last: 14 min ago   │
└─────────────────────┘
```

---

## iPhone App Screens

### 1. Home Screen
- The plant (large, animated, centre of screen)
- Today's progress bar below it (e.g. "1,250 / 2,000 ml")
- "Log a drink" button
- Current health fact (rotates daily)

### 2. History Screen
- Calendar view — each day coloured by goal completion
- Tap a day to see individual log entries

### 3. Settings Screen
- Daily goal (ml)
- Reminder window (start/end time)
- Reminder interval
- Apple Health sync toggle
- iCloud sync toggle (read-only display — sync happens automatically via CloudKit)
- About / Privacy Policy link

---

## Privacy

No data leaves the device except:
- **iCloud** — user's own CloudKit container, inaccessible to the developer
- **Apple Health** — user's own HealthKit store, inaccessible to the developer

No analytics. No crash reporting. No advertising SDK. No third-party frameworks.

The privacy policy (hosted on GitHub) states this in plain language and is
linked from the App Store listing and the Settings screen.

HealthKit permission prompt explains exactly what will be written:
> "Droplet would like to save your water intake to Apple Health so it appears
> alongside your other health data. Droplet cannot read any other Health data."

---

## Build Stages

### Stage 1 — Hello World
- Xcode project created with two targets: `Droplet` (iOS) and `Droplet Watch App` (watchOS)
- iPhone shows "Hello, Droplet!"
- Watch shows "Hello from your Watch!"
- Project committed to `code/programs/swift/droplet/`
- README.md and CHANGELOG.md created

### Stage 2 — Water Logging
- SwiftData model: `WaterEntry`, `UserSettings`
- Home screen: plant placeholder, progress bar, "Log a drink" button
- History screen: list of today's entries
- No notifications yet

### Stage 3 — Notifications
- Local notification scheduling
- Health facts array (50 facts)
- "Log a drink" notification action
- Settings screen: reminder window + interval

### Stage 4 — Plant Mascot
- Plant illustration with 6 states
- State machine driven by SwiftData aggregates
- Animations between states
- Recovering animation (gradual)

### Stage 5 — Dynamic App Icon
- Alternate app icons for each plant state
- Icon updated on foreground return and after each log

### Stage 6 — Apple Watch
- WatchConnectivity session
- Watch app main view
- Action Button handler
- Local cache on Watch

### Stage 7 — Watch Face Complication
- WidgetKit complication (circular)
- State-aware plant icon + progress count
- Updates via WCSession

### Stage 8 — HealthKit
- Request permission on first launch
- Write `HKQuantitySample(.dietaryWater)` on each log
- Read back today's total to avoid double-counting

### Stage 9 — App Store
- App icons all sizes
- Screenshots (iPhone 15 Pro, Apple Watch Ultra)
- App Store description and keywords
- Privacy policy page on GitHub
- TestFlight beta period (personal use)
- App Review submission

---

## App Store Compliance Notes

- **Guideline 4.2 (Minimum Functionality):** Droplet has Watch app, Action Button,
  HealthKit, complications, notifications, and animated mascot — well above the bar.
- **HealthKit:** Used meaningfully (logging dietary water). Privacy policy required
  and provided. No health data transmitted to any server.
- **Privacy Nutrition Label:** "Data Not Collected" — accurate and verifiable from
  source code.
- **Open Source:** Permitted by Apple. Source availability is noted in the App
  Store description.
- **Developer Program:** $99/year enrollment required before TestFlight or App Store.
