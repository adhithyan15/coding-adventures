/**
 * audit.test.ts — Comprehensive unit tests for the audit log module.
 *
 * The audit module is the Write-Ahead Log for the todo app. These tests
 * verify every exported function in isolation, using:
 *   - A Map-based mock storage (no actual IndexedDB needed)
 *   - A localStorage mock (no browser environment needed)
 *   - Fake timers for deterministic timestamps
 *   - vi.stubGlobal for deterministic UUIDs
 *
 * Test philosophy: each test exercises one behaviour. If a test fails,
 * you should immediately know WHAT is broken without reading other tests.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  getDeviceId,
  nextClockTick,
  extractEntityId,
  createAuditMiddleware,
  getActivitiesForEntity,
  getRecentActivities,
  compactEventLog,
  COMPACT_THRESHOLD,
} from "../audit.js";
import type { AuditEvent } from "../audit.js";
import type { AppState } from "../reducer.js";
import type { Task } from "../types.js";
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
} from "../actions.js";

// ── Mock localStorage ────────────────────────────────────────────────────────

/**
 * createMockLocalStorage — a Map-based localStorage mock.
 *
 * The real localStorage is a browser API that isn't available in Node/Vitest.
 * We replace it with a simple Map that implements the same interface.
 *
 * Why not use jsdom's localStorage?
 *   jsdom's localStorage is shared across tests, making tests order-dependent.
 *   A fresh Map per test guarantees complete isolation.
 */
function createMockLocalStorage() {
  const store = new Map<string, string>();
  return {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => store.set(key, value),
    removeItem: (key: string) => store.delete(key),
    clear: () => store.clear(),
    get length() { return store.size; },
    key: (index: number) => [...store.keys()][index] ?? null,
    _store: store,  // expose for assertions
  };
}

// ── Mock Storage ─────────────────────────────────────────────────────────────

/**
 * createMockStorage — an in-memory KVStorage implementation.
 *
 * Uses Maps to simulate IndexedDB stores. Mirrors the pattern used in
 * persistence.test.ts, with the addition of per-store tracking via
 * the `_stores` property for assertions.
 *
 * Two stores are pre-created: "events" and "snapshots".
 */
function createMockStorage() {
  const stores: Record<string, Map<string, AuditEvent | Record<string, unknown>>> = {
    events: new Map(),
    snapshots: new Map(),
  };
  return {
    put: vi.fn((storeName: string, value: Record<string, unknown>) => {
      stores[storeName]?.set(value.id as string, value);
      return Promise.resolve();
    }),
    delete: vi.fn((storeName: string, key: string) => {
      stores[storeName]?.delete(key);
      return Promise.resolve();
    }),
    get: vi.fn(() => Promise.resolve(undefined)),
    getAll: vi.fn((storeName: string) => {
      return Promise.resolve([...(stores[storeName]?.values() ?? [])]);
    }),
    open: vi.fn(() => Promise.resolve()),
    close: vi.fn(),
    _stores: stores,  // expose for assertions
  };
}

// ── Mock Store ────────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "test-task-id",
    title: "Test Task",
    description: "",
    status: "todo",
    priority: "medium",
    category: "",
    dueDate: null,
    dueTime: null,
    createdAt: 0,
    updatedAt: 0,
    completedAt: null,
    sortOrder: 0,
    ...overrides,
  };
}

function makeAppState(overrides: Partial<AppState> = {}): AppState {
  return { tasks: [], views: [], calendars: [], activeViewId: "", ...overrides };
}

function createMockStore(state: AppState) {
  return {
    getState: () => state,
    dispatch: vi.fn(),
    subscribe: vi.fn(),
    use: vi.fn(),
  };
}

// ── Test Setup ────────────────────────────────────────────────────────────────

let mockLocalStorage: ReturnType<typeof createMockLocalStorage>;

