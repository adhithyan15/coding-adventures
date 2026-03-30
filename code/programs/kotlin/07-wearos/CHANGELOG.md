# Changelog — WaterWear (07-wearos)

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

---

## [1.0.0] — 2026-03-29

### Added

- **WaterScreen.kt** — Single-screen Wear Compose UI using `Scaffold` from
  `androidx.wear.compose.material3` with `TimeText` at the top arc. Displays
  total ml (large, green when goal met), glass count `X / 8`, and a centred
  tap button sized with `ButtonDefaults.DefaultButtonSize` (52dp).

- **WaterEntry.kt** — Room `@Entity` mapping to the `water_entries` SQLite table.
  Stores `id` (auto-increment Long), `timestampMs` (epoch ms), `amountMl` (Int, 250).

- **WaterDao.kt** — Room `@Dao` with `insert(entry)` (suspend) and
  `todayTotalMl(startOfDayMs)` returning `Flow<Int>` for live UI updates.

- **WaterDatabase.kt** — Room `@Database` singleton (double-checked locking).
  Schema version 1, `exportSchema = false`.

- **WaterRepository.kt** — Wraps the DAO. `todayTotalMl: Flow<Int>` and
  `logDrink()` (switches to `Dispatchers.IO`). Computes midnight via `Calendar`.

- **WaterViewModel.kt** — `AndroidViewModel` exposing `todayTotalMl: StateFlow<Int>`
  via `stateIn(SharingStarted.Eagerly, initialValue = 0)`. `logDrink()` launches
  a coroutine in `viewModelScope`.

- **NotificationSchedule.kt** — Data-driven list of 8 `NotificationItem` objects
  (tag, hour, title, body). Same 07:00–21:00 schedule as iOS and Android stages.

- **NotificationHelper.kt** — Creates the notification channel and schedules
  all 8 alarms via `AlarmManager.setExactAndAllowWhileIdle()`. On API 31+ checks
  `canScheduleExactAlarms()` and falls back to `setAndAllowWhileIdle()` if denied.
  Self-contained object with no state.

- **NotificationReceiver.kt** — `BroadcastReceiver` that fires when an alarm
  triggers. Shows a `NotificationCompat` notification with a tap-to-open
  `MainActivity` content intent. Calls `NotificationHelper.scheduleAll()` to
  reschedule all alarms for the next day (self-perpetuating pattern).

- **BootReceiver.kt** — `BroadcastReceiver` listening for `BOOT_COMPLETED`.
  Calls `NotificationHelper.scheduleAll()` to restore all 8 alarms after
  the watch restarts (AlarmManager alarms are erased on reboot).

- **AndroidManifest.xml** — Declares `<uses-feature android:name="android.hardware.type.watch">`
  (marks app as WearOS-only), `WAKE_LOCK`, `POST_NOTIFICATIONS`,
  `RECEIVE_BOOT_COMPLETED`, `SCHEDULE_EXACT_ALARM` permissions, and both receivers.

- **app/build.gradle.kts** — WearOS-specific dependencies:
  `compose-material3:1.0.0-alpha25`, `compose-foundation:1.4.0`, `wear:1.3.0`,
  `wear-tooling-preview:1.0.0`, Room 2.6.1, KSP 2.1.0-1.0.29. `minSdk = 30`.

- **gradle/libs.versions.toml** — Version catalogue copied from stage 04.
  `androidx-material3` entry intentionally omitted to prevent accidental
  import of the phone Material3 library.

- **README.md** — Full documentation: architecture diagram, WearOS vs phone
  differences table, build instructions, ADB deployment guide.
