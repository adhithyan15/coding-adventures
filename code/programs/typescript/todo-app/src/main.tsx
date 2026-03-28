/**
 * main.tsx — Application entry point.
 *
 * Responsibilities:
 *   1. Open IndexedDB storage (with MemoryStorage fallback).
 *   2. Load persisted data or seed example todos on first visit.
 *   3. Attach persistence middleware to the store.
 *   4. Mount the React app into <div id="root">.
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
 * interface. The app works identically, it just loses persistence on reload.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
import type { KVStorage } from "@coding-adventures/indexeddb";
import { store } from "./state.js";
import { stateLoadAction } from "./actions.js";
import { createPersistenceMiddleware } from "./persistence.js";
import { seedTodos } from "./seed.js";
import { App } from "./App.js";
import type { TodoItem } from "./types.js";
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
      version: 1,
      stores: [
        {
          name: "todos",
          keyPath: "id",
          indexes: [
            { name: "status", keyPath: "status" },
            { name: "priority", keyPath: "priority" },
            { name: "category", keyPath: "category" },
          ],
        },
      ],
    });
    await idbStorage.open();
    storage = idbStorage;
  } catch {
    // Fallback for environments without IndexedDB
    const memStorage = new MemoryStorage([
      { name: "todos", keyPath: "id" },
    ]);
    await memStorage.open();
    storage = memStorage;
  }

  // ── 2. Load existing data ───────────────────────────────────────────────
  const todos = await storage.getAll<TodoItem>("todos");

  // ── 3. Attach persistence middleware BEFORE dispatching any actions ────
  //
  // The middleware must be registered before any dispatches so it can
  // intercept seed actions and persist them. Order matters!
  store.use(createPersistenceMiddleware(storage));

  if (todos.length > 0) {
    // Existing user — load their persisted data into the store
    store.dispatch(stateLoadAction(todos));
  } else {
    // First visit — seed example todos (persistence middleware saves them)
    seedTodos(store);
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
