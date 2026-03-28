/**
 * main.tsx — Application entry point.
 *
 * Responsibilities:
 *   1. Open IndexedDB storage (with MemoryStorage fallback).
 *   2. Load persisted tasks, views, and calendars.
 *   3. On first visit: seed tasks, views, and calendars.
 *   4. Attach persistence middleware to the store.
 *   5. Mount the React app into <div id="root">.
 *
 * The init() function is async because IndexedDB operations return Promises.
 * We await the database open and data load before rendering so that the
 * first render has the full state — no loading spinners needed.
 *
 * === Storage fallback strategy ===
 *
 * IndexedDB may be unavailable in some environments (private browsing in
 * older Safari, SSR, Node test runners). When it fails to open, we fall
 * back to MemoryStorage — an in-memory implementation of the same KVStorage
 * interface. The app works identically; it just loses persistence on reload.
 *
 * === IDB schema versions ===
 *
 * Version 1: { "todos": { keyPath: "id" } }
 *   — tasks stored under "todos" (legacy name from initial launch)
 *
 * Version 2:
 *   + "views"     : { keyPath: "id" }  — SavedView records
 *   + "calendars" : { keyPath: "id" }  — CalendarSettings records
 *
 * Version 3:
 *   + "events"    : { keyPath: "id" }  — append-only audit event log
 *   + "snapshots" : { keyPath: "id" }  — periodic state snapshots for compaction
 *
 * Version 4:
 *   + "projects"  : { keyPath: "id" }  — Project entities (default + user)
 *   + "edges"     : { keyPath: "id",   — Directed graph edges (contains, …)
 *                     indexes: ["fromId", "toId"] }
 *
 * Version 5 (this release):
 *   ~ "todos" renamed to "tasks" — store name now matches the domain
 *     language used throughout the codebase (Task, not Todo).
 *     Migration: IndexedDBStorage copies all "todos" records into "tasks"
 *     and deletes "todos" via the StoreSchema.renamedFrom mechanism.
 *
 * The onupgradeneeded handler inside IndexedDBStorage uses
 * `if (!db.objectStoreNames.contains(...))` to guard against re-creation.
 *
 * === Backward compatibility: dueTime normalization ===
 *
 * Tasks written in v1 (before the dueTime field was added) have no dueTime
 * property. We normalize them on load:
 *   tasks.map(t => ({ dueTime: null, ...t }))
 * This ensures all Task objects in the store conform to the current interface.
 * The spread `{ dueTime: null, ...t }` means existing dueTime values
 * (once they exist) are NOT overwritten — the default only applies when
 * the field is truly absent.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
import type { KVStorage } from "@coding-adventures/indexeddb";
import { store } from "./state.js";
import { stateLoadAction, setActiveViewAction, edgeAddAction } from "./actions.js";
import { createPersistenceMiddleware } from "./persistence.js";
import { createAuditMiddleware, compactEventLog, COMPACT_THRESHOLD } from "./audit.js";
import { initStorage } from "./storage.js";
import { seedTasks, seedViews, seedCalendars, seedDefaultProject, VIEW_ID_ALL_TASKS, PROJECT_ID_DEFAULT } from "./seed.js";
import { newEdgeId } from "./graph.js";
import { App } from "./App.js";
import type { Task, Project } from "./types.js";
import type { SavedView } from "./views.js";
import type { CalendarSettings } from "./calendar-settings.js";
import type { GraphEdge } from "./graph.js";
import "./styles/app.lattice";

async function init() {
  // ── 1. Open storage ─────────────────────────────────────────────────────
  //
  // Try IndexedDB first. If it fails (private browsing, Node, workers),
  // fall back to MemoryStorage which implements the same interface but
  // stores data in a Map (lost on page reload).
  let storage: KVStorage;
  try {
    const idbStorage = new IndexedDBStorage({
      dbName: "todo-app",
      version: 5,
      stores: [
        // "todos" renamed to "tasks" in v5 — renamedFrom triggers a cursor
        // migration: all records are copied from "todos" → "tasks" and the
        // old store is deleted. Safe to re-run: guard is inside IndexedDBStorage.
        {
          name: "tasks",
          keyPath: "id",
          renamedFrom: "todos",
          indexes: [
            { name: "status",   keyPath: "status" },
            { name: "priority", keyPath: "priority" },
            { name: "category", keyPath: "category" },
          ],
        },
        // v2: persisted views
        {
          name: "views",
          keyPath: "id",
        },
        // v2: persisted calendar settings
        {
          name: "calendars",
          keyPath: "id",
        },
        // v3: append-only audit event log
        {
          name: "events",
          keyPath: "id",
        },
        // v3: periodic state snapshots (for log compaction)
        {
          name: "snapshots",
          keyPath: "id",
        },
        // v4: project entities (default + user-created)
        {
          name: "projects",
          keyPath: "id",
        },
        // v4: directed graph edges (project→task "contains", future: depends-on)
        {
          name: "edges",
          keyPath: "id",
          indexes: [
            { name: "fromId", keyPath: "fromId" },
            { name: "toId",   keyPath: "toId" },
          ],
        },
      ],
    });
    await idbStorage.open();
    storage = idbStorage;
  } catch {
    // Fallback for environments without IndexedDB
    const memStorage = new MemoryStorage([
      { name: "tasks",     keyPath: "id" },
      { name: "views",     keyPath: "id" },
      { name: "calendars", keyPath: "id" },
      { name: "events",    keyPath: "id" },
      { name: "snapshots", keyPath: "id" },
      { name: "projects",  keyPath: "id" },
      { name: "edges",     keyPath: "id" },
    ]);
    await memStorage.open();
    storage = memStorage;
  }

  // ── 2. Load existing data ───────────────────────────────────────────────
  //
  // Load from all stores in parallel for performance.
  const [rawTasks, views, calendars, projects, edges] = await Promise.all([
    storage.getAll<Task>("tasks"),
    storage.getAll<SavedView>("views"),
    storage.getAll<CalendarSettings>("calendars"),
    storage.getAll<Project>("projects"),
    storage.getAll<GraphEdge>("edges"),
  ]);

  // Normalize tasks: fill in dueTime for records written before v2.
  // Pattern: { dueTime: null, ...t } — the default is overridden if
  // the task already has a dueTime value.
  const tasks = rawTasks.map((t) => ({ dueTime: null, ...t }));

  // ── 2b. Expose storage to the rest of the app ───────────────────────────
  //
  // Components that need to query IndexedDB directly (e.g., the task history
  // panel in TodoEditor) import getStorage() from storage.ts. We initialize
  // it here, before React mounts, so it's ready on the first render.
  initStorage(storage);

  // ── 3. Register middlewares — order matters ─────────────────────────────
  //
  // Middleware runs in registration order. We register:
  //   1. Audit first  — Write-Ahead Log: records intent BEFORE reducer runs.
  //   2. Persistence second — writes new state to IDB AFTER reducer runs.
  //
  // This ordering guarantees that if the app crashes mid-dispatch, the audit
  // log always has the event (it was written before the reducer), so recovery
  // is possible via replay.
  //
  // Must be registered before seed dispatches so seeded data is audited and
  // persisted.
  store.use(createAuditMiddleware(storage));
  store.use(createPersistenceMiddleware(storage));

  // ── 3a. Compact event log if it has grown large ─────────────────────────
  //
  // Check the event count BEFORE hydrating state (before any new events are
  // written). If it exceeds COMPACT_THRESHOLD, take a snapshot and trim old
  // events. We do this here (before React mounts) so compaction doesn't race
  // with user actions.
  const existingEvents = await storage.getAll("events");
  if (existingEvents.length > COMPACT_THRESHOLD) {
    // We compact against the CURRENT state before STATE_LOAD fires.
    // After hydration the state will be fuller, but this still trims most
    // old events. The next compaction cycle will catch any stragglers.
    await compactEventLog(storage, store.getState());
  }

  // ── 3b. Compact on page hide (background tab / close) ──────────────────
  //
  // The visibilitychange event fires when the user switches tabs or closes
  // the page. This is the right moment for housekeeping because:
  //   1. The page is still alive (not yet unloaded) — writes can complete.
  //   2. The user is not waiting for UI responsiveness.
  //   3. On mobile, the page may be killed after backgrounding — this is
  //      often our last chance to write before eviction.
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      compactEventLog(storage, store.getState());
    }
  });

  if (tasks.length > 0 && views.length > 0) {
    // Returning user — load all persisted data.
    // We don't persist activeViewId to IDB yet, so we default to the view
    // with the lowest sortOrder (the leftmost tab), which is the most
    // natural starting point. Fall back to VIEW_ID_ALL_TASKS if no views.
    const defaultView = [...views].sort((a, b) => a.sortOrder - b.sortOrder)[0];
    const storedActiveViewId = defaultView?.id ?? VIEW_ID_ALL_TASKS;
    store.dispatch(
      stateLoadAction(tasks, views, calendars, storedActiveViewId, projects, edges),
    );
  } else if (tasks.length > 0 && views.length === 0) {
    // User has tasks from v1 but no views yet (upgrading from v1→v2)
    // Seed views and calendars, keep existing tasks
    store.dispatch(
      stateLoadAction(tasks, [], calendars, "", projects, edges),
    );
    seedViews(store);
    if (calendars.length === 0) {
      seedCalendars(store);
    }
    store.dispatch(setActiveViewAction(VIEW_ID_ALL_TASKS));
  } else {
    // First visit — seed everything (tasks, views, calendars)
    seedTasks(store);
    seedViews(store);
    seedCalendars(store);
  }

  // ── 4b. Migrate projects (v3→v4 upgrade or first visit) ────────────────
  //
  // If no projects exist in state, this is either the first visit or a
  // v3→v4 upgrade. In both cases:
  //   1. Seed the default project.
  //   2. Create "contains" edges from the default project to every task.
  //
  // This runs AFTER the main hydration dispatch so store.getState().tasks
  // already contains the full task list when we iterate.
  //
  // On first visit, seedTasks() already dispatched TASK_CREATE actions that
  // atomically created edges. But projects.length === 0 still triggers this
  // branch, which would create duplicate edges. We guard by checking
  // store.getState().projects after the main dispatch.
  if (store.getState().projects.length === 0) {
    seedDefaultProject(store);
    const now = Date.now();
    for (const task of store.getState().tasks) {
      // Only create an edge if one doesn't already exist for this task
      const alreadyLinked = store
        .getState()
        .edges.some((e) => e.toId === task.id && e.fromId === PROJECT_ID_DEFAULT);
      if (!alreadyLinked) {
        store.dispatch(
          edgeAddAction({
            id: newEdgeId(),
            fromId: PROJECT_ID_DEFAULT,
            toId: task.id,
            label: "contains",
            createdAt: now,
          }),
        );
      }
    }
  }

  // ── 4. Mount React ──────────────────────────────────────────────────────
  const root = document.getElementById("root");
  if (!root) throw new Error("Root element #root not found");

  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

init();
