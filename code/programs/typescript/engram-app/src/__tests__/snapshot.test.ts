import { describe, expect, it } from "vitest";
import {
  createEngramSnapshot,
  ENGRAM_SNAPSHOT_VERSION,
  parseEngramSnapshot,
} from "../snapshot.js";
import type { AppState } from "../types.js";

const state: AppState = {
  decks: [
    {
      id: "deck",
      name: "Tamil",
      description: "Script",
      createdAt: 1,
    },
  ],
  cards: [
    {
      id: "card",
      deckId: "deck",
      front: "letter-a",
      back: "a",
      createdAt: 2,
    },
  ],
  cardProgress: [],
  sessions: [],
  reviews: [],
  activeSession: {
    sessionId: "session",
    deckId: "deck",
    queue: [],
    currentIndex: 0,
    revealed: false,
  },
};

describe("Engram snapshots", () => {
  it("exports only durable collection state", () => {
    const snapshot = createEngramSnapshot(state, 123);

    expect(snapshot.app).toBe("engram");
    expect(snapshot.version).toBe(ENGRAM_SNAPSHOT_VERSION);
    expect(snapshot.exportedAt).toBe(123);
    expect(snapshot.decks).toHaveLength(1);
    expect("activeSession" in snapshot).toBe(false);
  });

  it("round-trips valid snapshot JSON", () => {
    const snapshot = createEngramSnapshot(state, 123);
    const restored = parseEngramSnapshot(JSON.stringify(snapshot));

    expect(restored.decks[0]!.id).toBe("deck");
    expect(restored.cards[0]!.front).toBe("letter-a");
  });

  it("rejects invalid JSON", () => {
    expect(() => parseEngramSnapshot("{not-json")).toThrow("not valid JSON");
  });

  it("rejects non-Engram JSON", () => {
    expect(() => parseEngramSnapshot(JSON.stringify({ app: "other" }))).toThrow(
      "not an Engram backup",
    );
  });

  it("rejects unsupported versions", () => {
    const snapshot = createEngramSnapshot(state, 123);
    expect(() =>
      parseEngramSnapshot(JSON.stringify({ ...snapshot, version: 99 })),
    ).toThrow("Unsupported Engram backup version");
  });
});
