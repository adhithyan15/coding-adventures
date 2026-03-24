/**
 * main.tsx — Application entry point (V0.3: IndexedDB persistence).
 *
 * Responsibilities:
 *   1. Initialise i18n with the English locale strings.
 *   2. Open IndexedDB storage (with MemoryStorage fallback).
 *   3. Load persisted data or seed example templates on first visit.
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
 * interface. The app works identically, it just loses persistence on reload.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "@coding-adventures/ui-components";
import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
import type { KVStorage } from "@coding-adventures/indexeddb";
import { store } from "./state.js";
import { stateLoadAction } from "./actions.js";
import { createPersistenceMiddleware } from "./persistence.js";
import { seedTemplates } from "./seed.js";
import { App } from "./App.js";
import type { Template, Instance, TodoItem } from "./types.js";
import "@coding-adventures/ui-components/src/styles/theme.css";
import "./styles/app.css";
import en from "./i18n/locales/en.json";

async function init() {
  // ── 1. Initialise i18n ──────────────────────────────────────────────────
  initI18n({ en });

  // ── 2. Open storage ─────────────────────────────────────────────────────
  let storage: KVStorage;
  try {
    const idbStorage = new IndexedDBStorage({
      dbName: "checklist-app",
      version: 2,
      stores: [
        { name: "templates", keyPath: "id" },
        {
          name: "instances",
          keyPath: "id",
          indexes: [{ name: "templateId", keyPath: "templateId" }],
        },
        { name: "todos", keyPath: "id" },
      ],
    });
    await idbStorage.open();
    storage = idbStorage;
  } catch {
    // Fallback for environments without IndexedDB (private browsing, Node, etc.)
    const memStorage = new MemoryStorage([
      { name: "templates", keyPath: "id" },
      { name: "instances", keyPath: "id" },
      { name: "todos", keyPath: "id" },
    ]);
    await memStorage.open();
    storage = memStorage;
  }

  // ── 3. Load existing data ───────────────────────────────────────────────
  const templates = await storage.getAll<Template>("templates");
  const instances = await storage.getAll<Instance>("instances");
  const todos = await storage.getAll<TodoItem>("todos");

  // ── 4. Attach persistence middleware BEFORE dispatching any actions ────
  store.use(createPersistenceMiddleware(storage));

  if (templates.length > 0) {
    // Existing user — load their persisted data into the store
    store.dispatch(stateLoadAction(templates, instances, todos));
  } else {
    // First visit — seed example templates (persistence middleware saves them)
    seedTemplates(store);
  }

  // ── 5. Mount React ──────────────────────────────────────────────────────
  const root = document.getElementById("root");
  if (!root) throw new Error("Root element #root not found");

  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

init();
