/**
 * audit.ts — Audit log / event-sourcing module for the todo app.
 *
 * === What is an audit log? ===
 *
 * An audit log is a Write-Ahead Log (WAL) of user intent. Every time the
 * user performs an action (create task, delete task, update view, etc.), we
 * record the raw action BEFORE the reducer processes it. This gives us a
 * complete, replayable history of everything that ever happened.
 *
 * Think of it like a bank statement: not just your current balance, but every
 * deposit and withdrawal that led to it. The state at any point in time is
 * the sum of all events up to that point.
 *
 * === Why Write-Ahead? ===
 *
 * The audit middleware calls storage.put() BEFORE calling next() (the
 * reducer). This mirrors the Write-Ahead Log pattern used in databases like
 * PostgreSQL: commit the intent to disk first, THEN apply the change. If the
 * app crashes mid-dispatch, the log still has the event and it can be
 * replayed after recovery.
 *
 * === Vector Clocks ===
 *
 * A vector clock is a logical timestamp that tracks causality across
 * distributed devices. Instead of a single number, it's a map of
 * device → sequence number:
 *
 *   { "device-abc123": 42 }
 *
 * When this app runs on a single device, the clock looks like the above.
 * If the user someday syncs across devices, the clock grows:
 *
 *   { "device-abc123": 42, "device-xyz789": 17 }
 *
 * The key insight: Event A happened BEFORE Event B if A's clock is
 * component-wise ≤ B's clock. If neither dominates the other, the events
 * are concurrent (happened on different devices without knowledge of each
 * other). This is the foundation of CRDTs (Conflict-Free Replicated Data
 * Types) and eventual consistency.
 *
 * === Log Compaction ===
 *
 * An append-only log grows forever. We periodically compact it by:
 *   1. Taking a full state snapshot (current AppState + current clock)
 *   2. Deleting all events at or before the snapshot clock
 *
 * After compaction, to reconstruct state from scratch: load the snapshot,
 * then replay only the events AFTER the snapshot clock. This is exactly how
 * PostgreSQL's WAL + checkpoint system works.
 *
 * === localStorage for Clock/DeviceId ===
 *
 * The vector clock and device ID are stored in localStorage (not IndexedDB)
 * because:
 *   1. They must be read synchronously during dispatch (no async await)
 *   2. They need to survive page reloads
 *   3. They are tiny (< 200 bytes) — no need for IDB overhead
 *
 * IndexedDB operations are async, which would require making every dispatch
 * async — a breaking change to the store API. localStorage.getItem is
 * synchronous and always available.
 */

import type { Action } from "@coding-adventures/store";
import type { KVStorage } from "@coding-adventures/indexeddb";
import type { Middleware } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import {
  TASK_CREATE,
  TASK_UPDATE,
  TASK_DELETE,
  TASK_TOGGLE_STATUS,
  TASK_SET_STATUS,
  TASK_CLEAR_COMPLETED,
  VIEW_UPSERT,
  VIEW_SET_ACTIVE,
  CALENDAR_UPSERT,
  STATE_LOAD,
} from "./actions.js";

// ── Types ─────────────────────────────────────────────────────────────────────

/**
 * VectorClock — a logical timestamp for ordering events across devices.
 *
 * Maps deviceId → monotonic sequence number. On a single device this looks
 * like { "device-abc123": 42 }. When the app adds sync, additional device
 * entries appear automatically.
 *
 * Why not just use Date.now()?
 *
 * Wall-clock time has three problems:
 *   1. Clocks can drift between devices (±100ms is common).
 *   2. The system clock can be set backward (NTP corrections, user changes).
 *   3. Two events in the same millisecond have identical timestamps.
 *
 * Vector clocks are immune to all three: they only increment, they never
 * drift, and they capture causal ordering ("event A caused event B").
 *
 * We still store a timestamp field (Unix ms) as a tiebreaker for human-
 * readable display, but the clock is the authoritative ordering.
 */
export type VectorClock = Record<string, number>;