beforeEach(() => {
  // Install a fresh localStorage mock before each test.
  // This ensures no state bleeds between tests.
  mockLocalStorage = createMockLocalStorage();
  vi.stubGlobal("localStorage", mockLocalStorage);

  // Deterministic UUIDs: all crypto.randomUUID() calls return "test-uuid"
  // by default. Individual tests override this as needed.
  vi.stubGlobal("crypto", { randomUUID: () => "test-uuid" });

  // Freeze time at a known epoch for deterministic timestamps.
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-01-15T12:00:00.000Z"));
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

// ── Tests: COMPACT_THRESHOLD ─────────────────────────────────────────────────

describe("COMPACT_THRESHOLD", () => {
  it("is exported as a number", () => {
    expect(typeof COMPACT_THRESHOLD).toBe("number");
  });

  it("is 500", () => {
    // The threshold should be 500 so compaction happens roughly every 6-12
    // months of typical use. If this changes, update the comment in audit.ts.
    expect(COMPACT_THRESHOLD).toBe(500);
  });
});

// ── Tests: getDeviceId ────────────────────────────────────────────────────────

describe("getDeviceId", () => {
  it("generates a UUID on first call (key missing)", () => {
    // localStorage is empty — no device id stored yet.
    const id = getDeviceId();
    expect(id).toBe("test-uuid");
  });

  it("persists the generated UUID to localStorage", () => {
    getDeviceId();
    expect(mockLocalStorage.getItem("ca-device-id")).toBe("test-uuid");
  });

  it("returns the same ID on subsequent calls (stable identity)", () => {
    const id1 = getDeviceId();
    // Override UUID generator — should NOT be called again.
    vi.stubGlobal("crypto", { randomUUID: () => "different-uuid" });
    const id2 = getDeviceId();
    expect(id1).toBe(id2);
    expect(id1).toBe("test-uuid");  // the first-generated value
  });

  it("reads from localStorage key 'ca-device-id'", () => {
    mockLocalStorage.setItem("ca-device-id", "pre-existing-id");
    const id = getDeviceId();
    expect(id).toBe("pre-existing-id");
  });

  it("does NOT call crypto.randomUUID when id already exists", () => {
    mockLocalStorage.setItem("ca-device-id", "existing-id");
    const mockUUID = vi.fn(() => "should-not-be-called");
    vi.stubGlobal("crypto", { randomUUID: mockUUID });

    getDeviceId();

    expect(mockUUID).not.toHaveBeenCalled();
  });
});

// ── Tests: nextClockTick ──────────────────────────────────────────────────────

describe("nextClockTick", () => {
  it("starts at 1 on first call (no existing clock)", () => {
    const clock = nextClockTick("device-a");
    expect(clock).toEqual({ "device-a": 1 });
  });

  it("increments the device sequence number", () => {
    nextClockTick("device-a");  // seq = 1
    nextClockTick("device-a");  // seq = 2
    const clock = nextClockTick("device-a");  // seq = 3
    expect(clock["device-a"]).toBe(3);
  });

  it("persists the clock to localStorage", () => {
    nextClockTick("device-a");
    const stored = JSON.parse(mockLocalStorage.getItem("ca-vector-clock") ?? "{}") as Record<string, number>;
    expect(stored["device-a"]).toBe(1);
  });

  it("reads from localStorage key 'ca-vector-clock'", () => {
    // Pre-seed the clock
    mockLocalStorage.setItem("ca-vector-clock", JSON.stringify({ "device-a": 10 }));
    const clock = nextClockTick("device-a");
    expect(clock["device-a"]).toBe(11);
  });

  it("only increments the specified device, leaves others unchanged", () => {
    // Pre-seed with two devices
    mockLocalStorage.setItem(
      "ca-vector-clock",
      JSON.stringify({ "device-a": 5, "device-b": 3 }),
    );
    const clock = nextClockTick("device-a");
    expect(clock["device-a"]).toBe(6);
    expect(clock["device-b"]).toBe(3);  // unchanged
  });

  it("returns a new object with the updated clock", () => {
    const c1 = nextClockTick("device-a");
    const c2 = nextClockTick("device-a");
    expect(c1["device-a"]).toBe(1);
    expect(c2["device-a"]).toBe(2);
    // They are different objects (not the same reference)
    expect(c1).not.toBe(c2);
  });
});

// ── Tests: extractEntityId ────────────────────────────────────────────────────

describe("extractEntityId", () => {
  it("extracts id from TASK_CREATE", () => {
    const result = extractEntityId({ type: TASK_CREATE, id: "task-uuid-1" });
    expect(result).toEqual({ entityId: "task-uuid-1", entityType: "task" });
  });

  it("extracts taskId from TASK_UPDATE", () => {
    const result = extractEntityId({ type: TASK_UPDATE, taskId: "task-uuid-2" });
    expect(result).toEqual({ entityId: "task-uuid-2", entityType: "task" });
  });

  it("extracts taskId from TASK_DELETE", () => {
    const result = extractEntityId({ type: TASK_DELETE, taskId: "task-uuid-3" });
    expect(result).toEqual({ entityId: "task-uuid-3", entityType: "task" });
  });

  it("extracts taskId from TASK_TOGGLE_STATUS", () => {
    const result = extractEntityId({ type: TASK_TOGGLE_STATUS, taskId: "task-uuid-4" });
    expect(result).toEqual({ entityId: "task-uuid-4", entityType: "task" });
  });

  it("extracts taskId from TASK_SET_STATUS", () => {
    const result = extractEntityId({ type: TASK_SET_STATUS, taskId: "task-uuid-5", status: "done" });
    expect(result).toEqual({ entityId: "task-uuid-5", entityType: "task" });
  });

  it("returns null entityId for TASK_CLEAR_COMPLETED (batch op)", () => {
    const result = extractEntityId({ type: TASK_CLEAR_COMPLETED });
    expect(result).toEqual({ entityId: null, entityType: "task" });
  });

  it("extracts view.id from VIEW_UPSERT", () => {
    const result = extractEntityId({ type: VIEW_UPSERT, view: { id: "view-uuid-1" } });
    expect(result).toEqual({ entityId: "view-uuid-1", entityType: "view" });
  });

  it("returns null entityId for VIEW_UPSERT with no view", () => {
    const result = extractEntityId({ type: VIEW_UPSERT });
    expect(result).toEqual({ entityId: null, entityType: "view" });
  });

  it("extracts viewId from VIEW_SET_ACTIVE", () => {
    const result = extractEntityId({ type: VIEW_SET_ACTIVE, viewId: "view-uuid-2" });
    expect(result).toEqual({ entityId: "view-uuid-2", entityType: "view" });
  });

  it("extracts calendar.id from CALENDAR_UPSERT", () => {
    const result = extractEntityId({ type: CALENDAR_UPSERT, calendar: { id: "cal-uuid-1" } });
    expect(result).toEqual({ entityId: "cal-uuid-1", entityType: "calendar" });
  });

  it("returns system entityType for STATE_LOAD", () => {
    const result = extractEntityId({ type: STATE_LOAD, tasks: [], views: [], calendars: [], activeViewId: "" });
    expect(result).toEqual({ entityId: null, entityType: "system" });
  });

  it("returns null/null for unknown action types", () => {
    const result = extractEntityId({ type: "SOME_UNKNOWN_ACTION" });
    expect(result).toEqual({ entityId: null, entityType: null });
  });
});

// ── Tests: createAuditMiddleware ──────────────────────────────────────────────

describe("createAuditMiddleware", () => {
  it("calls next() exactly once for a normal action", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());
    const next = vi.fn();

    middleware(store, { type: TASK_CREATE, id: "task-1" }, next);

    expect(next).toHaveBeenCalledOnce();
  });

  it("writes an AuditEvent to the 'events' store before calling next()", () => {
    const storage = createMockStorage();

    // Track call order: we want to verify storage.put is called BEFORE next()
    const callOrder: string[] = [];
    storage.put.mockImplementation((storeName: string, value: Record<string, unknown>) => {
      callOrder.push("put");
      storage._stores[storeName]?.set(value.id as string, value);
      return Promise.resolve();
    });

    const next = vi.fn(() => { callOrder.push("next"); });
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: TASK_CREATE, id: "task-1" }, next);

    // Write-Ahead Log: put must happen BEFORE next
    expect(callOrder).toEqual(["put", "next"]);
  });

  it("stores event with correct actionType", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: TASK_CREATE, id: "task-1" }, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    expect(events).toHaveLength(1);
    expect(events[0]!.actionType).toBe(TASK_CREATE);
  });

  it("stores event with correct entityId and entityType", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: TASK_CREATE, id: "task-uuid-99" }, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    expect(events[0]!.entityId).toBe("task-uuid-99");
    expect(events[0]!.entityType).toBe("task");
  });

  it("stores event with the full action payload", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());
    const action = { type: TASK_UPDATE, taskId: "t-1", patch: { title: "New title" } };

    middleware(store, action, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    expect(events[0]!.action).toEqual(action);
  });

  it("stores event with a valid clock", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: TASK_CREATE, id: "task-1" }, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    const clock = events[0]!.clock;
    // Clock should have exactly one device entry with seq = 1
    const values = Object.values(clock);
    expect(values).toHaveLength(1);
    expect(values[0]).toBe(1);
  });

  it("stores event with the current timestamp", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    // System time is frozen at 2026-01-15T12:00:00.000Z
    const expectedTimestamp = new Date("2026-01-15T12:00:00.000Z").getTime();

    middleware(store, { type: TASK_CREATE, id: "task-1" }, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    expect(events[0]!.timestamp).toBe(expectedTimestamp);
  });

  it("assigns a unique UUID to each event", () => {
    let uuidCounter = 0;
    vi.stubGlobal("crypto", { randomUUID: () => `uuid-${++uuidCounter}` });

    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: TASK_CREATE, id: "uuid-1" }, vi.fn());
    middleware(store, { type: TASK_DELETE, taskId: "task-2" }, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    expect(events).toHaveLength(2);
    const ids = events.map((e) => e.id);
    // All IDs should be unique
    expect(new Set(ids).size).toBe(2);
  });

  it("SKIPS STATE_LOAD — does NOT write to storage", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: STATE_LOAD, tasks: [], views: [], calendars: [], activeViewId: "" }, vi.fn());

    expect(storage.put).not.toHaveBeenCalled();
  });

  it("SKIPS STATE_LOAD — still calls next()", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());
    const next = vi.fn();

    middleware(store, { type: STATE_LOAD, tasks: [], views: [], calendars: [], activeViewId: "" }, next);

    expect(next).toHaveBeenCalledOnce();
  });

  it("SKIPS VIEW_SET_ACTIVE — does NOT write to storage", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: VIEW_SET_ACTIVE, viewId: "view-1" }, vi.fn());

    expect(storage.put).not.toHaveBeenCalled();
  });

  it("SKIPS VIEW_SET_ACTIVE — still calls next()", () => {
    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());
    const next = vi.fn();

    middleware(store, { type: VIEW_SET_ACTIVE, viewId: "view-1" }, next);

    expect(next).toHaveBeenCalledOnce();
  });

  it("increments the clock for each logged action", () => {
    let counter = 0;
    vi.stubGlobal("crypto", { randomUUID: () => `uuid-${++counter}` });

    const storage = createMockStorage();
    const middleware = createAuditMiddleware(storage);
    const store = createMockStore(makeAppState());

    middleware(store, { type: TASK_CREATE, id: "uuid-1" }, vi.fn());
    middleware(store, { type: TASK_DELETE, taskId: "task-2" }, vi.fn());

    const events = [...storage._stores.events.values()] as AuditEvent[];
    // Sort by clock sum to ensure ordering
    events.sort((a, b) => Object.values(a.clock)[0]! - Object.values(b.clock)[0]!);

    const seqs = events.map((e) => Object.values(e.clock)[0]);
    expect(seqs[0]).toBe(1);
    expect(seqs[1]).toBe(2);
  });
});

