/**
 * main.tsx — Application entry point for Engram.
 *
 * Responsibilities:
 *   1. Initialise i18n with the English locale strings.
 *   2. Open IndexedDB storage (with MemoryStorage fallback).
 *   3. Load persisted data or seed the US State Capitals deck on first visit.
 *   4. Attach persistence middleware to the store.
 *   5. Mount the React app into <div id="root">.
 *
 * The init() function is async because IndexedDB operations return Promises.
 * We await the database open and data load before rendering so the first
 * render has the full state — no loading spinners needed.
 *
 * === IndexedDB stores ===
 *
 *   decks          keyPath: "id"
 *   cards          keyPath: "id"    index: deckId
 *   card_progress  keyPath: "cardId"
 *   sessions       keyPath: "id"    index: deckId
 *   reviews        keyPath: "id"
 *
 * === Storage fallback ===
 *
 * IndexedDB may be unavailable in some environments (private browsing in
 * older Safari, SSR, Node test runners). On failure we fall back to
 * MemoryStorage — an in-memory implementation of the same KVStorage
 * interface. The app works identically, just loses persistence on reload.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "@coding-adventures/ui-components";
import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
import type { KVStorage } from "@coding-adventures/indexeddb";
import { store } from "./state.js";
import { stateLoadAction } from "./actions.js";
import { createPersistenceMiddleware } from "./persistence.js";
import { seedDeck } from "./seed.js";
import { App } from "./App.js";
import type { Deck, Card, CardProgress, Session, Review } from "./types.js";
import "@coding-adventures/ui-components/src/styles/theme.css";
import "@coding-adventures/ui-components/src/styles/accessibility.css";
import "@coding-adventures/ui-components/src/styles/flash-card.css";
import "@coding-adventures/ui-components/src/styles/rating-buttons.css";
import "@coding-adventures/ui-components/src/styles/progress-bar.css";
import "./styles/app.css";
import en from "./i18n/locales/en.json";

async function init() {
  // ── 1. Initialise i18n ──────────────────────────────────────────────────
  initI18n({ en });

  // ── 2. Open storage ─────────────────────────────────────────────────────
  let storage: KVStorage;
  try {
    const idbStorage = new IndexedDBStorage({
      dbName: "engram-app",
      version: 1,
      stores: [
        { name: "decks", keyPath: "id" },
        {
          name: "cards",
          keyPath: "id",
          indexes: [{ name: "deckId", keyPath: "deckId" }],
        },
        { name: "card_progress", keyPath: "cardId" },
        {
          name: "sessions",
          keyPath: "id",
          indexes: [{ name: "deckId", keyPath: "deckId" }],
        },
        { name: "reviews", keyPath: "id" },
      ],
    });
    await idbStorage.open();
    storage = idbStorage;
  } catch {
    const memStorage = new MemoryStorage([
      { name: "decks", keyPath: "id" },
      { name: "cards", keyPath: "id" },
      { name: "card_progress", keyPath: "cardId" },
      { name: "sessions", keyPath: "id" },
      { name: "reviews", keyPath: "id" },
    ]);
    await memStorage.open();
    storage = memStorage;
  }

  // ── 3. Load existing data ───────────────────────────────────────────────
  const decks = await storage.getAll<Deck>("decks");
  const cards = await storage.getAll<Card>("cards");
  const cardProgress = await storage.getAll<CardProgress>("card_progress");
  const sessions = await storage.getAll<Session>("sessions");
  const reviews = await storage.getAll<Review>("reviews");

  // ── 4. Attach persistence middleware BEFORE dispatching any actions ─────
  store.use(createPersistenceMiddleware(storage));

  if (decks.length > 0) {
    // Returning user — restore their data
    store.dispatch(stateLoadAction(decks, cards, cardProgress, sessions, reviews));
  } else {
    // First visit — seed the US State Capitals deck
    seedDeck(store);
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
