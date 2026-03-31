# Changelog — 05-watch-sync

## [1.0.0] — 2026-03-29

### Added
- Bidirectional Watch ↔ iPhone sync via `WatchConnectivity.transferUserInfo`
- `ConnectivityManager` (iOS): `WCSessionDelegate`, sends and receives entries
- `WatchConnectivityManager` (watchOS): mirror of iOS manager, no inactive/deactivate stubs
- `SyncPayload` (Shared): lightweight `[String:Any]` serialisation struct
- `WaterEntry` (Shared): `@Model` with UUID deduplication key, shared across both targets
- Journal architecture: every drink is an immutable record; total is always recomputed from the log
- UUID-based idempotent deduplication: same entry delivered twice never creates a duplicate
- Original timestamp preserved on sync: Watch entries appear at the time the user drank, not the time they arrived on the phone
- Watch reachability indicator on iPhone; phone reachability indicator on Watch
- Haptic feedback on Watch confirming each logged drink

### Architecture decisions
- `transferUserInfo` chosen over `sendMessage` for guaranteed offline delivery
- `SyncPayload` extracted to `Sources/Shared/` so both targets compile it (not iOS-only)
- Journal design anticipates future Apple Health export (`HKQuantitySample` per entry)
