# Mobile 05 — Watch Sync

## Overview

The fifth app in the mobile learning series. Brings together Stage 03 (Watch
local storage) and Stage 04 (iPhone local storage) and adds bidirectional sync
via WatchConnectivity. Drinks logged on the Watch while the phone is out of
range are queued and delivered automatically when the two devices reconnect.

This stage teaches WatchConnectivity, guaranteed-delivery message transfer,
offline queue design, and conflict resolution between two independent stores.

---

## Learning Goals

- Activate and manage `WCSession` on both iOS and watchOS
- Distinguish `sendMessage` (live, requires reachability) from
  `transferUserInfo` (queued, guaranteed delivery — right for this use case)
- Design an offline queue using SwiftData on the Watch
- Flush the queue when `sessionReachabilityDidChange` fires
- Merge incoming entries from Watch into the iPhone's SwiftData store
- Avoid double-counting (idempotent sync via UUID deduplication)

---

## Why `transferUserInfo` Not `sendMessage`

```
sendMessage       — requires both apps running AND in Bluetooth range.
                    Fails silently if Watch app is not foregrounded.
                    Wrong for background logging (Action Button).

transferUserInfo  — queued by the OS. Delivered when connectivity returns,
                    even if neither app is running at delivery time.
                    Survives Watch restarts, phone restarts, and long gaps.
                    Correct for offline-first sync.
```

For water logging, the user may press the Action Button while swimming, hiking,
or working — phone nowhere near. `transferUserInfo` is the only primitive that
guarantees those logs reach the iPhone eventually.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Apple Watch (watchOS)                                        │
│                                                               │
│  SwiftData store                                              │
│  ┌────────────────┐    ┌──────────────────────────────────┐  │
│  │  WaterEntry[]  │    │  PendingSyncEntry[]               │  │
│  │  (local log)   │    │  (queued, not yet confirmed)      │  │
│  └────────────────┘    └──────────────────────────────────┘  │
│                                    │                          │
│                        transferUserInfo (OS-queued)           │
└────────────────────────────────────┼──────────────────────────┘
                                     │
                   Bluetooth / WiFi (when in range)
                                     │
┌────────────────────────────────────▼──────────────────────────┐
│  iPhone (iOS)                                                  │
│                                                               │
│  SwiftData store                                              │
│  ┌────────────────┐                                           │
│  │  WaterEntry[]  │  ← deduped by UUID before insert          │
│  │  (local log)   │                                           │
│  └────────────────┘                                           │
│          │                                                    │
│       HealthKit                                               │
└───────────────────────────────────────────────────────────────┘
```

---

## Offline Queue Design

When a drink is logged on the Watch:

1. Insert `WaterEntry` into Watch's local SwiftData store (immediate)
2. Insert `PendingSyncEntry` into a separate pending table (UUID + payload)
3. Attempt `transferUserInfo` immediately — OS queues it if unreachable

When the phone comes back in range:

4. `sessionReachabilityDidChange` fires on the Watch
5. Watch flushes any remaining `PendingSyncEntry` records via `transferUserInfo`
6. iPhone receives via `session(_:didReceiveUserInfo:)`
7. iPhone checks UUID against existing entries — inserts only if not present
8. Watch receives delivery confirmation (optional ACK via `sendMessage`)
9. `PendingSyncEntry` records deleted from Watch queue

### PendingSyncEntry Model

```swift
@Model
final class PendingSyncEntry {
    var id: UUID          // same UUID as the WaterEntry it represents
    var timestamp: Date
    var amountMl: Int
    var createdAt: Date   // for queue ordering / debugging