// ── Tests: getActivitiesForEntity ─────────────────────────────────────────────

describe("getActivitiesForEntity", () => {
  /**
   * seedEvents — helper to pre-populate storage with test events.
   * Each event gets a distinct clock value (1, 2, 3...) so sort order
   * is deterministic.
   */
  function seedEvents(
    storage: ReturnType<typeof createMockStorage>,
    overrides: Array<Partial<AuditEvent>>,
  ): AuditEvent[] {
    const events: AuditEvent[] = overrides.map((o, i) => ({
      id: `event-${i + 1}`,
      entityId: "entity-1",
      entityType: "task" as const,
      actionType: TASK_CREATE,
      clock: { "device-a": i + 1 },
      timestamp: 1000 * (i + 1),
      action: { type: TASK_CREATE },
      ...o,
    }));
    storage.getAll.mockResolvedValue(events);
    return events;
  }

  it("returns only events matching the given entityId", async () => {
    const storage = createMockStorage();
    seedEvents(storage, [
      { entityId: "target-entity" },
      { entityId: "other-entity", id: "event-2" },
      { entityId: "target-entity", id: "event-3", clock: { "device-a": 2 } },
    ]);

    const result = await getActivitiesForEntity(storage, "target-entity");
    expect(result).toHaveLength(2);
    expect(result.every((e) => e.entityId === "target-entity")).toBe(true);
  });

  it("returns empty array when no events match the entityId", async () => {
    const storage = createMockStorage();
    seedEvents(storage, [{ entityId: "someone-else" }]);

    const result = await getActivitiesForEntity(storage, "non-existent-entity");
    expect(result).toHaveLength(0);
  });

  it("returns empty array when storage is empty", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    const result = await getActivitiesForEntity(storage, "any-entity");
    expect(result).toHaveLength(0);
  });

  it("sorts events by clock ascending (oldest first)", async () => {
    const storage = createMockStorage();
    // Seed in reverse order to verify sorting
    seedEvents(storage, [
      { id: "event-c", clock: { "device-a": 3 }, timestamp: 3000 },
      { id: "event-a", clock: { "device-a": 1 }, timestamp: 1000 },
      { id: "event-b", clock: { "device-a": 2 }, timestamp: 2000 },
    ]);

    const result = await getActivitiesForEntity(storage, "entity-1");
    expect(result.map((e) => e.id)).toEqual(["event-a", "event-b", "event-c"]);
  });

  it("respects the 'since' option (filters by timestamp > since)", async () => {
    const storage = createMockStorage();
    seedEvents(storage, [
      { id: "event-1", timestamp: 1000, clock: { "device-a": 1 } },
      { id: "event-2", timestamp: 2000, clock: { "device-a": 2 } },
      { id: "event-3", timestamp: 3000, clock: { "device-a": 3 } },
    ]);

    const result = await getActivitiesForEntity(storage, "entity-1", { since: 1500 });
    // Only events with timestamp > 1500 (so 2000 and 3000)
    expect(result).toHaveLength(2);
    expect(result.map((e) => e.id)).toEqual(["event-2", "event-3"]);
  });

  it("'since' is exclusive (does NOT include events at exactly 'since')", async () => {
    const storage = createMockStorage();
    seedEvents(storage, [
      { id: "event-exact", timestamp: 1500, clock: { "device-a": 1 } },
    ]);

    const result = await getActivitiesForEntity(storage, "entity-1", { since: 1500 });
    // timestamp > since, not >=
    expect(result).toHaveLength(0);
  });

  it("respects the 'limit' option — returns last N events", async () => {
    const storage = createMockStorage();
    seedEvents(storage, [
      { id: "event-1", clock: { "device-a": 1 }, timestamp: 1000 },
      { id: "event-2", clock: { "device-a": 2 }, timestamp: 2000 },
      { id: "event-3", clock: { "device-a": 3 }, timestamp: 3000 },
      { id: "event-4", clock: { "device-a": 4 }, timestamp: 4000 },
    ]);

    const result = await getActivitiesForEntity(storage, "entity-1", { limit: 2 });
    // Should return the 2 most recent (last N after sorting oldest-first)
    expect(result).toHaveLength(2);
    expect(result.map((e) => e.id)).toEqual(["event-3", "event-4"]);
  });

  it("limit and since can be combined", async () => {
    const storage = createMockStorage();
    seedEvents(storage, [
      { id: "event-1", clock: { "device-a": 1 }, timestamp: 1000 },
      { id: "event-2", clock: { "device-a": 2 }, timestamp: 2000 },
      { id: "event-3", clock: { "device-a": 3 }, timestamp: 3000 },
      { id: "event-4", clock: { "device-a": 4 }, timestamp: 4000 },
    ]);

    // since=1500 → events 2,3,4 — then limit=2 → events 3,4
    const result = await getActivitiesForEntity(storage, "entity-1", {
      since: 1500,
      limit: 2,
    });
    expect(result).toHaveLength(2);
    expect(result.map((e) => e.id)).toEqual(["event-3", "event-4"]);
  });
});

