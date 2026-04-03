/**
 * main.tsx — Application entry point for the Journal app.
 *
 * Responsibilities:
 *   1. Initialise i18n with the English locale strings.
 *   2. Open storage via the Storage interface (IndexedDB with fallback).
 *   3. Load persisted entries or seed a welcome entry on first visit.
 *   4. Attach persistence middleware to the store.
 *   5. Mount the React app into <div id="root">.
 *
 * === Storage abstraction ===
 *
 * The app codes against the KVStorage interface from @coding-adventures/indexeddb.
 * The concrete backend (IndexedDBStorage or MemoryStorage) is selected here
 * at initialization time. No other code in the app knows which backend is active.
 *
 * Swapping to a different backend (Google Drive, SQLite, etc.) requires:
 *   1. Write a class implementing the Storage interface
 *   2. Change the instantiation below
 *   3. Zero changes to the store, reducer, middleware, or UI
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "@coding-adventures/ui-components";
import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
import type { KVStorage } from "@coding-adventures/indexeddb";
import { store } from "./state.js";
import { entriesLoadAction } from "./actions.js";
import { createPersistenceMiddleware } from "./persistence.js";
import { seedEntries } from "./seed.js";
import { App } from "./App.js";
import type { Entry } from "./types.js";
import "@coding-adventures/ui-components/src/styles/theme.css";
import "@coding-adventures/ui-components/src/styles/accessibility.css";
import "./styles/app.css";
import "./styles/preview.css";
import en from "./i18n/locales/en.json";

async function init() {
  // ── 1. Initialise i18n ──────────────────────────────────────────────────
  initI18n({ en });

  // ── 2. Open storage ─────────────────────────────────────────────────────
  //
  // Try IndexedDB first. If unavailable (private browsing, SSR, etc.),
  // fall back to MemoryStorage — an in-memory implementation of the same
  // interface. The app works identically, just loses persistence on reload.
  let storage: KVStorage;
  try {
    const idbStorage = new IndexedDBStorage({
      dbName: "journal-app",
      version: 1,
      stores: [
        {
          name: "entries",
          keyPath: "id",
          indexes: [{ name: "createdAt", keyPath: "createdAt" }],
        },
      ],
    });
    await idbStorage.open();
    storage = idbStorage;
  } catch {
    const memStorage = new MemoryStorage([
      { name: "entries", keyPath: "id" },
    ]);
    await memStorage.open();
    storage = memStorage;
  }

  // ── 3. Load existing data ───────────────────────────────────────────────
  const entries = await storage.getAll<Entry>("entries");

  // ── 4. Attach persistence middleware BEFORE dispatching any actions ─────
  store.use(createPersistenceMiddleware(storage));

  if (entries.length > 0) {
    // Returning user — restore their data
    store.dispatch(entriesLoadAction(entries));
  } else {
    // First visit — seed a welcome entry
    seedEntries(store);
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
