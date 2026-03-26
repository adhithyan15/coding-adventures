import { describe, it, expect } from "vitest";
import {
  buildSessionQueue,
  isDeckCaughtUp,
  getDeckStats,
  SESSION_SIZE,
  MAX_NEW_PER_SESSION,
} from "../queue.js";
import type { Card, CardProgress } from "../types.js";

// ── Helpers ─────────────────────────────────────────────────────────────────

const NOW = 1_700_000_000_000;
const ONE_DAY_MS = 24 * 60 * 60 * 1000;

function makeCard(id: string, deckId = "deck-1"): Card {
  return { id, deckId, front: `Q${id}`, back: `A${id}`, createdAt: 0 };
}

function makeProgress(
  cardId: string,
  overrides: Partial<CardProgress> = {},
): CardProgress {
  return {
    cardId,
    interval: 1,
    easeFactor: 2.5,
    nextDueAt: NOW - 1, // due by default
    timesSeen: 1,
    timesCorrect: 1,
    timesIncorrect: 0,
    lastSeenAt: NOW - ONE_DAY_MS,
    ...overrides,
  };
}

// ── buildSessionQueue ────────────────────────────────────────────────────────

describe("buildSessionQueue", () => {
  it("returns empty array for empty deck", () => {
    expect(buildSessionQueue([], [], "deck-1", NOW)).toEqual([]);
  });

  it("returns only new cards when no progress records exist", () => {
    const cards = Array.from({ length: 10 }, (_, i) => makeCard(`c${i}`));
    const queue = buildSessionQueue(cards, [], "deck-1", NOW);
    expect(queue.length).toBe(Math.min(10, MAX_NEW_PER_SESSION));
    expect(queue.every((c) => c.deckId === "deck-1")).toBe(true);
  });

  it("caps new cards at MAX_NEW_PER_SESSION", () => {
    const cards = Array.from({ length: 20 }, (_, i) => makeCard(`c${i}`));
    const queue = buildSessionQueue(cards, [], "deck-1", NOW);
    expect(queue.length).toBe(MAX_NEW_PER_SESSION);
  });

  it("returns only due cards when all cards have progress", () => {
    const cards = [makeCard("a"), makeCard("b"), makeCard("c")];
    const progress = cards.map((c) => makeProgress(c.id));
    const queue = buildSessionQueue(cards, progress, "deck-1", NOW);
    expect(queue.length).toBe(3);
  });

  it("excludes cards not yet due", () => {
    const cards = [makeCard("a"), makeCard("b")];
    const progress = [
      makeProgress("a", { nextDueAt: NOW - 1 }), // due
      makeProgress("b", { nextDueAt: NOW + ONE_DAY_MS }), // not due
    ];
    const queue = buildSessionQueue(cards, progress, "deck-1", NOW);
    expect(queue.length).toBe(1);
    expect(queue[0]!.id).toBe("a");
  });

  it("sorts due cards most-overdue-first", () => {
    const cards = [makeCard("recent"), makeCard("old")];
    const progress = [
      makeProgress("recent", { nextDueAt: NOW - 100 }),
      makeProgress("old", { nextDueAt: NOW - 10000 }),
    ];
    const queue = buildSessionQueue(cards, progress, "deck-1", NOW);
    expect(queue[0]!.id).toBe("old");
    expect(queue[1]!.id).toBe("recent");
  });

  it("caps total queue at SESSION_SIZE", () => {
    const cards = Array.from({ length: 30 }, (_, i) => makeCard(`c${i}`));
    const progress = cards.map((c) => makeProgress(c.id)); // all due
    const queue = buildSessionQueue(cards, progress, "deck-1", NOW);
    expect(queue.length).toBeLessThanOrEqual(SESSION_SIZE);
  });

  it("places due cards before new cards in the queue", () => {
    const dueCard = makeCard("due");
    const newCard = makeCard("new");
    const progress = [makeProgress("due")]; // only due has progress
    const queue = buildSessionQueue([dueCard, newCard], progress, "deck-1", NOW);
    expect(queue[0]!.id).toBe("due");
    expect(queue[1]!.id).toBe("new");
  });

  it("only includes cards from the specified deck", () => {
    const cards = [makeCard("a", "deck-1"), makeCard("b", "deck-2")];
    const queue = buildSessionQueue(cards, [], "deck-1", NOW);
    expect(queue.every((c) => c.deckId === "deck-1")).toBe(true);
  });

  it("returns empty when all cards are scheduled (not yet due)", () => {
    const cards = [makeCard("a"), makeCard("b")];
    const progress = cards.map((c) =>
      makeProgress(c.id, { nextDueAt: NOW + ONE_DAY_MS }),
    );
    const queue = buildSessionQueue(cards, progress, "deck-1", NOW);
    expect(queue.length).toBe(0);
  });

  it("mixes due and new cards up to SESSION_SIZE total", () => {
    const dueCards = Array.from({ length: 15 }, (_, i) =>
      makeCard(`due${i}`),
    );
    const newCards = Array.from({ length: 10 }, (_, i) => makeCard(`new${i}`));
    const progress = dueCards.map((c) => makeProgress(c.id));
    const queue = buildSessionQueue(
      [...dueCards, ...newCards],
      progress,
      "deck-1",
      NOW,
    );
    expect(queue.length).toBe(SESSION_SIZE);
    const newInQueue = queue.filter((c) => c.id.startsWith("new"));
    expect(newInQueue.length).toBeLessThanOrEqual(MAX_NEW_PER_SESSION);
  });
});

