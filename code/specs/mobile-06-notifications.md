# Mobile 06 — Glasses UI + Local Notifications

## Overview

The sixth app in the mobile learning series. Builds on the patterns from
Stage 05 (Watch sync) by adding two features to the iPhone app:

1. **Glasses UI**: Replace the raw-ml counter with 8 glass icons. As the user
   logs drinks, glasses fill left-to-right. Raw ml is still stored in SwiftData
   and shown as a secondary label (required for future Apple Health export).

2. **Local Notifications**: A static, hardcoded daily schedule using
   `UNUserNotificationCenter`. No server. No network. No user customisation.
   8 reminders per day, every 2 hours, with a special morning notification at
   07:00 that encourages 2 glasses after overnight dehydration.

This stage is iOS-only. watchOS notifications are a future stage.

---

## Learning Goals

- Convert a raw-number display into a glanceable icon grid
- Understand the `UNUserNotificationCenter` authorisation flow on iOS
- Schedule `UNCalendarNotificationTrigger` (wall-clock, not interval)
- Understand why calendar triggers survive app restarts and reboots
- Know the 64-notification system limit and how to stay under it
- Write testable notification logic by injecting the centre as a protocol
- Use `MockNotificationCenter` in unit tests (no real system centre needed)

---

## Key Design Decisions

### 1 glass = 250 ml

250 ml is the medically established "standard glass" (Institute of Medicine
Dietary Reference Intakes, 2004). The 8-glasses-per-day recommendation is
directly equivalent to 8 × 250 ml = 2,000 ml. Using a single constant
`glassSize = 250` means every calculation stays consistent and the ml ↔
glasses conversion is trivially reversible for Apple Health.

### Calendar trigger, not time interval

`UNCalendarNotificationTrigger` fires at a wall-clock time every day (e.g.,
07:00), independent of when the app was last launched. This is correct for
a health reminder app where the schedule is anchored to the user's day, not
to app sessions.

`UNTimeIntervalNotificationTrigger(timeInterval: 7200, repeats: true)` would
drift — if the app launches at 14:33, the first reminder fires at 16:33, not
17:00. Avoid it for scheduled reminders.

### No badge count

Badge numbers on health apps register as "failures" in the user's mind if
they are behind on hydration. This app is designed to encourage, not accuse.
`content.badge` is never set.

### Data stored as ml, displayed as glasses

Apple Health's `HKQuantitySample` for water uses millilitres (or fluid
ounces). The data layer never changes — `amountMl: Int` is the source of
truth. The glasses display is purely a UI computation:

```
filledCount = todayTotalMl / glassSize   (integer division)
```

---

## Static Notification Schedule

Science basis: adults need ~2–2.5L/day. Kidney filtration limits processing
to ~1L/hour maximum. Small, regular amounts (250 ml every 2 hours) across
waking hours is more effective than large boluses. Morning hydration after
sleep is especially important — ~500 ml is lost overnight through respiration.

| # | Time  | ID                          | Title                     | Body |
|---|-------|-----------------------------|---------------------------|------|
| 1 | 07:00 | `watersync.morning`         | Good morning! Start hydrated | You lose around 500 ml overnight just breathing. Two glasses now restores your baseline before the day begins. |
| 2 | 09:00 | `watersync.mid-morning`     | Mid-morning check-in      | Your brain is 75% water. Even mild dehydration — just 1–2% fluid loss — reduces focus and reaction time. |
| 3 | 11:00 | `watersync.late-morning`    | Nearly noon               | Water helps your kidneys flush waste continuously. Staying topped up keeps them running at full efficiency. |
| 4 | 13:00 | `watersync.lunch`           | Lunchtime hydration       | A glass before meals aids digestion and gives your stomach a head start on breaking down food. |
| 5 | 15:00 | `watersync.afternoon`       | Afternoon slump?          | The 3pm energy dip is often dehydration in disguise. A glass of water works faster than another coffee. |
| 6 | 17:00 | `watersync.late-afternoon`  | Late afternoon            | Muscles are about 75% water. If you exercise after work, start hydrating now — not when you arrive. |
| 7 | 19:00 | `watersync.evening`         | Evening reminder          | Your liver and kidneys work through the night processing today. Keep them well supplied. |
| 8 | 21:00 | `watersync.night`           | Last call for today       | Good time to stop for the night. Late drinking can interrupt sleep. Log your final glass and you are done. |

Copy notes:
- The 07:00 body is the only one that suggests two glasses
- The 21:00 body cues the user to stop — reducing anxiety about late hydration
- No emoji in notification copy (render inconsistently across lock-screen views)
- Each fact is independently interesting — no repetition across 8 slots
- All bodies are 80–110 characters — readable in 3 seconds on the lock screen

---

## Glasses UI

### Calculation

```
glassSize    = 250
goalGlasses  = 8
filledCount  = min(todayTotalMl / glassSize, goalGlasses)
```

Integer division is intentional. A glass is either full or not.

### Layout

8 drop icons in a horizontal row (or 2 rows of 4 on narrow devices):