/**
 * AuditEvent — one entry in the immutable event log.
 *
 * Every user action produces exactly one AuditEvent. The event is the
 * ground truth; the AppState is a derived view (the "projection") of events.
 *
 * Fields:
 *   id         — UUID for deduplication (safe to retry a put)
 *   entityId   — the UUID of the affected task/view/calendar
 *   entityType — which collection the entity belongs to
 *   actionType — denormalized from action.type for fast filtering without
 *                parsing the full action payload
 *   clock      — vector clock at the moment of dispatch
 *   timestamp  — wall-clock Unix ms, for display only (not ordering)
 *   action     — the full original action payload, for replay
 */
export interface AuditEvent {
  id: string;
  entityId: string | null;
  entityType: "task" | "view" | "calendar" | "system" | null;
  actionType: string;
  clock: VectorClock;
  timestamp: number;
  action: Action;
}

/**
 * StateSnapshot — a point-in-time copy of the entire AppState.
 *
 * Snapshots enable log compaction: once a snapshot exists at clock C,
 * all events with clock ≤ C are redundant (the snapshot already encodes
 * their effect) and can be deleted.
 *
 * Analogy: A Git commit is a snapshot + the diff that produced it. The
 * diff is what lets you see what changed; the snapshot is what you check
 * out. Git's garbage collection is analogous to our compaction.
 */
export interface StateSnapshot {
  id: string;
  clock: VectorClock;
  timestamp: number;
  state: AppState;
}

// ── Constants ─────────────────────────────────────────────────────────────────

/** localStorage key for the stable per-browser device identity. */
const DEVICE_ID_KEY = "ca-device-id";

/** localStorage key for the persisted vector clock. */
const VECTOR_CLOCK_KEY = "ca-vector-clock";

/**
 * COMPACT_THRESHOLD — compact the event log when it exceeds this many events.
 *
 * 500 events is roughly 6–12 months of active daily use (2–4 task actions
 * per day). At that point the log would be ~500KB, which is still tiny.
 * We compact not for size but to keep query performance O(1) rather than
 * O(n) where n = entire app history.
 */
export const COMPACT_THRESHOLD = 500;

// ── Device Identity ──────────────────────────────────────────────────────────

/**
 * getDeviceId — returns the stable per-browser-installation identity.
 *
 * The device ID is a UUID that persists in localStorage across page reloads.
 * It identifies this particular browser profile (not the user, not the device
 * hardware — just the combination of browser + profile + localStorage).
 *
 * Why a stable ID?
 *   To anchor the vector clock. "Device abc123 is at sequence 42" only
 *   makes sense if "device abc123" is stable across sessions. If we generated
 *   a new ID on every page load, the clock would restart from 0 and we'd
 *   lose causal ordering across sessions.
 *
 * Privacy note: The ID never leaves the browser unless the user opts into
 * sync. It's a local coordination primitive, not a tracking identifier.
 */
export function getDeviceId(): string {
  const stored = localStorage.getItem(DEVICE_ID_KEY);
  if (stored) return stored;

  // First visit: generate a fresh UUID and persist it.
  const id = crypto.randomUUID();
  localStorage.setItem(DEVICE_ID_KEY, id);
  return id;
}

// ── Vector Clock ──────────────────────────────────────────────────────────────

/**
 * parseClock — safely deserializes a VectorClock from a localStorage JSON string.
 *
 * Plain `JSON.parse` on localStorage values is risky because:
 *   1. Any same-origin script (including browser extensions) can write to
 *      localStorage. A malicious value like `{"__proto__":{"x":1}}` can
 *      pollute Object.prototype in some engine versions.
 *   2. An adversarially large clock (thousands of device keys) makes
 *      clockSum() — which iterates all values — slow enough to freeze the
 *      synchronous middleware dispatch loop (ReDoS-style DoS).
 *
 * This function validates:
 *   - The parsed value is a plain object (not array, not null)
 *   - It has at most MAX_CLOCK_ENTRIES device entries
 *   - Every key is a string, every value is a non-negative integer
 *
 * If validation fails, it returns {} (fresh clock) rather than crashing.
 */
const MAX_CLOCK_ENTRIES = 100;

function parseClock(raw: string | null): VectorClock {
  if (!raw) return {};
  try {
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      Array.isArray(parsed) ||
      Object.keys(parsed as object).length > MAX_CLOCK_ENTRIES
    ) {
      return {};
    }
    for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof k !== "string" || typeof v !== "number" || !Number.isInteger(v) || v < 0) {
        return {};
      }
    }
    return parsed as VectorClock;
  } catch {
    return {};
  }
}

