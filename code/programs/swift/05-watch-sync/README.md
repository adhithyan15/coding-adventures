# 05-watch-sync

Stage 05 of the mobile learning series. Bidirectional sync between an iPhone
app and an Apple Watch app using WatchConnectivity's `transferUserInfo`.

## What it does

- Log a drink on the iPhone → it appears on the Watch (and vice versa).
- Works offline: drinks logged when the devices aren't in range are
  queued by the OS and delivered when they reconnect.
- No data is ever overwritten — every drink is an immutable record.

## Key design: journal architecture + union merge

Every drink is stored as a separate `WaterEntry`:

```
UUID  |  timestamp (when the user drank)  |  amountMl
```

The displayed total is always recomputed: `sum(entries where date == today)`.
There is no mutable "running counter" anywhere in the code.

When the Watch syncs offline entries back to the iPhone, the iPhone does a
**union merge by UUID**: if the UUID already exists locally, skip it; if it's
new, insert it with the original timestamp. This means:

| Scenario | Result |
|---|---|
| Phone logs 2 drinks offline | 500 ml |
| Watch logs 3 drinks offline | 750 ml |
| After sync | 1250 ml (all 5 entries, no duplicates) |

This also sets up future **Apple Health export** cleanly: each `WaterEntry`
maps 1-to-1 to an `HKQuantitySample(type: .dietaryWater)`.

## Why `transferUserInfo` not `sendMessage`

| | `transferUserInfo` | `sendMessage` |
|---|---|---|
| Delivery | Guaranteed, OS-queued | Best-effort, live only |
| Works offline | ✅ | ❌ |
| Watch must be running | No | Yes |
| Right for water logging | ✅ | ❌ |

## Architecture

```
Sources/
  Shared/
    WaterEntry.swift        — @Model, shared between both targets
    SyncPayload.swift       — [String:Any] serialisation bridge
  WaterSync/                — iPhone target
    WaterSyncApp.swift
    ContentView.swift
    ConnectivityManager.swift
  WaterSyncWatch/           — watchOS target
    WaterSyncWatchApp.swift
    WatchContentView.swift
    WatchConnectivityManager.swift
    Info.plist
```

## Running

```bash
mise exec -- xcodegen generate
# Open WaterSync.xcodeproj in Xcode
# Run WaterSync on iPhone 17 simulator
# Run WaterSyncWatch on Apple Watch Ultra 3 simulator
```
