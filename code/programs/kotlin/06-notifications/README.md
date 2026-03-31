# 06-notifications — WaterNotify

**Stage 06** of the Android water-tracking series. This stage adds local push
notifications (AlarmManager + BroadcastReceiver) and a glasses-based UI on top of
the Room persistence layer introduced in stage 04.

## What it does

**WaterNotify** sends 8 daily hydration reminders at fixed times (7 AM – 9 PM) and
shows a row of glass icons (💧 filled / 🫙 empty) to visualise your daily progress
toward a 2,000 ml goal.

### Features

| Feature | Details |
|---|---|
| Glasses UI | 8 emoji icons in a LazyRow: 💧 = drunk, 🫙 = remaining |
| Progress labels | "X of 8 glasses" and "Y ml of 2000 ml" |
| Log button | Logs 250 ml per tap; persisted in Room SQLite |
| Daily reminders | 8 AlarmManager.setExact() alarms — fire even in Doze mode |
| Self-rescheduling | NotificationReceiver reschedules for next day after each fire |
| Boot persistence | BootReceiver reschedules all alarms after device reboot |
| Permission | POST_NOTIFICATIONS request at startup (Android 13+ / API 33+) |
| Permission rationale | Graceful fallback if user denies — explains benefit, offers Settings shortcut |

### Notification schedule

| Time | Title | Science fact |
|---|---|---|
| 7:00 AM | Good morning! Start hydrated | 500 ml lost overnight through breathing |
| 9:00 AM | Mid-morning check-in | Brain is 75% water; 1–2% loss reduces focus |
| 11:00 AM | Nearly noon | Kidneys filter waste continuously |
| 1:00 PM | Lunchtime hydration | Water before meals aids digestion |
| 3:00 PM | Afternoon slump? | 3pm dip is often dehydration in disguise |
| 5:00 PM | Late afternoon | Muscles are 75% water |
| 7:00 PM | Evening reminder | Liver + kidneys work overnight |
| 9:00 PM | Last call for today | Late water interrupts sleep |

## Architecture

```
MainActivity
  └── WaterScreen (Composable)
       └── GlassesRow (Composable)

WaterViewModel (AndroidViewModel)
  └── WaterRepository
       └── WaterDao ─── WaterDatabase (Room/SQLite)

NotificationHelper (object)          ← schedules alarms at startup + boot
NotificationReceiver (BroadcastReceiver) ← fires notification + reschedules
BootReceiver (BroadcastReceiver)     ← reschedules all after reboot
NotificationSchedule.kt              ← data class + list of 8 reminders
```

### How AlarmManager daily repeats work

Android has no direct equivalent of iOS's `UNCalendarNotificationTrigger(repeats: true)`.
We implement daily repetition with a self-rescheduling chain:

```
App launch
  └── NotificationHelper.scheduleAll()
        └── setExactAndAllowWhileIdle() × 8  ← one alarm per reminder

Alarm fires at 7:00 AM
  └── NotificationReceiver.onReceive()
        ├── NotificationManager.notify()     ← show the notification
        └── NotificationHelper.scheduleNextDay()  ← reschedule for tomorrow 7:00 AM

Device reboots (AlarmManager memory cleared)
  └── BootReceiver.onReceive()
        └── NotificationHelper.scheduleAll()  ← reschedule all 8
```

### Why AlarmManager, not WorkManager?

WorkManager is the right tool for *deferrable* background tasks (sync, uploads).
It honours Doze mode and may delay work by minutes or hours to save battery.

For hydration reminders that must fire at **7:00 AM sharp**, we use
`AlarmManager.setExactAndAllowWhileIdle()` which wakes the device even in Doze
mode. This is the same trade-off iOS makes with `UNCalendarNotificationTrigger`.

## How to build

### Prerequisites

- Android SDK 35 installed
- `mise` with Java configured (the project uses Gradle wrapper)

### Build

```bash
cd code/programs/kotlin/06-notifications
mise exec java -- ./gradlew assembleDebug
```

The APK will be at `app/build/outputs/apk/debug/app-debug.apk`.

### Install on device/emulator

```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

### Clean build

```bash
mise exec java -- ./gradlew clean assembleDebug
```

## File structure

```
06-notifications/
├── app/src/main/
│   ├── java/com/codingadventures/waternotify/
│   │   ├── MainActivity.kt          — entry point, permission request
│   │   ├── WaterScreen.kt           — main Composable screen
│   │   ├── GlassesRow.kt            — 8-glass emoji row
│   │   ├── WaterViewModel.kt        — StateFlow bridge between Room and UI
│   │   ├── WaterEntry.kt            — @Entity (same model as stage 04)
│   │   ├── WaterDao.kt              — @Dao with Flow<Int> todayTotal
│   │   ├── WaterDatabase.kt         — @Database singleton
│   │   ├── WaterRepository.kt       — threading + data access
│   │   ├── NotificationSchedule.kt  — HydrationReminder data class + 8 items
│   │   ├── NotificationHelper.kt    — channel + alarm scheduling
│   │   ├── NotificationReceiver.kt  — BroadcastReceiver: show + reschedule
│   │   └── BootReceiver.kt          — BroadcastReceiver: reschedule on boot
│   ├── res/values/strings.xml
│   └── AndroidManifest.xml
├── app/build.gradle.kts
├── gradle/libs.versions.toml
├── build.gradle.kts
├── settings.gradle.kts
├── gradlew / gradlew.bat
├── README.md
└── CHANGELOG.md
```

## How this relates to the iOS version

This Android stage directly parallels the iOS stage 06-notifications:

| iOS | Android |
|---|---|
| `UNUserNotificationCenter.requestAuthorization` | `ActivityResultContracts.RequestPermission` |
| `UNCalendarNotificationTrigger(repeats: true)` | `AlarmManager.setExact()` + self-reschedule |
| `UNNotificationRequest` | `NotificationCompat.Builder` + `NotificationManager.notify()` |
| `UNNotificationCenter` | `NotificationChannel` + `NotificationManager` |
| App delegate `didFinishLaunching` | `MainActivity.onCreate()` |
| Background push (automatic) | `BootReceiver` + `RECEIVE_BOOT_COMPLETED` |

The notification texts (titles, bodies, times) are identical between platforms,
maintained in `NotificationSchedule.kt` (Android) and the iOS equivalent.

## Key learnings

1. **AlarmManager vs. WorkManager**: Use AlarmManager for exact-time, time-sensitive
   triggers. Use WorkManager for deferrable background work.

2. **setExactAndAllowWhileIdle()**: The Doze-piercing variant of setExact(). Required
   so alarms fire even when the screen has been off overnight.

3. **Self-rescheduling pattern**: Fire once in NotificationReceiver, schedule the
   next day. BootReceiver seeds the chain again after reboots.

4. **PendingIntent requestCode uniqueness**: Each alarm needs a unique requestCode or
   AlarmManager overwrites earlier alarms with later ones.

5. **FLAG_IMMUTABLE**: Required since API 31 for security. The OS cannot mutate
   PendingIntents created with this flag.

6. **POST_NOTIFICATIONS runtime permission**: New in Android 13. Must be requested
   at runtime, not just declared in the manifest. Always show a rationale if denied.