/**
 * nextClockTick — increment this device's sequence number and persist.
 *
 * Each dispatch on this device advances the clock by 1. The returned clock
 * is a snapshot of the clock AFTER the increment — it is the clock value
 * that should be stamped onto the audit event being created.
 *
 * Why increment BEFORE creating the event (not after)?
 *   The event records "the clock AT WHICH this action was observed". The
 *   first action is at seq=1, the second at seq=2, etc. Starting at seq=0
 *   would conflict with "no actions yet" (which is also seq=0).
 *
 * Example progression on a single device:
 *   Before any actions: clock = {}  (or { device-abc: 0 })
 *   After action 1:     clock = { "device-abc": 1 }
 *   After action 2:     clock = { "device-abc": 2 }
 *
 * The function reads the current clock from localStorage, increments the
 * device's entry, writes back, and returns the new clock — all synchronously.
 */
export function nextClockTick(deviceId: string): VectorClock {
  // Read and validate clock — parseClock() guards against prototype pollution
  // and oversized clocks written by a malicious same-origin script.
  const clock: VectorClock = parseClock(localStorage.getItem(VECTOR_CLOCK_KEY));

  // Increment this device's sequence number (start from 0 if missing).
  clock[deviceId] = (clock[deviceId] ?? 0) + 1;

  // Persist the updated clock.
  localStorage.setItem(VECTOR_CLOCK_KEY, JSON.stringify(clock));

  return clock;
}

// ── Entity Extraction ─────────────────────────────────────────────────────────

/**
 * extractEntityId — extracts the primary entity affected by an action.
 *
 * Different action types carry their entity ID in different payload fields.
 * This function normalizes them into a single { entityId, entityType } pair.
 *
 * We need this for the "activity feed for entity X" query:
 *   getActivitiesForEntity(storage, taskId) → recent events for that task
 *
 * The entityType field lets us filter across all task events even when the
 * specific entityId is unknown (e.g., "show me all recent task changes").
 *
 * Truth table:
 *   Action type            | entityId                   | entityType
 *   -----------------------|----------------------------|-----------
 *   TASK_CREATE            | action.id (pre-generated)  | "task"
 *   TASK_UPDATE            | action.taskId              | "task"
 *   TASK_DELETE            | action.taskId              | "task"
 *   TASK_TOGGLE_STATUS     | action.taskId              | "task"
 *   TASK_SET_STATUS        | action.taskId              | "task"
 *   TASK_CLEAR_COMPLETED   | null (batch operation)     | "task"
 *   VIEW_UPSERT            | action.view?.id            | "view"
 *   VIEW_SET_ACTIVE        | action.viewId              | "view"
 *   CALENDAR_UPSERT        | action.calendar?.id        | "calendar"
 *   STATE_LOAD             | null (system event)        | "system"
 *   (default)              | null                       | null
 */
export function extractEntityId(action: Action): {
  entityId: string | null;
  entityType: AuditEvent["entityType"];
} {
  switch (action.type) {
    case TASK_CREATE:
      return {
        entityId: (action as Record<string, unknown>).id as string | null ?? null,
        entityType: "task",
      };

    case TASK_UPDATE:
    case TASK_DELETE:
    case TASK_TOGGLE_STATUS:
    case TASK_SET_STATUS:
      return {
        entityId: (action as Record<string, unknown>).taskId as string | null ?? null,
        entityType: "task",
      };

    case TASK_CLEAR_COMPLETED:
      return { entityId: null, entityType: "task" };

    case VIEW_UPSERT: {
      const view = (action as Record<string, unknown>).view as { id: string } | undefined;
      return { entityId: view?.id ?? null, entityType: "view" };
    }

    case VIEW_SET_ACTIVE:
      return {
        entityId: (action as Record<string, unknown>).viewId as string | null ?? null,
        entityType: "view",
      };

    case CALENDAR_UPSERT: {
      const calendar = (action as Record<string, unknown>).calendar as { id: string } | undefined;
      return { entityId: calendar?.id ?? null, entityType: "calendar" };
    }

    case STATE_LOAD:
      return { entityId: null, entityType: "system" };

    default:
      return { entityId: null, entityType: null };
  }
}

