/**
 * reducer.ts — Pure state transition function.
 *
 * The reducer is a pure function: (state, action) => newState.
 * It must not mutate the input state, perform I/O, or call Date.now().
 * All randomness and timestamps are passed in through actions.
 *
 * === ID generation ===
 *
 * crypto.randomUUID() is available in Electron's Chromium renderer and in
 * Node 19+. The fallback covers older environments (test runners, CI).
 *
 * === SM-2 integration ===
 *
 * The SESSION_RATE action triggers an SM-2 update via updateCardProgress()
 * or createInitialProgress(). The SM-2 functions live in sm2.ts; the
 * reducer calls them and integrates the result into state.
 */

import type { Action } from "@coding-adventures/store";
import type { AppState, Card, CardProgress, Deck, Rating } from "./types.js";
import {
  DECK_CREATE,
  CARD_CREATE,
  SESSION_START,
  SESSION_REVEAL,
  SESSION_RATE,
  SESSION_ADVANCE,
  SESSION_COMPLETE,
  STATE_LOAD,
} from "./actions.js";
import {
  updateCardProgress,
  createInitialProgress,
  INITIAL_EASE_FACTOR,
} from "./sm2.js";

export type { AppState };

// ── Initial state ───────────────────────────────────────────────────────────

export const initialState: AppState = {
  decks: [],
  cards: [],
  cardProgress: [],
  sessions: [],
  reviews: [],
  activeSession: null,
};

// ── ID generation ───────────────────────────────────────────────────────────

function generateId(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

// ── Reducer ─────────────────────────────────────────────────────────────────

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    // ── DECK_CREATE ──────────────────────────────────────────────────────────
    case DECK_CREATE: {
      const deck: Deck = {
        id: generateId(),
        name: action.name as string,
        description: action.description as string,
        createdAt: Date.now(),
      };
      return { ...state, decks: [...state.decks, deck] };
    }

    // ── CARD_CREATE ──────────────────────────────────────────────────────────
    case CARD_CREATE: {
      const card: Card = {
        id: generateId(),
        deckId: action.deckId as string,
        front: action.front as string,
        back: action.back as string,
        createdAt: Date.now(),
      };
      return { ...state, cards: [...state.cards, card] };
    }

    // ── SESSION_START ────────────────────────────────────────────────────────
    //
    // Creates a new Session record (persisted) and sets up the ephemeral
    // activeSession in memory. The queue is provided by the component via
    // buildSessionQueue() before dispatch.
    case SESSION_START: {
      const sessionId = action.sessionId as string;
      const deckId = action.deckId as string;
      const queue = action.queue as Card[];
      const session = {
        id: sessionId,
        deckId,
        status: "active" as const,
        startedAt: Date.now(),
        endedAt: null,
        cardsReviewed: 0,
        cardsCorrect: 0,
      };
      return {
        ...state,
        sessions: [...state.sessions, session],
        activeSession: {
          sessionId,
          deckId,
          queue,
          currentIndex: 0,
          revealed: false,
        },
      };
    }

    // ── SESSION_REVEAL ───────────────────────────────────────────────────────
    //
    // Flips the active card to show its back face.
    case SESSION_REVEAL: {
      if (!state.activeSession) return state;
      return {
        ...state,
        activeSession: { ...state.activeSession, revealed: true },
      };
    }

    // ── SESSION_RATE ─────────────────────────────────────────────────────────
    //
    // The most complex action: updates CardProgress via SM-2, logs a Review,
    // and updates the Session aggregate counters.
    case SESSION_RATE: {
      if (!state.activeSession) return state;

      const cardId = action.cardId as string;
      const sessionId = action.sessionId as string;
      const reviewId = action.reviewId as string;
      const rating = action.rating as Rating;
      const now = action.now as number;
      const isCorrect = rating !== "again";

      // ── 1. Upsert CardProgress ─────────────────────────────────────────────
      const existing = state.cardProgress.find((p) => p.cardId === cardId);
      let newProgress: CardProgress;

      if (existing) {
        newProgress = updateCardProgress(existing, rating, now);
      } else {
        // First time this card has been seen — create a fresh record
        newProgress = createInitialProgress(cardId, rating, now);
      }

      const updatedProgress = existing
        ? state.cardProgress.map((p) => (p.cardId === cardId ? newProgress : p))
        : [...state.cardProgress, newProgress];

      // ── 2. Append Review ───────────────────────────────────────────────────
      const review = {
        id: reviewId,
        sessionId,
        cardId,
        rating,
        reviewedAt: now,
      };

      // ── 3. Update Session counters ─────────────────────────────────────────
      const updatedSessions = state.sessions.map((s) => {
        if (s.id !== sessionId) return s;
        return {
          ...s,
          cardsReviewed: s.cardsReviewed + 1,
          cardsCorrect: isCorrect ? s.cardsCorrect + 1 : s.cardsCorrect,
        };
      });

      return {
        ...state,
        cardProgress: updatedProgress,
        reviews: [...state.reviews, review],
        sessions: updatedSessions,
      };
    }

    // ── SESSION_ADVANCE ──────────────────────────────────────────────────────
    //
    // Moves to the next card in the queue and resets the revealed flag.
    case SESSION_ADVANCE: {
      if (!state.activeSession) return state;
      return {
        ...state,
        activeSession: {
          ...state.activeSession,
          currentIndex: state.activeSession.currentIndex + 1,
          revealed: false,
        },
      };
    }

    // ── SESSION_COMPLETE ─────────────────────────────────────────────────────
    //
    // Marks the session as completed and clears the ephemeral activeSession.
    case SESSION_COMPLETE: {
      const completeSessionId = action.sessionId as string;
      const completeNow = action.now as number;
      const updatedSessions = state.sessions.map((s) => {
        if (s.id !== completeSessionId) return s;
        return {
          ...s,
          status: "completed" as const,
          endedAt: completeNow,
        };
      });
      return {
        ...state,
        sessions: updatedSessions,
        activeSession: null,
      };
    }

    // ── STATE_LOAD ───────────────────────────────────────────────────────────
    //
    // Bulk-loads all persisted data from IndexedDB on startup.
    // Replaces all arrays; activeSession is always null at startup.
    case STATE_LOAD: {
      return {
        ...state,
        decks: action.decks as Deck[],
        cards: action.cards as Card[],
        cardProgress: action.cardProgress as CardProgress[],
        sessions: action.sessions as AppState["sessions"],
        reviews: action.reviews as AppState["reviews"],
        activeSession: null,
      };
    }

    default:
      return state;
  }
}