// ── isDeckCaughtUp ───────────────────────────────────────────────────────────

describe("isDeckCaughtUp", () => {
  it("returns true for empty deck", () => {
    expect(isDeckCaughtUp([], [], "deck-1", NOW)).toBe(true);
  });

  it("returns false when there are new cards", () => {
    const cards = [makeCard("a")];
    expect(isDeckCaughtUp(cards, [], "deck-1", NOW)).toBe(false);
  });

  it("returns false when there are due cards", () => {
    const cards = [makeCard("a")];
    const progress = [makeProgress("a", { nextDueAt: NOW - 1 })];
    expect(isDeckCaughtUp(cards, progress, "deck-1", NOW)).toBe(false);
  });

  it("returns true when all cards are scheduled in the future", () => {
    const cards = [makeCard("a"), makeCard("b")];
    const progress = cards.map((c) =>
      makeProgress(c.id, { nextDueAt: NOW + ONE_DAY_MS }),
    );
    expect(isDeckCaughtUp(cards, progress, "deck-1", NOW)).toBe(true);
  });
});

// ── getDeckStats ─────────────────────────────────────────────────────────────

describe("getDeckStats", () => {
  it("counts new cards correctly", () => {
    const cards = [makeCard("a"), makeCard("b"), makeCard("c")];
    const stats = getDeckStats(cards, [], "deck-1", NOW);
    expect(stats.newCount).toBe(3);
    expect(stats.total).toBe(3);
  });

  it("counts mastered cards (interval > 21) correctly", () => {
    const cards = [makeCard("a")];
    const progress = [makeProgress("a", { interval: 30, nextDueAt: NOW + ONE_DAY_MS })];
    const stats = getDeckStats(cards, progress, "deck-1", NOW);
    expect(stats.masteredCount).toBe(1);
    expect(stats.learningCount).toBe(0);
  });

  it("counts learning cards (interval <= 21) correctly", () => {
    const cards = [makeCard("a")];
    const progress = [makeProgress("a", { interval: 10 })];
    const stats = getDeckStats(cards, progress, "deck-1", NOW);
    expect(stats.learningCount).toBe(1);
    expect(stats.masteredCount).toBe(0);
  });

  it("counts due cards correctly", () => {
    const cards = [makeCard("a"), makeCard("b")];
    const progress = [
      makeProgress("a", { nextDueAt: NOW - 1 }), // due
      makeProgress("b", { nextDueAt: NOW + ONE_DAY_MS }), // not due
    ];
    const stats = getDeckStats(cards, progress, "deck-1", NOW);
    expect(stats.dueCount).toBe(1);
  });

  it("returns zero averageEaseFactor for deck with no progress", () => {
    const cards = [makeCard("a")];
    const stats = getDeckStats(cards, [], "deck-1", NOW);
    expect(stats.averageEaseFactor).toBe(0);
  });

  it("computes averageEaseFactor correctly", () => {
    const cards = [makeCard("a"), makeCard("b")];
    const progress = [
      makeProgress("a", { easeFactor: 2.0 }),
      makeProgress("b", { easeFactor: 3.0 }),
    ];
    const stats = getDeckStats(cards, progress, "deck-1", NOW);
    expect(stats.averageEaseFactor).toBeCloseTo(2.5);
  });

  it("total equals number of deck cards", () => {
    const cards = Array.from({ length: 50 }, (_, i) => makeCard(`c${i}`));
    const stats = getDeckStats(cards, [], "deck-1", NOW);
    expect(stats.total).toBe(50);
  });

  it("only counts cards from the specified deck", () => {
    const cards = [makeCard("a", "deck-1"), makeCard("b", "deck-2")];
    const stats = getDeckStats(cards, [], "deck-1", NOW);
    expect(stats.total).toBe(1);
  });
});
