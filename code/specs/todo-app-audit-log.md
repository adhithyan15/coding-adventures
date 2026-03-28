# Spec: Audit Log & Event Sourcing — Todo App

## Status: Implemented (v1)

## Motivation

Every entity in the app (Task, SavedView, CalendarSettings) needs a complete
history of every change made to it. This serves two distinct purposes:

**1. Audit trail** — Who changed what, when, and on which device. Surfaces as:
- Activity feeds per entity ("this task was completed 3 times this week")
- Streak and habit detection ("completed every day for 14 days")
- Celebratory moments ("you've completed 100 tasks!")
- Custom views powered by historical data, not just current state
- Debug and support ("what sequence of events led to this state?")

**2. Replay / crash recovery** — If the current state store is corrupted or
lost, the entire AppState can be reconstructed from scratch by replaying the
event log through the reducer. The current state in IndexedDB is a materialized
view of the event log — not the source of truth.

A secondary goal is a foundation for future distributed sync. Each event carries
a **vector clock** that will eventually allow events from multiple devices to be
ordered correctly without a central arbiter.

---

## Core Insight: Flux Actions Are Already Events

The Flux pattern we already use makes event sourcing nearly free:

- Every state mutation is already expressed as an immutable, serializable action
  object (`TASK_CREATE`, `TASK_UPDATE`, etc.)
- The reducer is already a pure `(state, event) → state` function
- `store.dispatch()` is already a serial queue within a single JS thread

The only gap: the current persistence middleware saves the *resulting state* of
each action, not the *action itself*. Adding an audit middleware that saves the
action before the reducer runs completes the event sourcing model.

---

## Data Model

### `VectorClock`

```typescript
type VectorClock = Record<string, number>;
// deviceId → monotonic sequence number
//
// Single device today:     { "device-abc123": 42 }
// Two devices later:       { "device-abc123": 42, "device-xyz789": 17 }
```

A vector clock is the correct abstraction for distributed ordering. Today, with
a single device, it degenerates to a simple sequence number. When sync arrives,
each device gets its own counter and the vector grows additional entries — no
schema change required.

Rules:
- Each device increments only its own counter on every event
- `A happened-before B` iff `A.clock[d] ≤ B.clock[d]` for all devices `d`
- Events where neither happened-before the other are concurrent (conflict)

### `AuditEvent`

```typescript
interface AuditEvent {
  id: string;               // UUID — stable identifier for this event
  entityId: string | null;  // UUID of the entity this event primarily affects.
                            // null for multi-entity or system events.
  entityType:               // which entity type — for filtering
    "task" | "view" | "calendar" | "system" | null;
  actionType: string;       // denormalized copy of action.type — for fast
                            // filtering without deserializing the full action
  clock: VectorClock;       // { deviceId: seq } — ordering across devices
  timestamp: number;        // Unix ms — wall clock, tiebreaker only.
                            // NOT authoritative for ordering — use clock.
  action: Action;           // full action payload — enables replay
}
```

#### Why `entityId` at the top level?

Querying "all events for task X" is a hot path (activity feeds, streak
detection). If `entityId` were buried inside `action.payload`, every query
would require deserializing every event. Denormalizing it to the top level
allows an IndexedDB index scan when index queries are added.

#### Why `actionType` at the top level?

