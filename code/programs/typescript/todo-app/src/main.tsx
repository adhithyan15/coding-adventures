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
 * === IDB schema: version 2 ===
 *
 * Version 1: { "todos": { keyPath: "id" } }
 *   — tasks stored under "todos" (legacy name, kept for backward compat)
 *
 * Version 2 (this release):
 *   + "views"     : { keyPath: "id" }  — SavedView records
 *   + "calendars" : { keyPath: "id" }  — CalendarSettings records
 *
 * The "todos" store is unchanged — existing task data survives the upgrade.
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
import { stateLoadAction, setActiveViewAction } from "./actions.js";
import { createPersistenceMiddleware } from "./persistence.js";
import { seedTasks, seedViews, seedCalendars, VIEW_ID_ALL_TASKS } from "./seed.js";
import { App } from "./App.js";
import type { Task } from "./types.js";
import type { SavedView } from "./views.js";
import type { CalendarSettings } from "./calendar-settings.js";
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
      version: 2,
      stores: [
        // "todos" kept as-is for backward compat with v1 task data
        {
          name: "todos",
          keyPath: "id",
          indexes: [
            { name: "status",   keyPath: "status" },
            { name: "priority", keyPath: "priority" },
            { name: "category", keyPath: "category" },
          ],
        },
        // New in v2: persisted views
        {
          name: "views",
          keyPath: "id",
        },
        // New in v2: persisted calendar settings
        {
          name: "calendars",
          keyPath: "id",
        },
      ],
    });
    await idbStorage.open();
    storage = idbStorage;
  } catch {
    // Fallback for environments without IndexedDB
    const memStorage = new MemoryStorage([
      { name: "todos",     keyPath: "id" },
      { name: "views",     keyPath: "id" },
      { name: "calendars", keyPath: "id" },
    ]);
    await memStorage.open();
    storage = memStorage;
  }

  // ── 2. Load existing data ───────────────────────────────────────────────
  //
  // Load from all three stores in parallel for performance.
  const [rawTasks, views, calendars] = await Promise.all([
    storage.getAll<Task>("todos"),
    storage.getAll<SavedView>("views"),
    storage.getAll<CalendarSettings>("calendars"),
  ]);

  // Normalize tasks: fill in dueTime for records written before v2.
  // Pattern: { dueTime: null, ...t } — the default is overridden if
  // the task already has a dueTime value.
  const tasks = rawTasks.map((t) => ({ dueTime: null, ...t }));

  // ── 3. Attach persistence middleware BEFORE dispatching any actions ────
  //
  // Must be registered before seed dispatches so seeded data is persisted.
  store.use(createPersistenceMiddleware(storage));

  if (tasks.length > 0 && views.length > 0) {
    // Returning user — load all persisted data.
    // We don't persist activeViewId to IDB yet, so we default to the view
    // with the lowest sortOrder (the leftmost tab), which is the most
    // natural starting point. Fall back to VIEW_ID_ALL_TASKS if no views.
    const defaultView = [...views].sort((a, b) => a.sortOrder - b.sortOrder)[0];
    const storedActiveViewId = defaultView?.id ?? VIEW_ID_ALL_TASKS;
    store.dispatch(
      stateLoadAction(tasks, views, calendars, storedActiveViewId),
    );
  } else if (tasks.length > 0 && views.length === 0) {
    // User has tasks from v1 but no views yet (upgrading from v1→v2)
    // Seed views and calendars, keep existing tasks
    store.dispatch(
      stateLoadAction(tasks, [], calendars, ""),
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
