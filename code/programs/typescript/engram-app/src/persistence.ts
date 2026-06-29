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
 *   DECK_UPDATE        → put the updated deck
 *   DECK_DELETE        → delete deck + cascade delete cards, progress,
 *                        sessions, reviews (IDs captured BEFORE reducer)
 *   CARD_CREATE        → put the new card
 *   CARD_UPDATE        → put the updated card
 *   CARD_DELETE        → delete card + its CardProgress record
 *   SESSION_START      → put the new session (status: "active")
 *   SESSION_RATE       → put the updated CardProgress + put the new Review
 *                        + put the updated Session (counters)
 *   SESSION_COMPLETE   → put the updated Session (status: "completed")
 *   STATE_LOAD         → no-op for startup; full replace for imported backups
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
  DECK_UPDATE,
  DECK_DELETE,
  CARD_CREATE,
  CARD_UPDATE,
  CARD_DELETE,
  SESSION_START,
  SESSION_RATE,
  SESSION_COMPLETE,
  STATE_LOAD,
} from "./actions.js";

export function createPersistenceMiddleware(
  storage: KVStorage,
): Middleware<AppState> {
  return (store, action, next) => {
    const stateBefore = store.getState();

    // ── Pre-reducer capture ───────────────────────────────────────────────
    //
    // Delete actions remove entities from state. We need to know WHICH
    // entities to delete from storage BEFORE the reducer clears them.
    // Capture related IDs here, then fire storage.delete() after next().

    let deckDeleteIds: {
      deckId: string;
      cardIds: string[];
      progressCardIds: string[];
      sessionIds: string[];
      reviewIds: string[];
    } | null = null;

    let cardDeleteId: string | null = null;

    if (action.type === DECK_DELETE) {
      const deckId = action.deckId as string;
      const cardIds = stateBefore.cards
        .filter((c) => c.deckId === deckId)
        .map((c) => c.id);
      const progressCardIds = stateBefore.cardProgress
        .filter((p) => cardIds.includes(p.cardId))
        .map((p) => p.cardId);
      const sessionIds = stateBefore.sessions
        .filter((s) => s.deckId === deckId)
        .map((s) => s.id);
      const reviewIds = stateBefore.reviews
        .filter((r) => sessionIds.includes(r.sessionId))
        .map((r) => r.id);
      deckDeleteIds = { deckId, cardIds, progressCardIds, sessionIds, reviewIds };
    }

    if (action.type === CARD_DELETE) {
      cardDeleteId = action.cardId as string;
    }

    // ── Run the reducer ───────────────────────────────────────────────────
    next();

    const state = store.getState();

    // ── Post-reducer persistence ──────────────────────────────────────────

    switch (action.type) {
      case DECK_CREATE: {
        const deck = state.decks[state.decks.length - 1];
        if (deck) storage.put("decks", deck);
        break;
      }

      case DECK_UPDATE: {
        const deckId = action.deckId as string;
        const deck = state.decks.find((d) => d.id === deckId);
        if (deck) storage.put("decks", deck);
        break;
      }

      case DECK_DELETE: {
        if (!deckDeleteIds) break;
        storage.delete("decks", deckDeleteIds.deckId);
        for (const id of deckDeleteIds.cardIds) storage.delete("cards", id);
        for (const id of deckDeleteIds.progressCardIds) storage.delete("card_progress", id);
        for (const id of deckDeleteIds.sessionIds) storage.delete("sessions", id);
        for (const id of deckDeleteIds.reviewIds) storage.delete("reviews", id);
        break;
      }

      case CARD_CREATE: {
        const card = state.cards[state.cards.length - 1];
        if (card) storage.put("cards", card);
        break;
      }

      case CARD_UPDATE: {
        const cardId = action.cardId as string;
        const card = state.cards.find((c) => c.id === cardId);
        if (card) storage.put("cards", card);
        break;
      }

      case CARD_DELETE: {
        if (cardDeleteId) {
          storage.delete("cards", cardDeleteId);
          storage.delete("card_progress", cardDeleteId);
        }
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

      case STATE_LOAD: {
        if (action.persist === true) {
          void replacePersistedState(storage, stateBefore, state);
        }
        break;
      }

      // These actions only affect ephemeral activeSession — nothing to persist
      default:
        break;
    }
  };
}

async function replacePersistedState(
  storage: KVStorage,
  previous: AppState,
  next: AppState,
): Promise<void> {
  const deletes: Promise<void>[] = [
    ...previous.decks.map((deck) => storage.delete("decks", deck.id)),
    ...previous.cards.map((card) => storage.delete("cards", card.id)),
    ...previous.cardProgress.map((progress) =>
      storage.delete("card_progress", progress.cardId),
    ),
    ...previous.sessions.map((session) => storage.delete("sessions", session.id)),
    ...previous.reviews.map((review) => storage.delete("reviews", review.id)),
  ];
  await Promise.all(deletes);

  const puts: Promise<void>[] = [
    ...next.decks.map((deck) => storage.put("decks", deck)),
    ...next.cards.map((card) => storage.put("cards", card)),
    ...next.cardProgress.map((progress) =>
      storage.put("card_progress", progress),
    ),
    ...next.sessions.map((session) => storage.put("sessions", session)),
    ...next.reviews.map((review) => storage.put("reviews", review)),
  ];
  await Promise.all(puts);
}