// ── Audit Middleware ──────────────────────────────────────────────────────────

/**
 * createAuditMiddleware — factory that returns a WAL-style audit middleware.
 *
 * === Write-Ahead Log (WAL) Pattern ===
 *
 * The middleware logs the action BEFORE calling next() (the reducer). This
 * is the Write-Ahead Log pattern:
 *
 *   1. Write the intent to disk (audit log).
 *   2. THEN apply the change (reducer → new state).
 *
 * If the app crashes between steps 1 and 2, we can detect the un-applied
 * event on next startup and replay it. If we did it in reverse (apply first,
 * then log), a crash could leave us with state changes that have no log
 * entry — making the log incomplete.
 *
 * Contrast with the persistence middleware, which calls next() FIRST
 * (apply reducer), then reads the new state to persist it. Persistence
 * is post-reducer because it needs the computed new state (e.g., the
 * newly-assigned task id or updated timestamp). Audit is pre-reducer
 * because it records the raw intent.
 *
 * === Skipped action types ===
 *
 * STATE_LOAD: This is a hydration event, not user intent. It fires once on
 * startup to load data from IDB into memory. Logging it would pollute the
 * audit trail with "app started" noise and could cause infinite loops
 * (replay STATE_LOAD → triggers STATE_LOAD again).
 *
 * VIEW_SET_ACTIVE: Ephemeral navigation. Clicking between view tabs fires
 * this action constantly. It carries no semantic meaning for history
 * (knowing which tab was active is not useful audit data) and would
 * quickly dominate the event log.
 *
 * @param storage — the KVStorage instance for the "events" store
 */
export function createAuditMiddleware(storage: KVStorage): Middleware<AppState> {
  return (store, action, next) => {
    // Skip hydration events and ephemeral navigation — they're not user intent.
    if (action.type === STATE_LOAD || action.type === VIEW_SET_ACTIVE) {
      next();
      return;
    }

    // === Write-Ahead Log: log BEFORE applying the reducer ===
    //
    // Get the stable device identity and advance the logical clock.
    const deviceId = getDeviceId();
    const clock = nextClockTick(deviceId);

    // Extract which entity this action targets.
    const { entityId, entityType } = extractEntityId(action);

    // Build the audit event.
    const event: AuditEvent = {
      id: crypto.randomUUID(),
      entityId,
      entityType,
      actionType: action.type as string,
      clock,
      timestamp: Date.now(),
      action,
    };

    // Fire-and-forget write to the "events" store.
    // We do NOT await because dispatch is synchronous. The write happens
    // asynchronously in the background, just like persistence middleware.
    //
    // We attach a .catch() so that storage quota errors (e.g., IDB is full)
    // are surfaced to the console rather than silently swallowed. This
    // preserves the non-blocking dispatch contract while ensuring failures
    // are visible during development and don't violate the WAL guarantee
    // silently in production.
    storage.put("events", event).catch((err: unknown) => {
      console.warn("[audit] WAL write failed — event not persisted:", err);
    });

    // NOW apply the reducer. State changes AFTER the log entry exists.
    next();
  };
}

// ── Query Functions ──────────────────────────────────────────────────────────

/**
 * clockSum — computes the total logical time represented by a clock.
 *
 * For a single-device scenario, this is just the device's sequence number.
 * For multi-device, it's the sum of all devices' sequences — a rough
 * approximation of "how many total events have happened across all devices".
 *
 * This is used as an ordering key when sorting events by logical time.
 * It's not perfect for concurrent events across devices (there's no global
 * total order with distributed clocks), but it works well for the common
 * case of a single device.
 */
function clockSum(clock: VectorClock): number {
  return Object.values(clock).reduce((sum, seq) => sum + seq, 0);
}

/**
 * getActivitiesForEntity — retrieves the audit history for a specific entity.
 *
 * Use this to show a task's history: "Task X was created, then updated,
 * then marked done." The events are returned oldest-first (ascending clock
 * order) so they read like a timeline.
 *
 * @param storage   — the KVStorage instance
 * @param entityId  — the UUID of the task/view/calendar to query
 * @param options   — optional filters
 *   limit:  return only the N most recent events (after sorting)
 *   since:  return only events with timestamp > since (Unix ms)
 */