Same reason — filtering by action type (e.g., "all TASK_TOGGLE_STATUS events
to find completions") is common. Denormalizing avoids full-payload scans.

#### Why wall-clock `timestamp` is not sufficient for ordering

Two devices can have clock skew of seconds or minutes. A `timestamp`-ordered
sort would incorrectly order events from different devices. The vector clock
provides causal ordering; `timestamp` is only used as a tiebreaker when two
events have causally unrelated clocks.

### `StateSnapshot`

```typescript
interface StateSnapshot {
  id: string;
  clock: VectorClock;  // the clock at snapshot time
  timestamp: number;
  state: AppState;     // full serialized AppState
}
```

Snapshots are checkpoints. On startup, the app loads the latest snapshot then
replays only events that post-date it — not the entire event log. This keeps
replay O(recent events) not O(all events ever).

---

## Entity ID Pre-Generation

A subtle design constraint: the audit middleware runs **before** `next()` (i.e.,
before the reducer), which means it must know the new entity's UUID before the
reducer assigns it.

Previously, `crypto.randomUUID()` was called inside the reducer's `TASK_CREATE`
case. This made it impossible to log the new task's entityId pre-reducer.

**Fix:** Move UUID generation into `createTaskAction`. The action now carries
`id: crypto.randomUUID()`. The reducer reads `action.id` (with fallback to
`crypto.randomUUID()` for backward-compat with raw test objects). The audit
middleware can read the entityId from the action before the reducer ever runs.

This is also a correctness improvement for replay: replaying `TASK_CREATE`
events now always produces the same task ID (the one stored in the event),
not a new random one.

---

## Middleware Pipeline

```
dispatch(action)
    │
    ▼
┌─────────────────────────────────────────────┐
│ Audit Middleware  (runs BEFORE next())       │
│                                             │
│ 1. tick the device's vector clock            │
│ 2. extract entityId from action              │
│ 3. write AuditEvent to "events" store        │
│    (Write-Ahead Log — crash-safe)            │
│                                             │
│ next()                                      │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ Reducer  (pure function)                    │
│ (state, action) → newState                  │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ Persistence Middleware  (runs AFTER next())  │
│                                             │
│ updates "todos" / "views" / "calendars"     │
│ with new state records                      │
└─────────────────────────────────────────────┘
```

### Write-Ahead Log (WAL) guarantee

The audit middleware writes the event to "events" **before** calling `next()`.
If the app crashes between the audit write and the persistence write, the event
log is complete but the state store is one step behind. On next startup, the
app can detect and replay the un-applied event. This is the same pattern used
by databases (Postgres WAL, SQLite WAL) and distributed logs (Kafka).

### Actions not logged

Two action types are intentionally excluded from the audit log:

- **`STATE_LOAD`** — This is a hydration event dispatched once on startup to
  load persisted state into the store. It is not a user action and should not
  appear in activity feeds. Logging it would create a spurious event every
  time the app loads.

- **`VIEW_SET_ACTIVE`** — Tab navigation. Ephemeral, high-frequency, and
  semantically uninteresting for audit purposes. Logging it would drown out
  meaningful events in activity feeds.

---

## Storage Layout (IndexedDB v3)

```
"todos"     — current task state           (v1, unchanged)
"views"     — current view state           (v2, unchanged)
"calendars" — current calendar state       (v2, unchanged)
"events"    — append-only audit event log  (v3, NEW)
"snapshots" — periodic state checkpoints   (v3, NEW)
```

Future: add an index on `events.entityId` for O(log n) `getActivitiesForEntity`
queries. Currently implemented as a full scan + in-memory filter, which is
acceptable for ≤ 500 events (the compaction threshold).

---

## Query API

### `getActivitiesForEntity(storage, entityId, options?)`

Returns the ordered history of a single entity. The primary surface for
activity feeds and streak detection.

```typescript
getActivitiesForEntity(
  storage: KVStorage,
  entityId: string,
  options?: {
    limit?: number;   // return the N most recent events
    since?: number;   // Unix ms — only events after this timestamp
  }
): Promise<AuditEvent[]>
```

Returns events sorted oldest-first (natural replay order). Most callers want
"what happened to this task?" which is a chronological story.

### `getRecentActivities(storage, options?)`

Cross-entity recent activity. Powers a global activity feed.

```typescript
getRecentActivities(
  storage: KVStorage,
  options?: {
    limit?: number;
    since?: number;
    entityType?: "task" | "view" | "calendar" | "system" | null;
  }
): Promise<AuditEvent[]>
```

Returns events sorted newest-first (most recent at index 0). Designed for
"what happened recently" displays.

### Future queries (not yet implemented)

```typescript
// All task completion events in a date range — for streak detection
getCompletionEvents(taskId, dateRange): Promise<AuditEvent[]>

// All events for tasks with a given category — for habit analysis
getActivitiesForCategory(category, options): Promise<AuditEvent[]>

// Replay state from scratch or from a snapshot
replayFromEventLog(storage): Promise<AppState>
```

---

## Log Compaction

Without compaction, the event log grows without bound. Compaction writes a
full-state snapshot then deletes events that preceded it.

### Triggers

1. **On startup** — if the event count exceeds `COMPACT_THRESHOLD` (500), compact
   before mounting React. The user never sees it.

2. **On page hide** — `document.visibilitychange` fires when the tab is
   backgrounded or the browser closes. This is the most reliable "last chance"
   hook, especially on mobile where `beforeunload` is suppressed.

### Compaction procedure

```
1. Write StateSnapshot { clock, timestamp, state } to "snapshots"
   (crash here = no events deleted, safe to retry)

2. Read all events from "events"

3. Delete events where clock ≤ current clock
   (fire-and-forget — same trade-off as persistence writes)
```

Writing the snapshot before trimming events means a crash between steps 1 and 3
leaves you with a redundant snapshot and some extra events — both self-correcting
on the next compaction cycle.

### Compaction and multi-device (future)

When sync is added, compaction must be coordinated: you cannot trim events that
other devices haven't yet received. The snapshot must be marked with the
minimum vector clock across all known devices, not the local clock. This is
tracked as a future enhancement; for single-device, the local clock is always
the minimum.

---

## Device Identity

`getDeviceId()` returns a UUID stored in `localStorage` under `"ca-device-id"`.
Generated on first visit, stable for the lifetime of the browser installation.
This becomes the key in the vector clock for this device.

The vector clock itself is stored under `"ca-vector-clock"` in `localStorage`
as a JSON string. Using localStorage (not IndexedDB) for the clock is a
deliberate choice: the audit middleware is synchronous (it cannot `await` without
changing the entire middleware signature), so the clock must be readable and
writable without a Promise.

---

## Future Work

| Capability | Requires |
|---|---|
| `replayFromEventLog()` — full crash recovery | Already possible with current schema |
| Index scan for `getActivitiesForEntity` | Add IDB index on `events.entityId` |
| Streak detection | `getActivitiesForEntity` + date-grouping utility |
| Custom views powered by activity data | Query API + ViewConfig extension |
| Multi-device sync compaction coordination | Min-clock across devices |
| Undo / redo | Replay to event N-1 |
| Agent attribution | `parentEventId` field on AuditEvent |
| Per-entity retention policies | Configurable compaction per entityType |