// ── Tests: getRecentActivities ────────────────────────────────────────────────

describe("getRecentActivities", () => {
  function makeEvent(overrides: Partial<AuditEvent>): AuditEvent {
    return {
      id: "event-default",
      entityId: null,
      entityType: "task",
      actionType: TASK_CREATE,
      clock: { "device-a": 1 },
      timestamp: 1000,
      action: { type: TASK_CREATE },
      ...overrides,
    };
  }

  it("returns all events sorted by timestamp descending (most recent first)", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([
      makeEvent({ id: "e1", timestamp: 1000 }),
      makeEvent({ id: "e3", timestamp: 3000 }),
      makeEvent({ id: "e2", timestamp: 2000 }),
    ]);

    const result = await getRecentActivities(storage);
    expect(result.map((e) => e.id)).toEqual(["e3", "e2", "e1"]);
  });

  it("returns empty array when storage has no events", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    const result = await getRecentActivities(storage);
    expect(result).toHaveLength(0);
  });

  it("filters by entityType when provided", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([
      makeEvent({ id: "task-event", entityType: "task" }),
      makeEvent({ id: "view-event", entityType: "view", timestamp: 2000 }),
      makeEvent({ id: "cal-event",  entityType: "calendar", timestamp: 3000 }),
    ]);

    const result = await getRecentActivities(storage, { entityType: "task" });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("task-event");
  });

  it("returns all entity types when entityType not provided", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([
      makeEvent({ id: "task-event", entityType: "task" }),
      makeEvent({ id: "view-event", entityType: "view", timestamp: 2000 }),
    ]);

    const result = await getRecentActivities(storage);
    expect(result).toHaveLength(2);
  });

  it("filters by 'since' timestamp", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([
      makeEvent({ id: "old", timestamp: 500 }),
      makeEvent({ id: "recent", timestamp: 2000 }),
    ]);

    const result = await getRecentActivities(storage, { since: 1000 });
    expect(result).toHaveLength(1);
    expect(result[0]!.id).toBe("recent");
  });

  it("applies 'limit' after sorting", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([
      makeEvent({ id: "e1", timestamp: 1000 }),
      makeEvent({ id: "e2", timestamp: 2000 }),
      makeEvent({ id: "e3", timestamp: 3000 }),
    ]);

    const result = await getRecentActivities(storage, { limit: 2 });
    // Most recent first, limited to 2
    expect(result).toHaveLength(2);
    expect(result.map((e) => e.id)).toEqual(["e3", "e2"]);
  });

  it("entityType, since, and limit can all be combined", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([
      makeEvent({ id: "task-old",    entityType: "task",     timestamp: 500  }),
      makeEvent({ id: "task-1",      entityType: "task",     timestamp: 1500 }),
      makeEvent({ id: "task-2",      entityType: "task",     timestamp: 2500 }),
      makeEvent({ id: "task-3",      entityType: "task",     timestamp: 3500 }),
      makeEvent({ id: "view-recent", entityType: "view",     timestamp: 4000 }),
    ]);

    // Filter to tasks only, since=1000, limit=2 → tasks 1500,2500,3500 → limit to top 2
    const result = await getRecentActivities(storage, {
      entityType: "task",
      since: 1000,
      limit: 2,
    });
    expect(result).toHaveLength(2);
    expect(result.map((e) => e.id)).toEqual(["task-3", "task-2"]);
  });
});

