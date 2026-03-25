import { describe, it, expect } from "vitest";
import {
  updateCardProgress,
  createInitialProgress,
  INITIAL_EASE_FACTOR,
  MIN_EASE_FACTOR,
  MAX_EASE_FACTOR,
} from "../sm2.js";
import type { CardProgress } from "../types.js";

// ── Helpers ─────────────────────────────────────────────────────────────────

function makeProgress(overrides: Partial<CardProgress> = {}): CardProgress {
  return {
    cardId: "card-1",
    interval: 1,
    easeFactor: INITIAL_EASE_FACTOR,
    nextDueAt: 0,
    timesSeen: 0,
    timesCorrect: 0,
    timesIncorrect: 0,
    lastSeenAt: 0,
    ...overrides,
  };
}

const NOW = 1_700_000_000_000; // fixed timestamp for deterministic tests
const ONE_DAY_MS = 24 * 60 * 60 * 1000;

// ── createInitialProgress ───────────────────────────────────────────────────

describe("createInitialProgress", () => {
  it("creates a record with the given cardId", () => {
    const p = createInitialProgress("card-42", "good", NOW);
    expect(p.cardId).toBe("card-42");
  });

  it("starts timesSeen at 1 after creation", () => {
    const p = createInitialProgress("c", "good", NOW);
    expect(p.timesSeen).toBe(1);
  });

  it("good rating: timesCorrect=1, timesIncorrect=0", () => {
    const p = createInitialProgress("c", "good", NOW);
    expect(p.timesCorrect).toBe(1);
    expect(p.timesIncorrect).toBe(0);
  });

  it("again rating: timesCorrect=0, timesIncorrect=1", () => {
    const p = createInitialProgress("c", "again", NOW);
    expect(p.timesCorrect).toBe(0);
    expect(p.timesIncorrect).toBe(1);
  });

  it("sets lastSeenAt to now", () => {
    const p = createInitialProgress("c", "good", NOW);
    expect(p.lastSeenAt).toBe(NOW);
  });
});

// ── updateCardProgress — again ───────────────────────────────────────────────

describe("updateCardProgress — again (score=0)", () => {
  it("resets interval to 1", () => {
    const p = makeProgress({ interval: 10 });
    const next = updateCardProgress(p, "again", NOW);
    expect(next.interval).toBe(1);
  });

  it("decreases easeFactor by 0.20", () => {
    const p = makeProgress({ easeFactor: 2.5 });
    const next = updateCardProgress(p, "again", NOW);
    expect(next.easeFactor).toBeCloseTo(2.3);
  });

  it("clamps easeFactor to MIN_EASE_FACTOR", () => {
    const p = makeProgress({ easeFactor: 1.4 });
    const next = updateCardProgress(p, "again", NOW);
    expect(next.easeFactor).toBe(MIN_EASE_FACTOR);
  });

  it("sets nextDueAt to now + 1 day", () => {
    const p = makeProgress();
    const next = updateCardProgress(p, "again", NOW);
    expect(next.nextDueAt).toBe(NOW + ONE_DAY_MS);
  });

  it("increments timesIncorrect", () => {
    const p = makeProgress({ timesIncorrect: 2 });
    const next = updateCardProgress(p, "again", NOW);
    expect(next.timesIncorrect).toBe(3);
  });

  it("does not increment timesCorrect", () => {
    const p = makeProgress({ timesCorrect: 3 });
    const next = updateCardProgress(p, "again", NOW);
    expect(next.timesCorrect).toBe(3);
  });

  it("increments timesSeen", () => {
    const p = makeProgress({ timesSeen: 4 });
    const next = updateCardProgress(p, "again", NOW);
    expect(next.timesSeen).toBe(5);
  });

  it("does not mutate the input record", () => {
    const p = makeProgress({ interval: 5 });
    updateCardProgress(p, "again", NOW);
    expect(p.interval).toBe(5);
  });
});

// ── updateCardProgress — hard ────────────────────────────────────────────────

describe("updateCardProgress — hard (score=1)", () => {
  it("multiplies interval by 1.2 (rounded)", () => {
    const p = makeProgress({ interval: 5 });
    const next = updateCardProgress(p, "hard", NOW);
    expect(next.interval).toBe(Math.round(5 * 1.2)); // 6
  });

  it("interval floor is 1", () => {
    const p = makeProgress({ interval: 1 });
    const next = updateCardProgress(p, "hard", NOW);
    expect(next.interval).toBeGreaterThanOrEqual(1);
  });

  it("decreases easeFactor by 0.15", () => {
    const p = makeProgress({ easeFactor: 2.5 });
    const next = updateCardProgress(p, "hard", NOW);
    expect(next.easeFactor).toBeCloseTo(2.35);
  });

  it("clamps easeFactor to MIN_EASE_FACTOR", () => {
    const p = makeProgress({ easeFactor: 1.3 });
    const next = updateCardProgress(p, "hard", NOW);
    expect(next.easeFactor).toBe(MIN_EASE_FACTOR);
  });

  it("increments timesCorrect (score > 0)", () => {
    const p = makeProgress({ timesCorrect: 1 });
    const next = updateCardProgress(p, "hard", NOW);
    expect(next.timesCorrect).toBe(2);
  });

  it("does not increment timesIncorrect", () => {
    const p = makeProgress({ timesIncorrect: 1 });
    const next = updateCardProgress(p, "hard", NOW);
    expect(next.timesIncorrect).toBe(1);
  });
});

