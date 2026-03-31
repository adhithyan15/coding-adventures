# 07-wearos — WaterWear

**Stage 07 of the Kotlin learning series.** A standalone WearOS water-tracking
app that runs entirely on the watch — no phone companion required.

## What it does

WaterWear helps you meet your daily hydration goal of 2,000ml (8 × 250ml glasses):

- **Tap to log** — one tap records a 250ml glass and updates the total instantly
- **Live progress** — total ml and glass count displayed at a glance on the watch
- **Daily reminders** — 8 notifications throughout the day (07:00 – 21:00)
- **Persistent storage** — Room database stores every entry; totals survive app restarts
- **Reboot-safe** — BootReceiver reschedules all alarms after a watch restart

## What it teaches

| Concept | Where |
|---|---|
| Wear Compose `Scaffold` + `TimeText` | `WaterScreen.kt` |
| `androidx.wear.compose.material3` vs phone `material3` | `build.gradle.kts`, `WaterScreen.kt` |
| Room `@Entity` / `@Dao` / `@Database` | `WaterEntry.kt`, `WaterDao.kt`, `WaterDatabase.kt` |
| Repository + ViewModel pattern | `WaterRepository.kt`, `WaterViewModel.kt` |
| `Flow<Int>` → `StateFlow<Int>` → Compose `collectAsState()` | `WaterViewModel.kt`, `WaterScreen.kt` |
| AlarmManager exact alarms (Doze-safe) | `NotificationHelper.kt` |
| Self-rescheduling BroadcastReceiver | `NotificationReceiver.kt` |
| BootReceiver for alarm persistence | `BootReceiver.kt` |
| Data-driven notification schedule | `NotificationSchedule.kt` |

## How to build

```bash
cd code/programs/kotlin/07-wearos
mise exec java -- ./gradlew assembleDebug
```

The APK is at `app/build/outputs/apk/debug/app-debug.apk`.

## How to install on a watch

1. Enable Developer Options on the watch: Settings → About → tap Build Number 7 times.
2. Enable ADB debugging: Settings → Developer Options → ADB debugging.
3. Connect via ADB over Wi-Fi (WearOS 3+): `adb connect <watch-ip>:5555`
4. Install: `adb install app/build/outputs/apk/debug/app-debug.apk`

## Architecture

```
MainActivity
    └── WaterScreen (Compose UI)
            └── WaterViewModel
                    └── WaterRepository
                            └── WaterDao ← Room → water.db (SQLite)

NotificationHelper ←─── scheduleAll()
        └── AlarmManager (8 exact alarms)
                └── NotificationReceiver (fires + reschedules each alarm)

BootReceiver ──→ NotificationHelper.scheduleAll() (after reboot)
```

## WearOS vs Android phone differences

| Aspect | Phone | WearOS |
|---|---|---|
| Scaffold | `topBar`, `bottomBar`, `fab` slots | `timeText`, `vignette` slots |
| TimeText | Not available | Built-in clock arc at top of screen |
| Material library | `androidx.compose.material3` | `androidx.wear.compose.material3` |
| minSdk | Typically 21-26 | 30 (WearOS 3.0) |
| Screen | Rectangular, variable size | Round, ~393dp diameter |
| Button size | Varies | `ButtonDefaults.DefaultButtonSize` (52dp) |
| `<uses-feature>` | Optional | Required: `android.hardware.type.watch` |

## Notification schedule

| Time | Title | Key fact |
|---|---|---|
| 07:00 | Good morning! | You lose ~500ml overnight |
| 09:00 | Time to hydrate | Brain is 75% water |
| 11:00 | Nearly noon | Kidney efficiency |
| 13:00 | Lunchtime | Water aids digestion |
| 15:00 | Afternoon slump? | Dehydration causes 3pm dip |
| 17:00 | Late afternoon | Hydrate before workout |
| 19:00 | Evening reminder | Liver works overnight |
| 21:00 | Last call | Late water interrupts sleep |

## Stage progression

| Stage | Topic |
|---|---|
| 01 | Hello World |
| 02 | Water counter (in-memory) |
| 04 | Persistence with Room |
| 06 | Notifications (Android) |
| **07** | **WearOS standalone app** |
