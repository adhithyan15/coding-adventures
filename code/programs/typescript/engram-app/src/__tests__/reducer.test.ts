import { describe, it, expect } from "vitest";
import { reducer, initialState, getSessionCorrectPct, getSessionNewCount } from "../reducer.js";
import {
  createDeckAction,
  createCardAction,
  startSessionAction,
  revealAction,
  rateAction,
  advanceAction,
  completeSessionAction,
  stateLoadAction,
} from "../actions.js";
import type { Card } from "../types.js";

// ── Helpers ─────────────────────────────────────────────────────────────────

const NOW = 1_700_000_000_000;

function makeCard(id: string, deckId = "deck-1"): Card {
  return { id, deckId, front: `Q${id}`, back: `A${id}`, createdAt: 0 };
}

// ── DECK_CREATE ──────────────────────────────────────────────────────────────

describe("reducer — DECK_CREATE", () => {
  it("adds a deck to state.decks", () => {
    const next = reducer(initialState, createDeckAction("Spanish", "Basic vocab"));
    expect(next.decks).toHaveLength(1);
    expect(next.decks[0]!.name).toBe("Spanish");
  });

  it("deck has an id", () => {
    const next = reducer(initialState, createDeckAction("X", ""));
    expect(next.decks[0]!.id).toBeTruthy();
  });

  it("does not mutate other state", () => {
    const next = reducer(initialState, createDeckAction("X", ""));
    expect(next.cards).toHaveLength(0);
    expect(next.sessions).toHaveLength(0);
  });
});

// ── CARD_CREATE ──────────────────────────────────────────────────────────────

describe("reducer — CARD_CREATE", () => {
  it("adds a card to state.cards", () => {
    const next = reducer(initialState, createCardAction("deck-1", "Q", "A"));
    expect(next.cards).toHaveLength(1);
    expect(next.cards[0]!.front).toBe("Q");
    expect(next.cards[0]!.back).toBe("A");
  });

  it("card has correct deckId", () => {
    const next = reducer(initialState, createCardAction("deck-42", "Q", "A"));
    expect(next.cards[0]!.deckId).toBe("deck-42");
  });
});

// ── SESSION_START ────────────────────────────────────────────────────────────

describe("reducer — SESSION_START", () => {
  it("creates a session record", () => {
    const queue = [makeCard("c1")];
    const next = reducer(
      initialState,
      startSessionAction("deck-1", "sess-1", queue),
    );
    expect(next.sessions).toHaveLength(1);
    expect(next.sessions[0]!.id).toBe("sess-1");
    expect(next.sessions[0]!.status).toBe("active");
  });

  it("sets activeSession", () => {
    const queue = [makeCard("c1")];
    const next = reducer(
      initialState,
      startSessionAction("deck-1", "sess-1", queue),
    );
    expect(next.activeSession).not.toBeNull();
    expect(next.activeSession!.currentIndex).toBe(0);
    expect(next.activeSession!.revealed).toBe(false);
  });

  it("session starts with cardsReviewed=0", () => {
    const queue = [makeCard("c1")];
    const next = reducer(
      initialState,
      startSessionAction("deck-1", "sess-1", queue),
    );
    expect(next.sessions[0]!.cardsReviewed).toBe(0);
  });
});

// ── SESSION_REVEAL ───────────────────────────────────────────────────────────

describe("reducer — SESSION_REVEAL", () => {
  it("sets revealed to true", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, revealAction());
    expect(state.activeSession!.revealed).toBe(true);
  });

  it("is a no-op when no active session", () => {
    const next = reducer(initialState, revealAction());
    expect(next).toBe(initialState);
  });
});

// ── SESSION_RATE ─────────────────────────────────────────────────────────────