export async function getActivitiesForEntity(
  storage: KVStorage,
  entityId: string,
  options?: { limit?: number; since?: number },
): Promise<AuditEvent[]> {
  const all = await storage.getAll<AuditEvent>("events");

  // Filter to events that affect this specific entity.
  let events = all.filter((e) => e.entityId === entityId);

  // Apply optional time-based filter.
  if (options?.since !== undefined) {
    events = events.filter((e) => e.timestamp > options.since!);
  }

  // Sort ascending by logical clock (oldest first = most chronological).
  events.sort((a, b) => clockSum(a.clock) - clockSum(b.clock));

  // Apply limit: return the N most recent (tail of the sorted array).
  if (options?.limit !== undefined) {
    events = events.slice(-options.limit);
  }

  return events;
}

/**
 * getRecentActivities — retrieves a global activity feed.
 *
 * Use this for a "Recent activity" panel that shows all changes across
 * the entire app, optionally filtered by entity type. Events are sorted
 * newest-first (descending timestamp) for a news-feed feel.
 *
 * @param storage  — the KVStorage instance
 * @param options  — optional filters
 *   limit:       return only the N most recent events
 *   since:       return only events with timestamp > since (Unix ms)
 *   entityType:  filter to events affecting a particular entity type
 */
export async function getRecentActivities(
  storage: KVStorage,
  options?: { limit?: number; since?: number; entityType?: AuditEvent["entityType"] },
): Promise<AuditEvent[]> {
  const all = await storage.getAll<AuditEvent>("events");

  let events = all;

  // Filter by entity type if requested.
  if (options?.entityType !== undefined) {
    events = events.filter((e) => e.entityType === options.entityType);
  }

  // Filter by time if requested.
  if (options?.since !== undefined) {
    events = events.filter((e) => e.timestamp > options.since!);
  }

  // Sort descending by timestamp (most recent first).
  events.sort((a, b) => b.timestamp - a.timestamp);

  // Apply limit.
  if (options?.limit !== undefined) {
    events = events.slice(0, options.limit);
  }

  return events;
}

// ── Log Compaction ────────────────────────────────────────────────────────────

/**
 * compactEventLog — take a snapshot and trim obsolete events.
 *
 * Compaction is a two-step process:
 *
 *   1. Write a StateSnapshot to the "snapshots" store. The snapshot records:
 *      - The full current AppState (all tasks, views, calendars)
 *      - The current vector clock (so we know which events this snapshot covers)
 *
 *   2. Delete all events from the "events" store whose total clock sum is
 *      at or below the current clock's total sum. These events are redundant
 *      because the snapshot already incorporates their effect.
 *
 * === Why keep events with clock > snapshot? ===
 *
 * There may be a brief window where new events arrive between writing the
 * snapshot and deleting old events. We use the clock (not time) to determine
 * "at or before snapshot" — this is correct because the clock is monotonically
 * incremented for each event on this device.
 *
 * === Fire-and-forget deletes ===
 *
 * Event deletes are not awaited. If the page unloads mid-compaction, some
 * events may survive that could have been pruned. That's fine — they'll be
 * pruned on the next compaction cycle. We never delete the snapshot until
 * a newer snapshot exists, so correctness is maintained.
 *
 * @param storage — the KVStorage instance
 * @param state   — the current AppState to snapshot
 */
export async function compactEventLog(storage: KVStorage, state: AppState): Promise<void> {
  // Read and validate the current clock. parseClock() guards against malformed
  // values that could be written by a same-origin script.
  const currentClock: VectorClock = parseClock(localStorage.getItem(VECTOR_CLOCK_KEY));
  const currentClockSum = clockSum(currentClock);

  // Write the state snapshot to IDB.
  const snapshot: StateSnapshot = {
    id: crypto.randomUUID(),
    clock: { ...currentClock },
    timestamp: Date.now(),
    state,
  };
  await storage.put("snapshots", snapshot);

  // Retrieve all events and delete those at or before the snapshot clock.
  const events = await storage.getAll<AuditEvent>("events");
  for (const event of events) {
    if (clockSum(event.clock) <= currentClockSum) {
      // Fire-and-forget: don't await each delete.
      storage.delete("events", event.id);
    }
  }
}