// ── updateCardProgress — good ────────────────────────────────────────────────

describe("updateCardProgress — good (score=2)", () => {
  it("multiplies interval by easeFactor (rounded)", () => {
    const p = makeProgress({ interval: 4, easeFactor: 2.5 });
    const next = updateCardProgress(p, "good", NOW);
    expect(next.interval).toBe(Math.round(4 * 2.5)); // 10
  });

  it("interval floor is 1", () => {
    const p = makeProgress({ interval: 1, easeFactor: 1.3 });
    const next = updateCardProgress(p, "good", NOW);
    expect(next.interval).toBeGreaterThanOrEqual(1);
  });

  it("does not change easeFactor", () => {
    const p = makeProgress({ easeFactor: 2.5 });
    const next = updateCardProgress(p, "good", NOW);
    expect(next.easeFactor).toBe(2.5);
  });

  it("increments timesCorrect", () => {
    const p = makeProgress({ timesCorrect: 5 });
    const next = updateCardProgress(p, "good", NOW);
    expect(next.timesCorrect).toBe(6);
  });

  it("sets nextDueAt to now + interval days", () => {
    const p = makeProgress({ interval: 3 });
    const next = updateCardProgress(p, "good", NOW);
    expect(next.nextDueAt).toBe(NOW + next.interval * ONE_DAY_MS);
  });
});

// ── updateCardProgress — easy ────────────────────────────────────────────────

describe("updateCardProgress — easy (score=3)", () => {
  it("multiplies interval by easeFactor × 1.3 (rounded)", () => {
    const p = makeProgress({ interval: 4, easeFactor: 2.5 });
    const next = updateCardProgress(p, "easy", NOW);
    expect(next.interval).toBe(Math.round(4 * 2.5 * 1.3)); // 13
  });

  it("increases easeFactor by 0.15", () => {
    const p = makeProgress({ easeFactor: 2.5 });
    const next = updateCardProgress(p, "easy", NOW);
    expect(next.easeFactor).toBeCloseTo(2.65);
  });

  it("clamps easeFactor to MAX_EASE_FACTOR", () => {
    const p = makeProgress({ easeFactor: 3.95 });
    const next = updateCardProgress(p, "easy", NOW);
    expect(next.easeFactor).toBe(MAX_EASE_FACTOR);
  });

  it("increments timesCorrect", () => {
    const p = makeProgress({ timesCorrect: 2 });
    const next = updateCardProgress(p, "easy", NOW);
    expect(next.timesCorrect).toBe(3);
  });
});

// ── General invariants ───────────────────────────────────────────────────────

describe("updateCardProgress — invariants", () => {
  const ratings = ["again", "hard", "good", "easy"] as const;

  for (const rating of ratings) {
    it(`always increments timesSeen for rating=${rating}`, () => {
      const p = makeProgress({ timesSeen: 7 });
      const next = updateCardProgress(p, rating, NOW);
      expect(next.timesSeen).toBe(8);
    });

    it(`always sets lastSeenAt to now for rating=${rating}`, () => {
      const p = makeProgress();
      const next = updateCardProgress(p, rating, NOW);
      expect(next.lastSeenAt).toBe(NOW);
    });

    it(`easeFactor never goes below MIN for rating=${rating}`, () => {
      const p = makeProgress({ easeFactor: 1.3 });
      const next = updateCardProgress(p, rating, NOW);
      expect(next.easeFactor).toBeGreaterThanOrEqual(MIN_EASE_FACTOR);
    });

    it(`interval is always >= 1 for rating=${rating}`, () => {
      const p = makeProgress({ interval: 1 });
      const next = updateCardProgress(p, rating, NOW);
      expect(next.interval).toBeGreaterThanOrEqual(1);
    });

    it(`cardId is preserved for rating=${rating}`, () => {
      const p = makeProgress({ cardId: "preserved-id" });
      const next = updateCardProgress(p, rating, NOW);
      expect(next.cardId).toBe("preserved-id");
    });
  }
});
