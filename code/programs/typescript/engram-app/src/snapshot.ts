import type { AppState } from "./types.js";

export const ENGRAM_SNAPSHOT_VERSION = 1;

export interface EngramSnapshot {
  app: "engram";
  version: typeof ENGRAM_SNAPSHOT_VERSION;
  exportedAt: number;
  decks: AppState["decks"];
  cards: AppState["cards"];
  cardProgress: AppState["cardProgress"];
  sessions: AppState["sessions"];
  reviews: AppState["reviews"];
}

export type RestoredEngramState = Pick<
  AppState,
  "decks" | "cards" | "cardProgress" | "sessions" | "reviews"
>;

export function createEngramSnapshot(
  state: AppState,
  exportedAt = Date.now(),
): EngramSnapshot {
  return {
    app: "engram",
    version: ENGRAM_SNAPSHOT_VERSION,
    exportedAt,
    decks: [...state.decks],
    cards: [...state.cards],
    cardProgress: [...state.cardProgress],
    sessions: [...state.sessions],
    reviews: [...state.reviews],
  };
}

export function parseEngramSnapshot(text: string): RestoredEngramState {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("The selected file is not valid JSON.");
  }

  if (!isRecord(parsed) || parsed.app !== "engram") {
    throw new Error("The selected file is not an Engram backup.");
  }
  if (parsed.version !== ENGRAM_SNAPSHOT_VERSION) {
    throw new Error(
      `Unsupported Engram backup version: ${String(parsed.version)}`,
    );
  }

  assertArray(parsed.decks, "decks");
  assertArray(parsed.cards, "cards");
  assertArray(parsed.cardProgress, "cardProgress");
  assertArray(parsed.sessions, "sessions");
  assertArray(parsed.reviews, "reviews");

  return {
    decks: parsed.decks as AppState["decks"],
    cards: parsed.cards as AppState["cards"],
    cardProgress: parsed.cardProgress as AppState["cardProgress"],
    sessions: parsed.sessions as AppState["sessions"],
    reviews: parsed.reviews as AppState["reviews"],
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function assertArray(
  value: unknown,
  fieldName: keyof RestoredEngramState,
): asserts value is unknown[] {
  if (!Array.isArray(value)) {
    throw new Error(`Engram backup is missing "${fieldName}".`);
  }
}
