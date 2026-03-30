# 04 — Persistence (Android)

Part of the [mobile learning series](../../README.md). Extends the water
counter with local persistence via Room. Close the app, reopen it —
the counter is exactly where you left it.

## What this stage teaches

- Room `@Entity`, `@Dao`, `@Database`
- `Flow<Int>` queries that re-emit on every table change
- `ViewModel` + `StateFlow` binding to Compose UI via `collectAsState()`
- In-memory Room database for unit tests

## Running locally

```bash
cd code/programs/kotlin/04-persistence
./gradlew assembleDebug
# Install on emulator:
adb install app/build/outputs/apk/debug/app-debug.apk
```

## Testing persistence manually

```bash
# Log drinks, then force-stop the app:
adb shell am force-stop com.codingadventures.waterpersist
# Relaunch — counter should restore:
adb shell monkey -p com.codingadventures.waterpersist 1
```

## Series

| Stage | What it builds |
|-------|---------------|
| [01](../../kotlin/01-hello-world/) | Hello World |
| [02](../../kotlin/02-water-counter/) | Ephemeral counter |
| **04** | **Android — local persistence (this app)** |
| 05 | Watch ↔ iPhone sync (iOS only) |