// ── Computed helpers ─────────────────────────────────────────────────────────

/**
 * getSessionCorrectPct — compute the correct percentage for a completed session.
 */
export function getSessionCorrectPct(
  sessions: AppState["sessions"],
  sessionId: string,
): number {
  const session = sessions.find((s) => s.id === sessionId);
  if (!session || session.cardsReviewed === 0) return 0;
  return Math.round((session.cardsCorrect / session.cardsReviewed) * 100);
}

/**
 * getSessionNewCount — count how many cards in a session had no prior progress.
 *
 * A card is "new" in this session if it had no CardProgress before the session
 * started. We detect this by checking if its timesSeen after the session is
 * exactly the number of times it appeared in the session's reviews.
 *
 * Simpler approximation used here: count reviews for this session where the
 * card was seen for the first time (timesSeen === 1 in cardProgress).
 */
export function getSessionNewCount(
  cardProgress: CardProgress[],
  reviews: AppState["reviews"],
  sessionId: string,
): number {
  const sessionReviewCardIds = new Set(
    reviews.filter((r) => r.sessionId === sessionId).map((r) => r.cardId),
  );
  return cardProgress.filter(
    (p) => sessionReviewCardIds.has(p.cardId) && p.timesSeen === 1,
  ).length;
}

/**
 * newCardProgressRecord — build the default progress for a brand-new card.
 * Used only for type reference; actual creation goes through createInitialProgress.
 */
export function defaultProgress(cardId: string): CardProgress {
  return {
    cardId,
    interval: 1,
    easeFactor: INITIAL_EASE_FACTOR,
    nextDueAt: 0,
    timesSeen: 0,
    timesCorrect: 0,
    timesIncorrect: 0,
    lastSeenAt: 0,
  };
}
