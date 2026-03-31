# Changelog

All notable changes to **WaterNotify** (06-notifications) are documented here.

## [1.0.0] — 2026-03-29

### Added

**App: WaterNotify** — Android water-tracking app with glasses UI and daily local notifications.

#### Glasses UI
- `GlassesRow.kt`: horizontal `LazyRow` of 8 glass emoji icons (💧 filled, 🫙 empty)
- Icons update in real time as drinks are logged — one glass fills per 250 ml
- "X of 8 glasses" and "Y ml of 2000 ml" labels below the row
- Goal-met state: button and labels turn green at 2000 ml

#### Notification system
- `NotificationSchedule.kt`: `HydrationReminder` data class + list of 8 daily reminders (7 AM – 9 PM)
- `NotificationHelper.kt`: creates notification channel ("watersync_reminders", IMPORTANCE_DEFAULT), schedules all 8 alarms via `AlarmManager.setExactAndAllowWhileIdle()`
- `NotificationReceiver.kt`: `BroadcastReceiver` that posts the notification and immediately reschedules for the next day (self-rescheduling chain pattern)
- `BootReceiver.kt`: `BroadcastReceiver` for `BOOT_COMPLETED` that reschedules all 8 alarms after device reboot

#### Permissions
- `POST_NOTIFICATIONS` runtime request at app launch (Android 13+ / API 33+)
- Permission rationale screen if denied: explains hydration reminders, provides Settings shortcut, "Continue Anyway" escape hatch
- `RECEIVE_BOOT_COMPLETED`: allows `BootReceiver` to run after reboot
- `SCHEDULE_EXACT_ALARM` + `USE_EXACT_ALARM`: required for `AlarmManager.setExact()` on API 31+

#### Room persistence (carried forward from stage 04)
- `WaterEntry.kt`: `@Entity` with `id`, `timestampMs`, `amountMl` (250 ml default)
- `WaterDao.kt`: `@Insert` suspend function + `Flow<Int>` reactive today-total query
- `WaterDatabase.kt`: singleton `@Database` with double-checked locking
- `WaterRepository.kt`: threading (Dispatchers.IO for writes) + `startOfDayMs()` helper

#### Architecture
- `WaterViewModel.kt`: `AndroidViewModel` exposing `StateFlow<Int>` via `stateIn(SharingStarted.Eagerly)`
- `WaterScreen.kt`: main Composable — observes StateFlow via `collectAsState()`
- `MainActivity.kt`: thin Activity shell — sets up channel, schedules alarms, requests permission, hosts Composable tree

#### Build configuration
- `app/build.gradle.kts`: AGP 8.10.0 + Kotlin 2.1.0 + Compose BOM 2025.05.00 + Room 2.6.1 + KSP 2.1.0-1.0.29
- `gradle/libs.versions.toml`: centralised version catalogue
- `settings.gradle.kts`: `rootProject.name = "06-notifications"`, single `:app` module
- `minSdk = 26` (Android 8.0 Oreo — minimum for `NotificationChannel`)
- `targetSdk = 35` (Android 15)
- Gradle wrapper copied from stage 04

### Technical decisions

- **AlarmManager over WorkManager**: WorkManager is deferrable and may delay tasks by hours for battery savings. `setExactAndAllowWhileIdle()` guarantees delivery even in Doze mode — required for time-precise reminders.
- **Self-rescheduling in BroadcastReceiver**: `setExact()` fires once. We reschedule for tomorrow from inside `NotificationReceiver.onReceive()`. `BootReceiver` seeds the chain after reboots.
- **Doze piercing**: `setExactAndAllowWhileIdle()` used on API 23+ so morning reminders fire despite overnight Doze.
- **FLAG_IMMUTABLE**: Applied to all `PendingIntent`s as required since API 31 (Android 12).
- **Unique requestCode per alarm**: Each alarm uses `reminder.id` (0–7) as the `PendingIntent` requestCode, preventing one alarm from overwriting another.
- **NotificationCompat**: Used instead of `Notification.Builder` for API compatibility. `BigTextStyle` expands long body text.
- **No WorkManager dependency**: Zero new external dependencies beyond stage 04. `AlarmManager` and `NotificationManager` are part of the Android SDK.