// ── Tests: compactEventLog ────────────────────────────────────────────────────

describe("compactEventLog", () => {
  function makeEvent(id: string, clockSeq: number): AuditEvent {
    return {
      id,
      entityId: "task-1",
      entityType: "task",
      actionType: TASK_CREATE,
      clock: { "device-a": clockSeq },
      timestamp: clockSeq * 1000,
      action: { type: TASK_CREATE },
    };
  }

  it("writes a snapshot to the 'snapshots' store", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    const state = makeAppState({ tasks: [makeTask()] });
    await compactEventLog(storage, state);

    expect(storage.put).toHaveBeenCalledWith(
      "snapshots",
      expect.objectContaining({ state }),
    );
  });

  it("snapshot includes the current state", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    const task = makeTask({ id: "t-1", title: "Keep this" });
    const state = makeAppState({ tasks: [task] });
    await compactEventLog(storage, state);

    const snapshots = [...storage._stores.snapshots.values()];
    expect(snapshots).toHaveLength(1);
    const snapshot = snapshots[0] as { state: AppState };
    expect(snapshot.state.tasks[0]!.title).toBe("Keep this");
  });

  it("snapshot has a valid UUID as id", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    await compactEventLog(storage, makeAppState());

    const snapshots = [...storage._stores.snapshots.values()];
    expect(snapshots[0]).toHaveProperty("id", "test-uuid");
  });

  it("snapshot includes a clock from localStorage", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    // Pre-seed the clock
    mockLocalStorage.setItem("ca-vector-clock", JSON.stringify({ "device-a": 7 }));
    await compactEventLog(storage, makeAppState());

    const snapshots = [...storage._stores.snapshots.values()];
    const snapshot = snapshots[0] as { clock: Record<string, number> };
    expect(snapshot.clock).toEqual({ "device-a": 7 });
  });

  it("snapshot includes a timestamp", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    await compactEventLog(storage, makeAppState());

    const snapshots = [...storage._stores.snapshots.values()];
    const snapshot = snapshots[0] as { timestamp: number };
    const expectedTimestamp = new Date("2026-01-15T12:00:00.000Z").getTime();
    expect(snapshot.timestamp).toBe(expectedTimestamp);
  });

  it("deletes events at or below the current clock sum", async () => {
    const storage = createMockStorage();

    // Current clock: device-a = 3 (clockSum = 3)
    mockLocalStorage.setItem("ca-vector-clock", JSON.stringify({ "device-a": 3 }));

    // Events: seq 1, 2, 3 should be deleted; seq 4 should survive
    const events = [
      makeEvent("e1", 1),
      makeEvent("e2", 2),
      makeEvent("e3", 3),
      makeEvent("e4", 4),
    ];
    storage.getAll.mockResolvedValue(events);

    await compactEventLog(storage, makeAppState());

    // Allow async deletes to settle
    await vi.waitFor(() => {
      expect(storage.delete).toHaveBeenCalledWith("events", "e1");
      expect(storage.delete).toHaveBeenCalledWith("events", "e2");
      expect(storage.delete).toHaveBeenCalledWith("events", "e3");
    });

    // Event 4 (clockSum = 4 > 3) should NOT be deleted
    expect(storage.delete).not.toHaveBeenCalledWith("events", "e4");
  });

  it("does not delete any events when clock is empty (first compaction)", async () => {
    const storage = createMockStorage();
    // No clock in localStorage — clockSum = 0

    const events = [makeEvent("e1", 1), makeEvent("e2", 2)];
    storage.getAll.mockResolvedValue(events);

    await compactEventLog(storage, makeAppState());

    // Wait a tick for any async operations
    await Promise.resolve();

    // clockSum of current clock = 0, all events have clockSum ≥ 1
    // so none should be deleted
    expect(storage.delete).not.toHaveBeenCalledWith("events", "e1");
    expect(storage.delete).not.toHaveBeenCalledWith("events", "e2");
  });

  it("does nothing to events when event store is empty", async () => {
    const storage = createMockStorage();
    storage.getAll.mockResolvedValue([]);

    mockLocalStorage.setItem("ca-vector-clock", JSON.stringify({ "device-a": 10 }));
    await compactEventLog(storage, makeAppState());

    expect(storage.delete).not.toHaveBeenCalled();
  });
});