    init(from entry: WaterEntry) {
        self.id = entry.id
        self.timestamp = entry.timestamp
        self.amountMl = entry.amountMl
        self.createdAt = Date()
    }
}
```

---

## Conflict Resolution

Both devices have independent stores. A drink logged on the iPhone is not
automatically on the Watch and vice versa. For this stage, we use a simple
**union merge** — never delete, never overwrite:

```
Merge rule: INSERT OR IGNORE (by UUID)
```

- Watch → iPhone: entries transferred, iPhone inserts if UUID not present
- iPhone → Watch: nice-to-have for this stage, out of scope (Watch is
  read-only for incoming sync in Stage 05)

This means the iPhone always has the full picture. The Watch has its own local
picture. They are eventually consistent — not real-time consistent.

Full bidirectional sync (iPhone → Watch) is added when the watch face
complication is introduced (Stage 07), because the complication needs the
iPhone's total (which includes manual iPhone logs).

---

## WCSession Lifecycle

Both apps must activate `WCSession` at launch, before any UI appears.

```swift
// iOS — ConnectivityManager.swift
class ConnectivityManager: NSObject, WCSessionDelegate, ObservableObject {
    static let shared = ConnectivityManager()
    @Published var isWatchReachable = false

    func start() {
        guard WCSession.isSupported() else { return }
        WCSession.default.delegate = self
        WCSession.default.activate()
    }

    // Called when Watch sends a queued payload
    func session(_ session: WCSession,
                 didReceiveUserInfo userInfo: [String: Any]) {
        guard
            let idString = userInfo["id"] as? String,
            let id = UUID(uuidString: idString),
            let timestamp = userInfo["timestamp"] as? TimeInterval,
            let amountMl = userInfo["amountMl"] as? Int
        else { return }

        // Insert only if this UUID is not already in the store
        // (idempotent — safe to call multiple times with same data)
        insertIfAbsent(id: id,
                       timestamp: Date(timeIntervalSince1970: timestamp),
                       amountMl: amountMl)
    }

    func sessionReachabilityDidChange(_ session: WCSession) {
        DispatchQueue.main.async {
            self.isWatchReachable = session.isReachable
        }
    }
}
```

```swift
// watchOS — WatchConnectivityManager.swift
class WatchConnectivityManager: NSObject, WCSessionDelegate, ObservableObject {
    static let shared = WatchConnectivityManager()

    func start() {
        // isSupported() always returns true on watchOS — no guard needed
        WCSession.default.delegate = self
        WCSession.default.activate()
    }

    func syncEntry(_ entry: WaterEntry) {
        let payload: [String: Any] = [
            "id":        entry.id.uuidString,
            "timestamp": entry.timestamp.timeIntervalSince1970,
            "amountMl":  entry.amountMl
        ]
        // transferUserInfo is guaranteed delivery — OS queues and retries
        WCSession.default.transferUserInfo(payload)
    }

    func sessionReachabilityDidChange(_ session: WCSession) {
        if session.isReachable {
            flushPendingQueue()
        }
    }
}
```

---

## Sync Indicator on iPhone

The iPhone UI shows a subtle sync status:

```
┌──────────────────────────────┐
│         💧  1,500 ml today   │
│   ━━━━━━━━━━━━━━━━━░░░       │
│         of 2,000 ml          │
│                              │
│   ┌──────────────────────┐   │
│   │     Log a Drink      │   │
│   └──────────────────────┘   │
│                              │
│   ⌚ Watch synced · 3 min ago │  ← timestamp of last sync
└──────────────────────────────┘
```

If the Watch has never synced or is unreachable, the indicator reads:
`⌚ Watch not connected`

---

## Testing

| Test | What it verifies |
|------|-----------------|
| Queue flush on reconnect | Pending entries sent when `sessionReachabilityDidChange` fires |
| UUID deduplication | Same entry sent twice → only one row in iPhone store |
| Offline accumulation | 5 entries logged offline → all 5 delivered on reconnect |
| Payload encoding/decoding | `[String: Any]` round-trips without data loss |

WCSession cannot be unit-tested against real hardware — use mock delegates
and dependency injection to test the sync logic independently of the session.

---

## Out of Scope for This Stage

- iPhone → Watch sync (Stage 07, driven by complication needs)
- Conflict resolution beyond union merge
- HealthKit (Stage 08)
- Watch face complication (Stage 07)
- Plant mascot (Foveo)
