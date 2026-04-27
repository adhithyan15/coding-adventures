/**
 * persistence.ts — IndexedDB persistence middleware.
 *
 * After each action runs through the reducer, this middleware writes the
 * affected records to IndexedDB storage. It uses a fire-and-forget approach:
 * it calls put/delete without awaiting the Promise. This keeps the UI
 * responsive — dispatch returns immediately after the reducer completes.
 *
 * === What gets persisted ===
 *
 *   DECK_CREATE        → put the new deck
 *   CARD_CREATE        → put the new card
 *   SESSION_START      → put the new session (status: "active")
 *   SESSION_RATE       → put the updated CardProgress + put the new Review
 *                        + put the updated Session (counters)
 *   SESSION_COMPLETE   → put the updated Session (status: "completed")
 *   STATE_LOAD         → no-op (data came FROM storage)
 *   SESSION_REVEAL     → no-op (ephemeral UI state only)
 *   SESSION_ADVANCE    → no-op (ephemeral UI state only)
 *
 * activeSession is never persisted — it is ephemeral in-memory state that
 * is rebuilt from scratch when a new session starts.
 */

import type { KVStorage } from "@coding-adventures/indexeddb";
import type { Middleware } from "@coding-adventures/store";
import type { AppState } from "./reducer.js";
import {
  DECK_CREATE,
  CARD_CREATE,
  SESSION_START,
  SESSION_RATE,
  SESSION_COMPLETE,
} from "./actions.js";

export function createPersistenceMiddleware(
  storage: KVStorage,
): Middleware<AppState> {
  return (store, action, next) => {
    // Let the reducer run first so we can read the new state
    next();

    const state = store.getState();

    switch (action.type) {
      case DECK_CREATE: {
        const deck = state.decks[state.decks.length - 1];
        if (deck) storage.put("decks", deck);
        break;
      }

      case CARD_CREATE: {
        const card = state.cards[state.cards.length - 1];
        if (card) storage.put("cards", card);
        break;
      }

      case SESSION_START: {
        const session = state.sessions[state.sessions.length - 1];
        if (session) storage.put("sessions", session);
        break;
      }

      case SESSION_RATE: {
        const cardId = action.cardId as string;
        const sessionId = action.sessionId as string;

        // Persist the updated CardProgress record
        const progress = state.cardProgress.find((p) => p.cardId === cardId);
        if (progress) storage.put("card_progress", progress);

        // Persist the new Review record
        const review = state.reviews[state.reviews.length - 1];
        if (review) storage.put("reviews", review);

        // Persist the updated Session (cardsReviewed / cardsCorrect counters)
        const session = state.sessions.find((s) => s.id === sessionId);
        if (session) storage.put("sessions", session);
        break;
      }

      case SESSION_COMPLETE: {
        const sessionId = action.sessionId as string;
        const session = state.sessions.find((s) => s.id === sessionId);
        if (session) storage.put("sessions", session);
        break;
      }

      // These actions only affect ephemeral activeSession — nothing to persist
      default:
        break;
    }
  };
}