```
💧 💧 💧 ○ ○ ○ ○ ○     ← 3 filled, 5 empty
3 of 8 glasses
750 ml of 2,000 ml     ← secondary, for Apple Health awareness
```

- Filled: `drop.fill` in `.blue` (`.green` when goal met)
- Empty: `drop` in `.secondary`
- Layout: `LazyVGrid` with adaptive columns, min 30pt per icon
- Animation: spring scale on newly filled glass; `.numericText()` on labels
- Goal met: all icons turn green, button says "Goal met!"

### GlassesView component

`GlassesView(filledCount: Int, total: Int)` is a pure display component with
no SwiftData dependency. The parent computes `filledCount` and passes it in.
This separation makes the component independently testable.

---

## NotificationManager Architecture

```swift
@Observable final class NotificationManager: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NotificationManager()

    var authStatus: UNAuthorizationStatus = .notDetermined
    var nextReminderTime: Date?

    func setup()          // called in App.init() — request auth, schedule
    func scheduleAll()    // removeAll then add 8 requests
}
```

### Authorisation flow

```
App.init()
  │
  └─► NotificationManager.setup()
          │
          ├─ .notDetermined ──► system dialog
          │                         ├─ Allow  ──► scheduleAll()
          │                         └─ Deny   ──► authStatus = .denied
          │
          ├─ .authorized   ──► scheduleAll()
          └─ .denied       ──► update authStatus (show Settings banner)
```

### Scheduling (idempotent)

```swift
func scheduleAll() {
    center.removeAllPendingNotificationRequests()
    for item in NotificationSchedule.all {
        let content       = UNMutableNotificationContent()
        content.title     = item.title
        content.body      = item.body
        content.sound     = .default

        var dc            = DateComponents()
        dc.hour           = item.hour
        dc.minute         = 0
        let trigger       = UNCalendarNotificationTrigger(
                                dateMatching: dc, repeats: true)
        let request       = UNNotificationRequest(
                                identifier: item.id,
                                content: content,
                                trigger: trigger)
        center.add(request, withCompletionHandler: nil)
    }
}
```

Safe to call on every launch — `removeAll` + stable identifiers = idempotent.

### Foreground delivery

Implement `willPresent` delegate method returning `[.banner, .sound]` so
reminders appear even when the app is open. Without this, iOS silently
swallows the notification while the app is foreground.

### Protocol for testability

`UNUserNotificationCenter` cannot be instantiated in unit tests. Define a
minimal `NotificationScheduling` protocol that the real centre satisfies and
a `MockNotificationCenter` implements for tests.

---

## Unit Tests

### GlassesLogicTests

| Test | Expectation |
|------|-------------|
| 0 ml | 0 filled |
| 250 ml | 1 filled |
| 2000 ml | 8 filled |
| 2500 ml | 8 filled (capped) |
| 750 ml | 3 filled (integer division) |
| 300 ml | 1 filled (floor, not 1.2) |

### NotificationScheduleTests

| Test | Expectation |
|------|-------------|
| Count | Exactly 8 items |
| Unique IDs | No two items share an identifier |
| Hour range | All hours in [7, 21] |
| Ascending | Hours in ascending order |
| No duplicate hours | No two items at the same hour |
| Morning slot | Item with hour == 7 exists |
| Non-empty copy | All titles and bodies non-empty |

### NotificationManagerTests

| Test | Expectation |
|------|-------------|
| `scheduleAll` count | MockCenter receives exactly 8 `add` calls |
| Remove-before-add | `removeAll` called before first `add` |
| Stable identifiers | Each request ID matches schedule item ID |
| Calendar triggers | All triggers are `UNCalendarNotificationTrigger` |
| `repeats: true` | All triggers have `repeats == true` |
| Correct hour | 07:00 request has `trigger.dateComponents.hour == 7` |

---

## Known Gotchas

1. **Delegate before first scene**: Set `UNUserNotificationCenter.current().delegate`
   in `App.init()`, not in `onAppear`. Notifications delivered at launch may
   fire before `onAppear` runs.

2. **Calendar trigger, not interval**: See Key Design Decisions above.

3. **Always removeAll before scheduling**: Prevents stale requests accumulating
   across app updates or identifier changes.

4. **Never call `UNUserNotificationCenter.current()` in tests**: It requires
   a running app host. Use `MockNotificationCenter` injected via protocol.

5. **64-notification limit**: We use 8 repeating calendar triggers = 8 slots.
   Never approach the limit unless non-repeating per-date triggers are used.

6. **No `NSUserNotificationsUsageDescription`**: That key is macOS-only. iOS
   notification permission is purely runtime via `requestAuthorization`.

7. **Simulator permission reset during development**:
   `xcrun simctl privacy booted reset all com.codingadventures.waternotify`

---

## Out of Scope

- watchOS notifications (future stage)
- User-customisable notification times (future stage)
- Notification actions ("Log a Drink" button in banner) (future stage)
- Apple Health / HealthKit integration (future stage)
- iCloud sync (Foveo stage)
- Watch face complication (future stage)
