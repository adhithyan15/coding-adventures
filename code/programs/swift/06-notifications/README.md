# 06-notifications

Stage 06 of the mobile learning series. Adds a glasses-based UI and
science-backed local notifications to the iPhone water tracker.

## What's new

- **Glasses UI** — 8 drop icons fill left-to-right as you log drinks. Still
  stores data as ml underneath (required for Apple Health in a future stage).
- **Local notifications** — 8 daily reminders at fixed 2-hour intervals using
  `UNCalendarNotificationTrigger`. Fires at wall-clock times, not relative
  to app launch. Works across reboots and app updates.
- **Health facts** — each notification carries a distinct science fact about
  what water does in your body right now.
- **Morning emphasis** — the 07:00 reminder specifically encourages 2 glasses
  to compensate for overnight fluid loss (~500 ml via respiration).

## Glasses calculation

```
1 glass  = 250 ml  (IOM standard)
goal     = 8 glasses = 2,000 ml
filled   = min(todayMl / 250, 8)   ← integer division, no partial fills
```

Data is always stored as `amountMl: Int`. The glass display is a UI-only
computation that maps directly to `HKQuantitySample` for Apple Health export.

## Notification schedule

| Time  | Health fact topic |
|-------|-------------------|
| 07:00 | Overnight fluid loss (~500 ml) — encourages 2 glasses |
| 09:00 | Brain is 75% water — dehydration slows thinking |
| 11:00 | Kidney efficiency |
| 13:00 | Digestion before meals |
| 15:00 | Afternoon slump = dehydration disguise |
| 17:00 | Muscles are 75% water |
| 19:00 | Liver/kidney overnight processing |
| 21:00 | Stop here — late water interrupts sleep |

## Running

```bash
mise exec -- xcodegen generate
# Open WaterNotify.xcodeproj in Xcode
# Run on iPhone 17 simulator

# Reset notification permissions for testing:
xcrun simctl privacy booted reset all com.codingadventures.waternotify
```