describe("reducer — SESSION_RATE", () => {
  it("creates a CardProgress record for a new card", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, revealAction());
    state = reducer(
      state,
      rateAction("c1", "s1", "rev-1", "good", NOW),
    );
    expect(state.cardProgress).toHaveLength(1);
    expect(state.cardProgress[0]!.cardId).toBe("c1");
  });

  it("appends a Review record", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "rev-1", "good", NOW));
    expect(state.reviews).toHaveLength(1);
    expect(state.reviews[0]!.rating).toBe("good");
  });

  it("increments cardsReviewed on session", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "rev-1", "good", NOW));
    expect(state.sessions[0]!.cardsReviewed).toBe(1);
  });

  it("increments cardsCorrect for good rating", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "rev-1", "good", NOW));
    expect(state.sessions[0]!.cardsCorrect).toBe(1);
  });

  it("does not increment cardsCorrect for again rating", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "rev-1", "again", NOW));
    expect(state.sessions[0]!.cardsCorrect).toBe(0);
  });

  it("updates existing CardProgress on second review", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "rev-1", "good", NOW));
    state = reducer(state, rateAction("c1", "s1", "rev-2", "easy", NOW));
    expect(state.cardProgress).toHaveLength(1); // still just one record
    expect(state.cardProgress[0]!.timesSeen).toBe(2);
  });
});

// ── SESSION_ADVANCE ──────────────────────────────────────────────────────────

describe("reducer — SESSION_ADVANCE", () => {
  it("increments currentIndex", () => {
    const queue = [makeCard("c1"), makeCard("c2")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, advanceAction());
    expect(state.activeSession!.currentIndex).toBe(1);
  });

  it("resets revealed to false", () => {
    const queue = [makeCard("c1"), makeCard("c2")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, revealAction());
    state = reducer(state, advanceAction());
    expect(state.activeSession!.revealed).toBe(false);
  });
});

// ── SESSION_COMPLETE ─────────────────────────────────────────────────────────

describe("reducer — SESSION_COMPLETE", () => {
  it("sets session status to completed", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, completeSessionAction("s1", NOW));
    expect(state.sessions[0]!.status).toBe("completed");
  });

  it("sets endedAt timestamp", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, completeSessionAction("s1", NOW));
    expect(state.sessions[0]!.endedAt).toBe(NOW);
  });

  it("clears activeSession", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, completeSessionAction("s1", NOW));
    expect(state.activeSession).toBeNull();
  });
});

// ── STATE_LOAD ───────────────────────────────────────────────────────────────

describe("reducer — STATE_LOAD", () => {
  it("replaces all arrays with loaded data", () => {
    const decks = [{ id: "d1", name: "D", description: "", createdAt: 0 }];
    const next = reducer(
      initialState,
      stateLoadAction(decks, [], [], [], []),
    );
    expect(next.decks).toEqual(decks);
    expect(next.cards).toHaveLength(0);
  });

  it("sets activeSession to null", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, stateLoadAction([], [], [], [], []));
    expect(state.activeSession).toBeNull();
  });
});

// ── getSessionCorrectPct ─────────────────────────────────────────────────────

describe("getSessionCorrectPct", () => {
  it("returns 0 when session not found", () => {
    expect(getSessionCorrectPct([], "missing")).toBe(0);
  });

  it("returns 0 when cardsReviewed is 0", () => {
    const queue = [makeCard("c1")];
    const state = reducer(initialState, startSessionAction("d1", "s1", queue));
    expect(getSessionCorrectPct(state.sessions, "s1")).toBe(0);
  });

  it("returns 100 when all correct", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "r1", "good", NOW));
    expect(getSessionCorrectPct(state.sessions, "s1")).toBe(100);
  });

  it("returns 0 when all again", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "r1", "again", NOW));
    expect(getSessionCorrectPct(state.sessions, "s1")).toBe(0);
  });
});

// ── getSessionNewCount ───────────────────────────────────────────────────────

describe("getSessionNewCount", () => {
  it("returns 1 for a session with one new card reviewed", () => {
    const queue = [makeCard("c1")];
    let state = reducer(initialState, startSessionAction("d1", "s1", queue));
    state = reducer(state, rateAction("c1", "s1", "r1", "good", NOW));
    expect(getSessionNewCount(state.cardProgress, state.reviews, "s1")).toBe(1);
  });

  it("returns 0 for an empty session", () => {
    expect(getSessionNewCount([], [], "s1")).toBe(0);
  });
});
