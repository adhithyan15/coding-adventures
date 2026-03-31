import { describe, it, expect } from "vitest";
import { reducer, initialState, getSessionCorrectPct, getSessionNewCount } from "../reducer.js";
import {
  createDeckAction,
  updateDeckAction,
  deleteDeckAction,
  createCardAction,
  updateCardAction,
  deleteCardAction,
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

// ── DECK_UPDATE ──────────────────────────────────────────────────────────────

describe("reducer — DECK_UPDATE", () => {
  it("updates the name and description of an existing deck", () => {
    let state = reducer(initialState, createDeckAction("Old", "Old desc"));
    const deckId = state.decks[0]!.id;
    state = reducer(state, updateDeckAction(deckId, "New", "New desc"));
    expect(state.decks[0]!.name).toBe("New");
    expect(state.decks[0]!.description).toBe("New desc");
  });

  it("does not modify other decks", () => {
    let state = reducer(initialState, createDeckAction("A", "a"));
    state = reducer(state, createDeckAction("B", "b"));
    const deckIdA = state.decks[0]!.id;
    state = reducer(state, updateDeckAction(deckIdA, "A2", "a2"));
    expect(state.decks[1]!.name).toBe("B");
  });

  it("is a no-op for a missing deckId", () => {
    const state = reducer(initialState, createDeckAction("X", "x"));
    const next = reducer(state, updateDeckAction("nonexistent", "Y", "y"));
    expect(next.decks).toEqual(state.decks);
  });
});

// ── DECK_DELETE ──────────────────────────────────────────────────────────────

describe("reducer — DECK_DELETE", () => {
  function seededState() {
    let state = reducer(initialState, createDeckAction("Deck", "desc"));
    const deckId = state.decks[0]!.id;
    state = reducer(state, createCardAction(deckId, "Q1", "A1"));
    state = reducer(state, createCardAction(deckId, "Q2", "A2"));
    const cards = state.cards;
    // Start a session and rate a card to create progress + reviews + session
    state = reducer(
      state,
      startSessionAction(deckId, "sess-1", cards),
    );
    state = reducer(state, rateAction(cards[0]!.id, "sess-1", "rev-1", "good", NOW));
    return { state, deckId };
  }

  it("removes the deck", () => {
    const { state, deckId } = seededState();
    const next = reducer(state, deleteDeckAction(deckId));
    expect(next.decks).toHaveLength(0);
  });

  it("cascade-removes cards belonging to the deck", () => {
    const { state, deckId } = seededState();
    const next = reducer(state, deleteDeckAction(deckId));
    expect(next.cards).toHaveLength(0);
  });

  it("cascade-removes CardProgress for deleted cards", () => {
    const { state, deckId } = seededState();
    const next = reducer(state, deleteDeckAction(deckId));
    expect(next.cardProgress).toHaveLength(0);
  });

  it("cascade-removes sessions for the deck", () => {
    const { state, deckId } = seededState();
    const next = reducer(state, deleteDeckAction(deckId));
    expect(next.sessions).toHaveLength(0);
  });

  it("cascade-removes reviews from deleted sessions", () => {
    const { state, deckId } = seededState();
    const next = reducer(state, deleteDeckAction(deckId));
    expect(next.reviews).toHaveLength(0);
  });

  it("clears activeSession if it belongs to the deleted deck", () => {
    const { state, deckId } = seededState();
    expect(state.activeSession).not.toBeNull();
    const next = reducer(state, deleteDeckAction(deckId));
    expect(next.activeSession).toBeNull();
  });

  it("preserves unrelated decks and their data", () => {
    let state = reducer(initialState, createDeckAction("Keep", "keeper"));
    const keepId = state.decks[0]!.id;
    state = reducer(state, createCardAction(keepId, "KQ", "KA"));
    state = reducer(state, createDeckAction("Delete", "doomed"));
    const deleteId = state.decks[1]!.id;
    state = reducer(state, createCardAction(deleteId, "DQ", "DA"));

    state = reducer(state, deleteDeckAction(deleteId));
    expect(state.decks).toHaveLength(1);
    expect(state.decks[0]!.id).toBe(keepId);
    expect(state.cards).toHaveLength(1);
    expect(state.cards[0]!.deckId).toBe(keepId);
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

// ── CARD_UPDATE ──────────────────────────────────────────────────────────────

describe("reducer — CARD_UPDATE", () => {
  it("updates the front and back of an existing card", () => {
    let state = reducer(initialState, createCardAction("d1", "Old Q", "Old A"));
    const cardId = state.cards[0]!.id;
    state = reducer(state, updateCardAction(cardId, "New Q", "New A"));
    expect(state.cards[0]!.front).toBe("New Q");
    expect(state.cards[0]!.back).toBe("New A");
  });

  it("preserves other card fields", () => {
    let state = reducer(initialState, createCardAction("d1", "Q", "A"));
    const card = state.cards[0]!;
    state = reducer(state, updateCardAction(card.id, "Q2", "A2"));
    expect(state.cards[0]!.deckId).toBe("d1");
    expect(state.cards[0]!.createdAt).toBe(card.createdAt);
  });

  it("does not modify other cards", () => {
    let state = reducer(initialState, createCardAction("d1", "Q1", "A1"));
    state = reducer(state, createCardAction("d1", "Q2", "A2"));
    const firstId = state.cards[0]!.id;
    state = reducer(state, updateCardAction(firstId, "Q1x", "A1x"));
    expect(state.cards[1]!.front).toBe("Q2");
  });
});

// ── CARD_DELETE ──────────────────────────────────────────────────────────────

describe("reducer — CARD_DELETE", () => {
  it("removes the card", () => {
    let state = reducer(initialState, createCardAction("d1", "Q", "A"));
    const cardId = state.cards[0]!.id;
    state = reducer(state, deleteCardAction(cardId));
    expect(state.cards).toHaveLength(0);
  });

  it("removes the CardProgress for the deleted card", () => {
    let state = reducer(initialState, createCardAction("d1", "Q", "A"));
    const cardId = state.cards[0]!.id;
    state = reducer(
      state,
      startSessionAction("d1", "s1", state.cards),
    );
    state = reducer(state, rateAction(cardId, "s1", "r1", "good", NOW));
    expect(state.cardProgress).toHaveLength(1);
    state = reducer(state, deleteCardAction(cardId));
    expect(state.cardProgress).toHaveLength(0);
  });

  it("does not remove other cards", () => {
    let state = reducer(initialState, createCardAction("d1", "Q1", "A1"));
    state = reducer(state, createCardAction("d1", "Q2", "A2"));
    const firstId = state.cards[0]!.id;
    state = reducer(state, deleteCardAction(firstId));
    expect(state.cards).toHaveLength(1);
    expect(state.cards[0]!.front).toBe("Q2");
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
